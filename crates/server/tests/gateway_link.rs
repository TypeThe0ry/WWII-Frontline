//! Gateway link (M1-029): ticket handshake, registration retry, heartbeat cadence, result.
//! The "gateway" here is a small axum app; the real one is covered by its own tests.

mod common;

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use base64::Engine;
use common::*;
use ed25519_dalek::{Signer, SigningKey};
use frontline_protocol::dto::{TicketClaims, TICKET_DOMAIN};
use frontline_protocol::msg::c0::{RejectReason, ServerMsgC0};
use frontline_protocol::PROTOCOL_VERSION;
use frontline_server::gateway_link::client::{GatewayClient, Heartbeat, RegisterReq};
use frontline_server::gateway_link::result::{spool, MatchResult};
use frontline_server::gateway_link::ticket_verify::TicketVerifier;
use frontline_server::gateway_link::{heartbeat, TicketGate};

const T: Duration = Duration::from_secs(3);
const MATCH: [u8; 16] = [1; 16];

fn sk() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

fn ticket(jti: u8, issued: u64) -> Vec<u8> {
    let c = TicketClaims {
        player_id: 3,
        match_id: MATCH,
        nick: "Gaius".into(),
        exp: issued + 60,
        jti: [jti; 16],
        is_admin: false,
    };
    let bytes = c.encode();
    let mut m = TICKET_DOMAIN.to_vec();
    m.extend_from_slice(&bytes);
    let e = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    format!(
        "{}.{}",
        e.encode(&bytes),
        e.encode(sk().sign(&m).to_bytes())
    )
    .into_bytes()
}

struct Gate {
    gate: Arc<TicketGate>,
    now: Arc<AtomicU64>,
}

fn gate(armed: bool) -> Gate {
    let now = Arc::new(AtomicU64::new(10_000));
    let n = now.clone();
    let gate = TicketGate::new(Arc::new(move || n.load(Ordering::SeqCst)));
    if armed {
        gate.arm(TicketVerifier::new(vec![sk().verifying_key()]), MATCH);
    }
    Gate { gate, now }
}

async fn join(h: &Harness, t: Vec<u8>) -> Option<ServerMsgC0> {
    let mut c = Client::connect(&h.url).await.unwrap();
    c.send(
        frontline_protocol::Channel::C0,
        &hello_msg(PROTOCOL_VERSION, DATA_HASH, t),
    )
    .await;
    c.server_msg(T).await
}

fn counted(h: &Harness, r: &str) -> u64 {
    h.metrics
        .ticket_rejected_total
        .with_label_values(&[r])
        .get()
}

#[tokio::test]
async fn ticket_handshake_accepts_once_and_refuses_replay_expired_tampered() {
    let g = gate(true);
    let h = Harness::start_full(Kind::Ws, T, 16, Some(g.gate.clone())).await;
    let t = ticket(1, 10_000);
    assert!(matches!(
        join(&h, t.clone()).await,
        Some(ServerMsgC0::Welcome(_))
    ));
    // Replay inside the 60 s window.
    assert!(matches!(
        join(&h, t).await,
        Some(ServerMsgC0::Reject(r)) if r.reason == RejectReason::InvalidTicket
    ));
    assert_eq!(counted(&h, "replayed"), 1);
    // 59 s old is fine, 61 s old is expired.
    g.now.store(10_059, Ordering::SeqCst);
    assert!(matches!(
        join(&h, ticket(2, 10_000)).await,
        Some(ServerMsgC0::Welcome(_))
    ));
    g.now.store(10_061, Ordering::SeqCst);
    assert!(matches!(
        join(&h, ticket(3, 10_000)).await,
        Some(ServerMsgC0::Reject(_))
    ));
    assert_eq!(counted(&h, "expired"), 1);
    // Tampered and empty.
    let mut bad = ticket(4, 10_061);
    let n = bad.len();
    bad[n - 3] = if bad[n - 3] == b'A' { b'B' } else { b'A' };
    assert!(matches!(join(&h, bad).await, Some(ServerMsgC0::Reject(_))));
    assert!(matches!(
        join(&h, vec![]).await,
        Some(ServerMsgC0::Reject(_))
    ));
    assert_eq!(counted(&h, "bad_signature") + counted(&h, "malformed"), 2);
    h.stop().await;
}

