//! Supply Crate state.

use glam::Vec3;

use crate::components::Team;
use crate::ids::PlayerId;

#[derive(Debug, Clone, PartialEq)]
pub struct SupplyCrate {
    pub id: u32,
    pub team: Team,
    pub pos: Vec3,
    /// Supplies left in the crate (100 when new).
    pub supplies: u32,
    pub hp: f32,
    pub carried_by: Option<PlayerId>,
    /// Order the crate was put on the ground (FIFO eviction); `u32::MAX` while carried.
    pub seq: u32,
}

/// Damage classes (GAMEDESIGN 12.2: small arms x0.1, explosions x1.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrateDamage {
    Bullet,
    Explosion,
}

impl CrateDamage {
    pub fn mult(self) -> f32 {
        match self {
            CrateDamage::Bullet => 0.1,
            CrateDamage::Explosion => 1.0,
        }
    }
}
