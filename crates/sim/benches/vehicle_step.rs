//! `vehicle_step` (M1-022): 40 Shermans, half driving with random steering, half parked, on
//! flat terrain. One `World::step` with no players is dominated by the vehicle system.
//! Budget 2.0 ms per tick (ARCHITECTURE 6.10 stage 3).
//! Run: `cargo bench -p frontline-sim --features test-util --bench vehicle_step`.
use std::time::Instant;

use frontline_sim::vehicle::VehicleCtrl;
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, SimConfig, Team, Vec3, World,
};

fn flat() -> CollisionData {
    let mut v = Vec::new();
    for bz in 0..6 {
        for bx in 0..30 {
            v.push(CollisionChunk::new(
                bx,
                bz,
                vec![5.0f32; 65 * 65],
                vec![0; 64 * 64],
            ));
        }
    }
    CollisionData::from_chunks(v)
}

fn main() {
    let mut w = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 1,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        &flat(),
    );
    let mut ids = Vec::new();
    for i in 0..40u32 {
        let x = 100.0 + (i % 10) as f32 * 25.0;
        let z = 100.0 + (i / 10) as f32 * 25.0;
        ids.push(
            w.spawn_vehicle("us_m4a1_75", Team::Us, Vec3::new(x, 5.9, z), 0.0)
                .unwrap(),
        );
    }
    for (k, &id) in ids.iter().enumerate() {
        if k % 2 == 0 {
            w.set_vehicle_ctrl(
                id,
                VehicleCtrl {
                    throttle: 0.8,
                    steer: ((k % 5) as f32 - 2.0) * 0.2,
                    ..Default::default()
                },
            );
        }
    }
    for _ in 0..120 {
        w.step(&[]);
    }
    let ticks = 600;
    let mut best = f64::MAX;
    for _ in 0..5 {
        let t = Instant::now();
        for _ in 0..ticks {
            w.step(&[]);
        }
        best = best.min(t.elapsed().as_secs_f64() / ticks as f64);
    }
    println!(
        "vehicle_step: {:.3} ms per tick for 40 vehicles (20 driving); budget 2.0 ms",
        best * 1e3
    );
}
