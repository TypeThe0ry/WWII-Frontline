//! Shared helpers for the collision integration tests.
#![allow(dead_code)]

use std::path::PathBuf;

use frontline_sim::{CollisionChunk, CollisionData};

pub const REGEN: &str =
    "PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0";

pub fn map_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/m0")
}

/// Real M0 terrain, or `None` (with a message naming the regeneration command) when the
/// git-ignored binaries are absent. Set `FRONTLINE_REQUIRE_TERRAIN=1` to turn that into a failure.
pub fn real_terrain(test: &str) -> Option<CollisionData> {
    let dir = map_dir().join("chunks");
    let mut chunks = Vec::new();
    for bz in 0..4 {
        for bx in 0..4 {
            let p = dir.join(format!("c_{bx}_{bz}.bin"));
            match std::fs::read(&p) {
                Ok(bytes) => chunks.push(
                    CollisionChunk::from_bytes(bx, bz, &bytes)
                        .unwrap_or_else(|e| panic!("{}: {e}", p.display())),
                ),
                Err(_) => {
                    let msg = format!(
                        "{test}: SKIPPED, {} missing. Regenerate with: {REGEN}",
                        p.display()
                    );
                    assert!(std::env::var_os("FRONTLINE_REQUIRE_TERRAIN").is_none(), "{msg}");
                    eprintln!("{msg}");
                    return None;
                }
            }
        }
    }
    Some(CollisionData::from_chunks(chunks))
}

/// Reference heights (metres) of `heightmap_1m.r16`: 257x257, row = z.
pub fn reference_heightmap(test: &str) -> Option<Vec<f64>> {
    let p = map_dir().join("heightmap_1m.r16");
    let Ok(bytes) = std::fs::read(&p) else {
        eprintln!("{test}: SKIPPED, {} missing. Regenerate with: {REGEN}", p.display());
        return None;
    };
    assert_eq!(bytes.len(), 257 * 257 * 2);
    // hmin/hmax from meta.json (height.hmin_m / hmax_m).
    let meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(map_dir().join("meta.json")).unwrap()).unwrap();
    let hmin = meta["height"]["hmin_m"].as_f64().unwrap();
    let hmax = meta["height"]["hmax_m"].as_f64().unwrap();
    Some(
        bytes
            .chunks_exact(2)
            .map(|p| hmin + u16::from_le_bytes([p[0], p[1]]) as f64 / 65535.0 * (hmax - hmin))
            .collect(),
    )
}

/// Synthetic chunk sampling `f(world_x, world_z)` at the 65x65 sample positions.
pub fn synth_chunk(bx: i32, bz: i32, f: &dyn Fn(f32, f32) -> f32) -> CollisionChunk {
    let mut h = Vec::with_capacity(65 * 65);
    for iz in 0..65 {
        for ix in 0..65 {
            h.push(f(bx as f32 * 64.0 + ix as f32, bz as f32 * 64.0 + iz as f32));
        }
    }
    CollisionChunk::new(bx, bz, h, vec![0; 64 * 64])
}

/// `nx` x `nz` chunk grid of a synthetic surface.
pub fn synth_data(nx: i32, nz: i32, f: &dyn Fn(f32, f32) -> f32) -> CollisionData {
    let mut v = Vec::new();
    for bz in 0..nz {
        for bx in 0..nx {
            v.push(synth_chunk(bx, bz, f));
        }
    }
    CollisionData::from_chunks(v)
}

/// Deterministic xorshift for tests (independent of the sim RNG).
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f32()
    }
}
