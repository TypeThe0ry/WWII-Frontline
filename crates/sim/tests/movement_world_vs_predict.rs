//! `movement_world_vs_predict`: `World` (server) and `PredictSim` (client) give bit-identical
//! single-player results for the same inputs (ARCHITECTURE 8.1, P2).
mod common;

use common::*;
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::predict::PredictSim;
use frontline_sim::{
    CollisionData, GameData, GameMode, MapId, PlayerAuthState, SimConfig, SpawnReq, Team, Tick,
    Vec3, World,
};

fn cfg() -> SimConfig {
    SimConfig {
        tick_hz: 30,
        match_seed: 3,
        map: MapId(0),
        mode: GameMode::Sandbox,
        data: GameData::embedded(),
    }
}

fn inputs(n: usize) -> Vec<InputCmd> {
    let mut r = Rng(0xabcdef);
    let mut cur = cmd(0, 0, 0, 0, 0);
    (0..n)
        .map(|i| {
            if i % 5 == 0 {
                let b = r.next_u64();
                let mut btn = 0;
                if b & 3 == 0 {
                    btn |= BTN_JUMP;
                }
                if b >> 2 & 7 == 0 {
                    btn |= BTN_CROUCH;
                }
                if b >> 5 & 3 == 0 {
                    btn |= BTN_SPRINT;
                }
                cur = cmd(
                    ((r.next_u64() % 255) as i32 - 127) as i8,
                    ((r.next_u64() % 255) as i32 - 127) as i8,
                    (r.next_u64() & 0xffff) as u16,
                    btn,
                    (b >> 8 & 1) as u8,
                );
            }
            cur
        })
        .collect()
}

fn compare(col: &CollisionData, start: Vec3) {
    let mut w = World::new(cfg(), col);
    w.spawn_player(SpawnReq {
        player: 7,
        team: Team::Us,
        pos: start,
        yaw: 0.0,
    })
    .unwrap();
    let mut pr = PredictSim::new(cfg(), col, 7);
    pr.reset_to(&PlayerAuthState::spawn(start, 0.0), Tick(0));
    assert!(w.player_state(7).unwrap().bit_eq(&pr.state()));
    let cmds = inputs(3_000);
    for (i, c) in cmds.iter().enumerate() {
        w.step(&[(7, *c)]);
        let a = w.player_state(7).unwrap();
        let b = pr.step_player(c);
        assert!(a.bit_eq(&b), "tick {i}: world {a:?} vs predict {b:?}");
    }
    assert_eq!(pr.tick(), Tick(3_000));
    // The state survives its byte representation (self_state / Predictor.reset).
    let st = pr.state();
    assert_eq!(PlayerAuthState::from_bytes(&st.to_bytes()), Some(st));
}

#[test]
fn world_vs_predict_synthetic() {
    let col = synth_data(3, 3, &|x, z| {
        4.0 + 0.02 * x + 0.015 * z + 0.5 * ((x * 0.37) % 1.0)
    });
    compare(&col, Vec3::new(96.0, 8.0, 96.0));
}

#[test]
fn world_vs_predict_m0_terrain() {
    let Some(col) = real_terrain("world_vs_predict_m0_terrain") else {
        return;
    };
    let y = col.height_at(60.0, 60.0).unwrap() + 1.0;
    compare(&col, Vec3::new(60.0, y, 60.0));
}

#[test]
fn world_moves_players_in_id_order_and_independently() {
    let col = flat(2, 5.0);
    let mut w = World::new(cfg(), &col);
    for p in [2u8, 5, 9] {
        w.spawn_player(SpawnReq {
            player: p,
            team: Team::Ger,
            pos: Vec3::new(20.0 + f32::from(p), 5.02, 20.0),
            yaw: 0.0,
        })
        .unwrap();
    }
    for _ in 0..60 {
        w.step(&[
            (2, fwd(0, 0)),
            (5, fwd(BTN_SPRINT, 0)),
            (9, cmd(0, 0, 0, 0, 0)),
        ]);
    }
    let z = |w: &World, p| w.player_state(p).unwrap().pos.z;
    assert!((z(&w, 2) - 20.0 - 8.8).abs() < 0.4, "run 2 s: {}", z(&w, 2));
    assert!(
        (z(&w, 5) - 20.0 - 12.8).abs() < 0.4,
        "sprint 2 s: {}",
        z(&w, 5)
    );
    assert!((z(&w, 9) - 20.0).abs() < 1e-3, "idle");
    // Missing input repeats the last command.
    w.step(&[(2, fwd(0, 0))]);
    assert!(z(&w, 5) > z(&w, 9) + 12.0);
    assert_eq!(w.view().players().len(), 3);
}
