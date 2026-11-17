//! M1-015 acceptance 8: 40 grenades stepped for a tick plus one blast on 30 players (line of
//! sight and damage each). Budget 0.5 ms. Run: `cargo bench -p frontline-sim --bench explosion`.
use std::hint::black_box;
use std::time::Instant;

use frontline_sim::throwables::grenade::{self, Flight};
use frontline_sim::throwables::{explosion_damage, Occlusion};
use frontline_sim::{CollisionChunk, CollisionData, Vec3};

fn main() {
    let col = CollisionData::from_chunks(vec![CollisionChunk::new(
        0,
        0,
        vec![5.0; 65 * 65],
        vec![0; 64 * 64],
    )]);
    let n = 5_000u32;
    let t0 = Instant::now();
    let mut acc = 0.0f32;
    for i in 0..n {
        let mut gs: Vec<Flight> = (0..40)
            .map(|k| Flight {
                pos: Vec3::new(10.0 + k as f32, 8.0, 10.0),
                vel: Vec3::new(8.0, 5.0, 1.0),
                rest: 0,
            })
            .collect();
        for g in &mut gs {
            grenade::step(g, &col, 30, 0.4, 0.6);
        }
        let c = Vec3::new(20.0, 5.2, 20.0);
        for k in 0..30 {
            let chest = Vec3::new(20.0 + (k % 7) as f32, 6.2, 20.0 + (k / 7) as f32);
            let occ = if col.los(c, chest) {
                Occlusion::Clear
            } else {
                Occlusion::Solid
            };
            acc += explosion_damage(220.0, (chest - c).length(), 8.0, occ);
        }
        black_box((&gs, i));
    }
    let ms = t0.elapsed().as_secs_f64() * 1e3 / f64::from(n);
    println!("explosion: {ms:.4} ms per tick (40 grenades + 1 blast on 30 players), acc {acc}");
}
