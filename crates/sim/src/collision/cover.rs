//! Material-tagged graybox cover, separate from heightfield/KCC collision.

use std::collections::BTreeMap;

use glam::{Vec2, Vec3};

use super::{BuildingWall, ChunkKey, CollisionData, RayHit, CHUNK_SIZE_M};
use crate::ballistics::materials::{MaterialId, MaterialTable};
use crate::ballistics::penetration::{Layer, LayerWorld};
use crate::throwables::Occlusion;

/// Upright slab along a footprint edge or hedgerow segment.
#[derive(Debug, Clone, Copy)]
pub struct CoverSlab {
    center: Vec3,
    axis: Vec2,
    half: Vec3,
    material: MaterialId,
    blast: Occlusion,
}

impl CoverSlab {
    pub fn new(
        a: [f32; 2],
        b: [f32; 2],
        bottom: f32,
        top: f32,
        width: f32,
        material: MaterialId,
        blast: Occlusion,
    ) -> Option<Self> {
        Some(Self::from_building_wall(
            BuildingWall::new(a, b, bottom, top, width)?,
            material,
            blast,
        ))
    }

    pub fn from_building_wall(wall: BuildingWall, material: MaterialId, blast: Occlusion) -> Self {
        Self {
            center: wall.center,
            axis: wall.axis,
            half: wall.half,
            material,
            blast,
        }
    }

    fn side(&self) -> Vec2 {
        Vec2::new(-self.axis.y, self.axis.x)
    }

    fn intersect(&self, origin: Vec3, dir: Vec3, max_t: f32) -> Option<(f32, f32, Vec3)> {
        let local = |v: Vec3| {
            let xz = Vec2::new(v.x, v.z);
            Vec3::new(xz.dot(self.axis), v.y, xz.dot(self.side()))
        };
        let o = local(origin - self.center);
        let d = local(dir);
        let (mut enter, mut exit) = (0.0_f32, f32::INFINITY);
        let mut face = None;
        for i in 0..3 {
            if d[i].abs() < 1e-8 {
                if o[i].abs() > self.half[i] {
                    return None;
                }
                continue;
            }
            let a = (-self.half[i] - o[i]) / d[i];
            let b = (self.half[i] - o[i]) / d[i];
            let near = a.min(b);
            if near >= enter {
                enter = near;
                face = Some(i);
            }
            exit = exit.min(a.max(b));
            if exit <= enter || enter > max_t {
                return None;
            }
        }
        let normal = match face {
            Some(i) => {
                let axis = match i {
                    0 => Vec3::new(self.axis.x, 0.0, self.axis.y),
                    1 => Vec3::Y,
                    _ => Vec3::new(self.side().x, 0.0, self.side().y),
                };
                axis * -d[i].signum()
            }
            None => -dir, // already inside (e.g. an adjacent/overlapping slab)
        };
        Some((enter, exit, normal))
    }
}

#[derive(Debug, Default, Clone)]
pub(super) struct StaticCover {
    slabs: Vec<CoverSlab>,
    bins: BTreeMap<ChunkKey, Vec<usize>>,
}

fn bins(min: Vec2, max: Vec2) -> impl Iterator<Item = ChunkKey> {
    let cell = |x: f32| libm::floorf(x / CHUNK_SIZE_M) as i32;
    (cell(min.y)..=cell(max.y)).flat_map(move |z| (cell(min.x)..=cell(max.x)).map(move |x| (z, x)))
}

impl StaticCover {
    fn new(slabs: Vec<CoverSlab>) -> Self {
        let mut bins_by_key: BTreeMap<ChunkKey, Vec<usize>> = BTreeMap::new();
        for (i, slab) in slabs.iter().enumerate() {
            let half = slab.axis.abs() * slab.half.x + slab.side().abs() * slab.half.z;
            let mid = Vec2::new(slab.center.x, slab.center.z);
            for k in bins(mid - half, mid + half) {
                bins_by_key.entry(k).or_default().push(i);
            }
        }
        Self {
            slabs,
            bins: bins_by_key,
        }
    }

