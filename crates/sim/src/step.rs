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
    /// A player died (M0-013). `killer` is `None` when the killer has left the match;
    /// `weapon` indexes `GameData::weapons`; `headshot` = lethal hit in the head zone.
    Kill {
        victim: PlayerId,
        killer: Option<PlayerId>,
        weapon: u8,
        headshot: bool,
        dist_m: f32,
    },
    /// A grenade left a hand (M1-015).
    Thrown {
        thrower: EntityId,
        pos: Vec3,
        smoke: bool,
    },
    /// A blast went off at `pos` (`kind` 0 = grenade).
    Explosion {
        pos: Vec3,
        radius_m: f32,
        kind: u8,
    },
    SmokeStarted {
        pos: Vec3,
        radius_m: f32,
    },
    /// `count` smoke clouds ran out this tick.
    SmokeEnded {
        count: u8,
    },
    /// A melee strike landed (M1-013b); `damage_hp` has been subtracted, `backstab` = rear kill.
    MeleeHit {
        attacker: EntityId,
        target: EntityId,
        damage_hp: f32,
        backstab: bool,
    },
    /// A dead player re-entered the world at `pos` (M0-013).
    Spawn {
        player: PlayerId,
        pos: Vec3,
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
