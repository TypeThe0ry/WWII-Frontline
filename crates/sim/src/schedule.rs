//! The single-threaded system schedule. Order is fixed with `.chain()`:
//! `ingest_inputs -> movement -> weapons -> projectiles -> bookkeeping`.

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ExecutorKind;
use frontline_protocol::quant::{dequant_yaw, pitch_i16_to_rad, quant_yaw};
use glam::Vec3;

use crate::components::{
    CollisionRes, CurrentTick, EntityTable, EventBuf, Home, InputQueue, LastInput, MeleeTable,
    Motion, MoveParamsRes, Player, PlayerIndex, RespawnPolicyRes, SimCfgRes, SimRng, Team,
    Transform, WeaponTable,
};
use crate::health::HitRec;
use crate::health::{Health, HitLog, LifeState};
use crate::movement::{step_player_ex, MoveCtx};
use crate::projectile::{projectile_system as projectiles_system, Projectiles};
use crate::respawn::{respawn_time, RespawnQueue, SpawnPoints};
use crate::rng::{unit_f32, Purpose};
use crate::step::SimEvent;
use crate::weapon::melee::{melee_damage, melee_trace, step_melee, MELEE_WEAPON_IDX};
use crate::weapon::{is_sprinting, step_weapon, MeleeState, ShotCtx, WeaponState};
use crate::world::{neutral_input, DEFAULT_ROLE};
use frontline_protocol::msg::input::buttons;

pub fn ingest_inputs(
    inputs: Res<InputQueue>,
    index: Res<PlayerIndex>,
    mut q: Query<(&mut Transform, &mut LastInput, &LifeState)>,
) {
    for (pid, cmd) in &inputs.0 {
        let Some((_, e)) = index.map.get(pid) else {
            continue; // input for an unknown player is ignored
        };
        if let Ok((mut t, mut last, life)) = q.get_mut(*e) {
            if !life.is_alive() {
                continue; // dead players take no input (M0-013)
            }
            t.yaw = dequant_yaw(cmd.yaw, 16);
            t.pitch = pitch_i16_to_rad(cmd.pitch);
            last.cmd = *cmd;
        }
    }
}

/// Moves every player with [`step_player`], ascending `EntityId` (ARCHITECTURE 6.10 stage 2).
/// A player without a command this tick repeats its last one (input buffering is M0-018).
pub fn movement(
    params: Res<MoveParamsRes>,
    collision: Res<CollisionRes>,
    table: Res<EntityTable>,
    mut q: Query<(
        &mut Transform,
        &mut Motion,
        &LastInput,
        &LifeState,
        Option<&WeaponState>,
    )>,
) {
    let Some(p) = params.0 else {
        return;
    };
    for (_, e) in table.sorted() {
        if let Ok((mut t, mut m, last, life, w)) = q.get_mut(e) {
            if !life.is_alive() {
                continue;
            }
            let ctx = MoveCtx {
                reloading: w.is_some_and(|w| w.fsm.is_reloading()),
                load_kg: None,
            };
            let next = step_player_ex(&m.to_auth(&t), &last.cmd, &p, &collision.0, &ctx);
            t.pos = next.pos;
            t.yaw = next.yaw;
            t.pitch = next.pitch;
            *m = Motion::from_auth(&next);
        }
    }
}

/// Weapon stage (ARCHITECTURE 6.10 stage 4a): fire state machine, reload, recoil; spawns the
/// projectiles of this tick's shots. Uses the post-movement pose, ascending `EntityId`.
#[allow(clippy::too_many_arguments)]
pub fn weapons(
    wt: Res<WeaponTable>,
    params: Res<MoveParamsRes>,
    tick: Res<CurrentTick>,
    rng: Res<SimRng>,
    table: Res<EntityTable>,
    mut projs: ResMut<Projectiles>,
    mut events: ResMut<EventBuf>,
    mut q: Query<(
        &Transform,
        &Motion,
        &LastInput,
        &Team,
        &mut WeaponState,
        &LifeState,
        &MeleeState,
    )>,
) {
    let Some(mp) = params.0 else {
        return;
    };
    for (id, e) in table.sorted() {
        let Ok((t, m, last, team, mut w, life, melee)) = q.get_mut(e) else {
            continue;
        };
        if !life.is_alive() {
            continue;
        }
        let Some(wp) = wt.params(w.def_id) else {
            continue;
        };
        let ctx = ShotCtx {
            match_seed: rng.match_seed,
            tick: tick.0,
            entity: id,
        };
        let mut cmd = last.cmd;
        if !melee.is_idle() {
            // A running melee action blocks the trigger (M1-013b).
            cmd.buttons &= !buttons::FIRE;
        }
        let out = step_weapon(&mut w, wp, &cmd, &m.to_auth(t), &mp, ctx);
        if let Some(shot) = out.shot {
            let weapon = w.def_id;
            let v = wt.ballistics(weapon).map_or(0.0, |b| b.muzzle_velocity_mps);
            projs.spawn(id, *team, weapon, shot.muzzle, shot.dir * v);
            events.0.push(SimEvent::Shot {
                shooter: id,
                weapon,
                muzzle: shot.muzzle,
                dir: shot.dir,
            });
        }
        if out.dry_fire {
            events.0.push(SimEvent::DryFire { shooter: id });
        }
    }
}

