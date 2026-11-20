//! Gateway link (M1-029): ticket handshake, registration retry, heartbeat cadence, result.
//! In-process fixtures plus executable-level PI-076 checks.

#[path = "gateway_link/process.rs"]
mod process;

mod common;

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use base64::Engine;
use common::*;
use ed25519_dalek::{Signer, SigningKey};
use frontline_protocol::dto::{TicketClaims, TICKET_DOMAIN};
use frontline_protocol::msg::c0::{RejectReason, ServerMsgC0};
use frontline_protocol::{Channel, PROTOCOL_VERSION};
use frontline_server::gateway_link::client::{GatewayClient, Heartbeat, RegisterReq};
use frontline_server::gateway_link::result::{spool, MatchResult};
use frontline_server::gateway_link::ticket_verify::TicketVerifier;
use frontline_server::gateway_link::{heartbeat, Admitted, TicketGate};
use frontline_server::sim_thread::Command;
use frontline_sim::PlayerId;

const T: Duration = Duration::from_secs(3);
const MATCH: [u8; 16] = [1; 16];

fn sk() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

fn claims(jti: u8, issued: u64) -> TicketClaims {
    TicketClaims {
        player_id: 3,
        match_id: MATCH,
        nick: "Gaius".into(),
        exp: issued + 60,
        jti: [jti; 16],
        is_admin: false,
    }
}

fn ticket(jti: u8, issued: u64) -> Vec<u8> {
    sign_claim_bytes(&claims(jti, issued).encode())
}

fn sign_claim_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut m = TICKET_DOMAIN.to_vec();
    m.extend_from_slice(bytes);
    let e = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    format!("{}.{}", e.encode(bytes), e.encode(sk().sign(&m).to_bytes())).into_bytes()
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

async fn join_live(h: &Harness, ticket: Vec<u8>) -> (Client, Option<ServerMsgC0>) {
    let mut client = Client::connect(&h.url).await.unwrap();
    assert!(
        client
            .send(Channel::C0, &hello_msg(PROTOCOL_VERSION, DATA_HASH, ticket))
            .await
    );
    let reply = client.server_msg(T).await;
    (client, reply)
}

async fn join(h: &Harness, ticket: Vec<u8>) -> Option<ServerMsgC0> {
    join_live(h, ticket).await.1
}

