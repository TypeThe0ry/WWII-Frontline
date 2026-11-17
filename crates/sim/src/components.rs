//! ECS components and resources.
//!
//! Determinism checklist (review rule, ARCHITECTURE 8.1): code that influences simulation
//! state must never use a bare `Query::iter()`. Iterate [`EntityTable::sorted`] (ascending
//! `EntityId`) and use `Query::get`/`get_mut` instead. `check-sim-purity` covers banned
//! std APIs; this rule is enforced by review.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use frontline_protocol::msg::input::InputCmd;
use glam::Vec3;

use crate::collision::CollisionData;
use crate::ids::{EntityId, PlayerId, Tick};
use crate::movement::{MoveParams, PlayerAuthState, Stance};

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Player {
    pub id: PlayerId,
}

/// Stable simulation id of an entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimId(pub EntityId);

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub pos: Vec3,
    /// Radians, `[0, 2pi)`.
    pub yaw: f32,
    /// Radians, `[-pi/2, pi/2]`.
    pub pitch: f32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastInput {
    pub cmd: InputCmd,
}

/// Faction side (data files use lowercase `us` / `ger`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Team {
    Us = 0,
    Ger = 1,
}

/// Current tick; incremented at the start of every `step`.
#[derive(Resource, Debug, Clone, Copy)]
pub struct CurrentTick(pub Tick);

/// This tick's inputs, ascending by `PlayerId`.
#[derive(Resource, Debug, Default)]
pub struct InputQueue(pub Vec<(PlayerId, InputCmd)>);

/// `EntityId -> Entity`, ordered by `EntityId`.
#[derive(Resource, Debug, Default)]
pub struct EntityTable {
    map: BTreeMap<EntityId, Entity>,
}

impl EntityTable {
    pub fn insert(&mut self, id: EntityId, e: Entity) {
        self.map.insert(id, e);
    }
    pub fn remove(&mut self, id: EntityId) -> Option<Entity> {
        self.map.remove(&id)
    }
    pub fn get(&self, id: EntityId) -> Option<Entity> {
        self.map.get(&id).copied()
    }
    /// All entities in ascending `EntityId` order (the only sanctioned iteration order).
    pub fn sorted(&self) -> impl Iterator<Item = (EntityId, Entity)> + '_ {
        self.map.iter().map(|(k, v)| (*k, *v))
    }
}

/// `PlayerId -> (EntityId, Entity)`, ordered by `PlayerId`.
#[derive(Resource, Debug, Default)]
pub struct PlayerIndex {
    pub map: BTreeMap<PlayerId, (EntityId, Entity)>,
}

/// Match-level RNG state. Streams are derived per draw ([`crate::rng::Rng::for_`]); this
/// resource holds the seed and counts draws so the stream position is part of the hash.
#[derive(Resource, Debug)]
pub struct SimRng {
    pub match_seed: u64,
    pub draws: u64,
}

impl SimRng {
    pub fn stream(
        &mut self,
        tick: Tick,
        entity: EntityId,
        purpose: crate::rng::Purpose,
    ) -> crate::rng::Pcg64Mcg {
        self.draws += 1;
        crate::rng::Rng::for_(self.match_seed, tick, entity, purpose)
    }
}

/// Test-only switch (feature `test-util`): enables the RNG jitter system.
#[derive(Resource, Debug, Default)]
pub struct TestJitter(pub bool);

/// Movement state besides the pose (`Transform`): together they form a [`PlayerAuthState`].
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    pub vel: Vec3,
    pub stance: Stance,
    pub stance_blend: f32,
    pub stance_timer: u8,
    pub grounded: bool,
    pub blocked: bool,
}

impl Motion {
    pub fn from_auth(a: &PlayerAuthState) -> Self {
        Self {
            vel: a.vel,
            stance: a.stance,
            stance_blend: a.stance_blend,
            stance_timer: a.stance_timer,
            grounded: a.grounded,
            blocked: a.blocked,
        }
    }

    pub fn to_auth(&self, t: &Transform) -> PlayerAuthState {
        PlayerAuthState {
            pos: t.pos,
            vel: self.vel,
            yaw: t.yaw,
            pitch: t.pitch,
            stance: self.stance,
            stance_blend: self.stance_blend,
            stance_timer: self.stance_timer,
            grounded: self.grounded,
            blocked: self.blocked,
        }
    }
}

/// Movement parameters of the (single, M0) role every player uses; `None` disables movement.
#[derive(Resource, Debug, Clone, Copy)]
pub struct MoveParamsRes(pub Option<MoveParams>);

/// Terrain collision (owned here so that systems can query it).
#[derive(Resource)]
pub struct CollisionRes(pub CollisionData);
