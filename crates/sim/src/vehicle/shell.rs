//! Vehicle shells and hit resolution (M1-023c; GD 14.4): shell data from the vehicle
//! definition, the armour zone under a hit, and the full penetrate / no-pen / ricochet
//! outcome applied to hull, modules and crew.

use glam::{Quat, Vec3};
use serde::Deserialize;

use super::armor::{self, ArmorResult, ShellKind, ShellParams};
use super::internal;
use super::modules::{ModuleId, MODULE_COUNT};
use super::Vehicle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellType {
    Ap,
    He,
    Smoke,
}

/// A shell entry of the main gun's `shells` list.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ShellDef {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: ShellType,
    #[serde(default)]
    pub caliber_mm: f32,
    pub muzzle_velocity_mps: f32,
    #[serde(default)]
    pub p100_mm: f32,
    #[serde(default)]
    pub drop_pct_per_100m: f32,
    #[serde(default)]
    pub structure_damage_hp: f32,
    #[serde(default)]
    pub he_pen_mm: f32,
    #[serde(default)]
    pub explosion_damage_max_hp: f32,
    #[serde(default)]
    pub explosion_radius_m: f32,
    #[serde(default)]
    pub cloud_radius_m: f32,
    #[serde(default)]
    pub duration_s: f32,
}

/// One armour zone: thickness and slope from the vertical.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zone {
    pub thickness_mm: f32,
    pub slope_deg: f32,
}

pub const ZONE_COUNT: usize = 9;
/// Zone order of the data file.
pub const ZONE_IDS: [&str; ZONE_COUNT] = [
    "hull_front_upper",
    "hull_front_lower",
    "hull_side",
    "hull_rear",
    "hull_top",
    "turret_front",
    "turret_side",
    "turret_rear",
    "turret_top",
];
const FRONT_UPPER: usize = 0;
const FRONT_LOWER: usize = 1;
const HULL_SIDE: usize = 2;
const HULL_REAR: usize = 3;
const HULL_TOP: usize = 4;
const TURRET_FRONT: usize = 5;
const TURRET_SIDE: usize = 6;
const TURRET_REAR: usize = 7;
const TURRET_TOP: usize = 8;

/// Height (hull-local) above which a hit lands on the turret rather than the hull.
pub const TURRET_BASE_Y: f32 = 0.35;
/// Height below which a front hit is the lower glacis.
pub const LOWER_GLACIS_Y: f32 = -0.3;

/// Ray (hull-local) against the hull box. Returns entry distance, the entry point and which
/// face was crossed: `(axis, sign)` with `sign` the outward normal's sign on that axis.
pub fn ray_obb(o: Vec3, d: Vec3, half: Vec3, max_len: f32) -> Option<(f32, Vec3, usize, f32)> {
    let (mut t0, mut t1) = (0.0f32, max_len);
    let mut face = (usize::MAX, 0.0f32);
    for a in 0..3 {
        if d[a].abs() < 1e-8 {
            if o[a].abs() > half[a] {
                return None;
            }
            continue;
        }
        let (mut ta, mut tb) = ((-half[a] - o[a]) / d[a], (half[a] - o[a]) / d[a]);
        let sign = if d[a] > 0.0 { -1.0 } else { 1.0 };
        if ta > tb {
            core::mem::swap(&mut ta, &mut tb);
        }
        if ta > t0 && (sign < 0.0) == (d[a] > 0.0) {
            t0 = ta;
            face = (a, sign);
        }
        t1 = t1.min(tb);
        if t0 > t1 {
            return None;
        }
    }
    if face.0 == usize::MAX {
        return None; // started inside the box
    }
    Some((t0, o + d * t0, face.0, face.1))
}

/// Turret ring half-extents (X, Z) of the placeholder turret box (PI-070).
pub const TURRET_HALF_XZ: (f32, f32) = (1.1, 1.2);

/// Armour zone index and the outward plate normal (in the frame of the box that was hit)
/// for a hit on `face` of the hull or turret box. `entry_y` is hull-local.
pub fn zone_and_normal(
    turret: bool,
    entry_y: f32,
    axis: usize,
    sign: f32,
    zones: &[Zone; ZONE_COUNT],
) -> (usize, Vec3) {
    let idx = match (axis, turret) {
        (1, false) => HULL_TOP,
        (1, true) => TURRET_TOP,
        (2, false) if sign > 0.0 => {
            if entry_y >= LOWER_GLACIS_Y {
                FRONT_UPPER
            } else {
                FRONT_LOWER
            }
        }
        (2, false) => HULL_REAR,
        (2, true) if sign > 0.0 => TURRET_FRONT,
        (2, true) => TURRET_REAR,
        (_, false) => HULL_SIDE,
        (_, true) => TURRET_SIDE,
    };
    let slope = zones[idx].slope_deg.to_radians();
    let mut horiz = Vec3::ZERO;
    horiz[axis] = sign;
    let n = if axis == 1 {
        Vec3::new(0.0, sign, 0.0)
    } else {
        (horiz * libm::cosf(slope) + Vec3::Y * libm::sinf(slope)).normalize()
    };
    (idx, n)
}

/// Result of one shell hit on a vehicle, for the caller to turn into events and crew damage.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HitReport {
    pub zone: usize,
    pub armor: Option<ArmorKind>,
    pub module_damage: [f32; MODULE_COUNT],
    pub crew_damage: [f32; 3],
    pub hull_damage: f32,
    pub cookoff: bool,
    pub engine_fire: bool,
    pub destroyed_modules: Vec<ModuleId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorKind {
    Pen,
    NoPen,
    Ricochet,
}

