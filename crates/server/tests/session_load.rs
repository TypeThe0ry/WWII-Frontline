//! M0-018 performance: input ingestion budget (ARCHITECTURE §6.10 stage 1, <= 0.8 ms/tick at 100
//! players) and N connected players sending input at 30 Hz against the real 30 Hz sim thread
//! (acceptance 5: `server_tick_ms` P99 <= 28 ms, `input_queue_full_total == 0`).
//!
//! * `ingest_budget_100_players` runs everywhere; its 0.8 ms bound is only asserted in release.
//! * `load_smoke` (20 clients, 5 s) runs everywhere.
//! * `load_20_clients_60s` / `load_100_clients_60s` are the acceptance runs:
//!   `cargo test --release -p frontline-server --test session_load -- --ignored --nocapture`
//!   (env `FRONTLINE_LOAD_SECS` shortens them). Need the generated terrain.

mod common;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::sim_harness::*;
use common::*;
use frontline_protocol::Channel;
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::{InboundQueue, Sessions};
use frontline_server::shutdown::StopFlag;
use frontline_server::sim_bridge::{NetLink, SimBridge};
use frontline_server::sim_thread::{self, SimThreadConfig};
use frontline_server::validate::{Reject, ValidateCfg};
use frontline_sim::{CollisionData, GameData, GameMode, MapId, SimConfig, Team, World};

fn percentile(sorted: &[f64], q: f64) -> f64 {
    sorted[(((sorted.len() - 1) as f64) * q).ceil() as usize]
}

#[test]
fn ingest_budget_100_players() {
    let m = Metrics::new();
    let mut world = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 1,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        &CollisionData::empty(),
    );
    let mut br = SimBridge::new(m.clone(), None, None, ValidateCfg::default());
    for p in 0..100u8 {
        br.join(&mut world, p, if p % 2 == 0 { Team::Us } else { Team::Ger })
            .unwrap();
    }
    // Pre-built bundles: every client sends ticks k-2..=k with k = tick + 3.
    const TICKS: u32 = 3000;
    let bundles: Vec<Vec<_>> = (1..=TICKS)
        .map(|t| (0..100u8).map(|p| bundle_upto(p, t + 3)).collect())
        .collect();
    // Ticks 1..=3 arrive before the first tick, as they do for a client that runs ahead.
    for k in 1..=3 {
        for p in 0..100u8 {
            br.ingest_bundle(p, &bundle_upto(p, k), 1);
        }
    }
    let mut us: Vec<f64> = Vec::with_capacity(TICKS as usize);
    for t in 1..=TICKS {
        let t0 = Instant::now();
        for (p, b) in bundles[t as usize - 1].iter().enumerate() {
            br.ingest_bundle(p as u8, b, t);
        }
        let n = br.resolve(t).len();
        us.push(t0.elapsed().as_secs_f64() * 1e3);
        assert_eq!(n, 100);
    }
    us.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let (p50, p99, max) = (
        percentile(&us, 0.5),
        percentile(&us, 0.99),
        *us.last().unwrap(),
    );
    eprintln!(
        "input ingest + validation + resolve, 100 players x 3 cmds ({}): P50 {p50:.4} ms, P99 {p99:.4} ms, max {max:.4} ms (budget 0.8 ms)",
        if cfg!(debug_assertions) { "debug build" } else { "release build" }
    );
    assert_eq!(m.input_missing_ticks_total.get(), 0);
    assert_eq!(
        m.input_rejected_total
            .with_label_values(&["move_clamped"])
            .get(),
        0
    );
    if !cfg!(debug_assertions) {
        assert!(p99 <= 0.8, "ingest P99 {p99:.4} ms > 0.8 ms");
    }
}

struct LoadResult {
    p50: f64,
    p99: f64,
    max: f64,
    ticks: u64,
    queue_full: u64,
    overruns: u64,
    sent: u64,
}

