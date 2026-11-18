//! Snapshot export (M1-010 / M1-024): `VehicleView` and the packed `VehicleExtra`.
//!
//! `VehicleExtra` is 12 bytes: orientation as smallest-three (2-bit index + 3 x 10-bit,
//! 4 B), planar velocity as two `i16` in 1/256 m/s (4 B, for extrapolation), yaw rate as
//! `i16` in 1/1024 rad/s (2 B) and hull health as `u8` (1 B) + 1 B flags. Well inside the
//! 24 B per-entity delta budget.

use glam::{Quat, Vec3};

use super::Vehicle;
use crate::components::Team;
use crate::ids::EntityId;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleView {
    pub id: EntityId,
    pub team: Team,
    pub pos: Vec3,
    pub rot: Quat,
    pub vel: Vec3,
    pub hull_hp: f32,
    pub driver: Option<crate::ids::PlayerId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleExtra {
    pub rot: u32,
    pub vx: i16,
    pub vz: i16,
    pub yaw_rate: i16,
    pub hull_q: u8,
    pub flags: u8,
}

pub const EXTRA_BYTES: usize = 12;
const SQRT_HALF: f32 = core::f32::consts::FRAC_1_SQRT_2;

fn pack_quat(q: Quat) -> u32 {
    let a = [q.x, q.y, q.z, q.w];
    let mut big = 0;
    for i in 1..4 {
        if a[i].abs() > a[big].abs() {
            big = i;
        }
    }
    let sign = if a[big] < 0.0 { -1.0 } else { 1.0 };
    let mut out = big as u32;
    let mut shift = 2;
    for (i, &c) in a.iter().enumerate() {
        if i == big {
            continue;
        }
        let v = (c * sign / SQRT_HALF).clamp(-1.0, 1.0);
        let q10 = libm::roundf((v * 0.5 + 0.5) * 1023.0) as u32;
        out |= q10 << shift;
        shift += 10;
    }
    out
}

fn unpack_quat(bits: u32) -> Quat {
    let big = (bits & 3) as usize;
    let mut a = [0.0f32; 4];
    let mut shift = 2;
    let mut sum = 0.0;
    for (i, slot) in a.iter_mut().enumerate() {
        if i == big {
            continue;
        }
        let q10 = (bits >> shift) & 1023;
        shift += 10;
        let v = ((q10 as f32 / 1023.0) - 0.5) * 2.0 * SQRT_HALF;
        *slot = v;
        sum += v * v;
    }
    a[big] = libm::sqrtf((1.0 - sum).max(0.0));
    Quat::from_xyzw(a[0], a[1], a[2], a[3])
}

impl VehicleExtra {
    pub fn from_vehicle(v: &Vehicle) -> Self {
        let q = |x: f32, s: f32| libm::roundf((x * s).clamp(-32767.0, 32767.0)) as i16;
        VehicleExtra {
            rot: pack_quat(v.rot),
            vx: q(v.vel.x, 256.0),
            vz: q(v.vel.z, 256.0),
            yaw_rate: q(v.ang_vel.y, 1024.0),
            hull_q: crate::health::quant_health(v.hull_hp, v.params.hull_hp, 8),
            flags: u8::from(v.overturned),
        }
    }

    pub fn rotation(&self) -> Quat {
        unpack_quat(self.rot)
    }

    pub fn encode(&self) -> [u8; EXTRA_BYTES] {
        let mut b = [0u8; EXTRA_BYTES];
        b[0..4].copy_from_slice(&self.rot.to_le_bytes());
        b[4..6].copy_from_slice(&self.vx.to_le_bytes());
        b[6..8].copy_from_slice(&self.vz.to_le_bytes());
        b[8..10].copy_from_slice(&self.yaw_rate.to_le_bytes());
        b[10] = self.hull_q;
        b[11] = self.flags;
        b
    }

    pub fn decode(b: &[u8; EXTRA_BYTES]) -> Self {
        VehicleExtra {
            rot: u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            vx: i16::from_le_bytes([b[4], b[5]]),
            vz: i16::from_le_bytes([b[6], b[7]]),
            yaw_rate: i16::from_le_bytes([b[8], b[9]]),
            hull_q: b[10],
            flags: b[11],
        }
    }
}

impl Vehicle {
    pub fn view(&self) -> VehicleView {
        VehicleView {
            id: self.id,
            team: self.team,
            pos: self.pos,
            rot: self.rot,
            vel: self.vel,
            hull_hp: self.hull_hp,
            driver: self.seats.first().copied().flatten().map(|o| o.player),
        }
    }
}
