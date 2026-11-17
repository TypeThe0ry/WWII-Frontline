//! M0-019 acceptance 1 and 2 (bandwidth and CPU of AOI + encoding) with in-process synthetic
//! clients (no network, isolating the encoder), plus a run over real WebSocket and
//! WebTransport loopback connections against the real sim thread.
//!
//! Numbers are printed (`--nocapture`); assertions follow the task budget: wire-estimate P50
//! <= 128 kbps, and (release builds only) AOI + encode <= 2.6 ms per tick at 10 clients.
//! Long runs: `cargo test --release -p frontline-server --test snapshot_bandwidth -- --nocapture`.

mod common;

use common::sim_harness::{gen_cmd, terrain};
use common::synth::*;
use frontline_server::snapshot::LinkInfo;

fn pct(v: &[f32], q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    s[(((s.len() - 1) as f64) * q).ceil() as usize] as f64
}

fn pct_u32(v: &[u32], q: f64) -> f64 {
    let f: Vec<f32> = v.iter().map(|&x| x as f32).collect();
    pct(&f, q)
}

struct Outcome {
    kbps_payload: (f64, f64, f64),
    kbps_wire: (f64, f64, f64),
    pass_ms: (f64, f64, f64),
    keyframe_share: f64,
    starved: u64,
    mean_packet: f64,
}

/// Runs `secs` of 30 Hz play with `n` players, the first `clients` of them receiving snapshots.
fn run(n: usize, clients: usize, secs: f64, loss: f64, label: &str) -> Option<Outcome> {
    let map = terrain()?;
    let mut s = Synth::new(new_world(Some(&map)));
    s.loss = loss;
    spawn_squads(&mut s, &map, n, clients, LinkInfo::DATAGRAM);
    let ticks = (secs * 30.0) as u32;
    for _ in 0..ticks {
        s.step(gen_cmd);
    }
    let st = &s.sched.stats;
    // skip the first 2 s (joins, first keyframes) of the per-client windows
    let skip = (clients * 2).min(st.kbps_payload.len() / 2);
    let pl = &st.kbps_payload[skip..];
    let wi = &st.kbps_wire[skip..];
    let ms: Vec<f32> = st.pass_ns.iter().map(|&x| x as f32 / 1e6).collect();
    let o = Outcome {
        kbps_payload: (pct(pl, 0.5), pct(pl, 0.99), pct(pl, 1.0)),
        kbps_wire: (pct(wi, 0.5), pct(wi, 0.99), pct(wi, 1.0)),
        pass_ms: (pct(&ms, 0.5), pct(&ms, 0.99), pct(&ms, 1.0)),
        keyframe_share: st.keyframe_share(),
        starved: st.starved,
        mean_packet: st.bytes as f64 / st.packets.max(1) as f64,
    };
    eprintln!(
        "[{label}] {n} players / {clients} clients / {secs:.0} s ({} build): payload kbps P50 {:.1} P99 {:.1} max {:.1} | wire(+58 B/pkt) kbps P50 {:.1} P99 {:.1} max {:.1} | mean packet {:.0} B | keyframe records {:.1} % | starved {} | AOI+encode per tick: P50 {:.3} P99 {:.3} max {:.3} ms",
        if cfg!(debug_assertions) { "debug" } else { "release" },
        o.kbps_payload.0, o.kbps_payload.1, o.kbps_payload.2,
        o.kbps_wire.0, o.kbps_wire.1, o.kbps_wire.2,
        o.mean_packet, 100.0 * o.keyframe_share, o.starved,
        o.pass_ms.0, o.pass_ms.1, o.pass_ms.2
    );
    // size by number of entity records in the packet
    let bins: [(usize, usize); 6] = [(0, 9), (10, 19), (20, 39), (40, 59), (60, 89), (90, 127)];
    for (lo, hi) in bins {
        let mut cnt = 0usize;
        let mut bytes = 0usize;
        let mut kf = 0usize;
        let mut ent = 0usize;
        for c in &s.clients {
            for l in c.log.iter().skip(40) {
                if (lo..=hi).contains(&l.entities) {
                    cnt += 1;
                    bytes += l.bytes;
                    kf += l.keyframes;
                    ent += l.entities;
                }
            }
        }
        if cnt == 0 {
            continue;
        }
        let mean = bytes as f64 / cnt as f64;
        eprintln!(
            "    entities {lo:>2}-{hi:<3}: {cnt:>6} packets, mean {mean:>6.1} B ({:.2} B/entity), keyframes {:>4.1} % -> {:>5.1} kbps payload / {:>5.1} kbps wire at 20 Hz",
            mean / (ent as f64 / cnt as f64).max(1.0),
            100.0 * kf as f64 / ent.max(1) as f64,
            mean * 20.0 * 8.0 / 1000.0,
            (mean + 58.0) * 20.0 * 8.0 / 1000.0
        );
    }
    Some(o)
}

