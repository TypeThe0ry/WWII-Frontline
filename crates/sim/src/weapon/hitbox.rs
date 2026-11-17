//! Player hit volumes (ARCHITECTURE 6.7.1) and segment intersection.
//!
//! Four vertical capsules per player (head sphere, torso, thigh, shin) positioned relative to the
//! feet and blended between the standing, crouched and prone tables by `stance_blend` (0..2). The numbers are
//! initial values 【待调参】【待测 → REFERENCE_MEASUREMENTS M-15】; the target capsule used for
//! movement collision is a different (bigger) volume. Hit zones: the head sphere is `Head`, the
//! lowest quarter of the torso capsule `Pelvis`, the rest of it `Torso`, thigh and shin `Limb`.
//! `Neck` has no volume of its own in M0 (zone refinement is M1-014).

use glam::Vec3;

/// Body part that was hit; the damage multiplier comes from `gameplay.part_mult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HitZone {
    Head = 0,
    Neck = 1,
    Torso = 2,
    Pelvis = 3,
    Limb = 4,
}

/// One vertical capsule: axis from `(0, y0, 0)` to `(0, y1, 0)` relative to the feet, radius `r`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volume {
    pub y0: f32,
    pub y1: f32,
    pub r: f32,
    pub zone: HitZone,
}

const fn v(y0: f32, y1: f32, r: f32, zone: HitZone) -> Volume {
    Volume { y0, y1, r, zone }
}

/// Standing hit volumes (metres above the feet). 【待调参】
pub const STAND: [Volume; 4] = [
    v(1.66, 1.66, 0.13, HitZone::Head),
    v(1.05, 1.42, 0.20, HitZone::Torso),
    v(0.55, 0.85, 0.15, HitZone::Limb),
    v(0.12, 0.40, 0.11, HitZone::Limb),
];

/// Crouched hit volumes. 【待调参】
pub const CROUCH: [Volume; 4] = [
    v(1.06, 1.06, 0.13, HitZone::Head),
    v(0.62, 0.86, 0.20, HitZone::Torso),
    v(0.34, 0.52, 0.15, HitZone::Limb),
    v(0.10, 0.24, 0.11, HitZone::Limb),
];

/// Prone hit volumes (a body lying down is about 0.5 m tall). 【待调参】
pub const PRONE: [Volume; 4] = [
    v(0.40, 0.40, 0.11, HitZone::Head),
    v(0.22, 0.30, 0.20, HitZone::Torso),
    v(0.15, 0.24, 0.14, HitZone::Limb),
    v(0.10, 0.14, 0.10, HitZone::Limb),
];

/// Fraction of the torso capsule's height (from its bottom) that counts as `Pelvis`.
pub const PELVIS_FRACTION: f32 = 0.25;

/// Broad-phase box half-width around the feet (x / z) and height of the box, metres.
const BROAD_HALF_M: f32 = 0.45;
const BROAD_TOP_M: f32 = 1.9;

/// Hit volumes for a pose coordinate `0 (standing) .. 1 (crouched) .. 2 (prone)`.
pub fn volumes(blend: f32) -> [Volume; 4] {
    let t = blend.clamp(0.0, 2.0);
    let (from, to, b) = if t <= 1.0 {
        (&STAND, &CROUCH, t)
    } else {
        (&CROUCH, &PRONE, t - 1.0)
    };
    let mut out = *from;
    for (o, c) in out.iter_mut().zip(to.iter()) {
        o.y0 += (c.y0 - o.y0) * b;
        o.y1 += (c.y1 - o.y1) * b;
        o.r += (c.r - o.r) * b;
    }
    out
}

/// First point of the segment `p0 -> p1` (as a fraction `t` of the segment) inside the sphere.
fn seg_sphere(o: Vec3, d: Vec3, c: Vec3, r: f32) -> Option<f32> {
    let m = o - c;
    let cc = m.dot(m) - r * r;
    if cc <= 0.0 {
        return Some(0.0);
    }
    let a = d.dot(d);
    let b = m.dot(d);
    if a < 1e-12 || b > 0.0 {
        return None;
    }
    let disc = b * b - a * cc;
    if disc < 0.0 {
        return None;
    }
    let t = (-b - libm::sqrtf(disc)) / a;
    (t <= 1.0).then_some(t.max(0.0))
}

