//! Opt-in process checks: build `frontline-gateway`, then set `FRONTLINE_TEST_GATEWAY_BIN`
//! to its path and run `cargo test -p frontline-server --test gateway_link -- --ignored --nocapture`.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};

use serde_json::{json, Value};
use tokio::time::sleep;

use super::{sk, spawn_fake, Arc, Client, Duration, Engine, Fake, Instant, Ordering, TOKEN};
use super::{Signer, TicketClaims, PROTOCOL_VERSION, T, TICKET_DOMAIN};
use frontline_protocol::msg::c0::{RejectReason, ServerMsgC0};
use frontline_protocol::Channel;

const URL64: base64::engine::GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;
const POLL: Duration = Duration::from_millis(20);
const START_TIMEOUT: Duration = Duration::from_secs(12);

struct Process {
    child: Child,
    log: PathBuf,
}

impl Process {
    fn spawn(mut cmd: Command, dir: &Path, name: &str) -> Self {
        let log = dir.join(format!("{name}.log"));
        let file = File::create(&log).unwrap();
        let child = cmd
            .current_dir(dir)
            .stdin(Stdio::null())
            .stdout(file.try_clone().unwrap())
            .stderr(file)
            .spawn()
            .unwrap_or_else(|e| panic!("cannot launch {name}: {e}"));
        Self { child, log }
    }

    fn logs(&self) -> String {
        fs::read_to_string(&self.log).unwrap()
    }

    fn events(&self, name: &str) -> Vec<Value> {
        self.logs()
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|v| v["event"] == name)
            .collect()
    }

    async fn event(&mut self, name: &str) -> Value {
        let until = Instant::now() + START_TIMEOUT;
        loop {
            if let Some(event) = self.events(name).into_iter().next() {
                return event;
            }
            assert!(
                Instant::now() < until && self.child.try_wait().unwrap().is_none(),
                "missing {name}: {}",
                self.logs()
            );
            sleep(POLL).await;
        }
    }

    async fn exit(&mut self, timeout: Duration) -> ExitStatus {
        let until = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(
                Instant::now() < until,
                "child failed to exit: {}",
                self.logs()
            );
            sleep(POLL).await;
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn server(dir: &Path, gateway: &str, duration: &str, name: &str) -> Process {
    let pin = dir.join("ticket.pub");
    fs::write(&pin, URL64.encode(sk().verifying_key().as_bytes())).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_frontline-server"));
    cmd.env_clear()
        .env("GATEWAY_URL", gateway)
        .env("INTERNAL_API_TOKEN", TOKEN)
        .env("TICKET_PUBLIC_KEY_FILE", pin)
        .args(["--empty-world", "--ws-port", "0", "--metrics-port", "0"])
        .args(["--spool-dir", "spool", "--duration", duration]);
    Process::spawn(cmd, dir, name)
}

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(T)
        .build()
        .unwrap()
}

