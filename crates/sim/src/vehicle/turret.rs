//! Turret and gun elevation (M1-023b; GD 14.2): the gunner's aim is a *target* direction and
//! the turret slews towards it at a limited rate; elevation is clamped to the gun's limits.

/// Gun elevation rate as a fraction of the traverse rate (not in data yet; PI-068).
pub const ELEVATION_RATE_FRACTION: f32 = 0.5;
/// Traverse rate with the turret drive jammed (HP 0), deg/s (GD 14.5).
pub const JAMMED_TRAVERSE_DEG_S: f32 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurretParams {
    pub traverse_deg_s: f32,
    pub elev_min_deg: f32,
    pub elev_max_deg: f32,
}

/// Turret yaw relative to the hull (radians, wrapped to (-pi, pi]) and gun pitch (radians,
/// positive up).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Turret {
    pub yaw: f32,
    pub pitch: f32,
}

fn wrap_pi(a: f32) -> f32 {
    let tau = core::f32::consts::TAU;
    let mut x = a % tau;
    if x > core::f32::consts::PI {
        x -= tau;
    } else if x <= -core::f32::consts::PI {
        x += tau;
    }
    x
}

impl Turret {
    /// Slews towards `(target_yaw, target_pitch)` (hull-relative radians) for one tick.
    /// `rate_mult` is the module effect on traverse (1.0, 0.5 at turret drive <= 50% HP);
    /// `jammed` forces [`JAMMED_TRAVERSE_DEG_S`].
    pub fn step(
        &mut self,
        p: &TurretParams,
        target_yaw: f32,
        target_pitch: f32,
        rate_mult: f32,
        jammed: bool,
        dt: f32,
    ) {
        let rate_deg = if jammed {
            JAMMED_TRAVERSE_DEG_S
        } else {
            p.traverse_deg_s * rate_mult
        };
        let max_yaw = rate_deg.to_radians() * dt;
        let dy = wrap_pi(target_yaw - self.yaw);
        self.yaw = wrap_pi(self.yaw + dy.clamp(-max_yaw, max_yaw));
        let max_pitch = (p.traverse_deg_s * ELEVATION_RATE_FRACTION)
            .min(rate_deg)
            .to_radians()
            * dt;
        let tp = target_pitch.clamp(p.elev_min_deg.to_radians(), p.elev_max_deg.to_radians());
        let dp = tp - self.pitch;
        self.pitch += dp.clamp(-max_pitch, max_pitch);
    }
}
