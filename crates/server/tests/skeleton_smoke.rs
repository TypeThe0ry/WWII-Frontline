//! Headless smoke tests: start the real binary on random ports, scrape `/metrics`, check JSON
//! logs, and verify SIGTERM exits with code 0 well inside 100 ms.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

struct Server {
    child: Child,
    lines: mpsc::Receiver<String>,
    seen: Vec<String>,
}

impl Server {
    fn start(extra: &[&str]) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_frontline-server"))
            .args(["--metrics-port", "0", "--ws-port", "0", "--wt-port", "0"])
            .args(extra)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn server");
        let out = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for l in BufReader::new(out).lines().map_while(Result::ok) {
                if tx.send(l).is_err() {
                    break;
                }
            }
        });
        Server {
            child,
            lines: rx,
            seen: Vec::new(),
        }
    }

    fn metrics_port(&mut self) -> u16 {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if let Ok(l) = self.lines.recv_timeout(Duration::from_millis(100)) {
                self.seen.push(l.clone());
                if l.contains("\"metrics_listening\"") {
                    let v: serde_json::Value = serde_json::from_str(&l).unwrap();
                    return v["port"].as_u64().unwrap() as u16;
                }
            }
        }
        panic!(
            "server never reported its metrics port; saw {:?}",
            self.seen
        );
    }

    fn drain(&mut self) {
        while let Ok(l) = self.lines.recv_timeout(Duration::from_millis(300)) {
            self.seen.push(l);
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn scrape(port: u16) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    s.write_all(b"GET /metrics HTTP/1.1\r\nHost: x\r\n\r\n")
        .unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    out
}

#[test]
fn serves_metrics_and_logs_json_for_five_seconds() {
    let mut srv = Server::start(&[
        "--duration",
        "5",
        "--match-id",
        "123e4567-e89b-42d3-a456-426614174000",
    ]);
    let port = srv.metrics_port();
    std::thread::sleep(Duration::from_millis(1500));
    let body = scrape(port);
    assert!(body.starts_with("HTTP/1.1 200"), "{body}");
    let buckets = body
        .lines()
        .filter(|l| l.starts_with("server_tick_ms_bucket"))
        .count();
    assert!(buckets >= 10, "only {buckets} server_tick_ms buckets");
    for name in [
        "server_tick_interval_error_ms_bucket",
        "server_tick_overrun_total 0",
        "server_tick_overrun_consecutive_max 0",
        "server_players{team=\"us\"}",
        "server_bots 0",
        "server_uptime_seconds",
    ] {
        assert!(body.contains(name), "missing {name}\n{body}");
    }
    // The sim thread is ticking: count is > 0 after 1.5 s.
    let count: f64 = body
        .lines()
        .find(|l| l.starts_with("server_tick_ms_count"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .expect("server_tick_ms_count");
    assert!(count >= 30.0, "tick count {count}");

    // Wait for natural exit (--duration) and check status + logs.
    let status = srv.child.wait().unwrap();
    assert!(status.success(), "exit {status:?}");
    srv.drain();
    assert!(!srv.seen.is_empty());
    let mut events = Vec::new();
    for l in &srv.seen {
        let v: serde_json::Value =
            serde_json::from_str(l).unwrap_or_else(|e| panic!("bad JSON {l}: {e}"));
        for k in ["match_id", "tick", "level"] {
            assert!(v.get(k).is_some(), "missing {k} in {l}");
        }
        assert_eq!(v["match_id"], "123e4567-e89b-42d3-a456-426614174000");
        events.push(v["event"].as_str().unwrap_or("").to_string());
    }
    assert!(events.iter().any(|e| e == "startup"));
    let startup = srv.seen.iter().find(|l| l.contains("\"startup\"")).unwrap();
    for k in ["git_sha", "protocol_version", "data_hash"] {
        assert!(startup.contains(k), "startup line lacks {k}");
    }
}

#[test]
fn sigterm_exits_zero_within_100ms_and_writes_report() {
    let dir = std::env::temp_dir().join(format!("fl-smoke-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let report = dir.join("r.json");
    let mut srv = Server::start(&["--report", report.to_str().unwrap()]);
    let _ = srv.metrics_port();
    std::thread::sleep(Duration::from_millis(700));
    let t0 = Instant::now();
    let ok = Command::new("kill")
        .args(["-TERM", &srv.child.id().to_string()])
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let status = srv.child.wait().unwrap();
    let took = t0.elapsed();
    assert_eq!(status.code(), Some(0), "exit {status:?}");
    assert!(
        took < Duration::from_millis(100),
        "SIGTERM -> exit took {took:?}"
    );
    let r: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    assert!(r["ticks"].as_u64().unwrap() >= 10);
    assert!(r["tick_ms"]["p99"].is_number());
    assert!(r["tick_interval_error_ms"]["p99"].is_number());
    assert!(r["tick_overrun_total"].is_number());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn invalid_configuration_exits_nonzero_with_readable_error() {
    let out = Command::new(env!("CARGO_BIN_EXE_frontline-server"))
        .args(["--map", "atlantis"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("--map"), "{err}");
}
