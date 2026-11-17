//! `objectives` (M1-018): 3 capture points, 100 players clustered on them. Reports the per-tick
//! cost of a world step with the objectives system active minus the same world without any
//! objectives, i.e. the cost of capture itself. Budget 0.05 ms per tick.
//! Run: `cargo bench -p frontline-sim --bench objectives`.
use std::time::Instant;

use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, ObjectiveSpec, SimConfig, SpawnReq,
    Team, Vec3, World,
};

fn flat() -> CollisionData {
    let mut v = Vec::new();
    for bz in 0..3 {
        for bx in 0..5 {
            let h = vec![5.0f32; 65 * 65];
            v.push(CollisionChunk::new(bx, bz, h, vec![0; 64 * 64]));
        }
    }
    CollisionData::from_chunks(v)
}

fn run(with_objectives: bool) -> f64 {
    let cfg = SimConfig {
        tick_hz: 30,
        match_seed: 3,
        map: MapId(0),
        mode: GameMode::Sandbox,
        data: GameData::embedded(),
    };
    let mut w = World::new(cfg, &flat());
    let pts = [(60.0f32, 60.0f32), (160.0, 100.0), (260.0, 60.0)];
    if with_objectives {
        let specs: Vec<ObjectiveSpec> = pts
            .iter()
            .enumerate()
            .map(|(i, p)| ObjectiveSpec {
                id: i as u16,
                pos: Vec3::new(p.0, 5.0, p.1),
                radius_m: 12.0,
                owner: None,
            })
            .collect();
        w.set_objectives(&specs);
    }
    for i in 0..100u8 {
        let p = pts[usize::from(i) % 3];
        let k = f32::from(i / 3);
        w.spawn_player(SpawnReq {
            player: i,
            team: if i % 2 == 0 { Team::Ger } else { Team::Us },
            pos: Vec3::new(p.0 + (k % 6.0) - 3.0, 5.02, p.1 + (k / 6.0).floor() - 3.0),
            yaw: 0.0,
        })
        .unwrap();
    }
    let mut samples = Vec::new();
    for _ in 0..600u32 {
        let t0 = Instant::now();
        w.step(&[]);
        samples.push(t0.elapsed().as_secs_f64() * 1e3);
    }
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

fn main() {
    let a = run(true);
    let b = run(false);
    println!(
        "world step, 3 points, 100 players: with objectives {a:.3} ms, without {b:.3} ms, objectives {:.3} ms",
        a - b
    );
}
