//! Collision against the real M0 terrain (git-ignored binaries; skipped with a message that
//! names the regeneration command when absent, or failed when FRONTLINE_REQUIRE_TERRAIN=1).
mod common;

use common::*;
use frontline_sim::{CollisionData, Vec3};
use rapier3d::prelude::{QueryFilter, Ray, Vector};

fn ref_bilinear(h: &[f64], x: f64, z: f64) -> f64 {
    let (i, j) = (x.floor() as usize, z.floor() as usize);
    let (u, v) = (x - i as f64, z - j as f64);
    let at = |ix: usize, iz: usize| h[iz * 257 + ix];
    let lo = at(i, j) * (1.0 - u) + at(i + 1, j) * u;
    let hi = at(i, j + 1) * (1.0 - u) + at(i + 1, j + 1) * u;
    lo * (1.0 - v) + hi * v
}

#[test]
fn dequantisation_matches_reference_bilinear() {
    let Some(d) = real_terrain("dequantisation") else {
        return;
    };
    let Some(r) = reference_heightmap("dequantisation") else {
        return;
    };
    let mut rng = Rng(0x9e3779b97f4a7c15);
    let mut worst = 0.0f64;
    for _ in 0..1000 {
        let (x, z) = (rng.range(0.0, 255.99), rng.range(0.0, 255.99));
        let got = d.height_at(x, z).unwrap() as f64;
        let want = ref_bilinear(&r, x as f64, z as f64);
        worst = worst.max((got - want).abs());
    }
    println!("max |height_at - reference| over 1000 samples: {worst:.3e} m");
    assert!(worst <= 1e-4, "{worst}");
    // Exact grid corners including the far edge.
    assert!((d.height_at(256.0, 256.0).unwrap() as f64 - r[256 * 257 + 256]).abs() <= 1e-4);
    assert!((d.height_at(0.0, 0.0).unwrap() as f64 - r[0]).abs() <= 1e-4);
}

#[test]
fn seams_are_continuous_and_missing_chunks_are_none() {
    let Some(mut d) = real_terrain("seams") else {
        return;
    };
    let Some(r) = reference_heightmap("seams") else {
        return;
    };
    // Largest height step between adjacent samples bounds the gradient everywhere.
    let mut g = 0.0f64;
    for z in 0..257 {
        for x in 0..256 {
            g = g.max((r[z * 257 + x + 1] - r[z * 257 + x]).abs());
        }
    }
    for z in 0..256 {
        for x in 0..257 {
            g = g.max((r[(z + 1) * 257 + x] - r[z * 257 + x]).abs());
        }
    }
    let mut rng = Rng(7);
    let junctions = [
        (64.0f32, 64.0f32),
        (128.0, 128.0),
        (192.0, 64.0),
        (64.0, 192.0),
        (192.0, 192.0),
        (128.0, 64.0),
    ];
    let mut n = 0;
    for &(jx, jz) in junctions.iter().cycle().take(1000) {
        let x = jx + rng.range(-2.0, 2.0);
        let z = jz + rng.range(-2.0, 2.0);
        let h = d.height_at(x, z).unwrap() as f64;
        for (dx, dz) in [(0.01f32, 0.0f32), (0.0, 0.01)] {
            let h2 = d.height_at(x + dx, z + dz).unwrap() as f64;
            assert!(
                (h2 - h).abs() <= g * 0.01 + 1e-4,
                "seam jump at ({x},{z}): {} > {}",
                (h2 - h).abs(),
                g * 0.01
            );
        }
        n += 1;
    }
    // Samples straddling an exact seam coincide.
    for k in 0..200 {
        let z = 3.0 + k as f32 * 0.9;
        let a = d.height_at(64.0 - 1e-4, z).unwrap();
        let b = d.height_at(64.0 + 1e-4, z).unwrap();
        assert!((a - b).abs() < 1e-3);
    }
    println!("seam samples: {n}, global max sample step {g:.3} m");
    assert!(d.remove_chunk(1, 1));
    assert_eq!(d.height_at(96.0, 96.0), None);
    assert!(d.height_at(32.0, 32.0).is_some());
}

