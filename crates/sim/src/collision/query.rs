//! Terrain queries: height, normal, slope, ground probe, ray cast, line of sight.
//!
//! The surface is the Rapier height-field triangulation: per 1 m cell, corners
//! `a=(x0,z0) b=(x1,z0) c=(x0,z1) d=(x1,z1)`, triangles `[a,c,b]` (where `u+v <= 1`) and
//! `[c,d,b]`, diagonal `c-b`. Local `u` runs along +x, `v` along +z.

use glam::Vec3;
use rapier3d::prelude::{QueryFilter, Ray, Vector};

use super::{CollisionChunk, CollisionData, CHUNK_CELLS, CHUNK_SIZE_M, MAX_WALK_SLOPE_DEG};

const RAD_TO_DEG: f32 = 57.295_78;
const BARY_EPS: f32 = 1e-5;

/// Result of [`CollisionData::ground_probe`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundHit {
    pub point: Vec3,
    pub normal: Vec3,
    pub slope_deg: f32,
    pub material: u8,
}

/// Result of [`CollisionData::raycast`]; `t` is in metres along the normalised direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub material: u8,
}

struct Trace {
    hit: Option<RayHit>,
    /// Part of the segment lies outside the loaded chunks.
    missing: bool,
}

#[inline]
fn floor_i(v: f32) -> i32 {
    libm::floorf(v) as i32
}

#[inline]
fn slope_of(normal_y: f32) -> f32 {
    libm::acosf(normal_y.clamp(-1.0, 1.0)) * RAD_TO_DEG
}

/// Heights of the four cell corners `[a, b, c, d]` (see module docs).
#[inline]
fn corners(ch: &CollisionChunk, i: usize, j: usize) -> [f32; 4] {
    [
        ch.sample(i, j),
        ch.sample(i + 1, j),
        ch.sample(i, j + 1),
        ch.sample(i + 1, j + 1),
    ]
}

/// Triangle surface of one cell at local `(u, v)`: `(height, unit normal)`.
#[inline]
fn tri_surface(h: [f32; 4], u: f32, v: f32) -> (f32, Vec3) {
    let [a, b, c, d] = h;
    let (y, dx, dz) = if u + v <= 1.0 {
        (a + u * (b - a) + v * (c - a), b - a, c - a)
    } else {
        (d + (1.0 - u) * (c - d) + (1.0 - v) * (b - d), d - c, d - b)
    };
    let inv = 1.0 / libm::sqrtf(dx * dx + 1.0 + dz * dz);
    (y, Vec3::new(-dx * inv, inv, -dz * inv))
}

/// Moeller-Trumbore; returns `t >= 0` on hit (double sided).
#[inline]
fn ray_tri(o: Vec3, d: Vec3, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let tv = o - v0;
    let u = tv.dot(p) * inv;
    if !(-BARY_EPS..=1.0 + BARY_EPS).contains(&u) {
        return None;
    }
    let q = tv.cross(e1);
    let v = d.dot(q) * inv;
    if v < -BARY_EPS || u + v > 1.0 + BARY_EPS {
        return None;
    }
    let t = e2.dot(q) * inv;
    if t < 0.0 {
        None
    } else {
        Some(t)
    }
}

impl CollisionData {
    /// Chunk and local sample coordinates (`0..=64`) of a world position. A point exactly on a
    /// chunk border resolves to the neighbour when the primary chunk is missing.
    fn locate(&self, x: f32, z: f32) -> Option<(&CollisionChunk, f32, f32)> {
        if !x.is_finite() || !z.is_finite() {
            return None;
        }
        let bx = floor_i(x / CHUNK_SIZE_M);
        let bz = floor_i(z / CHUNK_SIZE_M);
        let lx = x - bx as f32 * CHUNK_SIZE_M;
        let lz = z - bz as f32 * CHUNK_SIZE_M;
        let xs: [(i32, f32); 2] = [(bx, lx), (bx - 1, CHUNK_SIZE_M)];
        let zs: [(i32, f32); 2] = [(bz, lz), (bz - 1, CHUNK_SIZE_M)];
        let nx = if lx == 0.0 { 2 } else { 1 };
        let nz = if lz == 0.0 { 2 } else { 1 };
        for &(cz, lz) in &zs[..nz] {
            for &(cx, lx) in &xs[..nx] {
                if let Some(ch) = self.chunks.get(&(cz, cx)) {
                    return Some((ch, lx, lz));
                }
            }
        }
        None
    }

