//! M1-028 integration: every route through the in-memory axum service with a manual clock.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use base64::Engine;
use ed25519_dalek::{SigningKey, VerifyingKey};
use frontline_gateway::app::{router, AppState};
use frontline_gateway::auth::ticket::{self, JtiCache, TicketError};
use frontline_gateway::clock::Clock;
use frontline_gateway::config::Config;
use frontline_gateway::registry::from_hex;
use frontline_protocol::PROTOCOL_VERSION;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

const T0: u64 = 1_800_000_000_000;
const INTERNAL: &str = "internal-test-token";

struct Rig {
    app: Router,
    clock: Clock,
}

async fn rig() -> Rig {
    rig_with(Config::test(), [5; 32]).await
}

async fn rig_with(cfg: Config, key: [u8; 32]) -> Rig {
    let clock = Clock::manual(T0);
    let st = AppState::new(cfg, clock.clone(), SigningKey::from_bytes(&key))
        .await
        .unwrap();
    Rig {
        app: router(st),
        clock,
    }
}

async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    ip: &str,
    auth: Option<&str>,
    body: Value,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut rb = Request::builder()
        .method(method)
        .uri(uri)
        .header("x-forwarded-for", ip)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(a) = auth {
        rb = rb.header(header::AUTHORIZATION, format!("Bearer {a}"));
    }
    let resp = app
        .clone()
        .oneshot(rb.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let (parts, b) = resp.into_parts();
    let bytes = b.collect().await.unwrap().to_bytes();
    let v = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (parts.status, parts.headers, v)
}

async fn guest(r: &Rig, ip: &str) -> (StatusCode, Value) {
    let (s, _, v) = call(&r.app, Method::POST, "/v1/guest", ip, None, json!({})).await;
    (s, v)
}

