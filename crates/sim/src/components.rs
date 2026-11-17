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
use crate::data::GameData;
use crate::ids::{EntityId, PlayerId, Tick};
use crate::movement::{MoveParams, PlayerAuthState, Stance};
use crate::step::SimEvent;
use crate::weapon::ballistics::BallisticsParams;
use crate::weapon::{WeaponParams, WeaponState};

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

pub use crate::health::{Health, LifeState};

/// Where a player joined (respawn fallback when its team has no spawn point).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Home {
    pub pos: Vec3,
    pub yaw: f32,
}

/// Respawn rule values of this match (`data/rules/respawn.json`).
#[derive(Resource, Debug, Clone, Copy)]
pub struct RespawnPolicyRes(pub crate::respawn::RespawnPolicy);

/// Events produced by systems during a step; drained into `StepOutput` by `World::step`.
#[derive(Resource, Debug, Default)]
pub struct EventBuf(pub Vec<SimEvent>);

/// Game data and tick rate for the systems.
#[derive(Resource, Clone, Copy)]
pub struct SimCfgRes {
    pub data: &'static GameData,
    pub hz: u8,
}

/// Per-weapon derived constants, indexed like `GameData::weapons`.
#[derive(Resource, Debug, Clone)]
pub struct WeaponTable {
    params: Vec<Option<WeaponParams>>,
    ballistics: Vec<BallisticsParams>,
}

impl WeaponTable {
    pub fn new(data: &GameData, hz: u8) -> Self {
        Self {
            params: data
                .weapons
                .iter()
                .map(|w| WeaponParams::from_def(w, hz).ok())
                .collect(),
            ballistics: data
                .weapons
                .iter()
                .map(BallisticsParams::from_def)
                .collect(),
        }
    }
    pub fn params(&self, weapon: u16) -> Option<&WeaponParams> {
        self.params.get(usize::from(weapon))?.as_ref()
    }
    pub fn ballistics(&self, weapon: u16) -> Option<BallisticsParams> {
        self.ballistics.get(usize::from(weapon)).copied()
    }
}

/// Spawns the player's starting weapon from the role's loadout of its team (M0: the single
/// default role, first loadout id; reserve = `ammo_mags - 1` clips: GAMEDESIGN 5.3 counts the one in the gun, PI-043).
pub fn starting_weapon(data: &GameData, role: &str, team: Team, hz: u8) -> Option<WeaponState> {
    let key = match team {
        Team::Us => "us",
        Team::Ger => "ger",
    };
    let lid = data
        .role(role)?
        .faction_overrides
        .get(key)?
        .loadout_ids
        .first()?;
    let lo = data.loadout(lid)?;
    let idx = data.weapons.iter().position(|w| w.id == lo.primary)?;
    let p = WeaponParams::from_def(&data.weapons[idx], hz).ok()?;
    let reserve = u32::from(p.mag_capacity) * lo.ammo_mags.saturating_sub(1);
    Some(WeaponState::new(
        idx as u16,
        p.mag_capacity,
        reserve.min(u32::from(u16::MAX)) as u16,
    ))
}
