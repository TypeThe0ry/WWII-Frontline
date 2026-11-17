//! Read-only world view (ARCHITECTURE 8.1).

use glam::Vec3;

use crate::components::Team;
use crate::ids::{EntityId, PlayerId};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerView {
    pub player: PlayerId,
    pub entity: EntityId,
    pub team: Team,
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleView {
    pub entity: EntityId,
    pub pos: Vec3,
}

/// Per-entity state for snapshots.
///
/// TODO(M0-006/M0-015): replace with `frontline_protocol::EntityState` once the protocol
/// crate exports it; the fields below are the sim-side source values (unquantised).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityStateView {
    pub entity: EntityId,
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Resources {
    pub manpower: i32,
    pub fuel: i32,
    pub ammo: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectiveView {
    pub id: u16,
}

/// Navigation hint data for bots (filled by M0-010+).
#[derive(Debug, Clone, Default)]
pub struct NavData {}

pub trait WorldView {
    /// Alive players in ascending `EntityId` order.
    fn players(&self) -> &[PlayerView];
    fn vehicles(&self) -> &[VehicleView];
    /// `None` for stale or unknown ids.
    fn entity_state(&self, id: EntityId) -> Option<EntityStateView>;
    /// Placeholder until M0-010 (collision): everything is visible.
    fn los(&self, from: Vec3, to: Vec3) -> bool;
    /// Placeholder until M0-010: flat ground at y = 0.
    fn height_at(&self, x: f32, z: f32) -> f32;
    /// Placeholder until M0-013: zero resources.
    fn team_resources(&self, team: Team) -> Resources;
    fn objectives(&self) -> &[ObjectiveView];
    fn nav_hint(&self) -> &NavData;
}
