//! Stance and movement modifiers for weapons and perception (GAMEDESIGN 9.5). All numbers
//! come from the role data via [`MoveParams`].

use super::params::MoveParams;
use super::stance::Stance;

/// Recoil multiplier of a stance (stand 1.00, crouch 0.75, prone 0.50).
pub fn stance_recoil(s: Stance, p: &MoveParams) -> f32 {
    p.recoil_mult[s as usize]
}

/// Spread multiplier of a stance (1.00 / 0.80 / 0.60).
pub fn stance_spread(s: Stance, p: &MoveParams) -> f32 {
    p.spread_mult[s as usize]
}

/// Visibility multiplier of a stance (1.00 / 0.85 / 0.65).
pub fn visibility(s: Stance, p: &MoveParams) -> f32 {
    p.visibility_mult[s as usize]
}

/// Hit-box height of a stance in metres (1.8 / 1.2 / 0.5).
pub fn hitbox_height(s: Stance, p: &MoveParams) -> f32 {
    p.hitbox_height_m[s as usize]
}

/// Spread multiplier from the horizontal speed: standing still 1.0, walking 1.5, running 2.5.
/// `None` while sprinting (cannot shoot). Thresholds sit halfway between the mode speeds.
pub fn move_spread(horizontal_mps: f32, p: &MoveParams) -> Option<f32> {
    let still = 0.1;
    let walk_run = 0.5 * (p.walk_mps + p.run_mps);
    let run_sprint = 0.5 * (p.run_mps + p.sprint_mps);
    if horizontal_mps < still {
        Some(1.0)
    } else if horizontal_mps < walk_run {
        Some(p.move_spread_walk_mult)
    } else if horizontal_mps < run_sprint {
        Some(p.move_spread_run_mult)
    } else {
        None
    }
}

/// Load speed modifier `clamp(1 - penalty * (load - ref), min, 1)` (GAMEDESIGN 9.1).
pub fn load_mult(load_kg: f32, p: &MoveParams) -> f32 {
    (1.0 - p.load_penalty_per_kg * (load_kg - p.load_ref_kg)).clamp(p.load_min_mult, 1.0)
}
