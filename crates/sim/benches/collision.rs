//! `collision_raycast` (acceptance: median of 10 000 seeded rays <= 800 m on M0 terrain < 5 ms)
//! and `ground_probe` (recorded). Plain `Instant` harness like `world_step_empty`.
//! Needs the git-ignored M0 terrain:
//! `PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0`.
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant; // benches are outside the purity scan (src only)

use frontline_sim::{CollisionChunk, CollisionData, Vec3};

struct Rng(u64);
impl Rng {
    fn f32(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f32()
    }
}

fn load() -> Option<CollisionData> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/m0/chunks");
    let mut v = Vec::new();
    for bz in 0..4 {
        for bx in 0..4 {
            let b = std::fs::read(dir.join(format!("c_{bx}_{bz}.bin"))).ok()?;
            v.push(CollisionChunk::from_bytes(bx, bz, &b).ok()?);
        }
    }
    Some(CollisionData::from_chunks(v))
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    let Some(d) = load() else {
        eprintln!(
            "collision bench SKIPPED: data/maps/m0/chunks missing. Regenerate: PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0"
        );
        return;
    };
    // Mixed rays: random origin 0.5..30 m above ground, random direction, length 1..800 m.
    let mut rng = Rng(0x1234_5678_9abc_def1);
    let mut rays = Vec::with_capacity(10_000);
    for _ in 0..10_000 {
        let (x, z) = (rng.range(0.0, 256.0), rng.range(0.0, 256.0));
        let y = d.height_at(x, z).unwrap() + rng.range(0.5, 30.0);
        let dir = Vec3::new(
            rng.range(-1.0, 1.0),
            rng.range(-0.5, 0.5),
            rng.range(-1.0, 1.0),
        )
        .normalize();
        rays.push((Vec3::new(x, y, z), dir, rng.range(1.0, 800.0)));
    }
    // Worst case: level rays inside the relief band (1..11 m) that mostly leave the map (full traversal).
    let mut flat = Vec::with_capacity(10_000);
    for _ in 0..10_000 {
        let (x, z) = (rng.range(0.0, 256.0), rng.range(0.0, 256.0));
        let a = rng.range(0.0, std::f32::consts::TAU);
        flat.push((
            Vec3::new(x, 6.0, z),
            Vec3::new(a.cos(), 0.0, a.sin()),
            800.0,
        ));
    }
    for (name, set) in [
        ("collision_raycast", &rays),
        ("collision_raycast_worst_level_in_relief", &flat),
    ] {
        let mut times = Vec::new();
        let mut hits = 0;
        for run in 0..21 {
            let t0 = Instant::now();
            let mut h = 0;
            for &(o, dir, max_t) in set.iter() {
                if black_box(d.raycast(black_box(o), black_box(dir), max_t)).is_some() {
                    h += 1;
                }
            }
            let dt = t0.elapsed().as_secs_f64() * 1e3;
            if run > 0 {
                times.push(dt); // run 0 warms up
            }
            hits = h;
        }
        println!(
            "{name}: 10000 rays, median {:.3} ms (min {:.3}, max {:.3}), {hits} hits",
            median(times.clone()),
            times.iter().cloned().fold(f64::INFINITY, f64::min),
            times.iter().cloned().fold(0.0, f64::max)
        );
    }
    // ground_probe
    let pts: Vec<Vec3> = (0..10_000)
        .map(|_| {
            Vec3::new(
                rng.range(0.0, 256.0),
                rng.range(2.0, 14.0),
                rng.range(0.0, 256.0),
            )
        })
        .collect();
    let mut times = Vec::new();
    for run in 0..21 {
        let t0 = Instant::now();
        let mut n = 0;
        for &p in &pts {
            if black_box(d.ground_probe(black_box(p), 3.0)).is_some() {
                n += 1;
            }
        }
        black_box(n);
        if run > 0 {
            times.push(t0.elapsed().as_secs_f64() * 1e3);
        }
    }
    let m = median(times);
    println!(
        "ground_probe: 10000 probes, median {m:.3} ms ({:.0} ns each)",
        m * 1e6 / 10_000.0
    );
}
