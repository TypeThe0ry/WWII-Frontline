//! Melee state machine and hit resolution (M1-013b, GAMEDESIGN 6.8).
//!
//! A press of `MELEE` while idle starts a windup of `windup_s`; when it elapses the strike is
//! resolved once against every living enemy inside the sector in front of the attacker, then a
//! cooldown of `cooldown_s` runs. The strike needs the target within `range_m` (horizontal
//! distance, +20 % for a bayonet lunge started while sprinting) and within +-35 degrees of the
//! attacker's facing. An attacker standing more than `backstab_angle_deg` away from the target's
//! facing (i.e. behind it) kills outright when `backstab_kill` is set. Friendly targets are
//! skipped (`ff_mult` is 0 in all melee data). All angle maths uses `libm`.

use bevy_ecs::prelude::Component;
use glam::Vec3;

use crate::data::defs::{MeleeClass, MeleeDef};

/// Half-width of the strike sector, degrees.
pub const SECTOR_HALF_DEG: f32 = 35.0;
/// Range multiplier of a bayonet lunge started while sprinting.
pub const LUNGE_RANGE_MULT: f32 = 1.2;
/// Largest height difference (m) between attacker feet and target feet for a strike to land.
pub const MAX_DY_M: f32 = 1.0;
/// `weapon` index reported in kill events for melee kills (not an index of `GameData::weapons`).
pub const MELEE_WEAPON_IDX: u16 = 0xFF;
const EPS: f32 = 1e-3;
/// Angle slack (degrees): wider than the 16-bit yaw step (0.0055 deg) so exact data angles count.
const ANGLE_EPS: f32 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeleeParams {
    pub damage_hp: f32,
    pub windup_ticks: u16,
    pub cooldown_ticks: u16,
    pub range_m: f32,
    pub backstab_deg: f32,
    pub backstab_kill: bool,
    pub ff_mult: f32,
    pub lunge: bool,
}

fn ticks(seconds: f64, hz: u8) -> u16 {
    libm::ceilf((seconds * f64::from(hz)) as f32 - 1e-4).clamp(1.0, 65_535.0) as u16
}

impl MeleeParams {
    pub fn from_def(def: &MeleeDef, hz: u8) -> Self {
        let g = &def.gameplay;
        Self {
            damage_hp: g.damage_hp as f32,
            windup_ticks: ticks(g.windup_s, hz),
            cooldown_ticks: ticks(g.cooldown_s, hz),
            range_m: g.range_m as f32,
            backstab_deg: g.backstab_angle_deg as f32,
            backstab_kill: g.backstab_kill,
            ff_mult: g.ff_mult as f32,
            lunge: def.class == MeleeClass::Bayonet,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MeleePhase {
    #[default]
    Idle = 0,
    Windup = 1,
    Cooldown = 2,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct MeleeState {
    pub phase: MeleePhase,
    pub ticks_left: u16,
    /// The windup began while sprinting with a bayonet.
    pub lunging: bool,
    /// Definition picked when the current windup started.
    pub(crate) variant: Option<usize>,
    /// Execution of a downed enemy in progress (M1-016b): target, ticks left (0 = none) and the
    /// attacker's hit points when it began (losing any ends it).
    pub exec_target: u16,
    pub exec_ticks: u16,
    pub exec_hp: f32,
}

impl MeleeState {
    pub fn is_idle(&self) -> bool {
        self.phase == MeleePhase::Idle
    }
    pub fn executing(&self) -> bool {
        self.exec_ticks > 0
    }
}

/// Execution duration, seconds (GAMEDESIGN 7.4).
pub const EXECUTION_S: f32 = 1.2;

/// What one tick of the state machine asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeleeStep {
    /// The windup elapsed this tick: resolve the strike now.
    pub strike: bool,
}

/// Advances the state machine one tick. `want` = MELEE held, `can_start` = alive, not vaulting.
pub fn step_melee(
    s: &mut MeleeState,
    p: &MeleeParams,
    want: bool,
    can_start: bool,
    sprinting: bool,
) -> MeleeStep {
    let mut out = MeleeStep::default();
    match s.phase {
        MeleePhase::Idle => {
            if want && can_start {
                s.phase = MeleePhase::Windup;
                s.ticks_left = p.windup_ticks;
                s.lunging = p.lunge && sprinting;
            }
        }
        MeleePhase::Windup => {
            s.ticks_left = s.ticks_left.saturating_sub(1);
            if s.ticks_left == 0 {
                out.strike = true;
                s.phase = MeleePhase::Cooldown;
                s.ticks_left = p.cooldown_ticks;
            }
        }
        MeleePhase::Cooldown => {
            s.ticks_left = s.ticks_left.saturating_sub(1);
            if s.ticks_left == 0 {
                s.phase = MeleePhase::Idle;
                s.lunging = false;
            }
        }
    }
    out
}

/// Result of a landed strike.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeleeHit {
    pub dist_m: f32,
    pub backstab: bool,
}

/// Unsigned angle in degrees between two horizontal vectors (x, z).
fn angle_deg(a: (f32, f32), b: (f32, f32)) -> f32 {
    let cross = a.0 * b.1 - a.1 * b.0;
    let dot = a.0 * b.0 + a.1 * b.1;
    libm::atan2f(libm::fabsf(cross), dot).to_degrees()
}

/// Whether a strike from `att` (feet position, yaw) lands on `tgt` (feet, yaw).
pub fn melee_trace(
    p: &MeleeParams,
    lunging: bool,
    att: Vec3,
    att_yaw: f32,
    tgt: Vec3,
    tgt_yaw: f32,
) -> Option<MeleeHit> {
    let (dx, dz) = (tgt.x - att.x, tgt.z - att.z);
    let dist = libm::sqrtf(dx * dx + dz * dz);
    let range = if lunging {
        p.range_m * LUNGE_RANGE_MULT
    } else {
        p.range_m
    };
    if dist > range + EPS || libm::fabsf(tgt.y - att.y) > MAX_DY_M {
        return None;
    }
    let fwd = (libm::sinf(att_yaw), libm::cosf(att_yaw));
    if dist > EPS && angle_deg(fwd, (dx, dz)) > SECTOR_HALF_DEG + ANGLE_EPS {
        return None;
    }
    // Angle between the target's facing and the direction from the target to the attacker.
    let tfwd = (libm::sinf(tgt_yaw), libm::cosf(tgt_yaw));
    let backstab =
        p.backstab_kill && dist > EPS && angle_deg(tfwd, (-dx, -dz)) >= p.backstab_deg - ANGLE_EPS;
    Some(MeleeHit {
        dist_m: dist,
        backstab,
    })
}

/// Damage of a landed strike against a target with `hp`.
pub fn melee_damage(p: &MeleeParams, hit: &MeleeHit, hp: f32) -> f32 {
    if hit.backstab {
        hp.max(p.damage_hp)
    } else {
        p.damage_hp
    }
}