/// First hit of the segment `o + t d` (`t` in `[0, 1]`, `o` relative to the feet) with a
/// vertical capsule of axis x = z = 0, `y0..y1`, radius `r`.
pub fn seg_capsule(o: Vec3, d: Vec3, y0: f32, y1: f32, r: f32) -> Option<f32> {
    let mut best: Option<f32> = None;
    let mut keep = |t: f32| best = Some(best.map_or(t, |b| b.min(t)));
    // Caps.
    if let Some(t) = seg_sphere(o, d, Vec3::new(0.0, y0, 0.0), r) {
        keep(t);
    }
    if let Some(t) = seg_sphere(o, d, Vec3::new(0.0, y1, 0.0), r) {
        keep(t);
    }
    // Cylinder part.
    let cc = o.x * o.x + o.z * o.z - r * r;
    if cc <= 0.0 && o.y >= y0 && o.y <= y1 {
        return Some(0.0);
    }
    let a = d.x * d.x + d.z * d.z;
    let b = o.x * d.x + o.z * d.z;
    if a > 1e-12 && cc > 0.0 && b < 0.0 {
        let disc = b * b - a * cc;
        if disc >= 0.0 {
            let t = (-b - libm::sqrtf(disc)) / a;
            if (0.0..=1.0).contains(&t) {
                let y = o.y + t * d.y;
                if y >= y0 && y <= y1 {
                    keep(t);
                }
            }
        }
    }
    best
}

