//! Collision logic on small synthetic height fields (no map binaries needed).
mod common;

use common::*;
use frontline_sim::{CollisionData, Vec3};

fn plane(sx: f32, sz: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, z| 5.0 + sx * x + sz * z
}

#[test]
fn bilinear_edges_and_corners() {
    let f = plane(0.5, 0.25);
    let d = synth_data(2, 2, &f);
    for &(x, z) in &[
        (0.0, 0.0),
        (64.0, 0.0),
        (0.0, 64.0),
        (128.0, 128.0),
        (63.999, 64.0),
        (64.0, 63.5),
    ] {
        let got = d.height_at(x, z).unwrap();
        assert!(
            (got - f(x, z)).abs() < 1e-3,
            "({x},{z}) {got} vs {}",
            f(x, z)
        );
    }
    // Interior of a cell: bilinear of a plane is exact.
    let got = d.height_at(10.3, 20.7).unwrap();
    assert!((got - f(10.3, 20.7)).abs() < 1e-3);
    // Outside / non finite.
    assert_eq!(d.height_at(-0.01, 5.0), None);
    assert_eq!(d.height_at(128.01, 5.0), None);
    assert_eq!(d.height_at(f32::NAN, 5.0), None);
}

#[test]
fn flat_and_45_degree_normals() {
    let flat = synth_data(1, 1, &|_, _| 3.0);
    let n = flat.normal_at(10.5, 10.5).unwrap();
    assert!((n - Vec3::Y).length() < 1e-6);
    assert!(flat.slope_deg_at(10.5, 10.5).unwrap() < 1e-3);

    let ramp = synth_data(1, 1, &|x, _| x);
    let n = ramp.normal_at(10.2, 7.7).unwrap();
    let s = std::f32::consts::FRAC_1_SQRT_2;
    assert!((n - Vec3::new(-s, s, 0.0)).length() < 1e-5, "{n:?}");
    let slope = ramp.slope_deg_at(10.2, 7.7).unwrap();
    assert!((slope - 45.0).abs() < 1e-3, "{slope}");
    assert!(!ramp.walkable(10.2, 7.7));
}

#[test]
fn walkable_threshold_is_35_degrees() {
    for (deg, ok) in [(30.0f32, true), (34.5, true), (35.5, false), (40.0, false)] {
        let k = deg.to_radians().tan();
        let d = synth_data(1, 1, &move |_, z| k * z);
        assert_eq!(d.walkable(20.5, 20.5), ok, "{deg} deg");
    }
    assert!(!CollisionData::empty().walkable(1.0, 1.0));
}

fn hill(x: f32, z: f32) -> f32 {
    // 20 m tall gaussian hill centred at (64, 64) over a 2 m plain.
    let (dx, dz) = (x - 64.0, z - 64.0);
    2.0 + 20.0 * (-(dx * dx + dz * dz) / (2.0 * 12.0 * 12.0)).exp()
}

#[test]
fn los_over_and_through_a_hill() {
    let d = synth_data(2, 2, &hill);
    let a = Vec3::new(10.0, 3.7, 64.0);
    let b = Vec3::new(118.0, 3.7, 64.0);
    assert!(!d.los(a, b), "hill blocks the low line");
    assert!(!d.los(b, a), "symmetric");
    let a_hi = Vec3::new(10.0, 40.0, 64.0);
    let b_hi = Vec3::new(118.0, 40.0, 64.0);
    assert!(d.los(a_hi, b_hi), "line far above the hill is clear");
    let c = Vec3::new(10.0, 3.7, 10.0);
    let e = Vec3::new(110.0, 3.7, 20.0);
    assert!(d.los(c, e), "a line passing well beside the hill is clear");
    // Starting under ground blocks.
    assert!(!d.los(Vec3::new(10.0, 0.0, 10.0), Vec3::new(20.0, 10.0, 10.0)));
    // Leaving the loaded area is not a clear line.
    assert!(!d.los(a_hi, Vec3::new(400.0, 40.0, 64.0)));
    assert!(!CollisionData::empty().los(a_hi, b_hi));
}