async fn assert_disconnected(h: &Harness, slot: PlayerId) {
    let command = tokio::time::timeout(T, async {
        loop {
            if let Ok(command) = h.commands.try_recv() {
                break command;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("ClientGone after disconnect");
    assert!(matches!(command, Command::ClientGone(id) if id == slot));
    assert!(h.ctx.sessions.handle_of(slot).is_none());
    assert_eq!(h.ctx.sessions.ticket_identity_of(slot), None);
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
async fn ws_ticket_identity_lives_and_dies_with_the_server_slot() {
    let g = gate(true);
    let h = Harness::start_full(Kind::Ws, T, 16, Some(g.gate)).await;
    let mut signed = claims(1, 10_000);
    signed.is_admin = true; // Retained as metadata, not used to authorize anything.
    let (client, reply) = join_live(&h, sign_claim_bytes(&signed.encode())).await;
    let Some(ServerMsgC0::Welcome(welcome)) = reply else {
        panic!("expected Welcome, got {reply:?}");
    };
    let slot = welcome.player_id;
    assert_eq!(slot, 0);
    assert_ne!(slot, signed.player_id);
    assert!(matches!(h.commands.try_recv(), Ok(Command::Join { player, .. }) if player == slot));
    let identity = Admitted {
        gateway_player: signed.player_id,
        jti: signed.jti,
        nick: signed.nick.clone(),
        is_admin: true,
    };
    assert_eq!(
        h.ctx.sessions.ticket_identity_of(slot),
        Some(identity.clone())
    );

    let mut wrong_match = claims(2, 10_000);
    wrong_match.match_id = [2; 16];
    let mut malformed = claims(3, 10_000).encode();
    malformed[0] = 0; // Signature is valid, but the claim format is not.
    for (bytes, reason) in [
        (wrong_match.encode(), "wrong_match"),
        (malformed, "malformed"),
    ] {
        assert!(matches!(
            join(&h, sign_claim_bytes(&bytes)).await,
            Some(ServerMsgC0::Reject(r)) if r.reason == RejectReason::InvalidTicket
        ));
        assert_eq!(counted(&h, reason), 1);
        assert!(h.commands.is_empty());
        assert!(h.ctx.sessions.handle_of(1).is_none());
        assert_eq!(h.ctx.sessions.ticket_identity_of(1), None);
        assert_eq!(
            h.ctx.sessions.ticket_identity_of(slot),
            Some(identity.clone())
        );
    }

    drop(client);
    assert_disconnected(&h, slot).await;

    let mut next = claims(4, 10_000);
    next.nick = "New Guest".into();
    let (_client, reply) = join_live(&h, sign_claim_bytes(&next.encode())).await;
    assert!(matches!(reply, Some(ServerMsgC0::Welcome(w)) if w.player_id == slot));
    assert_eq!(
        h.ctx.sessions.ticket_identity_of(slot),
        Some(Admitted {
            gateway_player: next.player_id,
            jti: next.jti,
            nick: next.nick,
            is_admin: false,
        })
    );
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
    let (client, reply) = join_live(&h, vec![]).await;
    let Some(ServerMsgC0::Welcome(welcome)) = reply else {
        panic!("expected anonymous Welcome, got {reply:?}");
    };
    let slot = welcome.player_id;
    assert!(matches!(h.commands.try_recv(), Ok(Command::Join { player, .. }) if player == slot));
    assert!(h.ctx.sessions.handle_of(slot).is_some());
    assert_eq!(h.ctx.sessions.ticket_identity_of(slot), None);
    drop(client);
    assert_disconnected(&h, slot).await;
    h.stop().await;
}

// ---- fake gateway ---------------------------------------------------------------------------

#[derive(Default)]
struct Fake {
    register_calls: AtomicU32,
    fail_register: AtomicU32,
    first_register_gate: Option<tokio::sync::Semaphore>,
    fail_result: std::sync::atomic::AtomicBool,
    fail_beat: std::sync::atomic::AtomicBool,
    beats: Mutex<Vec<(Instant, serde_json::Value)>>,
    results: Mutex<Vec<(String, serde_json::Value)>>,
    reports: Mutex<Vec<serde_json::Value>>,
    report_replays: Mutex<Vec<(i64, serde_json::Value)>>,
    chat: Mutex<Vec<serde_json::Value>>,
    actions: Mutex<Vec<serde_json::Value>>,
    action_acks: Mutex<Vec<(i64, serde_json::Value)>>,
    fail_ack_once: AtomicU32,
}

const TOKEN: &str = "internal-token";

fn authed(h: &HeaderMap) -> bool {
    h.get("authorization").and_then(|v| v.to_str().ok()) == Some(&format!("Bearer {TOKEN}"))
}

async fn spawn_fake(f: Arc<Fake>) -> String {
    use serde_json::{json, Value};

    async fn register(
        State(f): State<Arc<Fake>>,
        h: HeaderMap,
        Json(v): Json<serde_json::Value>,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        if !authed(&h) {
            return Err(StatusCode::UNAUTHORIZED);
        }
        // Use the real gateway's numeric u64 contract, not a string fixture.
        if v["data_hash"].as_u64().is_none() {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let call = f.register_calls.fetch_add(1, Ordering::SeqCst) + 1;
        if call == 1 {
            if let Some(gate) = &f.first_register_gate {
                gate.acquire().await.unwrap().forget();
            }
        }
        if f.fail_register.load(Ordering::SeqCst) > 0 {
            f.fail_register.fetch_sub(1, Ordering::SeqCst);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        Ok(Json(serde_json::json!({
            "match_id": format!("{call:032x}"),
            "ticket_public_key": base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(sk().verifying_key().as_bytes()),
            "ticket_key_fingerprint": "abcd"
        })))
    }
    async fn beat(
        State(f): State<Arc<Fake>>,
        h: HeaderMap,
        Path(_id): Path<String>,
        Json(v): Json<serde_json::Value>,
    ) -> StatusCode {
        if !authed(&h) {
            return StatusCode::UNAUTHORIZED;
        }
        f.beats.lock().unwrap().push((Instant::now(), v));
        if f.fail_beat.load(Ordering::SeqCst) {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::OK
        }
    }
    async fn result(
        State(f): State<Arc<Fake>>,
        h: HeaderMap,
        Path(id): Path<String>,
        Json(v): Json<serde_json::Value>,
    ) -> StatusCode {
        if !authed(&h) {
            return StatusCode::UNAUTHORIZED;
        }
        f.results.lock().unwrap().push((id, v));
        if f.fail_result.load(Ordering::SeqCst) {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::OK
        }
    }
    async fn report(
        State(f): State<Arc<Fake>>,
        h: HeaderMap,
        Json(v): Json<Value>,
    ) -> (StatusCode, Json<Value>) {
        if !authed(&h) {
            return (StatusCode::UNAUTHORIZED, Json(Value::Null));
        }
        let reply = match v["note"].as_str() {
            Some("duplicate") => (StatusCode::OK, Json(json!({"id":21}))),
            Some("rate limited") => (StatusCode::TOO_MANY_REQUESTS, Json(json!({"id":22}))),
            _ => (StatusCode::CREATED, Json(json!({"id":20}))),
        };
        f.reports.lock().unwrap().push(v);
        reply
    }
    async fn attach_replay(
        State(f): State<Arc<Fake>>,
        Path((_match_id, id)): Path<(String, i64)>,
        h: HeaderMap,
        Json(v): Json<Value>,
    ) -> StatusCode {
        if !authed(&h) {
            return StatusCode::UNAUTHORIZED;
        }
        f.report_replays.lock().unwrap().push((id, v));
        StatusCode::NO_CONTENT
    }
    async fn chat(
        State(f): State<Arc<Fake>>,
        Path(_match_id): Path<String>,
        h: HeaderMap,
        Json(v): Json<Value>,
    ) -> StatusCode {
        if !authed(&h) {
            return StatusCode::UNAUTHORIZED;
        }
        f.chat.lock().unwrap().push(v);
        StatusCode::NO_CONTENT
    }
    async fn actions(
        State(f): State<Arc<Fake>>,
        Path(_match_id): Path<String>,
        h: HeaderMap,
    ) -> Result<Json<Vec<Value>>, StatusCode> {
        if !authed(&h) {
            return Err(StatusCode::UNAUTHORIZED);
        }
        Ok(Json(f.actions.lock().unwrap().clone()))
    }
    async fn ack(
        State(f): State<Arc<Fake>>,
        Path((_match_id, id)): Path<(String, i64)>,
        h: HeaderMap,
        Json(v): Json<Value>,
    ) -> StatusCode {
        if !authed(&h) {
            return StatusCode::UNAUTHORIZED;
        }
        f.action_acks.lock().unwrap().push((id, v));
        if f.fail_ack_once
            .compare_exchange(id as u32, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return StatusCode::SERVICE_UNAVAILABLE;
        }
        f.actions
            .lock()
            .unwrap()
            .retain(|action| action["id"] != id);
        StatusCode::NO_CONTENT
    }
    let app = Router::new()
        .route("/internal/matches/register", post(register))
        .route("/internal/matches/{id}/heartbeat", post(beat))
        .route("/internal/matches/{id}/result", post(result))
        .route("/internal/report", post(report))
        .route(
            "/internal/matches/{id}/reports/{report_id}/replay",
            put(attach_replay),
        )
        .route("/internal/matches/{id}/chat", put(chat))
        .route("/internal/matches/{id}/actions", get(actions))
        .route("/internal/matches/{id}/actions/{action_id}/ack", post(ack))
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
        .map(|w| w[1].0.duration_since(w[0].0).as_millis() as i64 - 1000)
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
        score_summary: None,
    };
    GatewayClient::new(&url, TOKEN)
        .unwrap()
        .submit_result("0102030405060708090a0b0c0d0e0f10", &r)
        .await
        .unwrap();
    let got = f.results.lock().unwrap()[0].1.clone();
    assert_eq!(
        (got["players"].as_u64(), got["objectives"].as_u64()),
        (Some(12), Some(5))
    );

    let dead = GatewayClient::new("http://127.0.0.1:1", TOKEN).unwrap();
    assert!(dead.submit_result("abc", &r).await.is_err());
    let dir = tempfile::tempdir().unwrap();
    assert!(spool(dir.path(), "../ab-cd", &r).is_err());
    let p = spool(dir.path(), "0102030405060708090a0b0c0d0e0f10", &r).unwrap();
    assert_eq!(
        p.file_name().unwrap(),
        "0102030405060708090a0b0c0d0e0f10.json"
    );
    let back: MatchResult = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    assert_eq!(back, r);
}
