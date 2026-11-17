//! Spread (GAMEDESIGN 6.5): `theta = base * stance_spread * move_spread + bloom`, cone sampling.

use crate::movement::Stance;
use crate::rng::{unit_f32, Pcg64Mcg};
use glam::Vec3;

/// `stance_spread` per stance (GAMEDESIGN 9.5). 【待调参】
pub fn stance_spread(s: Stance) -> f32 {
    match s {
        Stance::Stand => 1.00,
        Stance::Crouch => 0.80,
        Stance::Prone => 0.60,
    }
}

/// Horizontal speeds (m/s) separating the `move_spread` classes: below `STILL_MPS` the player
/// counts as standing still, up to `WALK_MAX_MPS` as walking (walk 2.0 / crouch run 2.4 sit
/// just below 3.2, run 4.4 above), otherwise running. Sprinting cannot shoot. 【待调参】
pub const STILL_MPS: f32 = 0.3;
pub const WALK_MAX_MPS: f32 = 3.2;

/// `move_spread`: still 1.0, walking 1.5, running 2.5 (GAMEDESIGN 6.5).
pub fn move_spread(horizontal_speed_mps: f32) -> f32 {
    if horizontal_speed_mps < STILL_MPS {
        1.0
    } else if horizontal_speed_mps <= WALK_MAX_MPS {
        1.5
    } else {
        2.5
    }
}

/// Half-angle of the spread cone, degrees.
pub fn spread_deg(base_deg: f32, stance: Stance, horizontal_speed_mps: f32, bloom_deg: f32) -> f32 {
    base_deg * stance_spread(stance) * move_spread(horizontal_speed_mps) + bloom_deg
}

/// Unit vector from yaw / pitch (radians): yaw 0 faces +z and grows toward +x, pitch > 0 looks up.
pub fn view_dir(yaw: f32, pitch: f32) -> Vec3 {
    let (sy, cy) = (libm::sinf(yaw), libm::cosf(yaw));
    let (sp, cp) = (libm::sinf(pitch), libm::cosf(pitch));
    Vec3::new(sy * cp, sp, cy * cp)
}

/// Uniform direction (uniform in solid angle) inside the cone of half-angle `half_deg` around
/// the unit vector `dir`. Draws exactly two numbers from `rng`.
///
/// Uses `theta = 2 asin(sqrt(u) sin(theta_max / 2))`, which equals `acos(1 - u (1 - cos max))`
/// but keeps full `f32` precision for sub-degree cones.
pub fn sample_cone(rng: &mut Pcg64Mcg, dir: Vec3, half_deg: f32) -> Vec3 {
    let u = unit_f32(rng);
    let phi = unit_f32(rng) * core::f32::consts::TAU;
    let half = half_deg.clamp(0.0, 90.0).to_radians();
    let theta = 2.0 * libm::asinf(libm::sqrtf(u) * libm::sinf(0.5 * half));
    // Orthonormal basis around dir.
    let helper = if dir.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let right = dir.cross(helper).normalize();
    let up = right.cross(dir);
    let (st, ct) = (libm::sinf(theta), libm::cosf(theta));
    (dir * ct + (right * libm::cosf(phi) + up * libm::sinf(phi)) * st).normalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::Tick;
    use crate::rng::{Purpose, Rng};

    #[test]
    fn factors() {
        assert_eq!(move_spread(0.0), 1.0);
        assert_eq!(move_spread(2.0), 1.5);
        assert_eq!(move_spread(1.5), 1.5);
        assert_eq!(move_spread(2.4), 1.5);
        assert_eq!(move_spread(4.4), 2.5);
        assert_eq!(stance_spread(Stance::Crouch), 0.8);
        let t = spread_deg(3.5, Stance::Crouch, 0.0, 0.4);
        assert!((t - (3.5 * 0.8 + 0.4)).abs() < 1e-6);
    }

    #[test]
    fn view_dir_axes() {
        let f = view_dir(0.0, 0.0);
        assert!((f - Vec3::Z).length() < 1e-6);
        let r = view_dir(core::f32::consts::FRAC_PI_2, 0.0);
        assert!((r - Vec3::X).length() < 1e-6);
        let u = view_dir(0.0, core::f32::consts::FRAC_PI_4);
        assert!(u.y > 0.7 && u.z > 0.7);
    }

    #[test]
    fn cone_stays_inside_and_is_uniform() {
        let dir = view_dir(0.7, 0.2);
        let half = 3.5f32;
        let mut inside = 0;
        let mut within_half_radius = 0;
        let n = 20_000;
        for i in 0..n {
            let mut r = Rng::for_(1, Tick(i), 0, Purpose::Spread);
            let d = sample_cone(&mut r, dir, half);
            assert!((d.length() - 1.0).abs() < 1e-5);
            let ang = libm::acosf(d.dot(dir).clamp(-1.0, 1.0)).to_degrees();
            if ang <= half + 1e-3 {
                inside += 1;
            }
            // Uniform in solid angle: P(theta <= half/2) = sin^2(half/4)/sin^2(half/2) ~ 25 %.
            if ang <= half * 0.5 {
                within_half_radius += 1;
            }
        }
        assert_eq!(inside, n);
        let frac = within_half_radius as f32 / n as f32;
        assert!((frac - 0.25).abs() < 0.02, "inner-half fraction {frac}");
    }

    #[test]
    fn zero_cone_is_exact() {
        let dir = view_dir(1.0, -0.3);
        let mut r = Rng::for_(1, Tick(1), 1, Purpose::Spread);
        let d = sample_cone(&mut r, dir, 0.0);
        assert!((d - dir).length() < 1e-6);
    }
}
