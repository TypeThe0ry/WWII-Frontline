//! The single-threaded system schedule. Order is fixed with `.chain()`:
//! `ingest_inputs -> movement -> weapons -> projectiles -> bookkeeping`.

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ExecutorKind;
use frontline_protocol::quant::{dequant_yaw, pitch_i16_to_rad, quant_yaw};
use glam::Vec3;

use crate::components::{
    CollisionRes, CurrentTick, EntityTable, EventBuf, Home, InputQueue, LastInput, MeleeTable,
    Motion, MoveParamsRes, PlayerIndex, RespawnPolicyRes, SimCfgRes, SimRng, Team, Transform,
    WeaponTable,
};
use crate::health::HitRec;
use crate::health::{Health, HitLog, LifeState};
use crate::movement::{step_player_ex, MoveCtx};
use crate::projectile::{projectile_system as projectiles_system, Projectiles};
use crate::respawn::{RespawnQueue, SpawnPoints};
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
            if life.is_dead() {
                continue; // dead players take no input (M0-013); downed ones keep theirs (M1-016)
            }
            t.yaw = dequant_yaw(cmd.yaw, 16);
            t.pitch = pitch_i16_to_rad(cmd.pitch);
            last.cmd = *cmd;
        }
    }
}

/// Moves every player with [`step_player`], ascending `EntityId` (ARCHITECTURE 6.10 stage 2).
/// A player without a command this tick repeats its last one (input buffering is M0-018).
#[allow(clippy::type_complexity)]
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
        &MeleeState,
    )>,
) {
    let Some(p) = params.0 else {
        return;
    };
    for (_, e) in table.sorted() {
        if let Ok((mut t, mut m, last, life, w, melee)) = q.get_mut(e) {
            let mut cmd = last.cmd;
            if life.is_downed() {
                // Crawl (GAMEDESIGN 7.1): prone only, at the downed speed.
                let k = (p.crawl_mps / p.prone_mps).min(1.0);
                cmd.buttons = buttons::PRONE;
                cmd.move_x = (f32::from(cmd.move_x) * k) as i8;
                cmd.move_z = (f32::from(cmd.move_z) * k) as i8;
            } else if !life.is_alive() {
                continue;
            } else if melee.executing() {
                // The executioner stands still (GAMEDESIGN 7.4).
                cmd.move_x = 0;
                cmd.move_z = 0;
                cmd.buttons &= !(buttons::SPRINT | buttons::JUMP);
            }
            let ctx = MoveCtx {
                reloading: w.is_some_and(|w| w.fsm.is_reloading()),
                load_kg: None,
            };
            let next = step_player_ex(&m.to_auth(&t), &cmd, &p, &collision.0, &ctx);
            t.pos = next.pos;
            t.yaw = next.yaw;
            t.pitch = next.pitch;
            *m = Motion::from_auth(&next);
        }
    }
}