    /// `(chunk, cell i, cell j, u, v)` for a world position.
    fn locate_cell(&self, x: f32, z: f32) -> Option<(&CollisionChunk, usize, usize, f32, f32)> {
        let (ch, lx, lz) = self.locate(x, z)?;
        let max = (CHUNK_CELLS - 1) as i32;
        let i = floor_i(lx).clamp(0, max);
        let j = floor_i(lz).clamp(0, max);
        Some((ch, i as usize, j as usize, lx - i as f32, lz - j as f32))
    }

    /// Checks terrain coverage with the same chunk-border rules as `height_at`.
    pub(crate) fn has_terrain_at(&self, x: f32, z: f32) -> bool {
        self.locate(x, z).is_some()
    }

    /// Bilinear terrain height; `None` where no chunk is loaded.
    pub fn height_at(&self, x: f32, z: f32) -> Option<f32> {
        let (ch, i, j, u, v) = self.locate_cell(x, z)?;
        let [a, b, c, d] = corners(ch, i, j);
        let lo = a + u * (b - a);
        let hi = c + u * (d - c);
        Some(lo + v * (hi - lo))
    }

    /// Triangle-surface height, normal and material (the surface Rapier collides with).
    fn surface_at(&self, x: f32, z: f32) -> Option<(f32, Vec3, u8)> {
        let (ch, i, j, u, v) = self.locate_cell(x, z)?;
        let (y, n) = tri_surface(corners(ch, i, j), u, v);
        Some((y, n, ch.material(i, j)))
    }

    /// Material id of the terrain cell under `(x, z)`.
    pub fn material_at(&self, x: f32, z: f32) -> Option<u8> {
        self.surface_at(x, z).map(|s| s.2)
    }

    /// Unit surface normal (triangle of the cell containing the point).
    pub fn normal_at(&self, x: f32, z: f32) -> Option<Vec3> {
        self.surface_at(x, z).map(|s| s.1)
    }

    /// Angle between the surface normal and +Y, degrees.
    pub fn slope_deg_at(&self, x: f32, z: f32) -> Option<f32> {
        self.normal_at(x, z).map(|n| slope_of(n.y))
    }

    /// `slope <= MAX_WALK_SLOPE_DEG`; `false` where no chunk is loaded.
    pub fn walkable(&self, x: f32, z: f32) -> bool {
        self.slope_deg_at(x, z)
            .is_some_and(|s| s <= MAX_WALK_SLOPE_DEG)
    }

    /// Ground below `pos`: hit when the surface is at most `max_down` below `pos.y` (a
    /// position already inside the terrain hits too, at the surface point above it).
    pub fn ground_probe(&self, pos: Vec3, max_down: f32) -> Option<GroundHit> {
        let (y, normal, material) = self.surface_at(pos.x, pos.z)?;
        if pos.y - y > max_down {
            return None;
        }
        Some(GroundHit {
            point: Vec3::new(pos.x, y, pos.z),
            normal,
            slope_deg: slope_of(normal.y),
            material,
        })
    }

    /// Movement-only query. Keep the public terrain probes and cover rays independent.
    pub(crate) fn vertical_building_hit(
        &self,
        pos: Vec3,
        up: bool,
        max_t: f32,
    ) -> Option<(f32, Vec3)> {
        if self.buildings.is_empty() {
            return None;
        }
        let only_buildings =
            |_, c: &rapier3d::prelude::Collider| c.user_data == super::building::BUILDING_COLLIDER;
        let qp = self
            .physics
            .world()
            .query_pipeline_with_filter(QueryFilter::default().predicate(&only_buildings));
        let dir = if up { Vector::Y } else { -Vector::Y };
        let ray = Ray::new(Vector::from_array(pos.to_array()), dir);
        let (_, hit) = qp.cast_ray_and_get_normal(&ray, max_t, true)?;
        Some((hit.time_of_impact, Vec3::from_array(hit.normal.to_array())))
    }

