//! HQ spawn rules (GAMEDESIGN 10.2, 10.6, 10.8; M1-019a): spawn timing penalties, spawn
//! protection, spawn placement with anti spawn-kill, and the HQ protection zone.
//!
//! Everything here is opt-in: it only runs once a HQ was registered with
//! [`crate::World::set_hq`], so the M0 respawn behaviour (and its goldens) is unchanged.
//! Constants live here, not in `/data` (PI-060); the wave / base wait come from
//! `data/rules/respawn.json`.

use std::collections::BTreeMap;

use bevy_ecs::prelude::{Component, Resource};
use glam::Vec3;

use crate::components::Team;
use crate::ids::PlayerId;

pub mod garrison;
pub mod outpost;
pub mod place;
pub mod system;
pub mod timing;

/// HQ spawn area radius, metres (players spawn within this of the HQ; 8 m jitter in practice).
pub const HQ_SPAWN_RADIUS_M: f32 = 40.0;
/// Spawn jitter around the chosen spawn point, metres.
pub const SPAWN_JITTER_M: f32 = 8.0;
/// Enemy protection zone radius around a HQ, metres.
pub const HQ_ZONE_RADIUS_M: f32 = 150.0;
/// Seconds an enemy may stay in the zone before it takes damage.
pub const HQ_ZONE_WARN_S: u32 = 3;
/// Zone damage per second after the warning.
pub const HQ_ZONE_DPS: f32 = 40.0;
/// Spawn protection duration, seconds (`spawn.json spawn_protection_s`).
pub const PROTECTION_S: u32 = 3;
/// Dying again within this many seconds of spawning adds [`SUICIDE_PENALTY_S`].
pub const SUICIDE_WINDOW_S: u32 = 20;
pub const SUICIDE_PENALTY_S: u32 = 5;
/// 8 or more spawns at the same point within this window add [`FLOOD_PENALTY_S`].
pub const FLOOD_WINDOW_S: u32 = 10;
pub const FLOOD_COUNT: usize = 8;
pub const FLOOD_PENALTY_S: u32 = 2;
/// An enemy this close with a line of sight makes a spawn position unsafe.
pub const SPAWN_KILL_RADIUS_M: f32 = 10.0;
/// Candidates tried per spawn.
pub const SPAWN_TRIES: usize = 8;
/// Minimum distance between a spawned player and another one, metres.
pub const SPAWN_CLEARANCE_M: f32 = 0.8;

/// Spawn protection (damage x0) until this tick (exclusive); 0 = none. Cancelled early by
/// moving or shooting.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpawnProtection {
    pub until: u32,
}

/// Which kind of spawn point (M1-019b adds Garrison).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnKind {
    Hq,
    Garrison,
    Outpost,
}

/// HQ spawn state of the match.
#[derive(Resource, Debug, Clone, Default)]
pub struct SpawnRules {
    /// HQ position per team (`Team as usize`).
    pub hq: [Option<Vec3>; 2],
    /// Tick of each player's last spawn (suicide penalty).
    pub last_spawn: BTreeMap<PlayerId, u32>,
    /// Spawn ticks per team's HQ in the flood window.
    pub recent: [Vec<u32>; 2],
    /// Tick an enemy first stood in a HQ zone.
    pub zone_since: BTreeMap<PlayerId, u32>,
    /// Garrison each dead player picked (`World::request_spawn`).
    pub choice: BTreeMap<PlayerId, u16>,
    /// `respawn.json` base wait, ms (restored after the warm-up halving).
    pub base_wait_ms: Option<u32>,
}

impl SpawnRules {
    pub fn active(&self) -> bool {
        self.hq.iter().any(Option::is_some)
    }

    pub fn hq_of(&self, team: Team) -> Option<Vec3> {
        self.hq[team as usize]
    }
}
