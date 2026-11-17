//! Aiming: aim point, ballistic drop compensation, gaussian error, turn-rate limiting.

use core::f32::consts::{PI, TAU};

use frontline_sim::rng::{unit_f32, Pcg64Mcg};
use frontline_sim::Vec3;

use crate::difficulty::{Difficulty, Tuning};

/// Standard normal sample (Box-Muller, two draws).
pub fn gaussian(rng: &mut Pcg64Mcg) -> f32 {
    let u1 = unit_f32(rng).max(1e-7);
    let u2 = unit_f32(rng);
    libm::sqrtf(-2.0 * libm::logf(u1)) * libm::cosf(TAU * u2)
}

/// Standard deviation of the aim error (degrees) at `dist` metres:
/// `(base + per_100m * dist / 100) * aim_error_mult`.
pub fn aim_sigma_deg(dist: f32, t: &Tuning, d: &Difficulty) -> f32 {
    (t.aim_error_base_deg + t.aim_error_per_100m_deg * dist / 100.0) * d.aim_error_mult
}

/// `(yaw, pitch)` offsets in degrees for one think step.
pub fn aim_error_deg(rng: &mut Pcg64Mcg, dist: f32, t: &Tuning, d: &Difficulty) -> (f32, f32) {
    let s = aim_sigma_deg(dist, t, d);
    (gaussian(rng) * s, gaussian(rng) * s)
}

/// Gravity drop (m) of a round flying `dist` metres at `v0` (no drag; the aim error and the
/// bot's distance are far larger than the drag correction at M0 ranges).
pub fn drop_m(dist: f32, v0: f32, g: f32) -> f32 {
    if v0 <= 1.0 {
        return 0.0;
    }
    let t = dist / v0;
    0.5 * g * t * t
}

/// View angles `(yaw, pitch)` radians from `eye` to `point` (yaw 0 = +z, toward +x; pitch > 0 up).
pub fn angles_to(eye: Vec3, point: Vec3) -> (f32, f32) {
    let d = point - eye;
    let h = libm::sqrtf(d.x * d.x + d.z * d.z);
    (libm::atan2f(d.x, d.z), libm::atan2f(d.y, h))
}

/// Wraps an angle difference into `(-pi, pi]`.
pub fn wrap_pi(a: f32) -> f32 {
    let mut x = a % TAU;
    if x > PI {
        x -= TAU;
    } else if x <= -PI {
        x += TAU;
    }
    x
}

/// Moves `cur` toward `target` by at most `max_step` radians (shortest way round).
pub fn step_angle(cur: f32, target: f32, max_step: f32) -> f32 {
    let d = wrap_pi(target - cur);
    if d.abs() <= max_step {
        target
    } else {
        cur + d.signum() * max_step
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontline_sim::rng::{Purpose, Rng};
    use frontline_sim::Tick;

    #[test]
    fn gaussian_has_unit_variance() {
        let mut rng = Rng::for_(7, Tick(1), 3, Purpose::Bot);
        let n = 20_000;
        let v: Vec<f32> = (0..n).map(|_| gaussian(&mut rng)).collect();
        let mean = v.iter().sum::<f32>() / n as f32;
        let var = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / n as f32;
        assert!(mean.abs() < 0.03, "mean {mean}");
        assert!((var - 1.0).abs() < 0.05, "var {var}");
    }

    #[test]
    fn aim_error_grows_with_distance_and_difficulty() {
        let t = Tuning::embedded();
        let reg = Difficulty::embedded_default();
        let mut vet = reg;
        vet.aim_error_mult = 1.0;
        assert!(aim_sigma_deg(200.0, &t, &reg) > aim_sigma_deg(20.0, &t, &reg));
        assert!(aim_sigma_deg(100.0, &t, &reg) > aim_sigma_deg(100.0, &t, &vet));
        // Regular is x1.3 of the base error (GAMEDESIGN 20.6).
        assert!((reg.aim_error_mult - 1.3).abs() < 1e-6);
        // Empirical standard deviation matches the formula.
        let mut rng = Rng::for_(1, Tick(5), 9, Purpose::Bot);
        let dist = 100.0;
        let xs: Vec<f32> = (0..20_000)
            .map(|_| aim_error_deg(&mut rng, dist, &t, &reg).0)
            .collect();
        let m = xs.iter().sum::<f32>() / xs.len() as f32;
        let sd = (xs.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / xs.len() as f32).sqrt();
        let want = aim_sigma_deg(dist, &t, &reg);
        assert!((sd - want).abs() < 0.05 * want, "sd {sd} want {want}");
    }

    #[test]
    fn angle_helpers() {
        let (y, p) = angles_to(Vec3::ZERO, Vec3::new(10.0, 0.0, 10.0));
        assert!((y - core::f32::consts::FRAC_PI_4).abs() < 1e-5 && p.abs() < 1e-6);
        let (_, p) = angles_to(Vec3::ZERO, Vec3::new(0.0, 10.0, 10.0));
        assert!((p - core::f32::consts::FRAC_PI_4).abs() < 1e-5);
        // Shortest way round the wrap point.
        let s = step_angle(0.1, TAU - 0.1, 0.05);
        assert!(s < 0.1 && (0.1 - s - 0.05).abs() < 1e-6, "{s}");
        assert_eq!(step_angle(1.0, 1.02, 0.05), 1.02);
        assert!(drop_m(200.0, 850.0, 9.81) > 0.2 && drop_m(200.0, 850.0, 9.81) < 0.35);
    }
}
