//! `world_step_empty`: 10 and 100 players, neutral inputs. Records ns/step only (no gate).
use std::time::Instant; // benches are outside the purity scan (src only)

use frontline_protocol::msg::input::InputCmd;
use frontline_sim::{
    CollisionData, GameData, GameMode, MapId, SimConfig, SpawnReq, Team, World,
};

fn run(n: u8) {
    let cfg = SimConfig {
        tick_hz: 30,
        match_seed: 1,
        map: MapId(0),
        mode: GameMode::Sandbox,
        data: GameData::embedded(),
    };
    let mut w = World::new(cfg, &CollisionData::empty());
    for p in 0..n {
        w.spawn_player(SpawnReq {
            player: p,
            team: if p % 2 == 0 { Team::Us } else { Team::Ger },
            pos: glam_vec(p),
            yaw: 0.0,
        })
        .unwrap();
    }
    let cmd = InputCmd {
        seq: 0,
        tick: 0,
        move_x: 0,
        move_z: 0,
        yaw: 100,
        pitch: 0,
        buttons: 0,
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    };
    let inputs: Vec<_> = (0..n).map(|p| (p, cmd)).collect();
    for _ in 0..1000 {
        w.step(&inputs);
    }
    let iters = 20_000u32;
    let t0 = Instant::now();
    for _ in 0..iters {
        w.step(&inputs);
    }
    let ns = t0.elapsed().as_nanos() as f64 / iters as f64;
    println!("world_step_empty players={n}: {:.0} ns/step ({:.2} us)", ns, ns / 1000.0);
}

fn glam_vec(p: u8) -> frontline_sim::Vec3 {
    frontline_sim::Vec3::new(p as f32, 0.0, 0.0)
}

fn main() {
    run(10);
    run(100);
}