#[test]
fn raycast_cases() {
    let d = synth_data(2, 2, &hill);
    // Straight down onto the summit.
    let h = d
        .raycast(Vec3::new(64.0, 50.0, 64.0), -Vec3::Y, 100.0)
        .unwrap();
    let top = d.height_at(64.0, 64.0).unwrap();
    assert!((h.point.y - top).abs() < 0.05, "{h:?} vs {top}");
    assert!((h.t - (50.0 - h.point.y)).abs() < 1e-3);
    assert!(h.normal.y > 0.99);
    // Too short.
    assert!(d
        .raycast(Vec3::new(64.0, 50.0, 64.0), -Vec3::Y, 10.0)
        .is_none());
    // From inside the terrain: hit at t = 0.
    let h = d
        .raycast(Vec3::new(20.0, 0.5, 20.0), Vec3::X, 50.0)
        .unwrap();
    assert_eq!(h.t, 0.0);
    // Zero direction and negative range.
    assert!(d
        .raycast(Vec3::new(1.0, 50.0, 1.0), Vec3::ZERO, 10.0)
        .is_none());
    // Extremely far: terrain is only 128 m wide, the ray misses at 1e6 m without trouble.
    assert!(d
        .raycast(Vec3::new(1.0, 30.0, 1.0), Vec3::new(1.0, 0.001, 1.0), 1.0e6)
        .is_none());
    // Sloping ray across the chunk seam (x = 64) hits the far side of the hill.
    let h = d
        .raycast(
            Vec3::new(30.0, 25.0, 64.5),
            Vec3::new(1.0, -0.2, 0.0),
            200.0,
        )
        .unwrap();
    assert!(h.point.x > 30.0 && h.point.x < 128.0, "{h:?}");
    assert!((h.point.y - d.height_at(h.point.x, h.point.z).unwrap()).abs() < 0.2);
    // Reverse direction ray hits the near side.
    let h2 = d
        .raycast(
            Vec3::new(100.0, 25.0, 64.5),
            Vec3::new(-1.0, -0.2, 0.0),
            200.0,
        )
        .unwrap();
    assert!(h2.point.x < 100.0 && h2.point.x > h.point.x - 40.0);
}

#[test]
fn ground_probe_reports_surface() {
    let d = synth_data(1, 1, &|_, z| z * 0.5);
    let g = d.ground_probe(Vec3::new(10.0, 30.0, 20.0), 50.0).unwrap();
    assert!((g.point.y - 10.0).abs() < 1e-4);
    assert!((g.slope_deg - 26.565).abs() < 0.01, "{}", g.slope_deg);
    assert!(d.ground_probe(Vec3::new(10.0, 30.0, 20.0), 5.0).is_none());
    // Inside terrain: snapped up to the surface.
    let g = d.ground_probe(Vec3::new(10.0, 1.0, 20.0), 5.0).unwrap();
    assert!((g.point.y - 10.0).abs() < 1e-4);
    assert!(d.ground_probe(Vec3::new(-5.0, 30.0, 20.0), 50.0).is_none());
}

#[test]
fn insert_remove_and_missing_chunks() {
    let mut d = CollisionData::empty();
    assert_eq!(d.height_at(1.0, 1.0), None);
    d.insert_chunk(synth_chunk(0, 0, &|_, _| 1.0));
    d.insert_chunk(synth_chunk(1, 0, &|_, _| 2.0));
    assert_eq!(d.chunk_count(), 2);
    assert!((d.height_at(63.5, 1.0).unwrap() - 1.0).abs() < 1e-5);
    assert!((d.height_at(64.5, 1.0).unwrap() - 2.0).abs() < 1e-5);
    // Border sample belongs to the loaded neighbour when the primary one is missing.
    assert!((d.height_at(128.0, 1.0).unwrap() - 2.0).abs() < 1e-5);
    assert!(d.remove_chunk(1, 0));
    assert!(!d.remove_chunk(1, 0));
    assert_eq!(d.height_at(64.5, 1.0), None);
    assert!(!d.walkable(64.5, 1.0));
    assert!(
        (d.height_at(64.0, 1.0).unwrap() - 1.0).abs() < 1e-5,
        "exact border falls back to chunk 0"
    );
    assert_eq!(d.memory_bytes(), 65 * 65 * 4 + 64 * 64);
}

#[test]
fn rebuilds_are_identical_and_order_independent() {
    let f = hill;
    let chunks = |rev: bool| {
        let mut v: Vec<_> = (0..2)
            .flat_map(|bz| (0..2).map(move |bx| (bx, bz)))
            .map(|(bx, bz)| synth_chunk(bx, bz, &f))
            .collect();
        if rev {
            v.reverse();
        }
        v
    };
    let a = CollisionData::from_chunks(chunks(false));
    let b = CollisionData::from_chunks(chunks(false));
    let c = CollisionData::from_chunks(chunks(true));
    let cl = a.clone();
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(
        a.state_hash(),
        c.state_hash(),
        "insertion order must not matter"
    );
    assert_eq!(a.state_hash(), cl.state_hash());
    let order: Vec<_> = a.physics().handles().map(|(k, _)| k).collect();
    assert_eq!(
        order,
        vec![(0, 0), (0, 1), (1, 0), (1, 1)],
        "(bz, bx) ascending"
    );
    let order_b: Vec<_> = b.physics().handles().collect();
    let order_a: Vec<_> = a.physics().handles().collect();
    assert_eq!(order_a, order_b, "same collider handles");
    // Different data changes the hash.
    let other = synth_data(2, 2, &|x, z| hill(x, z) + 0.01);
    assert_ne!(a.state_hash(), other.state_hash());
}
