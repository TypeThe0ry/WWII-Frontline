//! The single-threaded system schedule. Order is fixed with `.chain()`:
//! `ingest_inputs -> movement -> weapons -> projectiles -> bookkeeping`.

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ExecutorKind;
use frontline_protocol::quant::{dequant_yaw, pitch_i16_to_rad};

use crate::components::{
    CollisionRes, CurrentTick, EntityTable, EventBuf, InputQueue, LastInput, Motion, MoveParamsRes,
    PlayerIndex, SimRng, Team, Transform, WeaponTable,
};
use crate::movement::step_player;
use crate::projectile::{projectile_system as projectiles_system, Projectiles};
use crate::step::SimEvent;
use crate::weapon::{step_weapon, ShotCtx, WeaponState};

pub fn ingest_inputs(
    inputs: Res<InputQueue>,
    index: Res<PlayerIndex>,
    mut q: Query<(&mut Transform, &mut LastInput)>,
) {
    for (pid, cmd) in &inputs.0 {
        let Some((_, e)) = index.map.get(pid) else {
            continue; // input for an unknown player is ignored
        };
        if let Ok((mut t, mut last)) = q.get_mut(*e) {
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
    mut q: Query<(&mut Transform, &mut Motion, &LastInput)>,
) {
    let Some(p) = params.0 else {
        return;
    };
    for (_, e) in table.sorted() {
        if let Ok((mut t, mut m, last)) = q.get_mut(e) {
            let next = step_player(&m.to_auth(&t), &last.cmd, &p, &collision.0);
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
    mut q: Query<(&Transform, &Motion, &LastInput, &Team, &mut WeaponState)>,
) {
    let Some(mp) = params.0 else {
        return;
    };
    for (id, e) in table.sorted() {
        let Ok((t, m, last, team, mut w)) = q.get_mut(e) else {
            continue;
        };
        let Some(wp) = wt.params(w.def_id) else {
            continue;
        };
        let ctx = ShotCtx {
            match_seed: rng.match_seed,
            tick: tick.0,
            entity: id,
        };
        let out = step_weapon(&mut w, wp, &last.cmd, &m.to_auth(t), &mp, ctx);
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

/// End-of-tick housekeeping. Currently a no-op slot (the view cache is refreshed by `World`).
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
            weapons,
            projectiles_system,
            bookkeeping,
        )
            .chain(),
    );
    #[cfg(not(feature = "test-util"))]
    s.add_systems(
        (
            ingest_inputs,
            movement,
            weapons,
            projectiles_system,
            bookkeeping,
        )
            .chain(),
    );
    s
}
