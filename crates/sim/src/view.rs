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
    /// Health quantised to 8 bit (`health_q`; 0 = dead, see [`crate::health::quant_health`]).
    pub health_q: u8,
    pub alive: bool,
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
    /// Hit points of an entity (`None` for stale ids; `<= 0` while dead).
    fn health(&self, _id: EntityId) -> Option<f32> {
        None
    }
    /// `false` for stale ids and dead players.
    fn alive(&self, _id: EntityId) -> bool {
        false
    }
    /// Alive players in ascending `EntityId` order.
    fn players(&self) -> &[PlayerView];
    fn vehicles(&self) -> &[VehicleView];
    /// `None` for stale or unknown ids.
    fn entity_state(&self, id: EntityId) -> Option<EntityStateView>;
    /// Clear terrain line of sight; `false` when blocked or outside the loaded collision.
    fn los(&self, from: Vec3, to: Vec3) -> bool;
    /// Bilinear terrain height; `None` where no collision chunk is loaded.
    fn height_at(&self, x: f32, z: f32) -> Option<f32>;
    /// Terrain slope in degrees at `(x, z)`; `None` where no collision chunk is loaded. The
    /// default derives it from [`height_at`](Self::height_at) over a 1 m central difference
    /// (the sim's [`World`](crate::World) overrides it with the exact triangle slope).
    fn slope_deg_at(&self, x: f32, z: f32) -> Option<f32> {
        let dx = self.height_at(x + 0.5, z)? - self.height_at(x - 0.5, z)?;
        let dz = self.height_at(x, z + 0.5)? - self.height_at(x, z - 0.5)?;
        Some(libm::atanf(libm::sqrtf(dx * dx + dz * dz)).to_degrees())
    }
    /// Placeholder: zero resources (no manpower / score model before M1).
    fn team_resources(&self, team: Team) -> Resources;
    fn objectives(&self) -> &[ObjectiveView];
    fn nav_hint(&self) -> &NavData;
}