/// Melee stage (M1-013b): advances every attacker's windup/cooldown and resolves strikes against
/// the post-movement poses, ascending `EntityId`; kills are attributed through the [`HitLog`].
#[allow(clippy::type_complexity)]
pub fn melee(
    mt: Res<MeleeTable>,
    table: Res<EntityTable>,
    mut log: ResMut<HitLog>,
    mut events: ResMut<EventBuf>,
    mut q: Query<(
        &Transform,
        &Motion,
        &LastInput,
        &Team,
        &LifeState,
        &mut MeleeState,
        &mut Health,
    )>,
) {
    struct Snap {
        id: crate::ids::EntityId,
        pos: Vec3,
        yaw: f32,
        team: Team,
    }
    let mut strikes: Vec<(crate::ids::EntityId, bool)> = Vec::new();
    for (id, e) in table.sorted() {
        let Ok((_, m, last, team, life, mut ms, _)) = q.get_mut(e) else {
            continue;
        };
        let Some(p) = mt.params(*team) else {
            continue;
        };
        if !life.is_alive() {
            *ms = MeleeState::default();
            continue;
        }
        let want = last.cmd.buttons & buttons::MELEE != 0;
        let sprinting = is_sprinting(&last.cmd);
        let out = step_melee(&mut ms, p, want, m.vault.is_none(), sprinting);
        if out.strike {
            strikes.push((id, ms.lunging));
        }
    }
    if strikes.is_empty() {
        return;
    }
    // Snapshot of the living players at strike time (poses do not change inside this stage).
    let snaps: Vec<Snap> = table
        .sorted()
        .filter_map(|(id, e)| {
            let (t, _, _, team, life, _, h) = q.get(e).ok()?;
            (life.is_alive() && h.hp > 0.0).then_some(Snap {
                id,
                pos: t.pos,
                yaw: t.yaw,
                team: *team,
            })
        })
        .collect();
    for (aid, lunging) in strikes {
        let Some(a) = snaps.iter().find(|s| s.id == aid) else {
            continue;
        };
        let Some(p) = mt.params(a.team) else {
            continue;
        };
        for tg in snaps.iter().filter(|s| s.id != aid && s.team != a.team) {
            let Some(hit) = melee_trace(p, lunging, a.pos, a.yaw, tg.pos, tg.yaw) else {
                continue;
            };
            let Some(e) = table.get(tg.id) else {
                continue;
            };
            let Ok((_, _, _, _, _, _, mut hp)) = q.get_mut(e) else {
                continue;
            };
            if hp.hp <= 0.0 {
                continue; // already killed by an earlier strike this tick
            }
            let dmg = melee_damage(p, &hit, hp.hp);
            hp.hp -= dmg;
            events.0.push(SimEvent::MeleeHit {
                attacker: aid,
                target: tg.id,
                damage_hp: dmg,
                backstab: hit.backstab,
            });
            log.0.push(HitRec {
                target: tg.id,
                shooter: aid,
                weapon: MELEE_WEAPON_IDX,
                zone: crate::weapon::HitZone::Torso,
                dist_m: hit.dist_m,
            });
        }
    }
}

/// Rules stage, deaths (ARCHITECTURE 6.10 stage 5): turns the lethal hits of this tick into
/// [`LifeState::Dead`] + [`SimEvent::Kill`] and schedules the respawn. Attribution is the last
/// damager (the one whose hit took `hp` to 0 or below); GAMEDESIGN 7.5 assists are M1-016.
#[allow(clippy::too_many_arguments)]
pub fn deaths(
    tick: Res<CurrentTick>,
    cfg: Res<SimCfgRes>,
    policy: Res<RespawnPolicyRes>,
    table: Res<EntityTable>,
    mut log: ResMut<HitLog>,
    mut queue: ResMut<RespawnQueue>,
    mut events: ResMut<EventBuf>,
    mut q: Query<(&Player, &mut Health, &mut LifeState)>,
) {
    if log.0.is_empty() {
        return;
    }
    for rec in std::mem::take(&mut log.0) {
        let Some(victim_e) = table.get(rec.target) else {
            continue;
        };
        let killer = table
            .get(rec.shooter)
            .and_then(|e| q.get(e).ok())
            .map(|(p, _, _)| p.id);
        let Ok((vp, mut hp, mut life)) = q.get_mut(victim_e) else {
            continue;
        };
        if !life.is_alive() || hp.hp > 0.0 {
            continue;
        }
        hp.hp = 0.0;
        let t_kia = tick.0;
        let respawn_tick =
            respawn_time(t_kia, policy.0.pick_tick(t_kia, cfg.hz), &policy.0, cfg.hz);
        let weapon = rec.weapon as u8;
        let headshot = rec.zone == crate::weapon::HitZone::Head;
        *life = LifeState::Dead {
            t_kia,
            killer,
            weapon,
            headshot,
            respawn_tick,
        };
        queue.schedule(vp.id, respawn_tick);
        events.0.push(SimEvent::Kill {
            victim: vp.id,
            killer,
            weapon,
            headshot,
            dist_m: rec.dist_m,
        });
    }
}

