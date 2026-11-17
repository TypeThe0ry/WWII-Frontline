//! WebTransport against the real `frontline-server` binary (M0-017).
//!
//! * `binary_serves_wt_with_dev_cert`: normal test; `--dev-cert --dev-echo` end to end.
//! * `stress_100_clients` (`#[ignore]`, acceptance 2): 100 `wtransport` clients, each 20 C1
//!   datagrams/s up and the 20/s C2 echo down, for `WT_STRESS_SECS` (default 120) seconds. The
//!   server runs as a child process so its CPU and tick metrics are its own:
//!   `cargo test --release -p frontline-server --test wt_stress -- --ignored --nocapture`
//!   Clients bind distinct 127.0.0.x addresses so the per-IP limit (4) stays in force.

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use common::*;
use frontline_protocol::msg::c0::{
    decode_c0_server, encode_c0_client, ClientMsgC0, Disconnect, ServerMsgC0,
};
use frontline_protocol::msg::input::{encode_input_bundle, InputBundle, InputCmd};
use frontline_protocol::Channel;
use wtransport::tls::{Sha256Digest, Sha256DigestFmt};

struct Server {
    child: Child,
    wt_port: u16,
    metrics_port: u16,
    hash: Sha256Digest,
}

impl Server {
    fn start(report: Option<&std::path::Path>) -> Server {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_frontline-server"));
        cmd.args([
            "--metrics-port",
            "0",
            "--ws-port",
            "0",
            "--wt-port",
            "0",
            "--dev-cert",
            "--dev-echo",
            "--empty-world",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
        if let Some(r) = report {
            cmd.arg("--report").arg(r);
        }
        let mut child = cmd.spawn().expect("spawn server");
        let out = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for l in BufReader::new(out).lines().map_while(Result::ok) {
                if tx.send(l).is_err() {
                    break;
                }
            }
        });
        let (mut wt, mut met, mut hash) = (None, None, None);
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline && (wt.is_none() || met.is_none()) {
            let Ok(l) = rx.recv_timeout(Duration::from_millis(100)) else {
                continue;
            };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&l) else {
                continue;
            };
            match v["event"].as_str() {
                Some("wt_listening") => {
                    wt = v["port"].as_u64().map(|p| p as u16);
                    hash = v["cert_sha256_hex"].as_str().map(str::to_owned);
                }
                Some("metrics_listening") => met = v["port"].as_u64().map(|p| p as u16),
                _ => {}
            }
        }
        // Keep draining so the child never blocks on a full pipe.
        std::thread::spawn(move || while rx.recv().is_ok() {});
        Server {
            child,
            wt_port: wt.expect("server never reported wt_listening"),
            metrics_port: met.expect("server never reported metrics_listening"),
            hash: Sha256Digest::from_str_fmt(
                // "aa:bb:..." form expected by from_str_fmt
                hash.expect("hash")
                    .as_bytes()
                    .chunks(2)
                    .map(|c| std::str::from_utf8(c).unwrap())
                    .collect::<Vec<_>>()
                    .join(":"),
                Sha256DigestFmt::DottedHex,
            )
            .expect("hash parse"),
        }
    }

    fn url(&self) -> String {
        format!("https://127.0.0.1:{}/wt", self.wt_port)
    }

    fn scrape(&self) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", self.metrics_port)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        s.write_all(b"GET /metrics HTTP/1.1\r\nHost: x\r\n\r\n")
            .unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        out
    }

    fn cpu_seconds(&self) -> f64 {
        // /proc/<pid>/stat fields 14 (utime) and 15 (stime), in clock ticks (CLK_TCK = 100).
        let stat =
            std::fs::read_to_string(format!("/proc/{}/stat", self.child.id())).unwrap_or_default();
        let after = stat.rsplit_once(')').map(|x| x.1).unwrap_or("");
        let f: Vec<&str> = after.split_whitespace().collect();
        let g = |i: usize| f.get(i).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0);
        (g(11) + g(12)) / 100.0
    }

    /// SIGTERM and wait for the exit code.
    fn terminate(&mut self) -> Option<i32> {
        let _ = Command::new("kill")
            .args(["-TERM", &self.child.id().to_string()])
            .status();
        self.child.wait().ok().and_then(|s| s.code())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn metric(text: &str, name: &str) -> f64 {
    text.lines()
        .find(|l| {
            l.starts_with(name) && l[name.len()..].starts_with([' ', '{']) && !l.starts_with('#')
        })
        .and_then(|l| l.rsplit(' ').next())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0)
}

/// Cumulative buckets `(le, count)` of `server_tick_ms`.
fn tick_buckets(text: &str) -> Vec<(f64, f64)> {
    text.lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("server_tick_ms_bucket{le=\"")?;
            let (le, tail) = rest.split_once("\"}")?;
            let le = if le == "+Inf" {
                f64::INFINITY
            } else {
                le.parse().ok()?
            };
            Some((le, tail.trim().parse().ok()?))
        })
        .collect()
}

