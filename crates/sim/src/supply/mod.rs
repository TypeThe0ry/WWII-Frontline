//! Supplies (GAMEDESIGN 12; M1-020): Supply Crates, the HQ Depot and point supply stores,
//! carrying, and the [`SupplyProvider`](crate::supply_iface::SupplyProvider) Garrison building
//! pays from. (Ammo Crates are M1-020b.) R6: this is the building currency, separate from the
//! Commander's Manpower / Munitions / Fuel pools (M2).

use std::collections::BTreeMap;

use bevy_ecs::prelude::{Component, Resource};
use glam::Vec3;

use crate::components::Team;
use crate::data::GameData;
use crate::ids::PlayerId;

pub mod ammo;
pub mod crates;
pub mod depot;
pub mod limits;
pub mod provider;
pub mod system;

pub use crates::{CrateDamage, SupplyCrate};
pub use depot::{Depot, SupplyPoint};

/// Seconds to pick up / put down a crate.
pub const INTERACT_RANGE_M: f32 = 2.0;
/// Carrying a crate slows the player to this fraction of its speed.
pub const CARRY_SPEED_MULT: f32 = 0.75;
/// No Supply Crate within this distance of an enemy-held strongpoint.
pub const ENEMY_POINT_EXCLUSION_M: f32 = 80.0;

/// Rule values in ticks / counts (from `data/rules/supplies.json`).
#[derive(Resource, Debug, Clone, Copy)]
pub struct SupplyCfg {
    pub crate_supplies: u32,
    pub crate_hp: f32,
    pub ground_max: usize,
    pub interact_ticks: u32,
    pub depot_interval: u32,
    pub depot_max: u32,
    pub point_interval: u32,
    pub point_max: u32,
}

impl SupplyCfg {
    pub fn from_data(data: &GameData, hz: u8) -> Self {
        let r = &data.rules.supplies;
        let t = |s: f64| libm::ceil(s * f64::from(hz)) as u32;
        Self {
            crate_supplies: r.supply_crate.content_supplies,
            crate_hp: r.supply_crate.health_hp as f32,
            ground_max: r.supply_crate.ground_max_crates as usize,
            interact_ticks: t(r.supply_crate.place_s),
            depot_interval: t(r.depot.interval_s),
            depot_max: r.depot.stock_max_crates,
            point_interval: t(r.point_depot.interval_s),
            point_max: r.point_depot.stock_max_crates,
        }
    }
}

/// The crate a Support player is carrying (by crate id).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Carrying(pub Option<u32>);

/// Whether the player is a Support (the only role that carries crates; real roles: M1-021).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Support(pub bool);

/// All Supplies state of the match. Inactive (and not hashed) until a Depot or point exists.
#[derive(Resource, Debug, Clone, Default)]
pub struct SupplyWorld {
    pub crates: BTreeMap<u32, SupplyCrate>,
    pub next_id: u32,
    /// Placement order counter (FIFO eviction).
    pub next_seq: u32,
    pub depots: Vec<Depot>,
    pub points: Vec<SupplyPoint>,
    /// Ticks each player has held USE.
    pub hold: BTreeMap<PlayerId, u32>,
    pub ammo: BTreeMap<u32, ammo::AmmoCrate>,
    /// Ticks each player has held USE next to an Ammo Crate.
    pub ammo_hold: BTreeMap<PlayerId, u32>,
    /// Ammo Crates each player placed this life.
    pub ammo_count: BTreeMap<PlayerId, u8>,
    /// Tick each player last placed an Ammo Crate.
    pub ammo_last: BTreeMap<PlayerId, u32>,
}

impl SupplyWorld {
    pub fn active(&self) -> bool {
        !self.depots.is_empty() || !self.points.is_empty() || self.next_id > 0
    }
}

pub fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// Whether `team` may put a Supply Crate at `pos` (not within 80 m of an enemy-held point).
pub fn can_place_supply_crate(
    mut objectives: impl Iterator<Item = (Option<Team>, Vec3)>,
    team: Team,
    pos: Vec3,
) -> bool {
    objectives.all(|(owner, objective_pos)| match owner {
        Some(ow) if ow != team => dist_xz(objective_pos, pos) >= ENEMY_POINT_EXCLUSION_M,
        _ => true,
    })
}
