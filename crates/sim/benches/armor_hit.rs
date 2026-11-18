//! `armor_hit` (M1-023): pure AP verdict cost. Budget <= 2 us per call.
//! Run: `cargo bench -p frontline-sim --bench armor_hit`.
use std::hint::black_box;
use std::time::Instant;

use frontline_sim::vehicle::armor::{armor_hit, ShellKind, ShellParams};
use frontline_sim::Vec3;

fn main() {
    let shell = ShellParams {
        kind: ShellKind::Ap,
        caliber_mm: 75.0,
        p100_mm: 82.0,
        drop_pct_per_100m: 3.5,
        structure_damage_hp: 400.0,
    };
    let n = Vec3::new(0.0, 0.0, 1.0);
    let iters = 1_000_000u32;
    let t0 = Instant::now();
    let mut pens = 0u32;
    for i in 0..iters {
        let a = (i % 80) as f32 * 0.0174;
        let v = Vec3::new(a.sin(), 0.0, -a.cos());
        let r = armor_hit(&shell, black_box(100.0 + (i % 500) as f32), n, v, black_box(51.0), 1.0);
        if matches!(r, frontline_sim::vehicle::armor::ArmorResult::Pen { .. }) {
            pens += 1;
        }
    }
    let dt = t0.elapsed();
    println!("armor_hit: {:.1} ns/call ({pens} pens)", dt.as_nanos() as f64 / iters as f64);
}
