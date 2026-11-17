//! Stance (stand / crouch) and the capsule-height transition.
//!
//! The transition is a linear blend `0 (standing) .. 1 (crouched)` that advances by
//! `1 / stance_ticks` per tick toward the target (`stance_ticks = ms * hz / 1000`, 7.5 for
//! 250 ms at 30 Hz). Rounding rule: the blend is clamped, so a full transition completes on
//! tick `ceil(7.5) = 8`; reversing mid-way continues from the current blend (the capsule
//! height never jumps). `stance_timer` reports the remaining ticks, `ceil(|target - blend| *
//! stance_ticks)`. Prone is M1-011.

use super::params::MoveParams;
use super::state::PlayerAuthState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Stance {
    #[default]
    Stand = 0,
    Crouch = 1,
}

impl Stance {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Stance::Stand),
            1 => Some(Stance::Crouch),
            _ => None,
        }
    }
    fn target_blend(self) -> f32 {
        match self {
            Stance::Stand => 0.0,
            Stance::Crouch => 1.0,
        }
    }
}

/// Moves `s.stance` to `want` and advances the blend by one tick.
pub(crate) fn advance(s: &mut PlayerAuthState, want: Stance, p: &MoveParams) {
    s.stance = want;
    let target = want.target_blend();
    let step = 1.0 / p.stance_ticks.max(1.0);
    s.stance_blend = if s.stance_blend < target {
        (s.stance_blend + step).min(target)
    } else {
        (s.stance_blend - step).max(target)
    };
    let remaining = (target - s.stance_blend).abs() * p.stance_ticks;
    // The epsilon keeps float dust from adding a phantom tick.
    s.stance_timer = libm::ceilf(remaining - 1e-4).clamp(0.0, 255.0) as u8;
}

/// Capsule height for a blend value.
pub fn capsule_height(blend: f32, p: &MoveParams) -> f32 {
    p.height_stand_m + (p.height_crouch_m - p.height_stand_m) * blend
}
