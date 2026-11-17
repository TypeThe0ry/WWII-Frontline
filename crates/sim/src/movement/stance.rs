//! Stance (stand / crouch / prone) and the capsule-height transition (GAMEDESIGN 9.2).
//!
//! `stance_blend` is a pose coordinate: `0` standing, `1` crouched, `2` prone. It moves toward
//! the target pose by a constant step per tick; the step is `|distance between the two settled
//! poses| / duration`, where the duration comes from the transition table of the role data
//! (stand<->crouch `stance_transition_ms`, crouch->prone 500 ms, prone->crouch 600 ms,
//! stand->prone 700 ms, prone->stand 900 ms). `stance_from` is the *previous commanded* stance,
//! so reversing mid-way continues from the current blend (the capsule never jumps) at the
//! speed of the reverse transition. A full transition completes on tick `ceil(duration)`;
//! `stance_timer` reports the remaining ticks, `ceil(|target - blend| / step)`.

use super::params::MoveParams;
use super::state::PlayerAuthState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Stance {
    #[default]
    Stand = 0,
    Crouch = 1,
    Prone = 2,
}

impl Stance {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Stance::Stand),
            1 => Some(Stance::Crouch),
            2 => Some(Stance::Prone),
            _ => None,
        }
    }

    /// Pose coordinate of the settled stance.
    pub fn pose(self) -> f32 {
        f32::from(self as u8)
    }
}

/// Transition length in ticks between two settled stances (0 for `from == to`).
pub fn duration_ticks(from: Stance, to: Stance, p: &MoveParams) -> f32 {
    use Stance::*;
    match (from, to) {
        (Stand, Crouch) | (Crouch, Stand) => p.stance_ticks,
        (Crouch, Prone) => p.ticks_c2p,
        (Prone, Crouch) => p.ticks_p2c,
        (Stand, Prone) => p.ticks_s2p,
        (Prone, Stand) => p.ticks_p2s,
        _ => 0.0,
    }
}

/// Moves `s.stance` to `want` and advances the pose by one tick.
pub(crate) fn advance(s: &mut PlayerAuthState, want: Stance, p: &MoveParams) {
    if want != s.stance {
        s.stance_from = s.stance;
        s.stance = want;
    }
    let target = want.pose();
    let dur = duration_ticks(s.stance_from, want, p).max(1.0);
    let step = (target - s.stance_from.pose()).abs() / dur;
    s.stance_blend = if s.stance_blend < target {
        (s.stance_blend + step).min(target)
    } else {
        (s.stance_blend - step).max(target)
    };
    let remaining = if step > 0.0 {
        (target - s.stance_blend).abs() / step
    } else {
        0.0
    };
    // The epsilon keeps float dust from adding a phantom tick.
    s.stance_timer = libm::ceilf(remaining - 1e-4).clamp(0.0, 255.0) as u8;
    if s.stance_timer == 0 {
        s.stance_from = want;
        s.stance_blend = target; // snap float dust
    }
}

fn lerp3(blend: f32, a: f32, b: f32, c: f32) -> f32 {
    let t = blend.clamp(0.0, 2.0);
    if t <= 1.0 {
        a + (b - a) * t
    } else {
        b + (c - b) * (t - 1.0)
    }
}

/// Capsule height for a pose coordinate.
pub fn capsule_height(blend: f32, p: &MoveParams) -> f32 {
    lerp3(blend, p.height_stand_m, p.height_crouch_m, p.height_prone_m)
}

/// Eye height above the feet for a pose coordinate.
pub fn eye_height(blend: f32, p: &MoveParams) -> f32 {
    lerp3(
        blend,
        p.eye_height_stand_m,
        p.eye_height_crouch_m,
        p.eye_height_prone_m,
    )
}

/// Capsule height of a settled stance.
pub fn stance_height(s: Stance, p: &MoveParams) -> f32 {
    capsule_height(s.pose(), p)
}
