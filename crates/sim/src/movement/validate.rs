//! Read-only movement validation for the server's anti-cheat (ARCHITECTURE 15.1, GAMEDESIGN 9.6).

use glam::Vec3;

use super::params::MoveParams;
use super::stance::Stance;

/// Server tolerance on top of the stance maximum.
pub const TOLERANCE: f32 = 1.15;

/// Fastest horizontal speed a player can legitimately reach in `stance` (no load penalty).
pub fn max_speed_for(stance: Stance, p: &MoveParams) -> f32 {
    match stance {
        Stance::Stand => p.sprint_mps,
        Stance::Crouch => p.crouch_run_mps,
        Stance::Prone => p.prone_fast_mps,
    }
}

/// Limits for a stretch of ticks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveBound {
    /// Horizontal speed limit including the tolerance.
    pub max_horizontal_mps: f32,
    /// Largest per-tick speed change the ramps allow (m/s^2), for the same stance.
    pub max_accel_mps2: f32,
}

impl MoveBound {
    /// Bound for a player who was in `a` and is now in `b` (the larger limit applies while
    /// the stance changes).
    pub fn new(a: Stance, b: Stance, p: &MoveParams) -> Self {
        let top = max_speed_for(a, p).max(max_speed_for(b, p));
        let ramp = p.accel_time_s.min(p.decel_time_s);
        Self {
            max_horizontal_mps: top * TOLERANCE,
            max_accel_mps2: if ramp > 0.0 {
                p.sprint_mps / ramp
            } else {
                f32::INFINITY
            },
        }
    }
}

/// A displacement faster than allowed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Violation {
    /// Observed average horizontal speed over the window, m/s.
    pub observed_mps: f32,
    pub limit_mps: f32,
}

/// Checks the horizontal displacement `prev -> next` over `ticks` ticks. Vertical motion
/// (falls, jumps) is not bounded here.
pub fn check_displacement(
    prev: Vec3,
    next: Vec3,
    ticks: u32,
    from: Stance,
    to: Stance,
    p: &MoveParams,
) -> Result<(), Violation> {
    if ticks == 0 {
        return Ok(());
    }
    let bound = MoveBound::new(from, to, p);
    let dx = next.x - prev.x;
    let dz = next.z - prev.z;
    let observed = libm::sqrtf(dx * dx + dz * dz) / (ticks as f32 * p.dt);
    if observed.is_finite() && observed <= bound.max_horizontal_mps {
        Ok(())
    } else {
        Err(Violation {
            observed_mps: observed,
            limit_mps: bound.max_horizontal_mps,
        })
    }
}