/// Weapon stage (ARCHITECTURE 6.10 stage 4a): fire state machine, reload, recoil; spawns the
/// projectiles of this tick's shots. Uses the post-movement pose, ascending `EntityId`.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
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
        &crate::medical::FireLock,
    )>,
) {
    let Some(mp) = params.0 else {
        return;
    };
    for (id, e) in table.sorted() {
        let Ok((t, m, last, team, mut w, life, melee, lock)) = q.get_mut(e) else {
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
        if !melee.is_idle() || tick.0 .0 < lock.0 .0 {
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

/// Melee stage (M1-013b, M1-016b): advances every attacker's windup/cooldown and resolves strikes
/// against the post-movement poses, ascending `EntityId`; a strike on a downed enemy starts an
/// execution (GAMEDESIGN 7.4) instead of damage. Kills are attributed through the [`HitLog`].
#[allow(clippy::type_complexity)]
pub fn melee(
    mt: Res<MeleeTable>,
    cfg: Res<SimCfgRes>,
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
        downed: bool,
    }
    let mut strikes: Vec<(crate::ids::EntityId, bool)> = Vec::new();
    // Executions in progress: (attacker, target) pairs that may finish this tick.
    let mut running: Vec<(crate::ids::EntityId, crate::ids::EntityId)> = Vec::new();
    for (id, e) in table.sorted() {
        let Ok((_, m, last, team, life, mut ms, hp)) = q.get_mut(e) else {
            continue;
        };
        let Some(p) = mt.params(*team) else {
            continue;
        };
        if !life.is_alive() {
            *ms = MeleeState::default();
            continue;
        }
        if ms.executing() {
            if hp.hp < ms.exec_hp - 0.01 {
                ms.exec_ticks = 0; // hit while executing: interrupted
            } else {
                running.push((id, ms.exec_target));
            }
        }
        let want = last.cmd.buttons & buttons::MELEE != 0;
        let sprinting = is_sprinting(&last.cmd);
        let can_start = m.vault.is_none() && !ms.executing();
        let out = step_melee(&mut ms, p, want, can_start, sprinting);
        if out.strike {
            strikes.push((id, ms.lunging));
        }
    }
    // Snapshot of the players who can be hit at strike time (poses do not change in this stage).
    let snaps: Vec<Snap> = table
        .sorted()
        .filter_map(|(id, e)| {
            let (t, _, _, team, life, _, h) = q.get(e).ok()?;
            (!life.is_dead() && h.hp > 0.0).then_some(Snap {
                id,
                pos: t.pos,
                yaw: t.yaw,
                team: *team,
                downed: life.is_downed(),
            })
        })
        .collect();
    // Progress the executions that were already running before this tick's strikes.
    for (aid, tid) in running {
        let (Some(a), Some(tg)) = (
            snaps.iter().find(|s| s.id == aid),
            snaps.iter().find(|s| s.id == tid),
        ) else {
            if let Some(e) = table.get(aid) {
                if let Ok((.., mut ms, _)) = q.get_mut(e) {
                    ms.exec_ticks = 0;
                }
            }
            continue;
        };
        let near = (a.pos - tg.pos).length() <= 2.5;
        let Some(ae) = table.get(aid) else { continue };
        let Ok((.., mut ms, _)) = q.get_mut(ae) else {
            continue;
        };
        if !tg.downed || !near {
            ms.exec_ticks = 0;
            continue;
        }
        ms.exec_ticks -= 1;
        if ms.exec_ticks == 0 {
            let Some(te) = table.get(tid) else { continue };
            let Ok((.., mut hp)) = q.get_mut(te) else {
                continue;
            };
            let dmg = hp.hp.max(1.0);
            hp.hp = 0.0;
            log.0.push(HitRec {
                target: tid,
                shooter: aid,
                weapon: MELEE_WEAPON_IDX,
                zone: crate::weapon::HitZone::Torso,
                dist_m: (a.pos - tg.pos).length(),
                damage: dmg,
                kind: crate::combat::DamageKind::Melee,
                direct_kia: false,
            });
        }
    }
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
            if tg.downed {
                // Strike on a downed enemy: start the execution, no damage (one at a time).
                let Some(ae) = table.get(aid) else { continue };
                let hp_now = q.get(ae).map_or(0.0, |r| r.6.hp);
                if let Ok((.., mut ms, _)) = q.get_mut(ae) {
                    if !ms.executing() {
                        ms.exec_target = tg.id;
                        ms.exec_ticks = ((crate::weapon::melee::EXECUTION_S * f32::from(cfg.hz))
                            .ceil() as u16)
                            .max(1);
                        ms.exec_hp = hp_now;
                    }
                }
                break;
            }
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
                damage: dmg,
                kind: crate::combat::DamageKind::Melee,
                direct_kia: hit.backstab,
            });
        }
    }
}

pub use crate::medical::{deaths, falls, medical, vitals};

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
        if !life.is_dead() {
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
            falls,
            melee,
            weapons,
            throwables,
            projectiles_system,
            crate::suppression::suppression,
            deaths,
            crate::suppression::suppression_assists,
            medical,
            vitals,
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
            falls,
            melee,
            weapons,
            throwables,
            projectiles_system,
            crate::suppression::suppression,
            deaths,
            crate::suppression::suppression_assists,
            medical,
            vitals,
            respawns,
            bookkeeping,
        )
            .chain(),
    );
    s
}
