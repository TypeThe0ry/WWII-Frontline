//! `spawn` (M1-019): 100 players, 6 active Garrisons and HQ rules. Reports the per-tick cost of
//! a world step with the spawn rules minus the same world without them. Budget 0.1 ms per tick.
//! Run: `cargo bench -p frontline-sim --bench spawn`.
use std::time::Instant;

use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::supply_iface::SupplyProvider;
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, SimConfig, SpawnReq, Team, Vec3,
    World,
};

struct Rich;
impl SupplyProvider for Rich {
    fn available(&self, _: Vec3, _: f32) -> u32 {
        1_000_000
    }
    fn consume(&mut self, _: Vec3, _: f32, _: u32) -> bool {
        true
    }
}

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

fn run(rules: bool) -> f64 {
    let cfg = SimConfig {
        tick_hz: 30,
        match_seed: 3,
        map: MapId(0),
        mode: GameMode::Sandbox,
        data: GameData::embedded(),
    };
    let mut w = World::new(cfg, &flat());
    for i in 0..100u8 {
        let (team, pos) = if i < 18 {
            // Builders: three US players on each of the six Garrison sites.
            let k = f32::from(i / 3);
            (
                Team::Us,
                Vec3::new(300.0 + k * 160.0 + f32::from(i % 3), 5.02, 300.0),
            )
        } else {
            (
                if i % 2 == 0 { Team::Us } else { Team::Ger },
                Vec3::new(300.0 + f32::from(i) * 12.0, 5.02, 100.0),
            )
        };
        w.spawn_player(SpawnReq {
            player: i,
            team,
            pos,
            yaw: 0.0,
        })
        .unwrap();
    }
    if rules {
        w.set_hq(Team::Us, Vec3::new(60.0, 5.0, 60.0));
        w.set_hq(Team::Ger, Vec3::new(1860.0, 5.0, 60.0));
        w.set_supply_provider(Box::new(Rich));
        w.set_officer(0, true);
        let objs: Vec<frontline_sim::ObjectiveSpec> = [380.0f32, 700.0, 1020.0]
            .iter()
            .enumerate()
            .map(|(i, &x)| frontline_sim::ObjectiveSpec {
                id: i as u16,
                pos: Vec3::new(x, 5.0, 1000.0),
                radius_m: 10.0,
                owner: None,
            })
            .collect();
        w.set_objectives(&objs);
        for k in 0..6u8 {
            let x = 300.0 + f32::from(k) * 160.0;
            w.place_garrison(0, Vec3::new(x, 5.0, 300.0)).unwrap();
        }
        for _ in 0..700 {
            let ins: Vec<_> = (0..100u8).map(|p| (p, input(p < 18))).collect();
            w.step(&ins);
        }
        assert!(w
            .garrison_list()
            .iter()
            .all(|g| g.state == frontline_sim::spawn::garrison::GarrisonState::Active));
    }
    let ins: Vec<_> = (0..100u8).map(|p| (p, input(false))).collect();
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
        "world step, 100 players, 6 Garrisons: with spawn rules {a:.3} ms, without {b:.3} ms, spawn {:.3} ms",
        a - b
    );
}