/// Hello for the real binary: protocol version and the embedded game-data hash.
fn real_hello() -> Vec<u8> {
    hello_msg(
        frontline_protocol::PROTOCOL_VERSION,
        frontline_sim::GameData::embedded().hash,
        vec![],
    )
}

fn bundle(seq: u16) -> Vec<u8> {
    encode_input_bundle(&InputBundle {
        ack_snapshot_tick: seq,
        pred_hash: 0,
        cmds: [InputCmd {
            seq,
            tick: seq as u32 + 1,
            move_x: 0,
            move_z: 0,
            yaw: 0,
            pitch: 0,
            buttons: 0,
            weapon_slot: 0,
            aux: 0,
            view_tick: 0,
        }]
        .into_iter()
        .collect(),
        fire: Default::default(),
    })
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn binary_serves_wt_with_dev_cert() {
    let mut srv = Server::start(None);
    let mut c = WtClient::connect(&srv.url(), srv.hash.clone())
        .await
        .expect("connect");
    assert!(c.send_c0(&real_hello()).await);
    match c.recv_within(Duration::from_secs(5)).await {
        Got::Frame(Channel::C0, p) => {
            assert!(matches!(decode_c0_server(&p), Ok(ServerMsgC0::Welcome(_))))
        }
        _ => panic!("no Welcome"),
    }
    assert!(c.send_datagram(Channel::C1, &bundle(7)));
    let mut echoed = false;
    for _ in 0..20 {
        if let Got::Frame(Channel::C2, p) = c.recv_within(Duration::from_millis(250)).await {
            assert_eq!(u16::from_le_bytes([p[0], p[1]]), 7);
            echoed = true;
            break;
        }
    }
    assert!(echoed, "no C2 echo from the real binary");
    let m = srv.scrape();
    assert_eq!(
        metric(&m, "net_transport_connections{transport=\"wt\"}") as i64,
        1
    );
    drop(c);
    assert_eq!(srv.terminate(), Some(0), "SIGTERM exit code");
}

struct ClientResult {
    sent: u32,
    received: u32,
    handshake_ms: f64,
    rtts_ms: Vec<f64>,
    closed_early: bool,
}

async fn run_client(
    i: usize,
    url: String,
    hash: Sha256Digest,
    secs: f64,
    go: std::sync::Arc<tokio::sync::Barrier>,
) -> ClientResult {
    // 25 source addresses x 4 connections each.
    let ip = [127, 0, 1 + (i / 4) as u8 / 100, 2 + (i / 4) as u8 % 100];
    let t0 = Instant::now();
    let mut c = WtClient::connect_from(ip, &url, hash)
        .await
        .expect("connect");
    assert!(c.send_c0(&real_hello()).await);
    loop {
        match c.recv_within(Duration::from_secs(10)).await {
            Got::Frame(Channel::C0, p) => match decode_c0_server(&p) {
                Ok(ServerMsgC0::Welcome(_)) => break,
                o => panic!("client {i}: {o:?}"),
            },
            Got::Frame(..) => {}
            _ => panic!("client {i}: no Welcome"),
        }
    }
    let handshake_ms = t0.elapsed().as_secs_f64() * 1e3;
    go.wait().await;
    let start = Instant::now();
    let mut sent_at: Vec<Instant> = Vec::new();
    let (mut received, mut closed_early) = (0u32, false);
    let mut rtts = Vec::new();
    let mut tick = tokio::time::interval(Duration::from_millis(50));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // Sends stop at `secs`; receiving continues for a short drain.
    loop {
        let sending = start.elapsed().as_secs_f64() < secs;
        if !sending && start.elapsed().as_secs_f64() > secs + 1.0 {
            break;
        }
        tokio::select! {
            _ = tick.tick(), if sending => {
                let seq = sent_at.len() as u16;
                if c.send_datagram(Channel::C1, &bundle(seq)) {
                    sent_at.push(Instant::now());
                }
            }
            g = c.recv_within(Duration::from_millis(100)) => match g {
                Got::Frame(Channel::C2, p) if p.len() == 4 => {
                    received += 1;
                    let seq = u16::from_le_bytes([p[0], p[1]]) as usize;
                    if let Some(t) = sent_at.get(seq) {
                        rtts.push(t.elapsed().as_secs_f64() * 1e3);
                    }
                }
                Got::Closed => {
                    closed_early = start.elapsed().as_secs_f64() < secs;
                    break;
                }
                _ => {}
            }
        }
    }
    let _ = c
        .send_c0(&encode_c0_client(&ClientMsgC0::Disconnect(Disconnect { reason: 0 })).unwrap())
        .await;
    ClientResult {
        sent: sent_at.len() as u32,
        received,
        handshake_ms,
        rtts_ms: rtts,
        closed_early,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "M0-017 acceptance 2: 100 clients x 120 s; run manually / nightly"]
async fn stress_100_clients() {
    let secs: f64 = std::env::var("WT_STRESS_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120.0);
    let n: usize = std::env::var("WT_STRESS_CLIENTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let report = std::env::temp_dir().join(format!("wt-stress-report-{}.json", std::process::id()));
    let mut srv = Server::start(Some(&report));
    let go = std::sync::Arc::new(tokio::sync::Barrier::new(n + 1));
    let tasks: Vec<_> = (0..n)
        .map(|i| tokio::spawn(run_client(i, srv.url(), srv.hash.clone(), secs, go.clone())))
        .collect();
    go.wait().await; // all handshakes done
    let before = srv.scrape();
    let cpu0 = srv.cpu_seconds();
    let t0 = Instant::now();
    let mut results = vec![];
    for t in tasks {
        results.push(t.await.expect("client task"));
    }
    let wall = t0.elapsed().as_secs_f64();
    let cpu1 = srv.cpu_seconds();
    let after = srv.scrape();

    let sent: u64 = results.iter().map(|r| r.sent as u64).sum();
    let recv: u64 = results.iter().map(|r| r.received as u64).sum();
    let c1_in = metric(&after, "net_c1_inbound_total") - metric(&before, "net_c1_inbound_total");
    let dg_out =
        metric(&after, "net_datagrams_out_total") - metric(&before, "net_datagrams_out_total");
    let up = c1_in / sent as f64;
    let down = recv as f64 / dg_out;
    let early = results.iter().filter(|r| r.closed_early).count();
    let mut rtts: Vec<f64> = results
        .iter()
        .flat_map(|r| r.rtts_ms.iter().copied())
        .collect();
    rtts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: f64| {
        rtts.get(((rtts.len() as f64 * p) as usize).min(rtts.len().saturating_sub(1)))
            .copied()
            .unwrap_or(f64::NAN)
    };
    let mut hs: Vec<f64> = results.iter().map(|r| r.handshake_ms).collect();
    hs.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // P99 bucket of server_tick_ms over the load window (delta of cumulative buckets).
    let (b0, b1) = (tick_buckets(&before), tick_buckets(&after));
    let total = b1.last().map(|x| x.1).unwrap_or(0.0) - b0.last().map(|x| x.1).unwrap_or(0.0);
    let p99_bucket = b1
        .iter()
        .zip(&b0)
        .find(|((_, c1), (_, c0))| c1 - c0 >= 0.99 * total)
        .map(|((le, _), _)| *le)
        .unwrap_or(f64::NAN);

    eprintln!("== wt_stress: {n} clients x {secs:.0} s (window {wall:.1} s) ==");
    eprintln!(
        "uplink  : client sent {sent}, server accepted {c1_in} -> {:.4} %",
        up * 100.0
    );
    eprintln!(
        "downlink: server sent {dg_out}, clients received {recv} -> {:.4} %",
        down * 100.0
    );
    eprintln!(
        "end-to-end echo: {recv}/{sent} = {:.4} %",
        recv as f64 / sent as f64 * 100.0
    );
    eprintln!(
        "unexpected session closes: {early}; oversize drops: {}; snapshots dropped: {}",
        metric(&after, "net_datagram_oversize_total"),
        metric(&after, "net_snapshots_dropped_total")
    );
    eprintln!(
        "echo RTT ms: P50 {:.2} P99 {:.2} max {:.2}; handshake ms: P50 {:.1} max {:.1}",
        q(0.5),
        q(0.99),
        q(1.0),
        hs[hs.len() / 2],
        hs[hs.len() - 1]
    );
    eprintln!(
        "server CPU: {:.2} s over {wall:.1} s = {:.1} % of one core",
        cpu1 - cpu0,
        (cpu1 - cpu0) / wall * 100.0
    );
    eprintln!("server_tick_ms P99 bucket upper bound over the window: {p99_bucket} ms");
    eprintln!(
        "input_queue_full: {}",
        metric(&after, "server_input_queue_full_total")
    );

    let code = srv.terminate();
    if let Ok(r) = std::fs::read_to_string(&report) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&r) {
            eprintln!(
                "server report (whole process): tick_ms {} interval_error_ms {} overruns {}",
                v["tick_ms"], v["tick_interval_error_ms"], v["tick_overrun_total"]
            );
        }
        let _ = std::fs::remove_file(&report);
    }
    assert_eq!(code, Some(0));
    assert_eq!(early, 0, "unexpected session closes");
    assert!(up >= 0.999, "uplink arrival {up}");
    assert!(down >= 0.999, "downlink arrival {down}");
}
