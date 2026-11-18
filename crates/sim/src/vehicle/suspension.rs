//! Raycast spring suspension: each mount casts down the hull's -Y axis onto the terrain.

use glam::Vec3;

use super::Vehicle;
use crate::collision::CollisionData;

/// Extra ray length below the rest length that still produces force (bump travel).
pub const BUMP_TRAVEL_M: f32 = 0.4;
/// Mount height below the hull centre, metres.
pub const MOUNT_DROP_M: f32 = 0.2;

#[derive(Debug, Clone, Copy)]
pub struct RayHit {
    /// Mount in world space.
    pub mount: Vec3,
    /// Mount offset from the centre of mass, world axes.
    pub arm: Vec3,
    /// Distance along the ray to the ground.
    pub dist: f32,
    /// +1 right-hand-side track (local x > 0), -1 left.
    pub side: f32,
}

/// Local mount positions: `rays_per_side` per track, evenly spaced over 80% of the length.
pub fn mount_local(v: &Vehicle, i: u32, side: f32) -> Vec3 {
    let n = v.params.rays_per_side.max(2);
    let len = v.params.half_extents.z * 2.0 * 0.8;
    let z = -len / 2.0 + len * i as f32 / (n - 1) as f32;
    Vec3::new(side * v.params.half_gauge, -MOUNT_DROP_M, z)
}

/// Ground distance along `dir` (unit, pointing down) from `origin`; `None` when the ground is
/// farther than `max_len` or off the loaded terrain. Two fixed-point passes on the height
/// field; a mount below the ground gives a negative distance.
pub fn ground_distance(col: &CollisionData, origin: Vec3, dir: Vec3, max_len: f32) -> Option<f32> {
    if dir.y > -0.05 {
        return None;
    }
    let mut t = (col.height_at(origin.x, origin.z)? - origin.y) / dir.y;
    for _ in 0..2 {
        let p = origin + dir * t;
        t = (col.height_at(p.x, p.z)? - origin.y) / dir.y;
    }
    (t <= max_len).then_some(t)
}