#[test]
fn raycast_agrees_with_rapier() {
    let Some(d) = real_terrain("raycast_vs_rapier") else {
        return;
    };
    let mut rng = Rng(0xfeed_beef);
    let (mut agree, mut total, mut hits, mut disagree) = (0, 0, 0, 0);
    for _ in 0..3000 {
        let o = Vec3::new(rng.range(1.0, 255.0), 0.0, rng.range(1.0, 255.0));
        let o = Vec3::new(
            o.x,
            d.height_at(o.x, o.z).unwrap() + rng.range(0.3, 25.0),
            o.z,
        );
        let dir = Vec3::new(
            rng.range(-1.0, 1.0),
            rng.range(-0.6, 0.3),
            rng.range(-1.0, 1.0),
        )
        .normalize();
        let max_t = 400.0;
        let ours = d.raycast(o, dir, max_t);
        let ray = Ray::new(Vector::new(o.x, o.y, o.z), Vector::new(dir.x, dir.y, dir.z));
        let theirs = d
            .physics()
            .world()
            .cast_ray(&ray, max_t, true, QueryFilter::default())
            .map(|(_, t)| t);
        total += 1;
        match (ours, theirs) {
            (None, None) => agree += 1,
            (Some(a), Some(b)) if (a.t - b).abs() < 2e-3 => {
                agree += 1;
                hits += 1;
            }
            (a, b) => {
                // Rapier 0.36.1 occasionally misses a long ray that crosses several height-field
                // colliders (PI-013). Referee with a brute-force march over the bilinear surface.
                let mut t = 0.0f32;
                let mut brute = None;
                while t <= max_t {
                    let p = o + dir * t;
                    if d.height_at(p.x, p.z).is_some_and(|h| p.y < h) {
                        brute = Some(t);
                        break;
                    }
                    t += 0.02;
                }
                let ok = match (a.map(|h| h.t), brute) {
                    (Some(x), Some(y)) => (x - y).abs() < 0.5,
                    (None, None) => true,
                    _ => false,
                };
                eprintln!(
                    "rapier disagrees: ours={:?} rapier={b:?} brute={brute:?} ok={ok}",
                    a.map(|h| h.t)
                );
                assert!(ok, "our ray cast disagrees with the brute-force reference");
                disagree += 1;
            }
        }
    }
    println!("raycast vs Rapier: {agree}/{total} agree ({hits} hits), {disagree} Rapier misses refereed by brute force");
    assert!(disagree * 200 <= total, "Rapier disagreement above 0.5%");
    assert!(hits > 500, "test rays must actually hit the terrain");
}

#[test]
fn embankment_slope_and_los_on_m0() {
    let Some(d) = real_terrain("embankment") else {
        return;
    };
    let s = d.slope_deg_at(128.0, 128.0).unwrap();
    assert!((s - 38.66).abs() < 0.5, "embankment slope {s}");
    assert!(!d.walkable(128.0, 128.0));
    assert!(d.walkable(128.0, 56.0), "US spawn pad is flat");
    // Low eye line across the embankment is blocked, a high one is clear.
    let us = Vec3::new(128.0, d.height_at(128.0, 56.0).unwrap() + 1.6, 56.0);
    let ger = Vec3::new(128.0, d.height_at(128.0, 200.0).unwrap() + 1.6, 200.0);
    assert!(!d.los(us, ger));
    assert!(d.los(us + Vec3::Y * 60.0, ger + Vec3::Y * 60.0));
    // Same-side line along the flat pad is clear.
    assert!(d.los(us, Vec3::new(140.0, us.y, 70.0)));
}

#[test]
fn build_twice_is_identical_and_memory_is_small() {
    let Some(a) = real_terrain("determinism") else {
        return;
    };
    let Some(b) = real_terrain("determinism") else {
        return;
    };
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.clone().state_hash(), a.state_hash());
    let ha: Vec<_> = a.physics().handles().collect();
    let hb: Vec<_> = b.physics().handles().collect();
    assert_eq!(ha, hb);
    assert_eq!(a.chunk_count(), 16);
    let mb = a.memory_bytes() as f64 / 1e6;
    println!(
        "16 chunks resident collision memory: {} B = {mb:.3} MB",
        a.memory_bytes()
    );
    assert!(mb < 5.0);
    let _ = CollisionData::empty();
}
