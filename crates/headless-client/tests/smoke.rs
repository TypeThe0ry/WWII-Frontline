//! End-to-end smoke of the headless client against a real `frontline-server` process (the server
//! is a separate binary: this crate never links it, ARCHITECTURE §4.5).
//!
//! * `smoke_ws` / `smoke_wt`: 3 clients for 8 s with 2 bots. Skipped (with a message) when the
//!   server binary is not built; `FRONTLINE_REQUIRE_SERVER=1` turns the skip into a failure.
//!   Build it first: `cargo build -p frontline-server`. `FRONTLINE_SERVER_BIN` overrides the path.
//! * `loadtest_smoke` (ignored): the 60 s / 3 client version of both transports ("loadtest-smoke").
//!   `cargo test -p frontline-headless-client --test smoke -- --ignored --nocapture`
//!
//! The terrain binaries (`data/maps/m0`) are needed by the server; without them the test skips too.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use clap::Parser;
use frontline_headless::cli::Args;
use frontline_headless::report::Report;
use frontline_headless::run::{build_ctx, run};

fn require() -> bool {
    std::env::var("FRONTLINE_REQUIRE_SERVER").is_ok_and(|v| v == "1")
}

fn server_bin() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("FRONTLINE_SERVER_BIN") {
        return Some(PathBuf::from(p));
    }
    // .../target/<profile>/deps/smoke-HASH -> .../target/<profile>/frontline-server
    let exe = std::env::current_exe().ok()?;
    let p = exe.parent()?.parent()?.join("frontline-server");
    p.is_file().then_some(p)
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Server {
    child: Child,
    log: PathBuf,
    ws: u16,
    wt: u16,
    cert_hex: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.log);
    }
}

fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let k = format!("\"{key}\":");
    let i = line.find(&k)? + k.len();
    let rest = &line[i..];
    let end = rest.find([',', '}']).unwrap_or(rest.len());
    Some(rest[..end].trim_matches('"'))
}

