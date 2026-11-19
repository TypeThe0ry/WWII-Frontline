//! Rapier query world with height-field and optional building colliders.

use std::collections::BTreeMap;

use rapier3d::prelude::{Array2, ColliderBuilder, ColliderHandle, PhysicsWorld, Vector};

use super::{
    BuildingCollision, ChunkKey, CollisionChunk, CHUNK_CELLS, CHUNK_SAMPLES, CHUNK_SIZE_M,
};
use crate::hash::HashWriter;

/// Static world colliders (no rigid bodies, never stepped; the broad-phase BVH is
/// refreshed with `detect_collisions` so that `query_pipeline()` is usable).
pub struct TerrainPhysics {
    world: PhysicsWorld,
    handles: BTreeMap<ChunkKey, ColliderHandle>,
    building_handles: Vec<ColliderHandle>,
}

impl Default for TerrainPhysics {
    fn default() -> Self {
        Self {
            world: PhysicsWorld {
                gravity: Vector::ZERO,
                ..PhysicsWorld::default()
            },
            handles: BTreeMap::new(),
            building_handles: Vec::new(),
        }
    }
}

impl TerrainPhysics {
    /// The Rapier world (colliders + query pipeline) for KCC and ray queries.
    pub fn world(&self) -> &PhysicsWorld {
        &self.world
    }

    /// Collider handles in ascending `(bz, bx)` key order.
    pub fn handles(&self) -> impl Iterator<Item = (ChunkKey, ColliderHandle)> + '_ {
        self.handles.iter().map(|(k, h)| (*k, *h))
    }

    pub(super) fn insert(&mut self, key: ChunkKey, chunk: &CollisionChunk) {
        // Array2 data is column-major: index = row + col * nrows, row = z, col = x.
        let n = CHUNK_SAMPLES;
        let mut data = Vec::with_capacity(n * n);
        for ix in 0..n {
            for iz in 0..n {
                data.push(chunk.sample(ix, iz));
            }
        }
        let heights = Array2::new(n, n, data);
        // Height-field local origin is its centre; vertical scale 1 keeps metres.
        let collider =
            ColliderBuilder::heightfield(heights, Vector::new(CHUNK_SIZE_M, 1.0, CHUNK_SIZE_M))
                .translation(Vector::new(
                    chunk.bx as f32 * CHUNK_SIZE_M + CHUNK_SIZE_M * 0.5,
                    0.0,
                    chunk.bz as f32 * CHUNK_SIZE_M + CHUNK_SIZE_M * 0.5,
                ))
                .build();
        let handle = self.world.insert_collider(collider, None);
        self.handles.insert(key, handle);
    }

    pub(super) fn remove(&mut self, key: ChunkKey) {
        if let Some(h) = self.handles.remove(&key) {
            self.world.remove_collider(h);
        }
    }

    pub(super) fn replace_buildings(&mut self, buildings: &BuildingCollision) {
        for handle in self.building_handles.drain(..) {
            self.world.remove_collider(handle);
        }
        self.building_handles.extend(
            buildings
                .colliders()
                .map(|collider| self.world.insert_collider(collider, None)),
        );
        self.refresh();
    }

    /// Updates the broad phase so that queries see the current colliders.
    pub(super) fn refresh(&mut self) {
        self.world.detect_collisions(&(), &());
    }

    pub(super) fn hash_into(&self, h: &mut HashWriter) {
        h.write_u32(self.handles.len() as u32);
        for (key, handle) in &self.handles {
            h.write_u32(key.0 as u32);
            h.write_u32(key.1 as u32);
            let (idx, generation) = handle.into_raw_parts();
            h.write_u32(idx);
            h.write_u32(generation);
            if let Some(c) = self.world.colliders.get(*handle) {
                let t = c.translation();
                h.write_f32(t.x);
                h.write_f32(t.y);
                h.write_f32(t.z);
            }
        }
    }
}

const _: () = assert!(CHUNK_CELLS + 1 == CHUNK_SAMPLES);
