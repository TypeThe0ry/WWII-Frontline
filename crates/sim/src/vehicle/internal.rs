//! Behind-armour damage of a penetrating hit (M1-023c; GD 14.4.1 step 8): the shell travels
//! `internal_len` metres inside the hull; modules and crew on that segment are hurt, and
//! everyone sharing a compartment with a hit crewman takes spillover.
//!
//! The internal layout (module boxes, seat positions) is a Sherman placeholder fixed here
//! until the vehicle data carries it (PI-069). Coordinates are hull-local: +X left, +Y up,
//! +Z forward, origin at the hull centre.

use glam::Vec3;

use super::modules::{ModuleId, MODULE_COUNT};

pub const INTERNAL_BASE_M: f32 = 0.6;
pub const INTERNAL_PER_P_REM_M: f32 = 1.0 / 40.0;
pub const INTERNAL_MAX_M: f32 = 3.0;
pub const CREW_DIRECT_FRAC: f32 = 0.25;
pub const CREW_SPILL_FRAC: f32 = 0.08;
pub const MODULE_FRAC: f32 = 1.0;
pub const HULL_PEN_FRAC: f32 = 0.5;
pub const HULL_MODULE_FRAC: f32 = 0.2;
pub const CREW_HIT_RADIUS_M: f32 = 0.3;

/// Axis-aligned box `(min, max)` in hull-local space.
type Aabb = (Vec3, Vec3);

const MODULE_BOXES: [(ModuleId, Aabb); MODULE_COUNT] = [
    (
        ModuleId::Engine,
        (Vec3::new(-0.8, -0.8, -2.6), Vec3::new(0.8, 0.2, -1.0)),
    ),
    (
        ModuleId::TrackL,
        (Vec3::new(0.9, -1.3, -2.6), Vec3::new(1.4, -0.3, 2.4)),
    ),
    (
        ModuleId::TrackR,
        (Vec3::new(-1.4, -1.3, -2.6), Vec3::new(-0.9, -0.3, 2.4)),
    ),
    (
        ModuleId::TurretDrive,
        (Vec3::new(-0.6, 0.4, -0.6), Vec3::new(0.6, 0.7, 0.6)),
    ),
    (
        ModuleId::MainGun,
        (Vec3::new(-0.3, 0.7, 0.0), Vec3::new(0.3, 1.2, 0.8)),
    ),
    (
        ModuleId::AmmoRack,
        (Vec3::new(-1.0, -0.9, 0.2), Vec3::new(1.0, -0.2, 1.2)),
    ),
];

/// Crew positions per seat (driver, gunner, commander) and their compartment (0 hull, 1 turret).
pub const CREW: [(Vec3, u8); 3] = [
    (Vec3::new(0.0, -0.2, 1.4), 0),
    (Vec3::new(0.0, 0.8, 0.0), 1),
    (Vec3::new(0.0, 1.0, -0.3), 1),
];

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InternalDamage {
    pub module: [f32; MODULE_COUNT],
    pub crew: [f32; 3],
    pub hull: f32,
    pub internal_len: f32,
}

pub fn internal_len(p_rem: f32) -> f32 {
    (INTERNAL_BASE_M + p_rem * INTERNAL_PER_P_REM_M).min(INTERNAL_MAX_M)
}

fn ray_box(o: Vec3, d: Vec3, len: f32, b: &Aabb) -> bool {
    let (mut t0, mut t1) = (0.0f32, len);
    for a in 0..3 {
        let (oa, da, lo, hi) = (o[a], d[a], b.0[a], b.1[a]);
        if da.abs() < 1e-6 {
            if oa < lo || oa > hi {
                return false;
            }
        } else {
            let (mut ta, mut tb) = ((lo - oa) / da, (hi - oa) / da);
            if ta > tb {
                core::mem::swap(&mut ta, &mut tb);
            }
            t0 = t0.max(ta);
            t1 = t1.min(tb);
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

fn ray_sphere(o: Vec3, d: Vec3, len: f32, c: Vec3, r: f32) -> bool {
    let t = (c - o).dot(d).clamp(0.0, len);
    (o + d * t - c).length_squared() <= r * r
}

/// `entry` and `dir` (unit) are hull-local; `dmg_pen` is the rolled structure damage.
/// `seat_alive[i]` says whether the crewman in seat `i` exists (dead/empty seats take none).
pub fn internal_damage(
    entry: Vec3,
    dir: Vec3,
    p_rem: f32,
    dmg_pen: f32,
    seat_alive: [bool; 3],
) -> InternalDamage {
    let len = internal_len(p_rem);
    let mut out = InternalDamage {
        internal_len: len,
        ..Default::default()
    };
    let mut module_total = 0.0;
    for (id, b) in MODULE_BOXES {
        if ray_box(entry, dir, len, &b) {
            let d = dmg_pen * MODULE_FRAC;
            out.module[id as usize] += d;
            module_total += d;
        }
    }
    let mut hit_comp = [false; 2];
    for (i, (pos, comp)) in CREW.iter().enumerate() {
        if seat_alive[i] && ray_sphere(entry, dir, len, *pos, CREW_HIT_RADIUS_M) {
            out.crew[i] += dmg_pen * CREW_DIRECT_FRAC;
            hit_comp[usize::from(*comp)] = true;
        }
    }
    for (i, (_, comp)) in CREW.iter().enumerate() {
        if seat_alive[i] && hit_comp[usize::from(*comp)] && out.crew[i] == 0.0 {
            out.crew[i] += dmg_pen * CREW_SPILL_FRAC;
        }
    }
    out.hull = dmg_pen * HULL_PEN_FRAC + module_total * HULL_MODULE_FRAC;
    out
}
