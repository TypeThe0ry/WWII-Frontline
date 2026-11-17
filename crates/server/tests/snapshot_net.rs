//! M0-019 acceptance 1 (second half): the real sim thread sending real snapshots over real
//! WebSocket and WebTransport loopback connections to decoding clients that acknowledge them.
//!
//! The synthetic clients do not predict, so their `pred_hash` never matches and the server sends
//! `pos_corr` in every packet (+6 B): the measured rates are slightly pessimistic.
//! `FRONTLINE_NET_SECS` sets the duration per transport (default 15 s).

mod common;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use common::sim_harness::*;
use common::*;
use frontline_protocol::snapshot::SnapshotDecoder;
use frontline_protocol::Channel;
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::{InboundQueue, Sessions};
use frontline_server::shutdown::StopFlag;
use frontline_server::sim_bridge::NetLink;
use frontline_server::sim_thread::{self, SimThreadConfig};
use frontline_sim::{GameMode, MapId};

static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Default, Debug)]
struct ClientStats {
    snapshots: u64,
    bytes: u64,
    decode_errors: u64,
    crc_flag: bool,
    first_has_corr: bool,
    entity_records: u64,
    keyframes: u64,
    last_tick: u32,
    tick_gaps: Vec<u32>,
}

async fn one_client(target: Target, secs: f64) -> ClientStats {
    let (mut c, w) = Conn::handshake(&target).await;
    let base = w.server_tick;
    let t0 = Instant::now();
    let mut dec = SnapshotDecoder::new();
    let mut st = ClientStats::default();
    let (mut ack, mut hash) = (0u16, 0u8);
    let mut next = Instant::now();
    let mut last_sent = 0u32;
    while t0.elapsed().as_secs_f64() < secs {
        let now = Instant::now();
        if now >= next {
            next += Duration::from_micros(33_333);
            let k = base + 1 + (t0.elapsed().as_secs_f64() * 30.0) as u32 + 3;
            if k > last_sent {
                last_sent = k;
                let mut b = bundle_upto(w.player_id, k);
                b.ack_snapshot_tick = ack;
                b.pred_hash = hash;
                if !c.send(Channel::C1, &encode(&b)).await {
                    break;
                }
            }
            continue;
        }
        match c.recv_within(next - now).await {
            Got::Frame(Channel::C2, p) => {
                st.bytes += p.len() as u64;
                match dec.decode(&p) {
                    Ok(s) => {
                        if st.snapshots == 0 {
                            st.first_has_corr = s.self_state.is_some_and(|x| x.pos_corr.is_some());
                        }
                        st.snapshots += 1;
                        st.crc_flag = p[0] & 0x02 != 0;
                        st.entity_records += s.entities.len() as u64;
                        st.keyframes += s.entities.keyframe.iter().filter(|&&k| k).count() as u64;
                        if st.last_tick != 0 {
                            st.tick_gaps.push(s.tick.wrapping_sub(st.last_tick));
                        }
                        st.last_tick = s.tick;
                        ack = s.tick as u16;
                        if let Some(corr) = s.self_state.and_then(|x| x.pos_corr) {
                            hash = frontline_protocol::snapshot::pred_hash::pred_hash_q(corr);
                        }
                    }
                    Err(_) => st.decode_errors += 1,
                }
            }
            Got::Closed => break,
            _ => {}
        }
    }
    st
}

async fn run_kind(kind: Kind, n: usize, secs: f64) -> Option<()> {
    let map = terrain()?;
    let _serial = ONE_AT_A_TIME.lock().await;
    frontline_server::logging::CURRENT_TICK.store(0, Ordering::Relaxed);
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
        kind == Kind::Wt,
    )
    .await;
    let target = srv.target(kind).clone();
    let mut tasks = Vec::new();
    for _ in 0..n {
        tasks.push(tokio::spawn(one_client(target.clone(), secs)));
    }
    let mut all = Vec::new();
    for t in tasks {
        all.push(t.await.expect("client task"));
    }
    stop.stop();
    let report = tokio::task::block_in_place(|| handle.join()).expect("sim report");
    srv.stop().await;

    let label = if kind == Kind::Wt { "WT" } else { "WS" };
    let mut kbps_payload: Vec<f64> = all
        .iter()
        .map(|s| s.bytes as f64 * 8.0 / 1000.0 / secs)
        .collect();
    let mut kbps_wire: Vec<f64> = all
        .iter()
        .map(|s| (s.bytes as f64 + 58.0 * s.snapshots as f64) * 8.0 / 1000.0 / secs)
        .collect();
    kbps_payload.sort_by(|a, b| a.partial_cmp(b).unwrap());
    kbps_wire.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let med = |v: &Vec<f64>| v[v.len() / 2];
    let hz: f64 = all.iter().map(|s| s.snapshots as f64 / secs).sum::<f64>() / n as f64;
    let kf: u64 = all.iter().map(|s| s.keyframes).sum();
    let rec: u64 = all.iter().map(|s| s.entity_records).sum();
    eprintln!(
        "[{label}] {n} clients x {secs:.0} s: {hz:.1} snapshots/s per client, payload kbps median {:.1} (max {:.1}), wire-estimate kbps median {:.1} (max {:.1}), keyframe records {:.1} %, decode errors {}, tick_ms P99 {:.2} ms, datagrams oversize {}, dropped {}",
        med(&kbps_payload),
        kbps_payload.last().unwrap(),
        med(&kbps_wire),
        kbps_wire.last().unwrap(),
        100.0 * kf as f64 / rec.max(1) as f64,
        all.iter().map(|s| s.decode_errors).sum::<u64>(),
        report.tick_ms.p99,
        metrics.net_datagram_oversize_total.get(),
        metrics.net_snapshots_dropped_total.get(),
    );
    for s in &all {
        assert_eq!(s.decode_errors, 0);
        assert!(s.first_has_corr, "the first snapshot must carry pos_corr");
        assert_eq!(
            s.crc_flag,
            kind == Kind::Wt,
            "CRC trailer only on datagrams"
        );
        // loose bounds: wall-clock rates on a loaded shared machine (nominal 20 / 15 Hz)
        assert!(s.snapshots as f64 / secs > if kind == Kind::Wt { 12.0 } else { 9.0 });
    }
    assert!(med(&kbps_wire) <= 128.0);
    assert_eq!(metrics.net_datagram_oversize_total.get(), 0);
    Some(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ten_clients_over_websocket_loopback() {
    let secs = std::env::var("FRONTLINE_NET_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(15.0);
    run_kind(Kind::Ws, 10, secs).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ten_clients_over_webtransport_loopback() {
    let secs = std::env::var("FRONTLINE_NET_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(15.0);
    run_kind(Kind::Wt, 10, secs).await;
}
