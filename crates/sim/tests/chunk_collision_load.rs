//! M1-035 integration test (`cargo test -p frontline-sim --test chunk_collision_load`):
//! verifies `FCOL` v1 chunk parsing, shared-edge continuity, and 200-ray visual-vs-Rapier
//! `HeightField` agreement directly against the real Carentan `heightmap_1m.r16` and chunk index.

use std::path::PathBuf;

use frontline_sim::collision::CHUNK_SAMPLES;
use frontline_sim::{CollisionChunk, CollisionData};
use rapier3d::prelude::{Ray, Vector};
use serde::Deserialize;

#[derive(Deserialize)]
struct MapMeta {
    height: HeightMeta,
}

#[derive(Deserialize)]
struct HeightMeta {
    samples: usize,
    hmin_m: f32,
    hmax_m: f32,
}

#[derive(Deserialize)]
struct ChunkIndex {
    count: usize,
    bx_range: [i32; 2],
    bz_range: [i32; 2],
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

/// Encode one 64 m chunk from the global 2049x2049 u16 heightmap into FCOL v1 bytes
/// (matching `tools/map-pipeline/map_pipeline/chunks.py` & `chunker.py`).
fn encode_fcol_v1(
    raw_u16: &[u16],
    grid_n: usize,
    bx: i32,
    bz: i32,
    hmin: f32,
    hmax: f32,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + CHUNK_SAMPLES * CHUNK_SAMPLES * 2 + 64 * 64);
    out.extend_from_slice(b"FCOL");
    out.push(1); // version
    out.push(0); // pad
    out.extend_from_slice(&(CHUNK_SAMPLES as u16).to_le_bytes());
    out.extend_from_slice(&hmin.to_le_bytes());
    out.extend_from_slice(&hmax.to_le_bytes());
    let z0 = (bz as usize) * 64;
    let x0 = (bx as usize) * 64;
    for iz in 0..CHUNK_SAMPLES {
        let row = (z0 + iz) * grid_n + x0;
        for ix in 0..CHUNK_SAMPLES {
            out.extend_from_slice(&raw_u16[row + ix].to_le_bytes());
        }
    }
    out.resize(out.len() + 64 * 64, 0u8);
    out
}

/// Visual height at world `(x, z)` using the `(0,0)-(1,1)` diagonal triangulation from `FORMAT.txt`.
fn tri_height_diag_00_11(h: &[f32], grid_n: usize, x: f32, z: f32) -> f32 {
    let j = (x.floor() as usize).min(grid_n - 2);
    let i = (z.floor() as usize).min(grid_n - 2);
    let fx = x - j as f32;
    let fz = z - i as f32;
    let h00 = h[i * grid_n + j];
    let h10 = h[i * grid_n + j + 1];
    let h01 = h[(i + 1) * grid_n + j];
    let h11 = h[(i + 1) * grid_n + j + 1];
    if fx >= fz {
        h00 + (h10 - h00) * fx + (h11 - h10) * fz
    } else {
        h00 + (h11 - h01) * fx + (h01 - h00) * fz
    }
}

/// Alternate `(1,0)-(0,1)` diagonal triangulation for empirical Rapier diagonal comparison.
fn tri_height_diag_10_01(h: &[f32], grid_n: usize, x: f32, z: f32) -> f32 {
    let j = (x.floor() as usize).min(grid_n - 2);
    let i = (z.floor() as usize).min(grid_n - 2);
    let fx = x - j as f32;
    let fz = z - i as f32;
    let h00 = h[i * grid_n + j];
    let h10 = h[i * grid_n + j + 1];
    let h01 = h[(i + 1) * grid_n + j];
    let h11 = h[(i + 1) * grid_n + j + 1];
    if fx + fz <= 1.0 {
        h00 + (h10 - h00) * fx + (h01 - h00) * fz
    } else {
        h11 + (h01 - h11) * (1.0 - fx) + (h10 - h11) * (1.0 - fz)
    }
}

struct Lcg(u64);
impl Lcg {
    fn next_f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32) / ((1u32 << 24) as f32)
    }
}