/// Starts the server, or `None` (after saying why) when the test has to be skipped.
fn start_server(bots: u32) -> Option<Server> {
    let Some(bin) = server_bin() else {
        let msg = "frontline-server binary not found (cargo build -p frontline-server)";
        assert!(!require(), "{msg}");
        eprintln!("SKIP: {msg}");
        return None;
    };
    let root = repo_root();
    if !root.join("data/maps/m0/meta.json").is_file() {
        let msg = "terrain missing (see DEVELOPING.md, Map data)";
        assert!(!require(), "{msg}");
        eprintln!("SKIP: {msg}");
        return None;
    }
    let log = std::env::temp_dir().join(format!(
        "headless-smoke-{}-{}.log",
        std::process::id(),
        rand_tag()
    ));
    let out = std::fs::File::create(&log).expect("log file");
    let child = Command::new(bin)
        .current_dir(&root)
        .args([
            "--ws-port",
            "0",
            "--wt-port",
            "0",
            "--metrics-port",
            "0",
            "--dev-cert",
        ])
        .args(["--bots", &bots.to_string()])
        .stdout(Stdio::from(out))
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn server");
    let mut s = Server {
        child,
        log,
        ws: 0,
        wt: 0,
        cert_hex: String::new(),
    };
    let end = Instant::now() + Duration::from_secs(20);
    while Instant::now() < end {
        let text = std::fs::read_to_string(&s.log).unwrap_or_default();
        let find = |ev: &str| {
            text.lines()
                .find(|l| l.contains(&format!("\"event\":\"{ev}\"")))
        };
        if let (Some(w), Some(t), Some(_)) = (
            find("ws_listening"),
            find("wt_listening"),
            find("metrics_listening"),
        ) {
            s.ws = field(w, "port")
                .and_then(|p| p.parse().ok())
                .expect("ws port");
            s.wt = field(t, "port")
                .and_then(|p| p.parse().ok())
                .expect("wt port");
            s.cert_hex = field(t, "cert_sha256_hex").expect("cert").to_string();
            return Some(s);
        }
        if let Ok(Some(st)) = s.child.try_wait() {
            panic!("server exited early: {st}\n{text}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("server did not report its ports");
}

fn rand_tag() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())
}

fn run_clients(s: &Server, transport: &str, count: u32, secs: u32) -> Report {
    run_clients_with(s, transport, count, secs, &[])
}

fn run_clients_with(s: &Server, transport: &str, count: u32, secs: u32, flags: &[&str]) -> Report {
    let (port, extra) = if transport == "wt" {
        (s.wt, vec!["--cert-hash".to_string(), s.cert_hex.clone()])
    } else {
        (s.ws, vec![])
    };
    let mut argv = vec![
        "headless-client".to_string(),
        "--url".into(),
        format!("127.0.0.1:{port}"),
        "--transport".into(),
        transport.into(),
        "--count".into(),
        count.to_string(),
        "--duration".into(),
        secs.to_string(),
        "--warmup".into(),
        "2".into(),
        "--ramp-ms".into(),
        "20".into(),
    ];
    argv.extend(extra);
    argv.extend(flags.iter().map(|f| f.to_string()));
    let args = Args::try_parse_from(argv).expect("args");
    args.validate().expect("valid");
    let ctx = build_ctx(&args).expect("ctx");
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("rt")
        .block_on(run(&args, ctx))
}

fn check(r: &Report, count: u32, min_snapshots: u64) {
    let a = &r.aggregate;
    eprintln!(
        "{}: joined {}/{}, down P50 {:.1} kbps (worst {:.1}), up P50 {:.1}, snapshots {}, decode_errors {}, missing_baseline {}, disconnects {}, corrections {}",
        r.transport, a.clients_joined, count, a.down_kbps.p50, a.down_kbps_p50_worst_client, a.up_kbps.p50,
        a.snapshots, a.decode_errors, a.missing_baseline, a.disconnects, a.corrections.total
    );
    assert_eq!(
        a.clients_joined,
        count,
        "every client joins: {:?}",
        r.per_client
            .iter()
            .map(|c| &c.join_error)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        a.disconnects,
        0,
        "{:?}",
        r.per_client
            .iter()
            .map(|c| &c.disconnect_reason)
            .collect::<Vec<_>>()
    );
    assert_eq!(a.decode_errors, 0);
    for c in &r.per_client {
        assert!(
            c.snapshots >= min_snapshots,
            "client {} got {} snapshots",
            c.id,
            c.snapshots
        );
        assert!(
            c.corrections.initial_syncs == 1,
            "client {} was placed once",
            c.id
        );
        assert!(
            c.distance_m > 5.0,
            "client {} walked {:.1} m",
            c.id,
            c.distance_m
        );
        assert!(
            c.rtt_ms.p50 > 0.0 && c.rtt_ms.p50 < 50.0,
            "loopback RTT {}",
            c.rtt_ms.p50
        );
        assert!(
            c.up_kbps.p50 > 5.0 && c.up_kbps.p50 < 32.0,
            "up {}",
            c.up_kbps.p50
        );
        assert!(
            c.down_kbps.p50 > 1.0 && c.down_kbps.p50 < 128.0,
            "down {}",
            c.down_kbps.p50
        );
    }
    assert!(a.down_kbps_p50_worst_client < 128.0);
}

#[test]
fn smoke_ws() {
    let Some(s) = start_server(2) else { return };
    let r = run_clients(&s, "ws", 3, 8);
    check(&r, 3, 40); // 15 Hz x ~6 s
}

#[test]
fn smoke_wt() {
    let Some(s) = start_server(2) else { return };
    let r = run_clients(&s, "wt", 3, 8);
    check(&r, 3, 60); // 20 Hz x ~6 s
}

#[test]
#[ignore = "loadtest-smoke: 60 s x 3 clients on both transports with 10 bots"]
fn loadtest_smoke() {
    let Some(s) = start_server(10) else { return };
    check(&run_clients(&s, "wt", 3, 60), 3, 800);
    check(&run_clients(&s, "ws", 3, 60), 3, 600);
}

#[cfg(feature = "anticheat-test")]
fn check_anticheat_mix(transport: &str, count: u32, duration: u32) {
    let Some(s) = start_server(0) else { return };
    let r = run_clients_with(&s, transport, count, duration, &["--anticheat-mix"]);
    assert_eq!(r.aggregate.clients_joined, count);
    assert_eq!(r.aggregate.decode_errors, 0);
    for c in &r.per_client {
        let reason = match c.id {
            3 => Some("server Disconnect, reason 2"),
            4 => Some("server Disconnect, reason 1"),
            _ => None,
        };
        assert_eq!(c.disconnect_reason.as_deref(), reason, "client {}", c.id);
        assert_eq!(
            c.disconnects,
            u64::from(reason.is_some()),
            "client {}",
            c.id
        );
        if reason.is_none() {
            assert!(
                c.connected_s >= f64::from(duration) - 0.1,
                "client {}",
                c.id
            );
        }
    }
}

#[cfg(feature = "anticheat-test")]
#[test]
fn anticheat_mix_ws() {
    check_anticheat_mix("ws", 6, 6);
}

#[cfg(feature = "anticheat-test")]
#[test]
fn anticheat_mix_wt() {
    check_anticheat_mix("wt", 6, 6);
}

#[cfg(feature = "anticheat-test")]
#[test]
#[ignore = "five-minute loopback anti-cheat mix"]
fn anticheat_mix_20_clients() {
    check_anticheat_mix("ws", 20, 300);
}
