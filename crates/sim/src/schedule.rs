//! The single-threaded system schedule. Order is fixed with `.chain()`:
//! `ingest_inputs -> movement -> (slot for later tasks: weapons, ...) -> bookkeeping`.

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ExecutorKind;
use frontline_protocol::quant::{dequant_yaw, pitch_i16_to_rad};

use crate::components::{
    CollisionRes, EntityTable, InputQueue, LastInput, Motion, MoveParamsRes, PlayerIndex, Transform,
};
use crate::movement::step_player;

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
    s.add_systems((ingest_inputs, test_jitter, movement, bookkeeping).chain());
    #[cfg(not(feature = "test-util"))]
    s.add_systems((ingest_inputs, movement, bookkeeping).chain());
    s
}
