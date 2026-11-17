//! `think_10` / `think_100` (bot step per tick, ARCHITECTURE 6.10 stage 6: <= 3 ms for <= 60
//! bots, 1 ms for the M0 ten) and `path_query` (16.2: <= 1 ms/tick of queries).
//!
//! Real M0 terrain, real `World` (fights, deaths, respawns happen), plain `Instant` harness like
//! the other benches; writes `reports/baselines/bots-m0.json`. Needs the git-ignored terrain:
//! `PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0`.
//! Run with `cargo bench -p frontline-bots --bench think`.

#[path = "../tests/common/mod.rs"]
mod common;

use std::time::Instant;

use common::*;
use frontline_bots::PathFinder;

fn q(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[(((v.len() - 1) as f64) * p).ceil() as usize]
}

fn think_bench(
    col: &frontline_sim::CollisionData,
    per_team: u8,
    warm: usize,
    ticks: usize,
) -> serde_json::Value {
    let mut a = Arena::new(col, 5);
    let map = a.bot_map();
    a.add_squads(&map, per_team);
    for _ in 0..warm {
        a.step();
    }
    a.think_ns.clear();
    a.path_ns.clear();
    let (k0, s0) = (a.kills, a.shots);
    for _ in 0..ticks {
        a.step();
    }
    let mut think: Vec<f64> = a.think_ns.iter().map(|n| *n as f64 / 1e6).collect();
    let mut path: Vec<f64> = a.path_ns.iter().map(|n| *n as f64 / 1e6).collect();
    let n = u32::from(per_team) * 2;
    let r = serde_json::json!({
        "bots": n,
        "ticks": ticks,
        "think_ms": { "median": q(&mut think, 0.5), "p99": q(&mut think, 0.99), "max": think.last().copied().unwrap() },
        "path_ms": { "median": q(&mut path, 0.5), "p99": q(&mut path, 0.99), "max": path.last().copied().unwrap() },
        "kills_in_window": a.kills - k0,
        "shots_in_window": a.shots - s0,
    });
    println!(
        "think_{n}: per-tick all-bot think  median {:.4} ms  P99 {:.4} ms  max {:.4} ms | path queries/tick median {:.4} P99 {:.4} max {:.4} ms ({} kills, {} shots in window)",
        q(&mut think, 0.5), q(&mut think, 0.99), think.last().unwrap(),
        q(&mut path, 0.5), q(&mut path, 0.99), path.last().unwrap(),
        a.kills - k0, a.shots - s0
    );
    r
}

fn path_bench(col: &frontline_sim::CollisionData) -> serde_json::Value {
    let a = Arena::new(col, 1);
    let map = a.bot_map();
    let nav = map.nav.as_ref().expect("nav grid");
    let pts = patrol_points();
    let mut pf = PathFinder::new();
    let mut out = Vec::new();
    let mut us = Vec::new();
    let mut expanded = 0u32;
    for _ in 0..40 {
        for p in &pts {
            for q2 in &pts {
                if p == q2 {
                    continue;
                }
                let t0 = Instant::now();
                let ok = pf.find(nav, *p, *q2, &mut out);
                us.push(t0.elapsed().as_secs_f64() * 1e6);
                assert!(ok);
                expanded = expanded.max(pf.last_expanded);
            }
        }
    }
    let r = serde_json::json!({
        "queries": us.len(),
        "median_us": q(&mut us, 0.5),
        "p99_us": q(&mut us, 0.99),
        "max_us": us.last().copied().unwrap(),
        "max_nodes_expanded": expanded,
    });
    println!(
        "path_query: patrol-point pairs on the 64x64 grid: median {:.1} us  P99 {:.1} us  max {:.1} us  (worst case {} nodes expanded)",
        q(&mut us, 0.5), q(&mut us, 0.99), us.last().unwrap(), expanded
    );
    r
}

fn main() {
    let Some(col) = terrain() else { return };
    let think_10 = think_bench(&col, 5, 300, 3600);
    let think_60 = think_bench(&col, 30, 300, 1800);
    let think_100 = think_bench(&col, 50, 300, 1800);
    let path = path_bench(&col);
    let report = serde_json::json!({
        "build": if cfg!(debug_assertions) { "debug" } else { "release" },
        "think_10": think_10, "think_60": think_60, "think_100": think_100, "path_query": path,
        "budgets_ms": { "think_10": 1.0, "think_60": 3.0, "path_per_tick": 1.0 },
    });
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reports/baselines");
    if std::fs::create_dir_all(&dir).is_ok() {
        let f = dir.join("bots-m0.json");
        if std::fs::write(&f, serde_json::to_string_pretty(&report).unwrap() + "\n").is_ok() {
            println!("wrote {}", f.display());
        }
    }
}