#[test]
fn chunk_collision_load_carentan_200_rays() {
    let carentan = repo_root().join("data/maps/carentan");
    let meta: MapMeta =
        serde_json::from_slice(&std::fs::read(carentan.join("meta.json")).expect("read meta.json"))
            .expect("parse meta.json");
    let index: ChunkIndex = serde_json::from_slice(
        &std::fs::read(carentan.join("m1/chunks/index.json")).expect("read index.json"),
    )
    .expect("parse index.json");
    assert_eq!(index.count, 380);
    assert_eq!(index.bx_range, [5, 24]);
    assert_eq!(index.bz_range, [8, 26]);

    let raw_bytes =
        std::fs::read(carentan.join("heightmap_1m.r16")).expect("read heightmap_1m.r16");
    let n = meta.height.samples;
    assert_eq!(raw_bytes.len(), n * n * 2);
    let raw_u16: Vec<u16> = raw_bytes
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    let span = meta.height.hmax_m - meta.height.hmin_m;
    let heights_m: Vec<f32> = raw_u16
        .iter()
        .map(|&q| meta.height.hmin_m + (q as f32) / 65535.0 * span)
        .collect();

    // Load a 4x4 patch of Carentan M1 chunks around the town/slopes (bx 18..=21, bz 12..=15)
    let mut chunks = Vec::new();
    let patch_bx = 18..=21;
    let patch_bz = 12..=15;
    for bz in patch_bz.clone() {
        for bx in patch_bx.clone() {
            let bytes = encode_fcol_v1(&raw_u16, n, bx, bz, meta.height.hmin_m, meta.height.hmax_m);
            let chunk = CollisionChunk::from_bytes(bx, bz, &bytes).expect("parse FCOL v1");
            chunks.push(chunk);
        }
    }
    let world = CollisionData::from_chunks(chunks);
    assert_eq!(world.chunk_count(), 16);

    // Check shared-edge equality across adjacent chunks
    for bz in 12..=15 {
        for bx in 18..=20 {
            let a = world.chunk(bx, bz).unwrap();
            let b = world.chunk(bx + 1, bz).unwrap();
            for iz in 0..CHUNK_SAMPLES {
                assert_eq!(a.sample(64, iz), b.sample(0, iz), "east-west seam mismatch");
            }
        }
    }
    for bz in 12..=14 {
        for bx in 18..=21 {
            let a = world.chunk(bx, bz).unwrap();
            let b = world.chunk(bx, bz + 1).unwrap();
            for ix in 0..CHUNK_SAMPLES {
                assert_eq!(
                    a.sample(ix, 64),
                    b.sample(ix, 0),
                    "north-south seam mismatch"
                );
            }
        }
    }

    // Cast 200 vertical rays through Rapier HeightField and compare against visual triangulation
    let qp = world.physics().world().query_pipeline();
    let mut rng = Lcg(0xC4_035);
    let mut max_d_00_11 = 0.0f32;
    let mut sum_d_00_11 = 0.0f32;
    let mut max_d_10_01 = 0.0f32;
    let mut sum_d_10_01 = 0.0f32;
    for _ in 0..200 {
        let x = 18.0 * 64.0 + rng.next_f32() * (4.0 * 64.0 - 0.02) + 0.01;
        let z = 12.0 * 64.0 + rng.next_f32() * (4.0 * 64.0 - 0.02) + 0.01;
        let ray = Ray::new(Vector::new(x, 100.0, z), Vector::new(0.0, -1.0, 0.0));
        let (_h, toi) = qp
            .cast_ray(&ray, 200.0, true)
            .expect("ray hits Carentan terrain");
        let rapier_y = 100.0 - toi;
        let v00 = tri_height_diag_00_11(&heights_m, n, x, z);
        let v10 = tri_height_diag_10_01(&heights_m, n, x, z);
        let d00 = (rapier_y - v00).abs();
        let d10 = (rapier_y - v10).abs();
        max_d_00_11 = max_d_00_11.max(d00);
        sum_d_00_11 += d00;
        max_d_10_01 = max_d_10_01.max(d10);
        sum_d_10_01 += d10;
    }
    let mean_00_11 = sum_d_00_11 / 200.0;
    let mean_10_01 = sum_d_10_01 / 200.0;
    println!(
        "Carentan 200 rays vs Rapier HeightField: diag(0,0)-(1,1) max={max_d_00_11:.6}m mean={mean_00_11:.6}m | diag(1,0)-(0,1) max={max_d_10_01:.6}m mean={mean_10_01:.6}m"
    );
    // Rapier 0.36.1 HeightField splits each 1 m quad along the (1,0)-(0,1) diagonal.
    // Verifying against (1,0)-(0,1) proves FCOL v1 loading & dequantisation match heightmap_1m.r16
    // to floating-point precision (<= 1e-4 m). The opposite (0,0)-(1,1) diagonal used by CDLOD
    // (grid-mesh.ts) deviates by up to 0.067 m on steep Carentan quads (registered in PI-084 续).
    assert!(
        max_d_10_01 <= 1e-4 && mean_10_01 <= 1e-4,
        "FCOL v1 vs Rapier HeightField height mismatch: max={max_d_10_01} mean={mean_10_01}"
    );
    assert!(
        mean_00_11 <= 0.01,
        "mean diagonal difference unexpectedly large: {mean_00_11}"
    );
}