/// Fraction of structure damage a non-penetrating hit does to the hull (GD 14.4.1 step 9).
pub const NO_PEN_HULL_FRAC: f32 = 0.03;
/// Damage luck on structure damage, `U(0.9, 1.1)`.
pub fn dmg_luck(u: f32) -> f32 {
    0.9 + 0.2 * u
}

pub fn ap_params(s: &ShellDef) -> ShellParams {
    ShellParams {
        kind: ShellKind::Ap,
        caliber_mm: s.caliber_mm,
        p100_mm: s.p100_mm,
        drop_pct_per_100m: s.drop_pct_per_100m,
        structure_damage_hp: s.structure_damage_hp,
    }
}

/// Random draws for one hit, each in `[0, 1)`.
#[derive(Debug, Clone, Copy)]
pub struct HitRolls {
    pub armor: f32,
    pub damage: f32,
    pub cookoff: f32,
    pub fire: f32,
}

/// Where a ray meets the vehicle: distance along the ray, hull-local entry point and ray
/// direction, the armour zone and its outward normal (hull-local).
#[derive(Debug, Clone, Copy)]
pub struct Located {
    pub t: f32,
    pub entry: Vec3,
    pub dir: Vec3,
    pub zone: usize,
    pub normal: Vec3,
}

pub fn locate(v: &Vehicle, origin_world: Vec3, dir_world: Vec3, max_len: f32) -> Option<Located> {
    let inv: Quat = v.rot.inverse();
    let o = inv * (origin_world - v.pos);
    let d = inv * dir_world;
    let half = v.params.half_extents;
    // Hull box: up to the roof; turret box: above it, rotated by the turret yaw.
    let hull_half = Vec3::new(half.x, 0.5 * (TURRET_BASE_Y + half.y), half.z);
    let hull_c = Vec3::new(0.0, 0.5 * (TURRET_BASE_Y - half.y), 0.0);
    let tur_half = Vec3::new(
        TURRET_HALF_XZ.0,
        0.5 * (half.y - TURRET_BASE_Y),
        TURRET_HALF_XZ.1,
    );
    let tur_c = Vec3::new(0.0, 0.5 * (TURRET_BASE_Y + half.y), 0.0);
    let ty = Quat::from_rotation_y(v.turret.yaw);
    let hit_h = ray_obb(o - hull_c, d, hull_half, max_len);
    let hit_t = ray_obb(
        ty.inverse() * (o - tur_c),
        ty.inverse() * d,
        tur_half,
        max_len,
    );
    let (is_turret, t, entry, axis, sign) = match (hit_h, hit_t) {
        (Some(h), Some(t)) if t.0 < h.0 => (true, t.0, ty * (t.1) + tur_c, t.2, t.3),
        (Some(h), _) => (false, h.0, h.1 + hull_c, h.2, h.3),
        (None, Some(t)) => (true, t.0, ty * (t.1) + tur_c, t.2, t.3),
        (None, None) => return None,
    };
    let (zone, n_box) = zone_and_normal(is_turret, entry.y, axis, sign, &v.params.zones);
    let n = if is_turret { ty * n_box } else { n_box };
    Some(Located {
        t,
        entry,
        dir: d,
        zone,
        normal: n,
    })
}

/// Resolves an AP-type shell hitting `v` along `dir_world` from `origin_world`, `dist_m`
/// from the muzzle. `seat_alive` flags the crew. Mutates hull HP and modules.
#[allow(clippy::too_many_arguments)]
pub fn resolve_ap_hit(
    v: &mut Vehicle,
    shell: &ShellParams,
    origin_world: Vec3,
    dir_world: Vec3,
    dist_m: f32,
    seat_alive: [bool; 3],
    rolls: HitRolls,
    max_len: f32,
) -> Option<HitReport> {
    let loc = locate(v, origin_world, dir_world, max_len)?;
    let (entry, d, zone, n) = (loc.entry, loc.dir, loc.zone, loc.normal);
    let thickness = v.params.zones[zone].thickness_mm;
    let res = armor::armor_hit(
        shell,
        dist_m,
        n,
        d,
        thickness,
        armor::luck_factor(rolls.armor),
    );
    let mut rep = HitReport {
        zone,
        ..Default::default()
    };
    let structure = shell.structure_damage_hp;
    match res {
        ArmorResult::Ricochet => rep.armor = Some(ArmorKind::Ricochet),
        ArmorResult::NoPen { .. } => {
            rep.armor = Some(ArmorKind::NoPen);
            rep.hull_damage = NO_PEN_HULL_FRAC * structure;
        }
        ArmorResult::Pen { p_rem, .. } => {
            rep.armor = Some(ArmorKind::Pen);
            let dmg_pen = structure * dmg_luck(rolls.damage);
            let inner = internal::internal_damage(entry, d, p_rem, dmg_pen, seat_alive);
            rep.module_damage = inner.module;
            rep.crew_damage = inner.crew;
            rep.hull_damage = inner.hull;
        }
    }
    v.hull_hp -= rep.hull_damage;
    for id in ModuleId::ALL {
        let dm = rep.module_damage[id as usize];
        if dm > 0.0 {
            let out = v
                .modules
                .damage(&v.params.module_specs, id, dm, rolls.cookoff, rolls.fire);
            if out.destroyed {
                rep.destroyed_modules.push(id);
            }
            rep.cookoff |= out.cookoff;
            rep.engine_fire |= out.fire;
        }
    }
    Some(rep)
}