    /// First terrain hit along `dir` (any length, normalised internally) within `max_t`
    /// metres. A ray starting below the surface hits at `t = 0`.
    pub fn raycast(&self, origin: Vec3, dir: Vec3, max_t: f32) -> Option<RayHit> {
        let len = dir.length();
        if len.is_nan() || len <= 1e-9 || !origin.is_finite() || max_t.is_nan() || max_t < 0.0 {
            return None;
        }
        self.trace(origin, dir / len, max_t).hit
    }

    /// Clear line between two points, accounting for terrain and static cover.
    pub fn los(&self, from: Vec3, to: Vec3) -> bool {
        self.terrain_los(from, to) && self.raycast_cover_segment(from, to).is_none()
    }

    /// Terrain-only visibility, conservative when the line leaves loaded chunks.
    pub fn terrain_los(&self, from: Vec3, to: Vec3) -> bool {
        let d = to - from;
        let len = d.length();
        if !from.is_finite() || !to.is_finite() {
            return false;
        }
        if len < 1e-6 {
            return self
                .surface_at(from.x, from.z)
                .is_some_and(|s| from.y >= s.0);
        }
        let t = self.trace(from, d / len, len);
        t.hit.is_none() && !t.missing
    }

    /// Loaded area as `(x0, x1, z0, z1)` in metres.
    fn bounds_m(&self) -> Option<(f32, f32, f32, f32)> {
        let b = self.bounds?;
        Some((
            b[0] as f32 * CHUNK_SIZE_M,
            (b[1] + 1) as f32 * CHUNK_SIZE_M,
            b[2] as f32 * CHUNK_SIZE_M,
            (b[3] + 1) as f32 * CHUNK_SIZE_M,
        ))
    }

