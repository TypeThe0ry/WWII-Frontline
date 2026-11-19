//! Height-field collision (M0-010; ARCHITECTURE 9.5, 5.3).
//!
//! Terrain is organised in 64 m chunks of 65x65 height samples (adjacent chunks share their
//! border samples, so the surface is seamless). [`CollisionData`] owns the loaded chunks
//! and a Rapier [`PhysicsWorld`](rapier3d::prelude::PhysicsWorld) holding one `HeightField`
//! collider per chunk (used by the kinematic character controller of M0-011).
//!
//! # Design decisions
//! * **Queries are our own code, Rapier is the shared collision world.** Height, normal,
//!   slope, ground probe, ray cast and LOS run on the chunk arrays directly (grid DDA over
//!   1 m cells, two triangles per cell with the *same* diagonal as Rapier's non-zigzag
//!   `HeightField` triangulation). That keeps them allocation free, usable without stepping
//!   a Rapier pipeline, and cheap enough for 10k rays per ms-scale budget. Tests
//!   cross-check the ray cast against Rapier's `cast_ray`.
//! * `height_at` is the bilinear interpolation of the four corner samples (the reference of
//!   acceptance 3); `normal_at` / `slope_deg_at` / `ground_probe` / `raycast` use the
//!   triangle surface, which is exactly what Rapier collides against.
//! * Missing chunks: every query returns `None` / `false`; `los` is `false` when the segment
//!   touches a missing chunk (conservative). Keeping players out of such chunks is M0-011's job.
//! * `sim` does no I/O: callers pass the bytes of `c_{bx}_{bz}.bin` to
//!   [`CollisionChunk::from_bytes`].
//! * Determinism: chunks live in a `BTreeMap` keyed `(bz, bx)`; [`CollisionData::from_chunks`]
//!   inserts into Rapier in ascending `(bz, bx)` order; floats use `f32` and `libm` only
//!   (Rapier is built with `enhanced-determinism`, see PLAN_ISSUES PI-013).

mod chunk;
mod cover;
mod heightfield;
mod query;

use std::collections::BTreeMap;

pub use chunk::{CollisionChunk, CollisionError};
pub use cover::CoverSlab;
pub use heightfield::TerrainPhysics;
pub use query::{GroundHit, RayHit};

use crate::hash::HashWriter;

/// Cells per chunk edge (metres).
pub const CHUNK_CELLS: usize = 64;
/// Height samples per chunk edge.
pub const CHUNK_SAMPLES: usize = CHUNK_CELLS + 1;
/// Chunk edge length in metres.
pub const CHUNK_SIZE_M: f32 = CHUNK_CELLS as f32;
/// Steepest walkable slope; M0-011 will take it from `/data` (REF M-29 pending).
pub const MAX_WALK_SLOPE_DEG: f32 = 35.0;

/// Chunk key: `(bz, bx)` so that BTreeMap order is the required `(bz, bx)` ascending order.
pub(crate) type ChunkKey = (i32, i32);

/// Loaded terrain collision.
#[derive(Default)]
pub struct CollisionData {
    chunks: BTreeMap<ChunkKey, CollisionChunk>,
    physics: TerrainPhysics,
    /// `[min_bx, max_bx, min_bz, max_bz]` of the loaded chunks.
    bounds: Option<[i32; 4]>,
    /// `(min, max)` height over all loaded chunks; lets ray casts clip to the relief band.
    height_range: (f32, f32),
    cover: cover::StaticCover,
}

impl std::fmt::Debug for CollisionData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollisionData")
            .field("chunks", &self.chunks.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Cloning rebuilds the Rapier world from the chunk list in ascending `(bz, bx)` order, so
/// a clone is bit-identical to a `from_chunks` build.
impl Clone for CollisionData {
    fn clone(&self) -> Self {
        let mut clone = Self::from_chunks(self.chunks.values().cloned());
        clone.cover = self.cover.clone();
        clone
    }
}

impl CollisionData {
    /// No geometry; every query returns `None` / `false`.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Builds from chunks in canonical `(bz, bx)` order regardless of the input order.
    pub fn from_chunks(chunks: impl IntoIterator<Item = CollisionChunk>) -> Self {
        let mut data = Self::empty();
        for c in chunks {
            data.chunks.insert((c.bz, c.bx), c);
        }
        let keys: Vec<ChunkKey> = data.chunks.keys().copied().collect();
        for k in keys {
            let c = &data.chunks[&k];
            data.physics.insert(k, c);
        }
        data.physics.refresh();
        data.recompute_bounds();
        data
    }

    /// Adds (or replaces) a chunk, including its Rapier collider.
    pub fn insert_chunk(&mut self, chunk: CollisionChunk) {
        let key = (chunk.bz, chunk.bx);
        self.physics.remove(key);
        self.physics.insert(key, &chunk);
        self.chunks.insert(key, chunk);
        self.physics.refresh();
        self.recompute_bounds();
    }

    /// Removes a chunk (client unload). Returns whether it was loaded.
    pub fn remove_chunk(&mut self, bx: i32, bz: i32) -> bool {
        let existed = self.chunks.remove(&(bz, bx)).is_some();
        if existed {
            self.physics.remove((bz, bx));
            self.physics.refresh();
            self.recompute_bounds();
        }
        existed
    }

    fn recompute_bounds(&mut self) {
        self.height_range = self
            .chunks
            .values()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |a, c| {
                (a.0.min(c.min_height()), a.1.max(c.max_height()))
            });
        self.bounds = self.chunks.keys().fold(None, |acc, &(bz, bx)| match acc {
            None => Some([bx, bx, bz, bz]),
            Some([a, b, c, d]) => Some([a.min(bx), b.max(bx), c.min(bz), d.max(bz)]),
        });
    }

    pub fn has_chunk(&self, bx: i32, bz: i32) -> bool {
        self.chunks.contains_key(&(bz, bx))
    }

    pub fn chunk(&self, bx: i32, bz: i32) -> Option<&CollisionChunk> {
        self.chunks.get(&(bz, bx))
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Loaded chunks in ascending `(bz, bx)` order.
    pub fn chunks(&self) -> impl Iterator<Item = &CollisionChunk> {
        self.chunks.values()
    }

    /// Bytes of height/material payload held (client memory budget, ARCHITECTURE 9.5).
    pub fn memory_bytes(&self) -> usize {
        self.chunks.values().map(CollisionChunk::memory_bytes).sum()
    }

    /// Rapier world holding one height-field collider per chunk (for the KCC).
    pub fn physics(&self) -> &TerrainPhysics {
        &self.physics
    }

    /// Digest of everything that defines the collision world: chunk keys and samples, then
    /// the Rapier collider order and poses. Equal for two builds of the same data.
    pub fn state_hash(&self) -> u64 {
        let mut h = HashWriter::new();
        h.write_u32(self.chunks.len() as u32);
        for c in self.chunks.values() {
            c.hash_into(&mut h);
        }
        self.physics.hash_into(&mut h);
        self.cover.hash_into(&mut h);
        h.finish()
    }
}
