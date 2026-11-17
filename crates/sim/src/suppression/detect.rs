//! Near-miss detection (GAMEDESIGN 8.1): closest distance between a projectile's sub-step
//! segment and a player's capsule axis, and the resulting suppression amount.

use glam::Vec3;

use super::{caliber_weight, D_MAX_M, EXPONENT, SPEED_REF_MPS};

/// Closest distance between segments `p0 -> p1` and `q0 -> q1`.
pub fn seg_seg_distance(p0: Vec3, p1: Vec3, q0: Vec3, q1: Vec3) -> f32 {
    let d1 = p1 - p0;
    let d2 = q1 - q0;
    let r = p0 - q0;
    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);
    let (s, t);
    if a <= 1e-12 && e <= 1e-12 {
        return r.length();
    }
    if a <= 1e-12 {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= 1e-12 {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            let mut s0 = if denom > 1e-12 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t0 = (b * s0 + f) / e;
            if t0 < 0.0 {
                t0 = 0.0;
                s0 = (-c / a).clamp(0.0, 1.0);
            } else if t0 > 1.0 {
                t0 = 1.0;
                s0 = ((b - c) / a).clamp(0.0, 1.0);
            }
            s = s0;
            t = t0;
        }
    }
    ((p0 + d1 * s) - (q0 + d2 * t)).length()
}

/// Suppression `s = weight x (1 - d/d_max)^1.2 x min(1, v/500)` for a pass at distance `d`
/// (`0` beyond `d_max`).
pub fn amount(class: &str, d: f32, speed_mps: f32) -> f32 {
    if !(0.0..=D_MAX_M).contains(&d) {
        return 0.0;
    }
    caliber_weight(class)
        * libm::powf(1.0 - d / D_MAX_M, EXPONENT)
        * (speed_mps / SPEED_REF_MPS).min(1.0)
}

/// Capsule axis of a player at `foot` whose head is `head_y` above the feet: from 0.2 m under the
/// feet to 0.2 m above the head.
pub fn axis(foot: Vec3, head_y: f32) -> (Vec3, Vec3) {
    (
        foot - Vec3::Y * super::AXIS_MARGIN_M,
        foot + Vec3::Y * (head_y + super::AXIS_MARGIN_M),
    )
}

/// Broad phase: the segment's box, grown by `d_max`, touches the player's column
/// (feet -0.2 m .. 2.1 m).
pub fn may_pass(p0: Vec3, p1: Vec3, foot: Vec3) -> bool {
    let lo = p0.min(p1) - Vec3::splat(super::D_MAX_M);
    let hi = p0.max(p1) + Vec3::splat(super::D_MAX_M);
    foot.x >= lo.x
        && foot.x <= hi.x
        && foot.z >= lo.z
        && foot.z <= hi.z
        && foot.y + 2.1 >= lo.y
        && foot.y - 0.2 <= hi.y
}