#[tokio::test]
async fn unarmed_gate_refuses_everyone_as_not_ready() {
    let g = gate(false);
    let h = Harness::start_full(Kind::Ws, T, 16, Some(g.gate.clone())).await;
    assert!(matches!(
        join(&h, ticket(1, 10_000)).await,
        Some(ServerMsgC0::Reject(_))
    ));
    assert_eq!(counted(&h, "not_ready"), 1);
    h.stop().await;
}

#[tokio::test]
async fn no_gate_means_no_ticket_is_needed() {
    let h = Harness::start().await;
    assert!(matches!(
        join(&h, vec![]).await,
        Some(ServerMsgC0::Welcome(_))
    ));
    h.stop().await;
}

// ---- fake gateway ---------------------------------------------------------------------------

#[derive(Default)]
struct Fake {
    register_calls: AtomicU32,
    fail_register: AtomicU32,
    beats: Mutex<Vec<Instant>>,
    results: Mutex<Vec<serde_json::Value>>,
}

const TOKEN: &str = "internal-token";

fn authed(h: &HeaderMap) -> bool {
    h.get("authorization").and_then(|v| v.to_str().ok()) == Some(&format!("Bearer {TOKEN}"))
}

async fn spawn_fake(f: Arc<Fake>) -> String {
    async fn register(
        State(f): State<Arc<Fake>>,
        h: HeaderMap,
        Json(v): Json<serde_json::Value>,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        if !authed(&h) {
            return Err(StatusCode::UNAUTHORIZED);
        }
        // Use the real gateway's numeric u64 contract, not a string fixture.
        if v["data_hash"].as_u64() != Some(u64::MAX) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        f.register_calls.fetch_add(1, Ordering::SeqCst);
        if f.fail_register.load(Ordering::SeqCst) > 0 {
            f.fail_register.fetch_sub(1, Ordering::SeqCst);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        Ok(Json(serde_json::json!({
            "match_id": "0102030405060708090a0b0c0d0e0f10",
            "ticket_public_key": base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(sk().verifying_key().as_bytes()),
            "ticket_key_fingerprint": "abcd"
        })))
    }
    async fn beat(
        State(f): State<Arc<Fake>>,
        h: HeaderMap,
        Path(_id): Path<String>,
        Json(_): Json<serde_json::Value>,
    ) -> StatusCode {
        if !authed(&h) {
            return StatusCode::UNAUTHORIZED;
        }
        f.beats.lock().unwrap().push(Instant::now());
        StatusCode::OK
    }
    async fn result(
        State(f): State<Arc<Fake>>,
        Path(_id): Path<String>,
        Json(v): Json<serde_json::Value>,
    ) -> StatusCode {
        f.results.lock().unwrap().push(v);
        StatusCode::OK
    }
    let app = Router::new()
        .route("/internal/matches/register", post(register))
        .route("/internal/matches/{id}/heartbeat", post(beat))
        .route("/internal/matches/{id}/result", post(result))
        .with_state(f);
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    format!("http://{addr}")
}

fn req() -> RegisterReq {
    RegisterReq {
        name: "t".into(),
        map: "m0".into(),
        mode: "sandbox".into(),
        region: "local".into(),
        tags: vec![],
        version: "x".into(),
        protocol_version: PROTOCOL_VERSION,
        data_hash: u64::MAX,
        cap: 16,
        addr: "127.0.0.1".into(),
        ws_port: 1,
        wt_port: 0,
    }
}

#[tokio::test]
async fn register_retries_three_times_then_gives_up() {
    let f = Arc::new(Fake::default());
    let url = spawn_fake(f.clone()).await;
    let c = GatewayClient::new(&url, TOKEN).unwrap();
    f.fail_register.store(2, Ordering::SeqCst);
    let ok = c
        .register_with_retry(&req(), 3, Duration::from_millis(10))
        .await
        .unwrap();
    assert_eq!(ok.match_id.len(), 32);
    assert_eq!(f.register_calls.load(Ordering::SeqCst), 3);

    f.register_calls.store(0, Ordering::SeqCst);
    f.fail_register.store(99, Ordering::SeqCst);
    let e = c
        .register_with_retry(&req(), 3, Duration::from_millis(10))
        .await
        .unwrap_err();
    assert_eq!(f.register_calls.load(Ordering::SeqCst), 3);
    assert!(e.to_string().contains("500"), "{e}");

    let wrong = GatewayClient::new(&url, "nope").unwrap();
    assert!(wrong
        .register(&req())
        .await
        .unwrap_err()
        .to_string()
        .contains("401"));
    assert!(GatewayClient::new("https://x", "t").is_err());
}

#[tokio::test]
async fn heartbeat_task_beats_once_a_second_and_survives_failures() {
    let f = Arc::new(Fake::default());
    let url = spawn_fake(f.clone()).await;
    let metrics = frontline_server::metrics::Metrics::new();
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    let src: heartbeat::Source = Arc::new(|| Heartbeat {
        humans: 2,
        bots: 6,
        phase: "playing".into(),
        time_left_s: 10,
    });
    let task = tokio::spawn(heartbeat::run(
        GatewayClient::new(&url, TOKEN).unwrap(),
        "0102030405060708090a0b0c0d0e0f10".into(),
        src.clone(),
        metrics.clone(),
        stop_rx.clone(),
        heartbeat::INTERVAL,
    ));
    tokio::time::sleep(Duration::from_millis(5_500)).await;
    let beats = f.beats.lock().unwrap().clone();
    assert!(
        (5..=6).contains(&beats.len()),
        "{} beats in 5.5 s",
        beats.len()
    );
    let worst = beats
        .windows(2)
        .map(|w| w[1].duration_since(w[0]).as_millis() as i64 - 1000)
        .map(i64::abs)
        .max()
        .unwrap();
    println!("beats={} worst interval jitter={worst} ms", beats.len());
    assert!(worst < 200, "jitter {worst} ms");
    assert_eq!(
        metrics
            .gateway_heartbeat_total
            .with_label_values(&["ok"])
            .get(),
        beats.len() as u64
    );
    stop_tx.send(true).unwrap();
    task.await.unwrap();

    // A dead gateway only produces error counts; the task keeps going until stopped.
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    let dead = GatewayClient::new("http://127.0.0.1:1", TOKEN).unwrap();
    let task = tokio::spawn(heartbeat::run(
        dead,
        "x".into(),
        src,
        metrics.clone(),
        stop_rx,
        Duration::from_millis(50),
    ));
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(!task.is_finished());
    assert!(
        metrics
            .gateway_heartbeat_total
            .with_label_values(&["error"])
            .get()
            >= 4
    );
    stop_tx.send(true).unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn result_is_posted_and_spooled_when_undeliverable() {
    let f = Arc::new(Fake::default());
    let url = spawn_fake(f.clone()).await;
    let r = MatchResult {
        players: 12,
        bots: 4,
        objectives: 5,
        duration_s: 600,
        ticks: 18_000,
        rows: vec![],
    };
    GatewayClient::new(&url, TOKEN)
        .unwrap()
        .submit_result("0102030405060708090a0b0c0d0e0f10", &r)
        .await
        .unwrap();
    let got = f.results.lock().unwrap()[0].clone();
    assert_eq!(
        (got["players"].as_u64(), got["objectives"].as_u64()),
        (Some(12), Some(5))
    );

    let dead = GatewayClient::new("http://127.0.0.1:1", TOKEN).unwrap();
    assert!(dead.submit_result("abc", &r).await.is_err());
    let dir = std::env::temp_dir().join(format!("fl-spool-{}", std::process::id()));
    let p = spool(&dir, "../ab-cd", &r).unwrap();
    assert_eq!(p.file_name().unwrap(), "ab-cd.json");
    let back: MatchResult = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    assert_eq!(back, r);
    let _ = std::fs::remove_dir_all(dir);
}
