//! Track drive: per-side speed targets, friction-limited traction at each suspension contact,
//! lateral grip and the slope cut-off (GD 14.3, 14.7).

use glam::Vec3;

use super::suspension::{ground_distance, mount_local, BUMP_TRAVEL_M};
use super::{Vehicle, REVERSE_FRACTION, TRACK_TAU_S};
use crate::collision::CollisionData;

/// Moves the slewed forward speed target toward the driver's command.
pub fn update_speed_target(v: &mut Vehicle, dt: f32) {
    let p = &v.params;
    let c = v.ctrl;
    let desired = if c.brake {
        0.0
    } else if c.throttle >= 0.0 {
        c.throttle.min(1.0) * p.max_speed
    } else {
        c.throttle.max(-1.0) * p.max_speed * REVERSE_FRACTION
    };
    // Speeding up takes `accel_time_s`; letting off or reversing is quicker, braking quickest.
    let rate = if c.brake {
        5.0 * p.accel
    } else if desired.abs() > v.speed_target.abs() && desired * v.speed_target >= 0.0 {
        p.accel
    } else {
        3.0 * p.accel
    };
    let step = rate * dt;
    v.speed_target += (desired - v.speed_target).clamp(-step, step);
}

/// Fraction of drive force left on the current slope: full up to `max_slope - 5` degrees,
/// zero from `max_slope` (GD 14.3).
pub fn drive_scale(slope_deg: f32, max_slope_deg: f32) -> f32 {
    ((max_slope_deg - slope_deg) / 5.0).clamp(0.0, 1.0)
}

/// Net force, torque about the centre of mass and the number of grounded rays for one
/// sub-step of length `h`.
pub fn forces(v: &Vehicle, col: &CollisionData, h: f32) -> (Vec3, Vec3, u32) {
    let p = &v.params;
    let fwd = v.forward();
    let up = v.up();
    let lat = v.rot * Vec3::X;
    let n = p.rays_per_side.max(2);
    let rays = 2 * n;
    let slope = col.slope_deg_at(v.pos.x, v.pos.z).unwrap_or(0.0);
    let scale = drive_scale(slope, p.max_slope_deg);
    let ctrl_turn = v.ctrl.steer.clamp(-1.0, 1.0) * p.turn_rate;
    let v_lat = v.vel.dot(lat);
    let k_track = p.mass / rays as f32 / TRACK_TAU_S;
    let drive_cap = p.track_force_max / rays as f32;

    let mut force = Vec3::ZERO;
    let mut torque = Vec3::ZERO;
    let mut grounded = 0;
    for side in [-1.0f32, 1.0] {
        // Local +X is the left-hand side; steering right speeds up the left track.
        let mut v_side = v.speed_target + side * ctrl_turn * p.half_gauge;
        v_side = v_side.clamp(-p.max_speed * REVERSE_FRACTION, p.max_speed);
        if scale <= 0.0 {
            v_side = 0.0;
        }
        for i in 0..n {
            let arm = v.rot * mount_local(v, i, side);
            let mount = v.pos + arm;
            let Some(dist) = ground_distance(col, mount, -up, p.rest_length) else {
                continue;
            };
            grounded += 1;
            let compression = (p.rest_length - dist).min(p.rest_length + BUMP_TRAVEL_M);
            let v_c = v.vel + v.ang_vel.cross(arm);
            let normal = (p.stiffness * compression - p.damping * v_c.dot(up)).max(0.0);
            let mut f = up * normal;
            // Traction on the ground plane, limited by an ellipse (forward / lateral grip).
            let v_cf = v_c.dot(fwd);
            let slowing = v_side.abs() < v_cf.abs() || v_side * v_cf < 0.0;
            let fwd_fric = p.mu_fwd * normal;
            let cap = if slowing || scale < 1.0 && v_side == 0.0 {
                fwd_fric
            } else {
                (drive_cap * scale.max(0.0)).min(fwd_fric)
            };
            let mut f_t = (k_track * (v_side - v_cf)).clamp(-cap, cap);
            let lat_fric = p.mu_lat * normal;
            let mut f_l = (-k_track * v_lat).clamp(-lat_fric, lat_fric);
            if fwd_fric > 0.0 && lat_fric > 0.0 {
                let e = (f_t / fwd_fric).powi(2) + (f_l / lat_fric).powi(2);
                if e > 1.0 {
                    let s = 1.0 / libm::sqrtf(e);
                    f_t *= s;
                    f_l *= s;
                }
            }
            f += fwd * f_t + lat * f_l;
            force += f;
            torque += arm.cross(f);
        }
    }
    let _ = h;
    (force, torque, grounded)
}
