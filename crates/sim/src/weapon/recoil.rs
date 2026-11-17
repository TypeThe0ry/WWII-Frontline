//! Recoil (GAMEDESIGN 6.5): per-shot kick, bloom, and the recovery after the firing pause.

use crate::movement::Stance;
use crate::rng::{unit_f32, Pcg64Mcg};

/// `stance_recoil` per stance (GAMEDESIGN 9.5). 【待调参】
pub fn stance_recoil(s: Stance) -> f32 {
    match s {
        Stance::Stand => 1.00,
        Stance::Crouch => 0.75,
        Stance::Prone => 0.50,
    }
}

/// Pause after the last shot before recovery starts, ms (GAMEDESIGN 6.5 `recoil_delay_s`,
/// default 0.12 s; global constant, not per weapon). 【待调参】
pub const RECOIL_DELAY_MS: f64 = 120.0;
/// Relative random spread of the vertical kick (+-15 %, GAMEDESIGN 6.5).
pub const KICK_JITTER: f32 = 0.15;

/// Standard normal sample (Box-Muller, `libm`); consumes two numbers from `rng`.
pub fn gaussian(rng: &mut Pcg64Mcg) -> f32 {
    let u1 = 1.0 - unit_f32(rng); // (0, 1]
    let u2 = unit_f32(rng);
    libm::sqrtf(-2.0 * libm::logf(u1)) * libm::cosf(core::f32::consts::TAU * u2)
}

/// Kick of one shot `(d_pitch_deg, d_yaw_deg)`: `vkick * stance * (1 +- 15 %)` up and
/// `N(0, hsigma) * stance` sideways. Consumes three numbers from `rng`.
pub fn kick(rng: &mut Pcg64Mcg, vkick_deg: f32, hsigma_deg: f32, stance: Stance) -> (f32, f32) {
    let s = stance_recoil(stance);
    let jitter = 1.0 + (unit_f32(rng) * 2.0 - 1.0) * KICK_JITTER;
    let dy = gaussian(rng) * hsigma_deg * s;
    (vkick_deg * s * jitter, dy)
}

/// One tick of recovery: the recoil vector shrinks by `recover_deg_s * dt` along its length;
/// bloom by `bloom_decay_deg_s * dt`.
pub fn recover(
    pitch: &mut f32,
    yaw: &mut f32,
    bloom: &mut f32,
    recover_deg_s: f32,
    decay_deg_s: f32,
    dt: f32,
) {
    let mag = libm::sqrtf(*pitch * *pitch + *yaw * *yaw);
    if mag > 0.0 {
        let new = (mag - recover_deg_s * dt).max(0.0);
        let k = new / mag;
        *pitch *= k;
        *yaw *= k;
    }
    *bloom = (*bloom - decay_deg_s * dt).max(0.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::Tick;
    use crate::rng::{Purpose, Rng};

    #[test]
    fn kick_statistics() {
        let (mut sum_p, mut sum_y, mut sq_y) = (0.0f64, 0.0f64, 0.0f64);
        let n = 20_000;
        for i in 0..n {
            let mut r = Rng::for_(3, Tick(i), 1, Purpose::Spread);
            let (p, y) = kick(&mut r, 1.6, 0.25, Stance::Stand);
            assert!((1.6 * 0.85 - 1e-4..=1.6 * 1.15 + 1e-4).contains(&p));
            sum_p += p as f64;
            sum_y += y as f64;
            sq_y += (y * y) as f64;
        }
        let nf = n as f64;
        assert!((sum_p / nf - 1.6).abs() < 0.01, "mean pitch {}", sum_p / nf);
        assert!((sum_y / nf).abs() < 0.01, "mean yaw {}", sum_y / nf);
        let sigma = (sq_y / nf).sqrt();
        assert!((sigma - 0.25).abs() < 0.01, "sigma {sigma}");
        // Crouching scales both by 0.75.
        let mut r = Rng::for_(3, Tick(0), 1, Purpose::Spread);
        let (ps, ys) = kick(&mut r, 1.6, 0.25, Stance::Stand);
        let mut r = Rng::for_(3, Tick(0), 1, Purpose::Spread);
        let (pc, yc) = kick(&mut r, 1.6, 0.25, Stance::Crouch);
        assert!((pc - 0.75 * ps).abs() < 1e-6 && (yc - 0.75 * ys).abs() < 1e-6);
    }

    #[test]
    fn recovery_runs_at_data_rate_and_stops_at_zero() {
        let (mut p, mut y, mut b) = (1.2f32, 0.0f32, 0.4f32);
        let dt = 1.0 / 30.0;
        // 12 deg/s: 1.2 deg takes 0.1 s = 3 ticks; bloom 3 deg/s: 0.4 deg takes 4 ticks.
        for _ in 0..3 {
            recover(&mut p, &mut y, &mut b, 12.0, 3.0, dt);
        }
        assert!(p.abs() < 1e-5, "{p}");
        assert!(b > 0.0);
        recover(&mut p, &mut y, &mut b, 12.0, 3.0, dt);
        recover(&mut p, &mut y, &mut b, 12.0, 3.0, dt);
        assert_eq!((p, y, b), (0.0, 0.0, 0.0));
        // Diagonal vectors shrink along their direction.
        let (mut p, mut y, mut b) = (0.3f32, 0.4f32, 0.0f32);
        recover(&mut p, &mut y, &mut b, 6.0, 0.0, dt); // 0.2 deg
        assert!((p - 0.3 * 0.3 / 0.5).abs() < 1e-5 + 0.0 && (y - 0.4 * 0.3 / 0.5).abs() < 1e-5);
    }
}
