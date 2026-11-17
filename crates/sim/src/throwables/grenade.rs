//! Thrown-grenade flight and fuse (GAMEDESIGN 6.9.1): gravity, four sub-steps per tick, bounce
//! with restitution and tangential friction against the terrain, rest detection.

use glam::Vec3;

use crate::collision::CollisionData;

pub const GRAVITY_MPS2: f32 = 9.81;
pub const SUBSTEPS: u32 = 4;
/// Below this speed for [`REST_TICKS`] ticks in a row the grenade stops.
pub const REST_SPEED_MPS: f32 = 0.3;
pub const REST_TICKS: u8 = 3;
/// Distance a bounced grenade is pushed off the surface, metres.
const SKIN_M: f32 = 0.02;
/// The throw leaves the hand pitched this much above the view line (GAMEDESIGN 6.9.1).
pub const THROW_LIFT_DEG: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flight {
    pub pos: Vec3,
    pub vel: Vec3,
    pub rest: u8,
}

/// Advances one tick of flight at `hz`; `restitution` scales the normal velocity kept on a bounce,
/// `friction` the tangential one.
pub fn step(f: &mut Flight, col: &CollisionData, hz: u8, restitution: f32, friction: f32) {
    let h = 1.0 / (f32::from(hz) * SUBSTEPS as f32);
    if f.vel == Vec3::ZERO {
        f.rest = REST_TICKS;
        return;
    }
    let mut touched = false;
    for _ in 0..SUBSTEPS {
        f.vel.y -= GRAVITY_MPS2 * h;
        let p0 = f.pos;
        let seg = f.vel * h;
        let len = seg.length();
        if len < 1e-9 {
            continue;
        }
        match col.raycast(p0, seg, len) {
            Some(hit) => {
                touched = true;
                let n = hit.normal;
                let vn = n * f.vel.dot(n);
                let vt = f.vel - vn;
                f.vel = vt * friction - vn * restitution;
                f.pos = hit.point + n * SKIN_M;
            }
            None => f.pos = p0 + seg,
        }
    }
    // One tick of gravity alone adds `g / hz` m/s, so that much vertical speed still counts as rest.
    let slack = GRAVITY_MPS2 / f32::from(hz);
    let horiz = libm::sqrtf(f.vel.x * f.vel.x + f.vel.z * f.vel.z);
    let near = col
        .height_at(f.pos.x, f.pos.z)
        .is_some_and(|g| f.pos.y - g < 0.1);
    if (touched || near) && horiz < REST_SPEED_MPS && libm::fabsf(f.vel.y) < REST_SPEED_MPS + slack
    {
        f.rest = f.rest.saturating_add(1);
        if f.rest >= REST_TICKS {
            f.vel = Vec3::ZERO;
        }
    } else {
        f.rest = 0;
    }
}
