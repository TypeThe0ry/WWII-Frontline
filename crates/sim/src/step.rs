//! Output of one simulation step. Variants are appended by later tasks.

use glam::Vec3;

use crate::ids::{EntityId, PlayerId};
use crate::weapon::HitZone;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SimEvent {
    PlayerSpawned {
        player: PlayerId,
        entity: EntityId,
    },
    PlayerRemoved {
        player: PlayerId,
        entity: EntityId,
    },
    /// A projectile left a barrel (ARCHITECTURE 6.8.4 `Shot`); `weapon` indexes `GameData::weapons`.
    Shot {
        shooter: EntityId,
        weapon: u16,
        muzzle: Vec3,
        /// Unit direction including recoil and spread.
        dir: Vec3,
    },
    /// Trigger pulled on an empty magazine.
    DryFire {
        shooter: EntityId,
    },
    /// A projectile hit the terrain (ARCHITECTURE 6.8.4 `Impact`).
    Impact {
        pos: Vec3,
        normal: Vec3,
        material: u8,
    },
    /// A projectile hit a player; `damage_hp` has been subtracted from the target's health.
    Hit {
        shooter: EntityId,
        target: EntityId,
        zone: HitZone,
        damage_hp: f32,
        dist_m: f32,
    },
}

/// Objective / resource / phase changes (M0-013+; empty for now).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuleChange {}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StepOutput {
    pub events: Vec<SimEvent>,
    pub rule_changes: Vec<RuleChange>,
}
