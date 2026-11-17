//! Soak: 10 WS clients, 1 Ping/s each, zero unexpected disconnects (M0-016 acceptance 1).
//! CI runs 60 s; the 10 min version is `#[ignore]` (`cargo test --release -- --ignored ws_soak_10min`).
//! `WS_SOAK_SECS` overrides the duration.

mod common;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use common::*;
use frontline_protocol::msg::c0::ServerMsgC0;
use frontline_protocol::Channel;

fn cpu_seconds() -> f64 {
    // utime + stime of this process in clock ticks (USER_HZ = 100 on Linux).
    let s = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    let rest = s.rsplit(')').next().unwrap_or("");
    let f: Vec<&str> = rest.split_whitespace().collect();
    let get = |i: usize| f.get(i).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    (get(11) + get(12)) / 100.0
}

async fn soak(secs: u64) {
    let h = Harness::start_cfg(Duration::from_secs(5), 64).await;
    // Fake sim: advance the published tick at 30 Hz.
    let tick = h.tick;
    let ticker = tokio::spawn(async move {
        let mut iv = tokio::time::interval(Duration::from_micros(33_333));
        loop {
            iv.tick().await;
            tick.fetch_add(1, Ordering::Relaxed);
        }
    });
    let cpu0 = cpu_seconds();
    let t0 = Instant::now();
    let mut tasks = vec![];
    for n in 0..10u32 {
        let url = h.url.clone();
        tasks.push(tokio::spawn(async move {
            let (mut c, _w) = Client::handshake(&url).await;
            let end = Instant::now() + Duration::from_secs(secs);
            let (mut pings, mut last_tick) = (0u32, 0u32);
            let mut iv = tokio::time::interval(Duration::from_secs(1));
            while Instant::now() < end {
                iv.tick().await;
                let token = n * 1_000_000 + pings;
                assert!(
                    c.send(Channel::C0, &ping_msg(token)).await,
                    "client {n}: send failed"
                );
                match c.server_msg(Duration::from_secs(2)).await {
                    Some(ServerMsgC0::Pong(p)) => {
                        assert_eq!(p.client_time, token, "client {n}: wrong echo");
                        assert!(p.server_tick >= last_tick, "server_tick went backwards");
                        last_tick = p.server_tick;
                    }
                    other => panic!(
                        "client {n}: unexpected disconnect/answer after {pings} pings: {other:?}"
                    ),
                }
                pings += 1;
            }
            (pings, last_tick)
        }));
    }
    // Midway through, all 10 connections must still be open (gauge sampled while they are live).
    tokio::time::sleep(Duration::from_secs(secs / 2)).await;
    assert_eq!(
        h.metrics.net_connections.get(),
        10,
        "connections dropped mid-soak"
    );
    let mut total = 0;
    let mut max_tick = 0;
    for t in tasks {
        let (p, tk) = t.await.expect("client task failed (unexpected disconnect)");
        total += p;
        max_tick = max_tick.max(tk);
    }
    let wall = t0.elapsed().as_secs_f64();
    let cpu = cpu_seconds() - cpu0;
    ticker.abort();
    eprintln!(
        "soak {secs}s: 10 clients, {total} pings echoed, 0 unexpected disconnects, server_tick reached {max_tick}; \
         process CPU (server + 10 in-process test clients) {:.2}% of one core = <= {:.3}% per connection; \
         malformed {} rejects ok",
        100.0 * cpu / wall,
        100.0 * cpu / wall / 10.0,
        h.metrics.net_frames_malformed_total.get()
    );
    assert_eq!(h.metrics.net_frames_malformed_total.get(), 0);
    assert!(
        max_tick as u64 >= secs * 25,
        "published tick did not advance"
    );
    h.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_soak_60s() {
    let secs = std::env::var("WS_SOAK_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);
    soak(secs).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "manual 10 min acceptance run"]
async fn ws_soak_10min() {
    soak(600).await;
}