async fn run_load(n: usize, secs: f64) -> Option<LoadResult> {
    let map = terrain()?;
    let metrics = Metrics::new();
    let sessions = Sessions::with_limits(metrics.clone(), 256, 128);
    let inbound = InboundQueue::new();
    let stop = StopFlag::new();
    let handle = sim_thread::spawn(
        SimThreadConfig {
            map_data: Some(map),
            net: Some(NetLink {
                inbound: inbound.rx.clone(),
                sessions: sessions.clone(),
            }),
            ..SimThreadConfig::bare(5, MapId(0), GameMode::Sandbox)
        },
        metrics.clone(),
        stop.clone(),
    )
    .expect("sim thread");
    let srv = start_listeners(
        metrics.clone(),
        sessions,
        &inbound,
        handle.commands_tx.clone(),
        false,
    )
    .await;

    let sent = Arc::new(AtomicU64::new(0));
    let quit = Arc::new(AtomicBool::new(false));
    let mut tasks = Vec::new();
    for _ in 0..n {
        let target = srv.ws_target.clone();
        let (sent, quit) = (sent.clone(), quit.clone());
        tasks.push(tokio::spawn(async move {
            let (mut c, w) = Conn::handshake(&target).await;
            let base = w.server_tick;
            let t0 = Instant::now();
            let mut iv = tokio::time::interval(Duration::from_micros(33_333));
            iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut last = 0u32;
            while !quit.load(Ordering::Relaxed) {
                iv.tick().await;
                // Newest command 3 ticks ahead of the server's clock; never repeat a tick.
                let k = base + 1 + (t0.elapsed().as_secs_f64() * 30.0) as u32 + 3;
                if k <= last {
                    continue;
                }
                last = k;
                if !c
                    .send(Channel::C1, &encode(&bundle_upto(w.player_id, k)))
                    .await
                {
                    break;
                }
                sent.fetch_add(1, Ordering::Relaxed);
            }
            drop(c);
        }));
    }
    // Warm-up (joins, first allocations), then measure only the steady state.
    tokio::time::sleep(Duration::from_secs_f64(secs)).await;
    quit.store(true, Ordering::Relaxed);
    for t in tasks {
        let _ = t.await;
    }
    stop.stop();
    let report = tokio::task::block_in_place(|| handle.join()).expect("sim thread report");
    let sent = sent.load(Ordering::Relaxed);
    let accepted = metrics.input_accepted_total.get();
    let rej = |r: &str| metrics.input_rejected_total.with_label_values(&[r]).get();
    eprintln!(
        "{n} clients x {secs:.0} s: {} ticks, tick_ms P50 {:.3} / P99 {:.3} / max {:.3}, interval error P99 {:.3} ms, overruns {}, queue_full {}",
        report.ticks, report.tick_ms.p50, report.tick_ms.p99, report.tick_ms.max,
        report.tick_interval_error_ms.p99, report.tick_overrun_total, report.input_queue_full_total
    );
    eprintln!(
        "  bundles sent {sent}, c1 delivered {}, cmds accepted {accepted}, duplicates {}, too_old {}, too_far {}, other rejects {}, missing player-ticks {}, stalls {}, kicks {}",
        metrics.net_c1_inbound_total.get(),
        rej("duplicate"),
        rej("too_old"),
        rej("too_far"),
        Reject::LABELS
            .iter()
            .filter(|l| !["duplicate", "too_old", "too_far"].contains(l))
            .map(|l| rej(l))
            .sum::<u64>(),
        metrics.input_missing_ticks_total.get(),
        metrics.session_stall_total.get(),
        metrics.session_kicked_total.get(),
    );
    srv.stop().await;
    Some(LoadResult {
        p50: report.tick_ms.p50,
        p99: report.tick_ms.p99,
        max: report.tick_ms.max,
        ticks: report.ticks,
        queue_full: report.input_queue_full_total,
        overruns: report.tick_overrun_total,
        sent,
    })
}

fn check(r: &LoadResult) {
    assert!(r.sent > 0);
    assert!(
        r.p99 <= 28.0,
        "tick_ms P99 {:.3} ms > 28 ms (max {:.3})",
        r.p99,
        r.max
    );
    assert_eq!(r.queue_full, 0, "input_queue_full_total");
    let _ = (r.p50, r.ticks, r.overruns);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn load_smoke() {
    if let Some(r) = run_load(20, 5.0).await {
        check(&r);
    }
}

fn secs() -> f64 {
    std::env::var("FRONTLINE_LOAD_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60.0)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "M0-018 acceptance 5: 20 clients x 60 s; run with --release --ignored"]
async fn load_20_clients_60s() {
    if let Some(r) = run_load(20, secs()).await {
        check(&r);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "100 clients x 60 s (extrapolation check); run with --release --ignored"]
async fn load_100_clients_60s() {
    if let Some(r) = run_load(100, secs()).await {
        check(&r);
    }
}