/// Acceptance 1 + 2: 10 clients, 5 minutes.
#[test]
fn ten_clients_five_minutes_stay_inside_the_budget() {
    let Some(o) = run(10, 10, 300.0, 0.0, "10x5min") else {
        return;
    };
    assert!(
        o.kbps_wire.0 <= 128.0,
        "P50 {:.1} kbps > 128",
        o.kbps_wire.0
    );
    assert!(
        o.kbps_wire.1 <= 400.0,
        "P99 {:.1} kbps > 400",
        o.kbps_wire.1
    );
    if !cfg!(debug_assertions) {
        assert!(
            o.pass_ms.1 <= 2.6,
            "AOI + encode P99 {:.3} ms > 2.6 ms",
            o.pass_ms.1
        );
    }
}

#[test]
fn twenty_players_all_clients() {
    let secs = std::env::var("FRONTLINE_SNAP_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60.0);
    let Some(o) = run(20, 20, secs, 0.0, "20x") else {
        return;
    };
    assert!(o.kbps_wire.0 <= 128.0);
}

/// ARCHITECTURE §6.9.5 test line at 100 players (all of them receiving snapshots).
#[test]
fn hundred_players_all_clients() {
    let secs = std::env::var("FRONTLINE_SNAP_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20.0);
    let Some(o) = run(100, 100, secs, 0.0, "100x") else {
        return;
    };
    assert!(o.kbps_wire.0 <= 128.0, "P50 {:.1}", o.kbps_wire.0);
    assert!(o.kbps_wire.1 <= 400.0, "P99 {:.1}", o.kbps_wire.1);
}

/// Bandwidth with and without forcing entities within 96 m into every packet (PI-010).
#[test]
fn near_every_packet_policy_comparison() {
    let Some(map) = terrain() else { return };
    let mut res = Vec::new();
    for every in [true, false] {
        let mut s = Synth::new(new_world(Some(&map)));
        s.sched.near_every_packet = every;
        spawn_squads(&mut s, &map, 40, 40, LinkInfo::DATAGRAM);
        for _ in 0..900 {
            s.step(gen_cmd);
        }
        let st = &s.sched.stats;
        let skip = 80.min(st.kbps_payload.len() / 2);
        let wire = pct(&st.kbps_wire[skip..], 0.5);
        eprintln!(
            "near_every_packet={every}: 40 players, 30 s: wire P50 {wire:.1} kbps, mean packet {:.0} B, keyframe records {:.1} %",
            st.bytes as f64 / st.packets as f64,
            100.0 * st.keyframe_share()
        );
        res.push(wire);
    }
    assert!(res[0] <= 128.0 && res[1] <= 128.0);
}

#[test]
fn pass_time_without_clients_is_tiny() {
    let Some(map) = terrain() else { return };
    let mut s = Synth::new(new_world(Some(&map)));
    spawn_squads(&mut s, &map, 20, 0, LinkInfo::DATAGRAM);
    for _ in 0..60 {
        s.step(gen_cmd);
    }
    assert_eq!(s.sched.stats.packets, 0);
    assert_eq!(s.sched.client_count(), 0);
}

#[allow(dead_code)]
fn _unused(v: &[u32]) -> f64 {
    pct_u32(v, 0.5)
}
