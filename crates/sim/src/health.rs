//! Hit points and life state (GAMEDESIGN 7.1 / 7.2, M0-013).
//!
//! M0 has no downed / revive state (M1-016): `hp <= 0` is death. A dead player keeps its entity
//! (stable `EntityId`, corpse position) in [`LifeState::Dead`] until the respawn tick; it takes no
//! input, fires nothing and cannot be hit.

use bevy_ecs::prelude::{Component, Resource};

use crate::ids::{EntityId, PlayerId, Tick};
use crate::weapon::HitZone;

/// Hit points of a player (`max_hp` from the role, GAMEDESIGN 7.2).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Health {
    pub hp: f32,
    pub max_hp: f32,
}

impl Health {
    pub fn full(max_hp: f32) -> Self {
        Self { hp: max_hp, max_hp }
    }
}

/// Alive or dead (with the data the kill feed and the respawn timer need).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LifeState {
    #[default]
    Alive,
    /// Down but not out (M1-016, GAMEDESIGN 7.1): `Health.hp` is the downed hit points and falls by
    /// bleeding; any damage takes more. A medic can revive, running out of hp means KIA.
    Downed {
        t_down: Tick,
        /// Who downed them (`None` when that player has left).
        killer: Option<PlayerId>,
        weapon: u8,
        /// Revives within the fatigue window when they went down (shortens the bleed time).
        fatigue: u8,
        /// Ticks the give-up button has been held.
        give_up_ticks: u16,
    },
    Dead {
        /// Tick at which the lethal hit landed.
        t_kia: Tick,
        /// Player that dealt the lethal hit (`None` when that player has left).
        killer: Option<PlayerId>,
        /// Weapon index (`GameData::weapons`) of the lethal projectile.
        weapon: u8,
        headshot: bool,
        /// Tick at which the player respawns (see [`crate::respawn::respawn_time`]).
        respawn_tick: Tick,
    },
}

impl LifeState {
    pub fn is_alive(&self) -> bool {
        matches!(self, LifeState::Alive)
    }
    pub fn is_downed(&self) -> bool {
        matches!(self, LifeState::Downed { .. })
    }
    /// KIA (waiting to respawn); a downed player is not dead.
    pub fn is_dead(&self) -> bool {
        matches!(self, LifeState::Dead { .. })
    }
}

/// Public name used by `World::player_life`.
pub type LifeStateView = LifeState;

/// One damaging projectile hit, recorded by the projectile stage for the rules stage (kill
/// attribution = last damager, GAMEDESIGN 7.5 minimal version).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitRec {
    pub target: EntityId,
    pub shooter: EntityId,
    pub weapon: u16,
    pub zone: HitZone,
    pub dist_m: f32,
    /// Hit points the hit took off.
    pub damage: f32,
    pub kind: crate::combat::DamageKind,
    /// Kills outright (a melee backstab) instead of downing.
    pub direct_kia: bool,
}

#[derive(Resource, Debug, Default)]
pub struct HitLog(pub Vec<HitRec>);

/// Health to `bits` (1..=8) quantisation for the snapshot `health_q` field: 0 means dead, any
/// positive hp maps to at least 1.
pub fn quant_health(hp: f32, max_hp: f32, bits: u8) -> u8 {
    // NaN and non-positive values count as dead.
    if hp.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater)
        || max_hp.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater)
    {
        return 0;
    }
    let max = (1u32 << bits.clamp(1, 8)) - 1;
    let q = libm::ceilf((hp / max_hp).min(1.0) * max as f32) as u32;
    q.clamp(1, max) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quant_health_edges() {
        assert_eq!(quant_health(0.0, 100.0, 8), 0);
        assert_eq!(quant_health(-5.0, 100.0, 8), 0);
        assert_eq!(quant_health(0.01, 100.0, 8), 1);
        assert_eq!(quant_health(100.0, 100.0, 8), 255);
        assert_eq!(quant_health(100.0, 100.0, 3), 7);
        assert_eq!(quant_health(150.0, 100.0, 8), 255);
        assert_eq!(quant_health(50.0, 100.0, 3), 4);
    }
}
