//! Lean (GAMEDESIGN 9.4, M1-011b): `PlayerAuthState::lean` moves toward the commanded side by
//! `1 / lean_ticks` per tick (0.15 s = 4.5 ticks). Leaning is cancelled by sprinting and vaulting.
//! The head moves sideways by `lean * lean_head_offset_m`; prone, leaning is only a peek that
//! raises the head by `|lean| * prone_peek_m`.

use frontline_protocol::msg::input::{buttons, InputCmd};
use glam::Vec3;

use super::params::MoveParams;
use super::stance::Stance;
use super::state::PlayerAuthState;

/// Commanded lean side: `-1` left, `+1` right, `0` none (both buttons cancel).
pub fn command(cmd: &InputCmd) -> f32 {
    let l = cmd.buttons & buttons::LEAN_L != 0;
    let r = cmd.buttons & buttons::LEAN_R != 0;
    f32::from(i8::from(r) - i8::from(l))
}

/// One tick of lean toward `target` (`-1..=1`).
pub(crate) fn advance(s: &mut PlayerAuthState, target: f32, p: &MoveParams) {
    let step = 1.0 / p.lean_ticks.max(1.0);
    s.lean = if s.lean < target {
        (s.lean + step).min(target)
    } else {
        (s.lean - step).max(target)
    };
}

/// Lean angle in degrees (roll of the upper body / camera).
pub fn angle_deg(s: &PlayerAuthState, p: &MoveParams) -> f32 {
    s.lean * p.lean_deg
}

/// World-space offset of the head from its upright position (to add to the eye position).
pub fn head_offset(s: &PlayerAuthState, p: &MoveParams) -> Vec3 {
    if s.lean == 0.0 {
        return Vec3::ZERO;
    }
    if s.stance == Stance::Prone && s.stance_blend >= Stance::Prone.pose() {
        return Vec3::new(0.0, libm::fabsf(s.lean) * p.prone_peek_m, 0.0);
    }
    // Right-hand side of the view: (cos yaw, -sin yaw) on (x, z).
    let d = s.lean * p.lean_head_offset_m;
    Vec3::new(libm::cosf(s.yaw) * d, 0.0, -libm::sinf(s.yaw) * d)
}

/// Spread multiplier while leaning (`lean_spread_mult`, else 1).
pub fn spread_mult(s: &PlayerAuthState, p: &MoveParams) -> f32 {
    if libm::fabsf(s.lean) > 0.0 && s.stance != Stance::Prone {
        p.lean_spread_mult
    } else {
        1.0
    }
}