async fn metrics(http: &reqwest::Client, port: u64) -> String {
    http.get(format!("http://127.0.0.1:{port}/metrics"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .text()
        .await
        .unwrap()
}

fn metric(text: &str, name: &str) -> u64 {
    text.lines()
        .filter_map(|line| line.split_once(' '))
        .find_map(|(n, val)| (n == name).then(|| val.parse().unwrap()))
        .unwrap_or_else(|| panic!("metric {name} missing"))
}

async fn hello(port: u64, ticket: Vec<u8>) -> (Client, ServerMsgC0) {
    let mut client = Client::connect(&format!("ws://127.0.0.1:{port}"))
        .await
        .unwrap();
    let hash = frontline_sim::GameData::embedded().hash;
    client
        .send(
            Channel::C0,
            &super::hello_msg(PROTOCOL_VERSION, hash, ticket),
        )
        .await;
    let reply = client.server_msg(T).await.expect("Hello reply");
    (client, reply)
}

async fn expect_reject(port: u64, ticket: Vec<u8>) {
    let (mut client, reply) = hello(port, ticket).await;
    assert!(
        matches!(reply, ServerMsgC0::Reject(r) if r.reason == RejectReason::InvalidTicket),
        "{reply:?}"
    );
    assert!(client.expect_closed(T).await);
}

#[tokio::test]
async fn process_registration_500_exits_after_three_attempts() {
    let f = Arc::new(Fake {
        first_register_gate: Some(tokio::sync::Semaphore::new(0)),
        ..Fake::default()
    });
    f.fail_register.store(3, Ordering::SeqCst);
    let url = spawn_fake(f.clone()).await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = server(dir.path(), &url, "60", "register-fail");
    let ws = s.event("ws_listening").await["port"].as_u64().unwrap();
    let p = s.event("metrics_listening").await["port"].as_u64().unwrap();
    expect_reject(ws, vec![]).await;
    let text = metrics(&http(), p).await;
    assert_eq!(metric(&text, "gateway_registered"), 0);
    assert_eq!(
        metric(&text, "ticket_rejected_total{reason=\"not_ready\"}"),
        1
    );
    f.first_register_gate.as_ref().unwrap().add_permits(1);
    let status = s.exit(START_TIMEOUT).await;
    assert_eq!(status.code(), Some(1), "{}", s.logs());
    assert_eq!(f.register_calls.load(Ordering::SeqCst), 3);
    let failures = s.events("gateway_register_failed");
    assert_eq!(failures.len(), 3);
    for (i, log) in failures.iter().enumerate() {
        assert_eq!(log["attempt"], i + 1);
        assert_eq!(log["error"], "gateway answered HTTP 500");
    }
    assert_eq!(
        s.events("fatal")[0]["error"],
        "gateway registration failed after 3 attempts: gateway answered HTTP 500"
    );
    eprintln!("registration: 3 HTTP 500 attempts, exit code 1, not_ready=1");
}

#[tokio::test]
async fn process_result_is_spooled_and_retried_by_next_process() {
    let f = Arc::new(Fake::default());
    f.fail_result.store(true, Ordering::SeqCst);
    let url = spawn_fake(f.clone()).await;
    let dir = tempfile::tempdir().unwrap();
    let mut first = server(dir.path(), &url, "1.5", "first");
    let ws = first.event("ws_listening").await["port"].as_u64().unwrap();
    let id = first.event("gateway_registered").await["gateway_match_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut claims = fixture_claims(&id);
    claims.exp = (frontline_server::gateway_link::system_now_secs())() + 60;
    let (client, welcome) = hello(ws, sign(&claims)).await;
    assert!(matches!(welcome, ServerMsgC0::Welcome(_)));
    assert!(
        first.exit(START_TIMEOUT).await.success(),
        "{}",
        first.logs()
    );
    drop(client);
    assert_eq!(first.events("result_spooled").len(), 1);
    let spool_file = dir.path().join("spool").join(format!("{id}.json"));
    let payload: Value = serde_json::from_reader(File::open(&spool_file).unwrap()).unwrap();
    assert_eq!(payload["players"], 1);
    assert_eq!(payload["objectives"], 0);
    assert!(payload["ticks"].as_u64().unwrap() >= 30);
    assert_eq!(
        f.results.lock().unwrap().as_slice(),
        &[(id.clone(), payload.clone())]
    );

    // An invalid leftover is not deleted and cannot prevent a valid file from replaying.
    let broken = spool_file.with_file_name("ffffffffffffffffffffffffffffffff.json");
    fs::write(&broken, "invalid JSON").unwrap();
    f.fail_result.store(false, Ordering::SeqCst);
    let mut second = server(dir.path(), &url, "0.5", "second");
    let replay = second.event("result_replayed").await;
    assert_eq!(replay["gateway_match_id"], id);
    assert!(!spool_file.exists());
    assert!(
        second.exit(START_TIMEOUT).await.success(),
        "{}",
        second.logs()
    );
    assert!(broken.exists());
    assert_eq!(second.events("result_replay_failed").len(), 1);
    let posts = f.results.lock().unwrap();
    assert_eq!(posts.len(), 3);
    assert_eq!(posts[0], posts[1]);
    assert_ne!(posts[1].0, posts[2].0);
    eprintln!(
        "result: players={}, objectives={}, ticks={}, identical prior-ID payload replayed",
        payload["players"], payload["objectives"], payload["ticks"]
    );
}

#[tokio::test]
async fn process_heartbeats_warn_and_recover_without_ending_match() {
    let fake = Arc::new(Fake::default());
    fake.fail_beat.store(true, Ordering::SeqCst);
    let url = spawn_fake(fake.clone()).await;
    let dir = tempfile::tempdir().unwrap();
    let mut server = server(dir.path(), &url, "4.5", "heartbeat-outage");
    let port = server.event("metrics_listening").await["port"]
        .as_u64()
        .unwrap();
    let warning = server.event("gateway_heartbeat_failing").await;
    assert_eq!(warning["error"], "gateway answered HTTP 503");
    let http = http();
    let failed = metric(
        &metrics(&http, port).await,
        "gateway_heartbeat_total{result=\"error\"}",
    );
    assert!(failed >= 3);
    fake.fail_beat.store(false, Ordering::SeqCst);
    server.event("gateway_heartbeat_recovered").await;
    assert!(
        metric(
            &metrics(&http, port).await,
            "gateway_heartbeat_total{result=\"ok\"}"
        ) >= 1
    );
    assert!(server.exit(T).await.success(), "{}", server.logs());
    assert_eq!(server.events("gateway_heartbeat_failing").len(), 1);
    assert_eq!(fake.results.lock().unwrap().len(), 1);
    eprintln!(
        "heartbeat outage: {failed} failed attempts, one warning, recovered and finished the match"
    );
}

#[tokio::test]
#[ignore = "measures a full 60 seconds of wall-clock heartbeat cadence"]
async fn process_heartbeat_60s_cadence() {
    let f = Arc::new(Fake::default());
    let url = spawn_fake(f.clone()).await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = server(dir.path(), &url, "60", "cadence");
    let p = s.event("metrics_listening").await["port"].as_u64().unwrap();
    s.event("gateway_registered").await;
    assert_eq!(metric(&metrics(&http(), p).await, "gateway_registered"), 1);
    assert!(
        s.exit(Duration::from_secs(66)).await.success(),
        "{}",
        s.logs()
    );
    let beats = f.beats.lock().unwrap();
    assert!(
        (58..=62).contains(&beats.len()),
        "{} heartbeats in 60 s",
        beats.len()
    );
    let mut jitter = beats
        .windows(2)
        .map(|w| (w[1].0.duration_since(w[0].0).as_secs_f64() - 1.0).abs() * 1000.0)
        .collect::<Vec<_>>();
    jitter.sort_by(f64::total_cmp);
    let p99 = jitter[(jitter.len() * 99).div_ceil(100) - 1];
    eprintln!(
        "heartbeat: count={} in 60 s, interval jitter p99={p99:.3} ms",
        beats.len()
    );
    assert!(p99 < 200.0);
    assert!(beats.first().unwrap().1["time_left_s"].as_u64().unwrap() >= 59);
    assert!(beats.last().unwrap().1["time_left_s"].as_u64().unwrap() <= 3);
}

fn sign(claims: &TicketClaims) -> Vec<u8> {
    let bytes = claims.encode();
    let message = [TICKET_DOMAIN, bytes.as_slice()].concat();
    format!(
        "{}.{}",
        URL64.encode(&bytes),
        URL64.encode(sk().sign(&message).to_bytes())
    )
    .into_bytes()
}

fn fixture_claims(id: &str) -> TicketClaims {
    let match_id = std::array::from_fn(|i| u8::from_str_radix(&id[i * 2..i * 2 + 2], 16).unwrap());
    TicketClaims {
        player_id: 3,
        match_id,
        nick: "Fixture".into(),
        exp: 1,
        jti: [42; 16],
        is_admin: false,
    }
}

async fn servers(http: &reqwest::Client, url: &str) -> reqwest::Result<Vec<Value>> {
    http.get(format!("{url}/v1/servers"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

#[tokio::test]
#[ignore = "requires a real frontline-gateway binary via FRONTLINE_TEST_GATEWAY_BIN"]
async fn real_gateway_process_ticket_and_staleness() {
    let path =
        std::env::var_os("FRONTLINE_TEST_GATEWAY_BIN").expect("set FRONTLINE_TEST_GATEWAY_BIN");
    let binary = fs::canonicalize(path).expect("locate frontline-gateway");
    let dir = tempfile::tempdir().unwrap();
    let seed = dir.path().join("ticket.seed");
    fs::write(&seed, sk().to_bytes()).unwrap();
    // Older gateway binaries log the requested port and cannot report a kernel-assigned port.
    let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = socket.local_addr().unwrap();
    let url = format!("http://{addr}");
    let mut cmd = Command::new(binary);
    cmd.env_clear()
        .env("GATEWAY_BIND", addr.to_string())
        .env("TICKET_SIGNING_KEY_FILE", seed)
        .env("INTERNAL_API_TOKEN", TOKEN)
        .env("TOKEN_SECRET", "fixture-only-guest-token-signing-key")
        .env("TOKIO_WORKER_THREADS", "2");
    drop(socket);
    let mut gateway = Process::spawn(cmd, dir.path(), "gateway");
    let http = http();
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        if let Ok(list) = servers(&http, &url).await {
            assert!(list.is_empty());
            break;
        }
        assert!(
            Instant::now() < deadline && gateway.child.try_wait().unwrap().is_none(),
            "gateway not ready: {}",
            gateway.logs()
        );
        sleep(POLL).await;
    }

    let mut s = server(dir.path(), &url, "45", "real-match");
    let p = s.event("metrics_listening").await["port"].as_u64().unwrap();
    let ws = s.event("ws_listening").await["port"].as_u64().unwrap();
    let registered = s.event("gateway_registered").await;
    let id = registered["gateway_match_id"].as_str().unwrap();
    assert_eq!(metric(&metrics(&http, p).await, "gateway_registered"), 1);
    let list = servers(&http, &url).await.unwrap();
    assert!(list
        .iter()
        .any(|v| v["match_id"] == id && v["compatible"] == true));
    let guest: Value = http
        .post(format!("{url}/v1/guest"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let ticket: Value = http
        .post(format!("{url}/v1/servers/{id}/ticket"))
        .bearer_auth(guest["token"].as_str().unwrap())
        .json(&json!({"nick":"Fixture"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ticket["ws_port"], ws);
    assert_eq!(ticket["addr"], "127.0.0.1");
    assert_eq!(ticket["expires_in_s"], 60);
    let raw = ticket["ticket"].as_str().unwrap().as_bytes().to_vec();
    let (client, reply) = hello(ws, raw.clone()).await;
    assert!(matches!(reply, ServerMsgC0::Welcome(_)));
    expect_reject(ws, raw.clone()).await;
    let mut tampered = raw;
    let first = tampered.iter().position(|&b| b == b'.').unwrap() + 1;
    tampered[first] = if tampered[first] == b'A' { b'B' } else { b'A' };
    expect_reject(ws, tampered).await;
    let mut expired = fixture_claims(id);
    expired.exp = (frontline_server::gateway_link::system_now_secs())().saturating_sub(1);
    expect_reject(ws, sign(&expired)).await;
    let text = metrics(&http, p).await;
    for reason in ["replayed", "bad_signature", "expired"] {
        assert_eq!(
            metric(
                &text,
                &format!("ticket_rejected_total{{reason=\"{reason}\"}}")
            ),
            1
        );
    }

    // Kill immediately after observing a new acknowledgement so the gateway delists via
    // its heartbeat timeout. The sampling error is roughly one poll plus local HTTP time.
    let ok = "gateway_heartbeat_total{result=\"ok\"}";
    let previous = metric(&text, ok);
    let deadline = Instant::now() + T;
    let last_seen;
    loop {
        let poll_start = Instant::now();
        if metric(&metrics(&http, p).await, ok) > previous {
            last_seen = poll_start;
            s.child.kill().unwrap();
            assert!(!s.exit(T).await.success());
            break;
        }
        assert!(Instant::now() < deadline, "no new heartbeat");
        sleep(POLL).await;
    }
    drop(client);
    sleep(Duration::from_millis(4750).saturating_sub(last_seen.elapsed())).await;
    assert!(servers(&http, &url)
        .await
        .unwrap()
        .iter()
        .any(|v| v["match_id"] == id && v["humans"] == 1));
    let gone_after = loop {
        if !servers(&http, &url)
            .await
            .unwrap()
            .iter()
            .any(|v| v["match_id"] == id)
        {
            break last_seen.elapsed();
        }
        assert!(
            last_seen.elapsed() < Duration::from_millis(5500),
            "real gateway failed to delist stale match"
        );
        sleep(POLL).await;
    };
    assert!((Duration::from_millis(4900)..=Duration::from_millis(5500)).contains(&gone_after));
    let mut graceful = server(dir.path(), &url, "1.0", "graceful-match");
    let registration = graceful.event("gateway_registered").await;
    let graceful_id = registration["gateway_match_id"].as_str().unwrap();
    assert!(servers(&http, &url)
        .await
        .unwrap()
        .iter()
        .any(|v| v["match_id"] == graceful_id));
    assert!(graceful.exit(T).await.success(), "{}", graceful.logs());
    assert!(graceful.events("result_spooled").is_empty());
    assert!(graceful.events("result_lost").is_empty());
    assert!(servers(&http, &url)
        .await
        .unwrap()
        .iter()
        .all(|v| v["match_id"] != graceful_id));
    eprintln!("real gateway: registered=1, Welcome, replay/expired/signature rejects=1 each, delisted {:.3} s after last observed heartbeat, graceful match delisted on result", gone_after.as_secs_f64());
}
