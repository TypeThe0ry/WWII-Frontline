//! `projectile_step` (M1-012; ARCHITECTURE 6.10 stage 4): one projectile segment = 4 sub-steps
//! plus one layer query with penetration, and a tick of 400 projectiles. Plain `Instant` harness.
//! Run: `cargo bench -p frontline-sim --bench projectile_step`.
use std::hint::black_box;
use std::time::Instant;

use frontline_sim::ballistics::{
    step_tick, trace_with_penetration, Layer, LayerWorld, MaterialTable, PenAmmo,
};
use frontline_sim::rng::{Purpose, Rng};
use frontline_sim::{Tick, Vec3};

struct Wall(frontline_sim::ballistics::MaterialId);
impl LayerWorld for Wall {
    fn first_layer(&self, o: Vec3, d: Vec3, _max_t: f32) -> Option<Layer> {
        let t = (5.0 - o.x) / d.x;
        (t > 0.0).then(|| Layer {
            material: self.0,
            entry: o + d * t,
            exit: o + d * ((5.05 - o.x) / d.x),
            normal: -Vec3::X,
        })
    }
}

fn median_ns(mut f: impl FnMut(), iters: usize, rounds: usize) -> f64 {
    let mut v: Vec<f64> = (0..rounds)
        .map(|_| {
            let t = Instant::now();
            for _ in 0..iters {
                f();
            }
            t.elapsed().as_nanos() as f64 / iters as f64
        })
        .collect();
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn main() {
    let table = MaterialTable::embedded();
    let wall = Wall(table.id_of("thin_wood").unwrap());
    let ammo = PenAmmo { pen_power: 220.0 };
    let dir = Vec3::new(1.0, 0.05, 0.0).normalize();
    let mut rng = Rng::for_(1, Tick(1), 1, Purpose::Penetration);
    let seg = median_ns(
        || {
            let s = step_tick(
                black_box(Vec3::ZERO),
                black_box(Vec3::new(853.0, 10.0, 0.0)),
                0.00075,
                9.81,
                1.0 / 30.0,
            );
            let r = trace_with_penetration(
                &wall,
                &table,
                black_box(s.from),
                black_box(dir),
                30.0,
                &ammo,
                &mut rng,
            );
            black_box(r);
        },
        20_000,
        15,
    );
    let tick400 = median_ns(
        || {
            for i in 0..400 {
                let s = step_tick(
                    Vec3::new(i as f32, 1.5, 0.0),
                    Vec3::new(853.0, 10.0, 0.0),
                    0.00075,
                    9.81,
                    1.0 / 30.0,
                );
                black_box(s);
            }
        },
        200,
        15,
    ) / 1e6;
    println!(
        "projectile segment (4 sub-steps + 1 layer trace): median {seg:.0} ns  (budget 5000 ns)"
    );
    println!("400 projectiles integrated per tick: median {tick400:.3} ms  (budget 2.5 ms incl. hit tests)");
}