/// Segment vs. the broad-phase box of a player standing at `foot`.
pub fn seg_aabb_overlap(p0: Vec3, p1: Vec3, foot: Vec3) -> bool {
    let lo = Vec3::new(foot.x - BROAD_HALF_M, foot.y - 0.05, foot.z - BROAD_HALF_M);
    let hi = Vec3::new(
        foot.x + BROAD_HALF_M,
        foot.y + BROAD_TOP_M,
        foot.z + BROAD_HALF_M,
    );
    let d = p1 - p0;
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for i in 0..3 {
        let (o, dd, l, h) = (p0[i], d[i], lo[i], hi[i]);
        if dd.abs() < 1e-9 {
            if o < l || o > h {
                return false;
            }
        } else {
            let (mut a, mut b) = ((l - o) / dd, (h - o) / dd);
            if a > b {
                core::mem::swap(&mut a, &mut b);
            }
            t0 = t0.max(a);
            t1 = t1.min(b);
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

/// Result of [`segment_hit`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyHit {
    /// Fraction of the segment `p0 -> p1` at the entry point.
    pub t: f32,
    pub zone: HitZone,
}

/// Earliest hit of the segment `p0 -> p1` with the four volumes of a player at `foot`.
pub fn segment_hit(foot: Vec3, blend: f32, p0: Vec3, p1: Vec3) -> Option<BodyHit> {
    let o = p0 - foot;
    let d = p1 - p0;
    let mut best: Option<BodyHit> = None;
    for vol in volumes(blend) {
        if let Some(t) = seg_capsule(o, d, vol.y0, vol.y1, vol.r) {
            if best.is_none_or(|b| t < b.t) {
                let mut zone = vol.zone;
                if zone == HitZone::Torso {
                    let hit_y = o.y + t * d.y;
                    let lo = vol.y0 - vol.r;
                    let hi = vol.y1 + vol.r;
                    if hit_y < lo + PELVIS_FRACTION * (hi - lo) {
                        zone = HitZone::Pelvis;
                    }
                }
                best = Some(BodyHit { t, zone });
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_ray_hits_each_zone() {
        let foot = Vec3::new(10.0, 5.0, 10.0);
        let from = |y: f32| Vec3::new(0.0, 5.0 + y, 10.0);
        let to = |y: f32| Vec3::new(20.0, 5.0 + y, 10.0);
        let zone = |y: f32| segment_hit(foot, 0.0, from(y), to(y)).map(|h| h.zone);
        assert_eq!(zone(1.66), Some(HitZone::Head));
        assert_eq!(zone(1.30), Some(HitZone::Torso));
        assert_eq!(zone(0.95), Some(HitZone::Pelvis));
        assert_eq!(zone(0.70), Some(HitZone::Limb));
        assert_eq!(zone(0.25), Some(HitZone::Limb));
        assert_eq!(zone(1.95), None, "above the head");
        assert_eq!(
            zone(0.005),
            None,
            "below the shin capsule (bottom at 0.01 m)"
        );
        // Entry point of the torso: x = 10 - 0.20.
        let h = segment_hit(foot, 0.0, from(1.3), to(1.3)).unwrap();
        assert!((h.t * 20.0 - 9.8).abs() < 1e-3, "{}", h.t);
        // Beside the player.
        assert!(segment_hit(
            foot,
            0.0,
            Vec3::new(0.0, 6.3, 10.5),
            Vec3::new(20.0, 6.3, 10.5)
        )
        .is_none());
    }

    #[test]
    fn crouch_lowers_the_volumes() {
        let foot = Vec3::ZERO;
        let a = Vec3::new(-5.0, 1.66, 0.0);
        let b = Vec3::new(5.0, 1.66, 0.0);
        assert_eq!(segment_hit(foot, 0.0, a, b).unwrap().zone, HitZone::Head);
        assert!(
            segment_hit(foot, 1.0, a, b).is_none(),
            "head ducked below 1.66 m"
        );
        let a = Vec3::new(-5.0, 1.06, 0.0);
        let b = Vec3::new(5.0, 1.06, 0.0);
        assert_eq!(segment_hit(foot, 1.0, a, b).unwrap().zone, HitZone::Head);
        // Blend interpolates monotonically.
        assert!(volumes(0.5)[0].y0 < volumes(0.0)[0].y0 && volumes(0.5)[0].y0 > volumes(1.0)[0].y0);
    }

    #[test]
    fn vertical_and_inside_segments() {
        let foot = Vec3::ZERO;
        // Straight down onto the head from above: hits the head sphere top at 1.79.
        let h = segment_hit(
            foot,
            0.0,
            Vec3::new(0.0, 3.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        )
        .unwrap();
        assert_eq!(h.zone, HitZone::Head);
        assert!((3.0 - h.t * 3.0 - 1.79).abs() < 1e-3);
        // Starting inside a volume: t = 0.
        let h = segment_hit(
            foot,
            0.0,
            Vec3::new(0.0, 1.2, 0.0),
            Vec3::new(5.0, 1.2, 0.0),
        )
        .unwrap();
        assert_eq!(h.t, 0.0);
        // Too short to arrive.
        assert!(segment_hit(
            foot,
            0.0,
            Vec3::new(-5.0, 1.3, 0.0),
            Vec3::new(-1.0, 1.3, 0.0)
        )
        .is_none());
    }

    #[test]
    fn broad_phase_is_conservative() {
        let foot = Vec3::new(3.0, 2.0, 3.0);
        for y in [0.05, 0.5, 1.0, 1.5, 1.75] {
            let (p0, p1) = (
                Vec3::new(-10.0, 2.0 + y, 3.0),
                Vec3::new(10.0, 2.0 + y, 3.0),
            );
            if segment_hit(foot, 0.0, p0, p1).is_some() {
                assert!(seg_aabb_overlap(p0, p1, foot), "y {y}");
            }
        }
        assert!(!seg_aabb_overlap(
            Vec3::new(-10.0, 9.0, 3.0),
            Vec3::new(10.0, 9.0, 3.0),
            foot
        ));
        assert!(!seg_aabb_overlap(
            Vec3::new(-10.0, 3.0, 6.0),
            Vec3::new(10.0, 3.0, 6.0),
            foot
        ));
    }
}
