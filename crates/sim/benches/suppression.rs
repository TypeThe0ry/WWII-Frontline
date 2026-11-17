//! `suppression` (M1-017 acceptance 9): 50 shooters firing at 50 enemies, 100 players in the
//! world. Reports the per-tick cost of a world step with 50 new near-missing bullets per tick,
//! with the targets on the enemy team (detection runs) minus the same with all players on one
//! team (detection skipped), i.e. the cost of suppression itself. Budget 0.3 ms per tick.
//! Run: `cargo bench -p frontline-sim --bench suppression`.
use std::time::Instant;

use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, SimConfig, SpawnReq, Team, Vec3,
    World,
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

fn run(enemies: bool) -> f64 {
    let data = GameData::embedded();
    let cfg = SimConfig {
        tick_hz: 30,
        match_seed: 3,
        map: MapId(0),
        mode: GameMode::Sandbox,
        data,
    };
    let mut w = World::new(cfg, &flat());
    for i in 0..100u8 {
        let shooter = i < 50;
        let row = f32::from(i % 50);
        w.spawn_player(SpawnReq {
            player: i,
            team: if shooter || !enemies {
                Team::Ger
            } else {
                Team::Us
            },
            pos: Vec3::new(if shooter { 40.0 } else { 140.0 }, 5.02, 20.0 + row * 3.0),
            yaw: 0.0,
        })
        .unwrap();
    }
    let weapon = data
        .weapons
        .iter()
        .position(|x| x.id == "us_m1_garand")
        .unwrap() as u16;
    let mut samples = Vec::new();
    for t in 0..600u32 {
        for s in 0..50u8 {
            // 2 m to the side of the player in the opposite column.
            let o = Vec3::new(40.5, 6.4, 20.0 + f32::from(s) * 3.0);
            let dir = Vec3::new(100.0, 0.0, 2.0 + (t % 3) as f32 * 0.4).normalize();
            let _ = w.spawn_projectile(s, weapon, o, dir * 853.0);
        }
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
        "world step, 50 bullets/tick, 100 players: with enemies {a:.3} ms, same team {b:.3} ms, suppression {:.3} ms",
        a - b
    );
}
