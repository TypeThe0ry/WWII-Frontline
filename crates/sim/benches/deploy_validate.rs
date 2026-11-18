//! `deploy_validate` (M1-021): 100 roster players (50 per team in squads of 6) each submit
//! a valid HQ deploy; reports the total and per-request cost. Budget 0.3 ms for all 100,
//! 5 us per request.
//! Run: `cargo bench -p frontline-sim --features test-util --bench deploy_validate`.
use std::time::Instant;

use frontline_sim::deploy::{DeployReq, SPAWN_HQ};
use frontline_sim::squad::{SquadKind, SquadOp};
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
    let data = GameData::embedded();
    let mut w = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 1,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data,
        },
        &flat(),
    );
    w.set_hq(Team::Us, Vec3::new(60.0, 5.0, 60.0));
    w.set_hq(Team::Ger, Vec3::new(1860.0, 5.0, 60.0));
    let role = data.roles.iter().position(|r| r.id == "rifleman").unwrap() as u8;
    let lo_us = data
        .loadouts
        .iter()
        .position(|l| l.id == "us_rifleman_1")
        .unwrap() as u8;
    let lo_ger = data
        .loadouts
        .iter()
        .position(|l| l.id == "ger_rifleman_1")
        .unwrap() as u8;
    let mut reqs = Vec::new();
    for i in 0..50u8 {
        w.roster_add(i, Team::Us).unwrap();
        w.roster_add(64 + i, Team::Ger).unwrap();
    }
    for (base, team_lo) in [(0u8, lo_us), (64, lo_ger)] {
        for s in 0..9u8 {
            let lead = base + s * 6;
            if lead >= base + 50 {
                break;
            }
            w.squad_op(
                lead,
                SquadOp::Create {
                    kind: SquadKind::Infantry,
                    suffix: String::new(),
                },
            )
            .unwrap();
            let sid = w.squads().squad_of(lead).unwrap().id;
            for m in (lead + 1)..(lead + 6).min(base + 50) {
                w.squad_op(m, SquadOp::Join { squad: sid }).unwrap();
            }
        }
        for p in base..base + 50 {
            let sid = w.squads().squad_of(p).map_or(0, |s| s.id as u8);
            reqs.push((
                p,
                DeployReq {
                    spawn_point: SPAWN_HQ,
                    role,
                    loadout: team_lo,
                    squad: sid,
                },
            ));
        }
    }
    assert_eq!(reqs.len(), 100);
    for (p, r) in &reqs {
        w.validate_deploy(*p, r).expect("valid");
    }
    let iters = 2000;
    let mut best = f64::MAX;
    for _ in 0..5 {
        let t = Instant::now();
        for _ in 0..iters {
            for (p, r) in &reqs {
                std::hint::black_box(w.validate_deploy(*p, r)).ok();
            }
        }
        best = best.min(t.elapsed().as_secs_f64() / iters as f64);
    }
    println!(
        "deploy_validate: {:.1} us for 100 requests ({:.2} us each); budget 300 us / 5 us",
        best * 1e6,
        best * 1e6 / 100.0
    );
}
