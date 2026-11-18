//! Hull collisions, crushing and overturn (GD 7.6, 14.5).
//!
//! The sim has no static props yet, so "walls" are terrain: a probe row at the nose (tail
//! when reversing) a little above the belly meets terrain that rises more than the tracks
//! can climb. Impact damage `0.5 v^2` above 25 km/h (v in m/s).

use glam::Vec3;

use super::seats::KPH;
use super::Vehicle;
use crate::collision::CollisionData;

pub const IMPACT_MIN_MPS: f32 = 25.0 * KPH;
/// Crush bands: at or above KIA speed the victim dies, above the hurt speed takes 70 HP.
pub const CRUSH_KIA_MPS: f32 = 15.0 * KPH;
pub const CRUSH_HURT_MPS: f32 = 5.0 * KPH;
pub const CRUSH_HURT_HP: f32 = 70.0;
/// Same victim is not crushed again inside this many ticks' worth of seconds.
pub const CRUSH_REPEAT_S: f32 = 2.0;
/// Overturn: tilt beyond this for [`OVERTURN_S`].
pub const OVERTURN_DEG: f32 = 70.0;
pub const OVERTURN_S: f32 = 3.0;
/// Probe height above the mount line, metres (belly clearance).
const PROBE_UP_M: f32 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Impact {
    /// Speed into the obstacle, m/s.
    pub speed: f32,
    /// Horizontal outward normal of the obstacle.
    pub normal: Vec3,
}

/// Probes the leading edge; `Some` when it is inside rising terrain and moving into it.
pub fn probe(v: &Vehicle, col: &CollisionData) -> Option<Impact> {
    let fwd_speed = v.forward_speed();
    if fwd_speed.abs() < 0.05 {
        return None;
    }
    let dir = if fwd_speed > 0.0 { 1.0 } else { -1.0 };
    let he = v.params.half_extents;
    let mut worst: Option<Impact> = None;
    for fx in [-0.8f32, 0.0, 0.8] {
        let local = Vec3::new(
            fx * he.x,
            -super::suspension::MOUNT_DROP_M + PROBE_UP_M,
            dir * he.z,
        );
        let p = v.pos + v.rot * local;
        let Some(h) = col.height_at(p.x, p.z) else {
            continue;
        };
        if h <= p.y {
            continue;
        }
        let n = col.normal_at(p.x, p.z).unwrap_or(Vec3::Y);
        let mut nh = Vec3::new(n.x, 0.0, n.z);
        nh = if nh.length() > 1e-3 {
            nh.normalize()
        } else {
            -v.forward() * dir
        };
        let into = -v.vel.dot(nh);
        if into <= 0.0 {
            continue;
        }
        if worst.is_none_or(|w| into > w.speed) {
            worst = Some(Impact {
                speed: into,
                normal: nh,
            });
        }
    }
    worst
}

/// Whether `pos` is under the hull footprint (with a margin) and at a similar height.
pub fn under_hull(v: &Vehicle, pos: Vec3) -> bool {
    let local = v.rot.inverse() * (pos - v.pos);
    let he = v.params.half_extents;
    local.x.abs() < he.x + 0.2 && local.z.abs() < he.z && local.y.abs() < 1.6
}

/// Hull tilt from upright, degrees.
pub fn tilt_deg(v: &Vehicle) -> f32 {
    v.up().dot(Vec3::Y).clamp(-1.0, 1.0).acos().to_degrees()
}