/// Rules stage, respawns: players whose spawn tick has come re-enter at their team's spawn point
/// (plus a deterministic offset of at most `spawn_jitter_m`), full health and weapon, facing the
/// map centre. No spawn protection in M0.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn respawns(
    tick: Res<CurrentTick>,
    cfg: Res<SimCfgRes>,
    points: Res<SpawnPoints>,
    policy: Res<RespawnPolicyRes>,
    col: Res<CollisionRes>,
    index: Res<PlayerIndex>,
    mut rng: ResMut<SimRng>,
    mut queue: ResMut<RespawnQueue>,
    mut events: ResMut<EventBuf>,
    mut q: Query<(
        &mut Transform,
        &mut Motion,
        &mut Health,
        &mut LifeState,
        &mut LastInput,
        &Team,
        &Home,
        Option<&mut WeaponState>,
    )>,
) {
    if queue.is_empty() {
        return;
    }
    for pid in queue.pop_due(tick.0) {
        let Some(&(eid, e)) = index.map.get(&pid) else {
            continue;
        };
        let Ok((mut t, mut m, mut hp, mut life, mut last, team, home, weapon)) = q.get_mut(e)
        else {
            continue;
        };
        if life.is_alive() {
            continue;
        }
        let base = points.team(*team).unwrap_or(home.pos);
        let mut r = rng.stream(tick.0, eid, Purpose::Respawn);
        let ang = unit_f32(&mut r) * core::f32::consts::TAU;
        let rad = policy.0.spawn_jitter_m * libm::sqrtf(unit_f32(&mut r));
        let x = base.x + rad * libm::cosf(ang);
        let z = base.z + rad * libm::sinf(ang);
        let y = col.0.height_at(x, z).map_or(base.y, |h| h + 0.02);
        let pos = Vec3::new(x, y, z);
        let yaw = match points.center {
            Some((cx, cz)) if (cx - x).abs() + (cz - z).abs() > 1e-3 => {
                libm::atan2f(cx - x, cz - z)
            }
            _ => home.yaw,
        };
        let code = quant_yaw(yaw, 16).code;
        let yaw = dequant_yaw(code, 16);
        t.pos = pos;
        t.yaw = yaw;
        t.pitch = 0.0;
        *m = Motion::from_auth(&crate::movement::PlayerAuthState::spawn(pos, yaw));
        hp.hp = hp.max_hp;
        *life = LifeState::Alive;
        let mut idle = neutral_input();
        idle.yaw = code;
        last.cmd = idle;
        if let (Some(mut w), Some(fresh)) = (
            weapon,
            crate::components::starting_weapon(cfg.data, DEFAULT_ROLE, *team, cfg.hz),
        ) {
            *w = fresh;
        }
        events.0.push(SimEvent::Spawn { player: pid, pos });
    }
}

/// End-of-tick housekeeping. Currently a no-op slot (the view cache is refreshed by `World`).
pub use crate::throwables::system::throwables;

pub fn bookkeeping(_table: Res<EntityTable>) {}

/// Test-only: each tick, draw once per entity (ascending `EntityId`) and jitter the position.
#[cfg(feature = "test-util")]
pub fn test_jitter(
    switch: Res<crate::components::TestJitter>,
    tick: Res<crate::components::CurrentTick>,
    table: Res<EntityTable>,
    mut rng: ResMut<crate::components::SimRng>,
    mut q: Query<&mut Transform>,
) {
    use crate::rng::{unit_f32, Purpose};
    if !switch.0 {
        return;
    }
    for (id, e) in table.sorted() {
        let mut r = rng.stream(tick.0, id, Purpose::Misc);
        let dx = unit_f32(&mut r) - 0.5;
        if let Ok(mut t) = q.get_mut(e) {
            t.pos.x += dx;
            t.pos.z -= dx * 0.5;
        }
    }
}

pub fn build_schedule() -> Schedule {
    let mut s = Schedule::default();
    s.set_executor_kind(ExecutorKind::SingleThreaded);
    #[cfg(feature = "test-util")]
    s.add_systems(
        (
            ingest_inputs,
            test_jitter,
            movement,
            melee,
            weapons,
            throwables,
            projectiles_system,
            deaths,
            respawns,
            bookkeeping,
        )
            .chain(),
    );
    #[cfg(not(feature = "test-util"))]
    s.add_systems(
        (
            ingest_inputs,
            movement,
            melee,
            weapons,
            throwables,
            projectiles_system,
            deaths,
            respawns,
            bookkeeping,
        )
            .chain(),
    );
    s
}