    fn candidates(&self, from: Vec3, to: Vec3) -> impl Iterator<Item = (usize, &CoverSlab)> {
        let a = Vec2::new(from.x, from.z);
        let b = Vec2::new(to.x, to.z);
        bins(a.min(b), a.max(b))
            .filter_map(|k| self.bins.get(&k))
            .flatten()
            .map(|&i| (i, &self.slabs[i]))
    }

    fn first(
        &self,
        a: Vec3,
        dir: Vec3,
        len: f32,
        accept: impl Fn(MaterialId, f32) -> bool,
    ) -> Option<(RayHit, Layer)> {
        if self.slabs.is_empty()
            || !a.is_finite()
            || !dir.is_finite()
            || !len.is_finite()
            || dir == Vec3::ZERO
            || len <= 0.0
        {
            return None;
        }
        let mut nearest = None;
        for (i, s) in self.candidates(a, a + dir * len) {
            if let Some((t, exit, normal)) = s.intersect(a, dir, len) {
                let thickness_mm = ((a + dir * exit) - (a + dir * t)).length() * 1000.0;
                if accept(s.material, thickness_mm)
                    && nearest.is_none_or(|(best, j, _, _, _)| (t, i) < (best, j))
                {
                    nearest = Some((t, i, exit, normal, s.material));
                }
            }
        }
        let (t, _, exit, normal, material) = nearest?;
        let point = a + dir * t;
        Some((
            RayHit {
                t,
                point,
                normal,
                material: material.0,
            },
            Layer {
                material,
                entry: point,
                exit: a + dir * exit,
                normal,
            },
        ))
    }

    pub(super) fn hash_into(&self, h: &mut crate::hash::HashWriter) {
        if self.slabs.is_empty() {
            return;
        }
        h.write_u32(self.slabs.len() as u32);
        for s in &self.slabs {
            for x in s
                .center
                .to_array()
                .into_iter()
                .chain(s.axis.to_array())
                .chain(s.half.to_array())
            {
                h.write_f32(x);
            }
            h.write_u8(s.material.0);
            h.write_f32(s.blast.mult());
        }
    }
}

impl CollisionData {
    pub fn install_cover(&mut self, slabs: Vec<CoverSlab>) {
        self.cover = StaticCover::new(slabs);
    }

    pub fn cover_count(&self) -> usize {
        self.cover.slabs.len()
    }

    /// Only static cover; `material` is a MaterialTable index, not a terrain surface byte.
    pub fn raycast_cover_segment(&self, a: Vec3, b: Vec3) -> Option<RayHit> {
        if self.cover.slabs.is_empty() {
            return None;
        }
        let d = b - a;
        let len = d.length();
        self.cover
            .first(a, d / len, len, |_, _| true)
            .map(|(hit, _)| hit)
    }

    pub(crate) fn ballistic_cover(
        &self,
        a: Vec3,
        dir: Vec3,
        len: f32,
        table: &MaterialTable,
    ) -> Option<(RayHit, Layer)> {
        self.cover.first(a, dir, len, |id, thickness_mm| {
            table.get(id).is_none_or(|m| thickness_mm >= m.min_mm)
        })
    }

    /// Strongest cover intersected, irrespective of order; missing terrain is conservative.
    pub fn blast_cover(&self, a: Vec3, b: Vec3) -> Occlusion {
        if !self.terrain_los(a, b) {
            return Occlusion::Solid;
        }
        if self.cover.slabs.is_empty() {
            return Occlusion::Clear;
        }
        let d = b - a;
        let len = d.length();
        if len <= 1e-6 {
            return Occlusion::Clear;
        }
        let dir = d / len;
        let mut result = Occlusion::Clear;
        for (_, s) in self.cover.candidates(a, b) {
            if s.blast != Occlusion::Clear && s.intersect(a, dir, len).is_some() {
                if s.blast == Occlusion::Solid {
                    return Occlusion::Solid;
                }
                result = Occlusion::Thin;
            }
        }
        result
    }
}

impl LayerWorld for CollisionData {
    fn first_layer(&self, origin: Vec3, dir: Vec3, max_t: f32) -> Option<Layer> {
        self.cover
            .first(origin, dir, max_t, |_, _| true)
            .map(|(_, layer)| layer)
    }
}