    fn trace(&self, o: Vec3, d: Vec3, max_t: f32) -> Trace {
        let mut out = Trace {
            hit: None,
            missing: false,
        };
        let Some((x0, x1, z0, z1)) = self.bounds_m() else {
            out.missing = true;
            return out;
        };
        // Starting below the surface.
        if let Some((y, n, m)) = self.surface_at(o.x, o.z) {
            if o.y < y {
                out.hit = Some(RayHit {
                    t: 0.0,
                    point: o,
                    normal: n,
                    material: m,
                });
                return out;
            }
        }
        // Clip to the loaded rectangle.
        let (mut t_in, mut t_out) = (0.0f32, max_t);
        for (oc, dc, lo, hi) in [(o.x, d.x, x0, x1), (o.z, d.z, z0, z1)] {
            if dc.abs() < 1e-12 {
                if oc < lo || oc > hi {
                    out.missing = true;
                    return out;
                }
            } else {
                let (mut ta, mut tb) = ((lo - oc) / dc, (hi - oc) / dc);
                if ta > tb {
                    std::mem::swap(&mut ta, &mut tb);
                }
                t_in = t_in.max(ta);
                t_out = t_out.min(tb);
            }
        }
        if t_in > 0.0 || t_out < max_t {
            out.missing = true;
        }
        // Clip to the height band of the loaded terrain: nothing can be hit outside it.
        let (ymin, ymax) = (self.height_range.0 - 1e-3, self.height_range.1 + 1e-3);
        if d.y.abs() < 1e-12 {
            if o.y > ymax || o.y < ymin {
                return out;
            }
        } else {
            let (mut ta, mut tb) = ((ymin - o.y) / d.y, (ymax - o.y) / d.y);
            if ta > tb {
                std::mem::swap(&mut ta, &mut tb);
            }
            t_in = t_in.max(ta);
            t_out = t_out.min(tb);
        }
        if t_in > t_out {
            return out;
        }

        let px = o.x + d.x * t_in;
        let pz = o.z + d.z * t_in;
        let (cx_min, cx_max) = (x0 as i32, x1 as i32 - 1);
        let (cz_min, cz_max) = (z0 as i32, z1 as i32 - 1);
        let mut cx = floor_i(px).clamp(cx_min, cx_max);
        let mut cz = floor_i(pz).clamp(cz_min, cz_max);
        let step_x: i32 = if d.x > 1e-12 {
            1
        } else if d.x < -1e-12 {
            -1
        } else {
            0
        };
        let step_z: i32 = if d.z > 1e-12 {
            1
        } else if d.z < -1e-12 {
            -1
        } else {
            0
        };
        let next_bound = |c: i32, s: i32, o: f32, dc: f32| -> f32 {
            match s {
                1 => ((c + 1) as f32 - o) / dc,
                -1 => (c as f32 - o) / dc,
                _ => f32::INFINITY,
            }
        };
        let mut tmax_x = next_bound(cx, step_x, o.x, d.x);
        let mut tmax_z = next_bound(cz, step_z, o.z, d.z);
        let tdx = if step_x != 0 {
            1.0 / d.x.abs()
        } else {
            f32::INFINITY
        };
        let tdz = if step_z != 0 {
            1.0 / d.z.abs()
        } else {
            f32::INFINITY
        };

        let mut t_cur = t_in;
        let mut cache_key = (i32::MIN, i32::MIN);
        let mut cache: Option<&CollisionChunk> = None;
        loop {
            let t_next = tmax_x.min(tmax_z).min(t_out);
            let bx = cx.div_euclid(CHUNK_CELLS as i32);
            let bz = cz.div_euclid(CHUNK_CELLS as i32);
            if cache_key != (bz, bx) {
                cache_key = (bz, bx);
                cache = self.chunks.get(&(bz, bx));
            }
            match cache {
                None => out.missing = true,
                Some(ch) => {
                    let y_a = o.y + d.y * t_cur;
                    let y_b = o.y + d.y * t_next;
                    let (lo, hi) = if y_a < y_b { (y_a, y_b) } else { (y_b, y_a) };
                    if hi >= ch.min_height() - 1e-4 && lo <= ch.max_height() + 1e-4 {
                        let i = cx.rem_euclid(CHUNK_CELLS as i32) as usize;
                        let j = cz.rem_euclid(CHUNK_CELLS as i32) as usize;
                        let h = corners(ch, i, j);
                        let cmin = h[0].min(h[1]).min(h[2]).min(h[3]);
                        let cmax = h[0].max(h[1]).max(h[2]).max(h[3]);
                        if hi >= cmin - 1e-4 && lo <= cmax + 1e-4 {
                            if let Some(hit) = cell_hit(ch, i, j, h, cx, cz, o, d, max_t) {
                                out.hit = Some(hit);
                                return out;
                            }
                        }
                    }
                }
            }
            if t_next >= t_out {
                break;
            }
            if tmax_x < tmax_z {
                cx += step_x;
                t_cur = tmax_x;
                tmax_x += tdx;
            } else {
                cz += step_z;
                t_cur = tmax_z;
                tmax_z += tdz;
            }
            if cx < cx_min || cx > cx_max || cz < cz_min || cz > cz_max {
                break;
            }
        }
        out
    }
}

/// Nearest of the two triangles of cell `(i, j)` hit by the ray.
#[allow(clippy::too_many_arguments)]
#[inline]
fn cell_hit(
    ch: &CollisionChunk,
    i: usize,
    j: usize,
    h: [f32; 4],
    cx: i32,
    cz: i32,
    o: Vec3,
    d: Vec3,
    max_t: f32,
) -> Option<RayHit> {
    let (fx, fz) = (cx as f32, cz as f32);
    let a = Vec3::new(fx, h[0], fz);
    let b = Vec3::new(fx + 1.0, h[1], fz);
    let c = Vec3::new(fx, h[2], fz + 1.0);
    let dd = Vec3::new(fx + 1.0, h[3], fz + 1.0);
    let t1 = ray_tri(o, d, a, c, b);
    let t2 = ray_tri(o, d, c, dd, b);
    let (t, tri2) = match (t1, t2) {
        (Some(x), Some(y)) => {
            if x <= y {
                (x, false)
            } else {
                (y, true)
            }
        }
        (Some(x), None) => (x, false),
        (None, Some(y)) => (y, true),
        (None, None) => return None,
    };
    if t > max_t {
        return None;
    }
    let (e1, e2) = if tri2 {
        (dd - c, b - c)
    } else {
        (c - a, b - a)
    };
    let mut n = e1.cross(e2).normalize();
    if n.y < 0.0 {
        n = -n;
    }
    Some(RayHit {
        t,
        point: o + d * t,
        normal: n,
        material: ch.material(i, j),
    })
}
