//! `supply` (M1-020): 40 Supply Crates, 6 Ammo Crates, a Depot and 100 players. Reports the
//! per-tick cost of a world step with the supply systems active minus the same world without
//! any supply state. Budget 0.1 ms per tick.
//! Run: `cargo bench -p frontline-sim --features test-util --bench supply`.
use std::time::Instant;

use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, SimConfig, SpawnReq, Team, Vec3,
    World,
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

fn input(use_btn: bool) -> InputCmd {
    InputCmd {
        seq: 0,
        tick: 0,
        move_x: 0,
        move_z: 0,
        yaw: 49152,
        pitch: 0,
        buttons: if use_btn { buttons::USE } else { 0 },
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    }
}

fn run(supply: bool) -> f64 {
    let cfg = SimConfig {
        tick_hz: 30,
        match_seed: 3,
        map: MapId(0),
        mode: GameMode::Sandbox,
        data: GameData::embedded(),
    };
    let mut w = World::new(cfg, &flat());
    for i in 0..100u8 {
        w.spawn_player(SpawnReq {
            player: i,
            team: if i % 2 == 0 { Team::Us } else { Team::Ger },
            pos: Vec3::new(300.0 + f32::from(i) * 8.0, 5.02, 100.0),
            yaw: 0.0,
        })
        .unwrap();
        if i < 6 {
            w.set_support(i, true);
        }
    }
    if supply {
        w.add_depot(Team::Us, Vec3::new(60.0, 5.0, 60.0));
        for k in 0..40u32 {
            let x = 200.0 + (k % 10) as f32 * 3.0;
            let z = 200.0 + (k / 10) as f32 * 3.0;
            w.debug_add_crate(Team::Us, Vec3::new(x, 5.0, z), 100);
        }
        for p in 0..6u8 {
            w.place_ammo_crate(p).unwrap();
        }
    }
    // Players hold USE (the supply system's interaction scan) every tick.
    let ins: Vec<_> = (0..100u8).map(|p| (p, input(true))).collect();
    let mut samples = Vec::new();
    for _ in 0..600u32 {
        let t0 = Instant::now();
        w.step(&ins);
        samples.push(t0.elapsed().as_secs_f64() * 1e3);
    }
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

fn main() {
    let a = run(true);
    let b = run(false);
    println!(
        "world step, 100 players, 40 crates + 6 ammo: with supply {a:.3} ms, without {b:.3} ms, supply {:.3} ms",
        a - b
    );
}