async fn register(r: &Rig, version: u16, cap: u8) -> String {
    register_response(r, version, cap).await["match_id"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn register_response(r: &Rig, version: u16, cap: u8) -> Value {
    let (s, _, v) = call(
        &r.app,
        Method::POST,
        "/internal/matches/register",
        "10.9.9.9",
        Some(INTERNAL),
        json!({"name":"S","map":"carentan","mode":"conquest","region":"eu","tags":["m1"],
               "version":"0.1","protocol_version":version,"data_hash":u64::MAX,"cap":cap,
               "addr":"play.example","ws_port":8443,"wt_port":4433}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    v
}

async fn beat(r: &Rig, id: &str, humans: u8) -> StatusCode {
    call(
        &r.app,
        Method::POST,
        &format!("/internal/matches/{id}/heartbeat"),
        "10.9.9.9",
        Some(INTERNAL),
        json!({"humans":humans,"bots":0,"phase":"live","time_left_s":600}),
    )
    .await
    .0
}

async fn ticket_req(r: &Rig, id: &str, ip: &str, tok: Option<&str>, nick: &str) -> StatusCode {
    call(
        &r.app,
        Method::POST,
        &format!("/v1/servers/{id}/ticket"),
        ip,
        tok,
        json!({ "nick": nick }),
    )
    .await
    .0
}

// ------------------------------------------------------------------ guest + token

#[tokio::test]
async fn guest_token_is_24h_and_expired_tokens_are_401() {
    let r = rig().await;
    let id = register(&r, PROTOCOL_VERSION, 16).await;
    let (s, g) = guest(&r, "1.1.1.1").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(g["expires_in_s"], 86_400);
    assert_eq!(g["guest_id"].as_str().unwrap().len(), 32);
    let tok = g["token"].as_str().unwrap().to_owned();
    assert_eq!(
        ticket_req(&r, &id, "1.1.1.1", Some(&tok), "Gaius").await,
        StatusCode::OK
    );
    r.clock.advance_ms(86_400_000);
    beat(&r, &id, 0).await;
    assert_eq!(
        ticket_req(&r, &id, "1.1.1.1", Some(&tok), "Gaius").await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        ticket_req(&r, &id, "1.1.1.2", None, "Gaius").await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        ticket_req(&r, &id, "1.1.1.2", Some("a.b.c"), "Gaius").await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn a_valid_token_renews_the_same_guest() {
    let r = rig().await;
    let (_, g) = guest(&r, "2.2.2.2").await;
    r.clock.advance_ms(3_600_000);
    let (_, _, g2) = call(
        &r.app,
        Method::POST,
        "/v1/guest",
        "2.2.2.2",
        None,
        json!({"token": g["token"]}),
    )
    .await;
    assert_eq!(g["guest_id"], g2["guest_id"]);
    assert_ne!(g["token"], g2["token"]);
}

// ------------------------------------------------------------------ ticket

#[tokio::test]
async fn ticket_verifies_with_the_registered_match_identity_and_is_single_use() {
    let r = rig().await;
    let registration = register_response(&r, PROTOCOL_VERSION, 16).await;
    let id = registration["match_id"].as_str().unwrap();
    let other_id = register(&r, PROTOCOL_VERSION, 16).await;
    let (_, g) = guest(&r, "3.3.3.3").await;
    let (s, _, t) = call(
        &r.app,
        Method::POST,
        &format!("/v1/servers/{id}/ticket"),
        "3.3.3.3",
        g["token"].as_str(),
        json!({"nick":"Ｇａｉｕｓ", "player_id":127, "is_admin":true, "match_id":other_id}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(t["expires_in_s"], 60);
    assert_eq!(
        (t["wt_port"].as_u64(), t["ws_port"].as_u64()),
        (Some(4433), Some(8443))
    );
    let key_bytes: [u8; 32] = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(registration["ticket_public_key"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    let fingerprint: String = Sha256::digest(key_bytes)[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(registration["ticket_key_fingerprint"], fingerprint);
    let pk = VerifyingKey::from_bytes(&key_bytes).unwrap();
    let mid = from_hex(id).unwrap();
    let mut cache = JtiCache::default();
    let now = r.clock.now_s();
    let tk = t["ticket"].as_str().unwrap();
    assert_eq!(
        ticket::verify(&pk, tk, now + 1, &from_hex(&other_id).unwrap(), &mut cache),
        Err(TicketError::WrongMatch)
    );
    let c = ticket::verify(&pk, tk, now + 1, &mid, &mut cache).unwrap();
    assert_eq!(c.match_id, mid);
    assert_eq!(c.player_id, 0);
    assert_eq!(c.nick, "Gaius");
    assert!(!c.is_admin);
    assert_eq!(c.exp, now + 60);
    assert_eq!(
        ticket::verify(&pk, tk, now + 2, &mid, &mut cache),
        Err(TicketError::Replayed)
    );
}

#[tokio::test]
async fn ticket_refusals() {
    let r = rig().await;
    let id = register(&r, PROTOCOL_VERSION, 1).await;
    let (_, g) = guest(&r, "4.4.4.4").await;
    let tok = g["token"].as_str().unwrap();
    assert_eq!(
        ticket_req(&r, &id, "4.4.4.4", Some(tok), "ab").await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        ticket_req(&r, &"0".repeat(32), "4.4.4.4", Some(tok), "Gaius").await,
        StatusCode::NOT_FOUND
    );
    // Full: one human on a cap-1 server.
    beat(&r, &id, 1).await;
    assert_eq!(
        ticket_req(&r, &id, "4.4.4.4", Some(tok), "Gaius").await,
        StatusCode::CONFLICT
    );
    // Stale: no heartbeat for 6 s.
    beat(&r, &id, 0).await;
    r.clock.advance_ms(6_000);
    assert_eq!(
        ticket_req(&r, &id, "4.4.4.4", Some(tok), "Gaius").await,
        StatusCode::GONE
    );
}

// ------------------------------------------------------------------ rate limits

#[tokio::test]
async fn ip_limit_is_10_per_minute_across_login_and_ticket() {
    let r = rig().await;
    for i in 0..10 {
        let (s, _) = guest(&r, "5.5.5.5").await;
        assert_eq!(s, StatusCode::OK, "request {i}");
    }
    let (s, h, _) = call(
        &r.app,
        Method::POST,
        "/v1/guest",
        "5.5.5.5",
        None,
        json!({}),
    )
    .await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
    assert!(h.get(header::RETRY_AFTER).is_some());
    assert_eq!(
        guest(&r, "5.5.5.6").await.0,
        StatusCode::OK,
        "another IP is unaffected"
    );
    r.clock.advance_ms(61_000);
    assert_eq!(
        guest(&r, "5.5.5.5").await.0,
        StatusCode::OK,
        "window passed"
    );
}

#[tokio::test]
async fn account_limit_is_6_tickets_per_minute() {
    let r = rig().await;
    let id = register(&r, PROTOCOL_VERSION, 100).await;
    let (_, g) = guest(&r, "6.0.0.0").await;
    let tok = g["token"].as_str().unwrap();
    for i in 0..6 {
        // A fresh IP each time so only the account limiter can fire.
        let s = ticket_req(&r, &id, &format!("6.0.0.{}", i + 1), Some(tok), "Gaius").await;
        assert_eq!(s, StatusCode::OK, "ticket {i}");
    }
    assert_eq!(
        ticket_req(&r, &id, "6.0.0.9", Some(tok), "Gaius").await,
        StatusCode::TOO_MANY_REQUESTS
    );
    r.clock.advance_ms(61_000);
    beat(&r, &id, 0).await;
    assert_eq!(
        ticket_req(&r, &id, "6.0.0.9", Some(tok), "Gaius").await,
        StatusCode::OK
    );
}

// ------------------------------------------------------------------ server list

#[tokio::test]
async fn list_hides_servers_without_a_heartbeat_and_flags_old_versions() {
    let r = rig().await;
    let a = register(&r, PROTOCOL_VERSION, 16).await;
    let b = register(&r, PROTOCOL_VERSION, 16).await;
    let c = register(&r, PROTOCOL_VERSION - 1, 16).await;
    r.clock.advance_ms(3_000);
    beat(&r, &a, 4).await;
    beat(&r, &c, 1).await;
    r.clock.advance_ms(3_000); // b silent for 6 s, a and c for 3 s
    let (s, _, v) = call(
        &r.app,
        Method::GET,
        "/v1/servers",
        "7.7.7.7",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    let find = |id: &str| arr.iter().find(|x| x["match_id"] == id).cloned();
    assert!(find(&b).is_none());
    assert_eq!(find(&a).unwrap()["compatible"], true);
    assert_eq!(find(&a).unwrap()["humans"], 4);
    assert_eq!(find(&c).unwrap()["compatible"], false);
}

#[tokio::test]
async fn ticket_refuses_an_incompatible_registration() {
    let r = rig().await;
    let id = register(&r, PROTOCOL_VERSION - 1, 16).await;
    let (_, g) = guest(&r, "7.7.7.7").await;
    let token = g["token"].as_str().unwrap();
    let (status, _, body) = call(
        &r.app,
        Method::POST,
        &format!("/v1/servers/{id}/ticket"),
        "7.7.7.7",
        Some(token),
        json!({"nick":"Refused Nick"}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body, json!({"error":"protocol_mismatch"}));

    let (status, _, renewed) = call(
        &r.app,
        Method::POST,
        "/v1/guest",
        "7.7.7.7",
        None,
        json!({"token":token}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(renewed["nick_suggestion"], g["nick_suggestion"]);
}

// ------------------------------------------------------------------ internal API

#[tokio::test]
async fn internal_api_requires_the_token_and_known_ids() {
    let r = rig().await;
    let id = register(&r, PROTOCOL_VERSION, 16).await;
    let hb = json!({"humans":1,"bots":0,"phase":"live","time_left_s":1});
    let uri = format!("/internal/matches/{id}/heartbeat");
    let post = |uri: String, auth: Option<&'static str>, body: Value| {
        let app = r.app.clone();
        async move {
            call(&app, Method::POST, &uri, "8.8.8.8", auth, body)
                .await
                .0
        }
    };
    assert_eq!(
        post(uri.clone(), None, hb.clone()).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        post(uri.clone(), Some("wrong"), hb.clone()).await,
        StatusCode::UNAUTHORIZED
    );
    let rnd = format!("/internal/matches/{}/heartbeat", "ab".repeat(16));
    assert_eq!(
        post(rnd, Some(INTERNAL), hb.clone()).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(post(uri, Some(INTERNAL), hb).await, StatusCode::NO_CONTENT);
    // Result: stored, ends the match.
    let res = format!("/internal/matches/{id}/result");
    assert_eq!(
        post(res.clone(), None, json!({})).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        post(res, Some(INTERNAL), json!({"winner":"us"})).await,
        StatusCode::NO_CONTENT
    );
    let (_, _, v) = call(
        &r.app,
        Method::GET,
        "/v1/servers",
        "8.8.8.8",
        None,
        Value::Null,
    )
    .await;
    assert!(
        v.as_array().unwrap().is_empty(),
        "ended matches leave the list"
    );
    assert_eq!(
        post("/internal/matches/register".into(), None, json!({})).await,
        StatusCode::UNAUTHORIZED
    );
}

// ------------------------------------------------------------------ headers

#[tokio::test]
async fn responses_are_no_store_and_cors_is_origin_checked() {
    let r = rig().await;
    let send = |origin: &'static str| {
        let app = r.app.clone();
        async move {
            app.oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/v1/guest")
                    .header(header::ORIGIN, origin)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
        }
    };
    let good = send("https://play.example").await;
    assert_eq!(good.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        good.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        "https://play.example"
    );
    let evil = send("https://evil.example").await;
    assert!(evil
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .is_none());
}

#[test]
fn route_table_snapshot() {
    assert_eq!(
        frontline_gateway::routes::ROUTE_TABLE,
        &[
            "POST /v1/guest",
            "GET /v1/servers",
            "POST /v1/servers/{match_id}/ticket",
            "POST /internal/matches/register",
            "POST /internal/matches/{id}/heartbeat",
            "POST /internal/matches/{id}/result",
            "GET /healthz",
            "GET /metrics",
        ]
    );
}

#[tokio::test]
async fn sqlite_reopen_preserves_guest_match_ticket_cursor_and_first_result() {
    use sqlx::{Connection, SqliteConnection};

    let dir = tempfile::tempdir().unwrap();
    let cfg = Config {
        database_url: format!("sqlite://{}", dir.path().join("gateway.db").display()),
        ..Config::test()
    };
    let r = rig_with(cfg.clone(), [5; 32]).await;
    let live = register(&r, PROTOCOL_VERSION, 16).await;
    let ended = register(&r, PROTOCOL_VERSION, 16).await;
    assert_eq!(beat(&r, &live, 4).await, StatusCode::NO_CONTENT);
    let (_, profile) = guest(&r, "1.2.3.4").await;
    let token = profile["token"].as_str().unwrap();
    let uri = format!("/v1/servers/{live}/ticket");
    let match_id = from_hex(&live).unwrap();
    let issue = |app: Router| {
        let uri = uri.clone();
        async move {
            let (status, _, response) = call(
                &app,
                Method::POST,
                &uri,
                "1.2.3.4",
                Some(token),
                json!({"nick":"①ﬁＡ"}),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            ticket::verify(
                &SigningKey::from_bytes(&[5; 32]).verifying_key(),
                response["ticket"].as_str().unwrap(),
                T0 / 1000,
                &match_id,
                &mut JtiCache::default(),
            )
            .unwrap()
        }
    };
    let first = issue(r.app.clone()).await;
    assert_eq!(first.player_id, 0);
    assert_eq!(first.nick, "1fiA");
    let res = format!("/internal/matches/{ended}/result");
    assert_eq!(
        call(
            &r.app,
            Method::POST,
            &res,
            "1.2.3.4",
            Some(INTERNAL),
            json!({"winner":1})
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    drop(r);

    let r = rig_with(cfg.clone(), [5; 32]).await;
    let (_, _, renewed) = call(
        &r.app,
        Method::POST,
        "/v1/guest",
        "1.2.3.4",
        None,
        json!({"token":token}),
    )
    .await;
    assert_eq!(renewed["guest_id"], profile["guest_id"]);
    assert_eq!(renewed["nick_suggestion"], "1fiA");
    let (_, _, listed) = call(
        &r.app,
        Method::GET,
        "/v1/servers",
        "1.2.3.4",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["match_id"], live);
    assert_eq!(listed[0]["humans"], 4);
    assert_eq!(issue(r.app.clone()).await.player_id, 1);
    assert_eq!(beat(&r, &ended, 4).await, StatusCode::NOT_FOUND);
    assert_eq!(
        call(
            &r.app,
            Method::POST,
            &res,
            "1.2.3.4",
            Some(INTERNAL),
            json!({"winner":2})
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );

    let mut conn = SqliteConnection::connect(&cfg.database_url).await.unwrap();
    let payloads: Vec<String> = sqlx::query_scalar("SELECT payload FROM results")
        .fetch_all(&mut conn)
        .await
        .unwrap();
    assert_eq!(payloads, [r#"{"winner":1}"#]);
    let hash: String = sqlx::query_scalar("SELECT registration FROM matches WHERE id = ?")
        .bind(&live)
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&hash).unwrap()["data_hash"],
        u64::MAX
    );

    let rotated = rig_with(cfg, [7; 32]).await;
    assert_eq!(beat(&rotated, &live, 4).await, StatusCode::NOT_FOUND);
    assert_eq!(
        ticket_req(&rotated, &live, "1.2.3.4", Some(token), "Guest").await,
        StatusCode::GONE
    );
}

#[tokio::test]
async fn prometheus_counts_bounded_routes_auth_failures_and_storage_errors() {
    use sqlx::{Connection, SqliteConnection};

    let dir = tempfile::tempdir().unwrap();
    let cfg = Config {
        database_url: format!("sqlite://{}", dir.path().join("gateway.db").display()),
        ..Config::test()
    };
    let r = rig_with(cfg.clone(), [5; 32]).await;
    let id = register(&r, PROTOCOL_VERSION, 16).await;
    assert_eq!(
        ticket_req(&r, &id, "1.2.3.4", Some("bad-fixture-token"), "Guest").await,
        StatusCode::UNAUTHORIZED
    );
    let (status, _, profile) = call(
        &r.app,
        Method::POST,
        "/v1/guest",
        "1.2.3.4",
        None,
        json!({"token":"bad-fixture-token"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = profile["token"].as_str().unwrap();
    assert_eq!(
        ticket_req(&r, &id, "1.2.3.4", Some(token), "Guest").await,
        StatusCode::OK
    );
    let mut conn = SqliteConnection::connect(&cfg.database_url).await.unwrap();
    sqlx::query("DROP TABLE guests")
        .execute(&mut conn)
        .await
        .unwrap();
    let (status, _, response) = call(
        &r.app,
        Method::POST,
        "/v1/guest",
        "1.2.3.4",
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(response, json!({"error":"storage_unavailable"}));
    assert_eq!(
        ticket_req(&r, &id, "1.2.3.4", Some(token), "Guest").await,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let next_player: u8 = sqlx::query_scalar("SELECT next_player FROM matches WHERE id = ?")
        .bind(&id)
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(
        next_player, 1,
        "the failed guest write rolls the cursor back"
    );

    let resp = r
        .app
        .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers()[header::CONTENT_TYPE]
        .to_str()
        .unwrap()
        .starts_with("text/plain"));
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let text = std::str::from_utf8(&body).unwrap();
    for expected in [
        "gateway_http_requests_total{route=\"/v1/servers/{match_id}/ticket\",status_class=\"4xx\"} 1",
        "gateway_http_requests_total{route=\"/v1/guest\",status_class=\"5xx\"} 1",
        "gateway_http_request_duration_seconds_count{route=\"/v1/guest\"} 2",
        "gateway_ticket_storage_duration_seconds_count{stage=\"pool_wait\"} 2",
        "gateway_ticket_storage_duration_seconds_count{stage=\"transaction\"} 2",
        "gateway_auth_attempts_total{operation=\"ticket\",result=\"invalid\"} 1",
        "gateway_auth_attempts_total{operation=\"guest_renewal\",result=\"invalid\"} 1",
    ] {
        assert!(text.lines().any(|line| line == expected), "missing {expected} in {text}");
    }
    assert!(!text.contains(&id));
    assert!(!text.contains("bad-fixture-token"));
}

#[tokio::test]
async fn request_logs_do_not_contain_credentials() {
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().write(bytes)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let sink = Sink(Arc::default());
    let writer = sink.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(move || writer.clone())
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let r = rig().await;
    let id = register(&r, PROTOCOL_VERSION, 16).await;
    let (_, profile) = guest(&r, "1.2.3.4").await;
    let token = profile["token"].as_str().unwrap();
    let (status, _, response) = call(
        &r.app,
        Method::POST,
        &format!("/v1/servers/{id}/ticket"),
        "1.2.3.4",
        Some(token),
        json!({"nick":"Guest"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let log = sink.0.lock().unwrap();
    let log = std::str::from_utf8(&log).unwrap();
    assert!(log.contains("gateway request"));
    for secret in [token, response["ticket"].as_str().unwrap(), INTERNAL] {
        assert!(!log.contains(secret), "credential leaked in request logs");
    }
}
