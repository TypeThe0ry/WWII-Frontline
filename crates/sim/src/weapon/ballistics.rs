//! Projectile motion (GAMEDESIGN 6.4) and damage falloff (6.6).
//!
//! Per tick the projectile advances `SUBSTEPS` (4) sub-steps of `h = dt / 4` (120 Hz at 30 Hz):
//! `a = (0, -g, 0) - k |v| v;  v += a h;  x += v h` (semi-implicit Euler, exactly as the design
//! document writes it). `tests/ballistics_vs_reference.rs` compares against an RK4 solution.

use glam::Vec3;

use crate::data::defs::{FalloffPoint, WeaponDef};

/// Integration sub-steps per tick (GAMEDESIGN 6.4).
pub const SUBSTEPS: u32 = 4;
/// A projectile is removed after this much flight time, seconds.
pub const MAX_FLIGHT_S: f32 = 4.0;
/// ... or when its speed falls below this, m/s (a lobbed shot is not lethal).
pub const MIN_SPEED_MPS: f32 = 150.0;
/// Muzzle offset along the shot direction from the eye, metres (task M0-012). 【待调参】
pub const MUZZLE_OFFSET_M: f32 = 0.5;

/// Drag and gravity of one weapon, from `gameplay.ballistics`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BallisticsParams {
    pub drag_k_per_m: f32,
    pub gravity_mps2: f32,
    pub muzzle_velocity_mps: f32,
}

impl BallisticsParams {
    pub fn from_def(def: &WeaponDef) -> Self {
        let b = &def.gameplay.ballistics;
        Self {
            drag_k_per_m: b.drag_k_per_m as f32,
            gravity_mps2: b.gravity_mps2 as f32,
            muzzle_velocity_mps: b.muzzle_velocity_mps as f32,
        }
    }
}

/// One sub-step of length `h` seconds.
#[inline]
pub fn substep(pos: &mut Vec3, vel: &mut Vec3, k: f32, g: f32, h: f32) {
    let speed = libm::sqrtf(vel.x * vel.x + vel.y * vel.y + vel.z * vel.z);
    let drag = k * speed;
    vel.x -= drag * vel.x * h;
    vel.y -= (g + drag * vel.y) * h;
    vel.z -= drag * vel.z * h;
    pos.x += vel.x * h;
    pos.y += vel.y * h;
    pos.z += vel.z * h;
}

/// Maximum number of sub-steps of a projectile's life (`MAX_FLIGHT_S` at `hz` ticks/s).
pub fn max_substeps(hz: u8) -> u32 {
    (MAX_FLIGHT_S * f32::from(hz)) as u32 * SUBSTEPS
}

/// Damage multiplier at distance `d` (m): piecewise linear through the data points, constant
/// before the first and after the last (GAMEDESIGN 6.6 "衰减到 min 后保持").
pub fn falloff(points: &[FalloffPoint], d: f32) -> f32 {
    let (Some(first), Some(last)) = (points.first(), points.last()) else {
        return 1.0;
    };
    let d = f64::from(d);
    if d <= first.dist_m {
        return first.mult as f32;
    }
    for pair in points.windows(2) {
        if d <= pair[1].dist_m {
            let span = pair[1].dist_m - pair[0].dist_m;
            let t = if span > 0.0 {
                (d - pair[0].dist_m) / span
            } else {
                1.0
            };
            return (pair[0].mult + t * (pair[1].mult - pair[0].mult)) as f32;
        }
    }
    last.mult as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts() -> Vec<FalloffPoint> {
        [(0.0, 1.0), (100.0, 1.0), (600.0, 0.72)]
            .iter()
            .map(|&(dist_m, mult)| FalloffPoint { dist_m, mult })
            .collect()
    }

    #[test]
    fn falloff_interpolation_and_bounds() {
        let p = pts();
        assert_eq!(falloff(&p, 0.0), 1.0);
        assert_eq!(falloff(&p, 100.0), 1.0, "F0 itself is still full damage");
        assert!(
            (falloff(&p, 350.0) - 0.86).abs() < 1e-6,
            "midpoint of the slope"
        );
        assert!((falloff(&p, 300.0) - (1.0 - 0.28 * 200.0 / 500.0)).abs() < 1e-6);
        assert_eq!(falloff(&p, 600.0), 0.72);
        assert_eq!(falloff(&p, 5000.0), 0.72, "held after the last point");
        assert_eq!(falloff(&[], 10.0), 1.0);
    }

    #[test]
    fn vacuum_fall_matches_closed_form_up_to_euler_bias() {
        // k = 0: y = -g t^2/2 (1 + 1/N) for N sub-steps (semi-implicit Euler).
        let (mut p, mut v) = (Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0));
        let h = 1.0 / 120.0;
        for _ in 0..120 {
            substep(&mut p, &mut v, 0.0, 9.81, h);
        }
        assert!((p.x - 100.0).abs() < 1e-3);
        let exact = -0.5 * 9.81;
        assert!((p.y / exact - 1.0 - 1.0 / 120.0).abs() < 1e-3, "{}", p.y);
    }

    #[test]
    fn max_substeps_is_four_seconds() {
        assert_eq!(max_substeps(30), 480);
    }
}
