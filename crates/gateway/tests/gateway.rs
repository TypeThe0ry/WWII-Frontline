//! M1-028 integration: every route through the in-memory axum service with a manual clock.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use ed25519_dalek::SigningKey;
use frontline_gateway::app::{router, AppState, Shared};
use frontline_gateway::auth::ticket::{self, JtiCache};
use frontline_gateway::clock::Clock;
use frontline_gateway::config::Config;
use frontline_gateway::registry::from_hex;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const T0: u64 = 1_800_000_000_000;
const INTERNAL: &str = "internal-test-token";

struct Rig {
    app: Router,
    st: Shared,
    clock: Clock,
}

fn rig() -> Rig {
    let clock = Clock::manual(T0);
    let st = AppState::new(
        Config::test(),
        clock.clone(),
        SigningKey::from_bytes(&[5u8; 32]),
    );
    Rig {
        app: router(st.clone()),
        st,
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
    let (s, _, v) = call(
        &r.app,
        Method::POST,
        "/internal/matches/register",
        "10.9.9.9",
        Some(INTERNAL),
        json!({"name":"S","map":"carentan","mode":"conquest","region":"eu","tags":["m1"],
               "version":"0.1","protocol_version":version,"data_hash":1,"cap":cap,
               "addr":"play.example","ws_port":8443,"wt_port":4433}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    v["match_id"].as_str().unwrap().to_owned()
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
    let r = rig();
    let id = register(&r, 3, 16).await;
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
    let r = rig();
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
async fn ticket_verifies_with_the_public_key_and_is_single_use() {
    let r = rig();
    let id = register(&r, 3, 16).await;
    let (_, g) = guest(&r, "3.3.3.3").await;
    let (s, _, t) = call(
        &r.app,
        Method::POST,
        &format!("/v1/servers/{id}/ticket"),
        "3.3.3.3",
        g["token"].as_str(),
        json!({"nick":"Ｇａｉｕｓ"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(t["expires_in_s"], 60);
    assert_eq!(
        (t["wt_port"].as_u64(), t["ws_port"].as_u64()),
        (Some(4433), Some(8443))
    );
    let pk = SigningKey::from_bytes(&[5u8; 32]).verifying_key();
    let mid = from_hex(&id).unwrap();
    let mut cache = JtiCache::default();
    let now = r.clock.now_s();
    let tk = t["ticket"].as_str().unwrap();
    let c = ticket::verify(&pk, tk, now + 1, &mid, &mut cache).unwrap();
    assert_eq!(c.nick, "Gaius");
    assert_eq!(c.exp, now + 60);
    assert!(ticket::verify(&pk, tk, now + 2, &mid, &mut cache).is_err());
}

#[tokio::test]
async fn ticket_refusals() {
    let r = rig();
    let id = register(&r, 3, 1).await;
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
    let r = rig();
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
    let r = rig();
    let id = register(&r, 3, 100).await;
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
    let r = rig();
    let a = register(&r, 3, 16).await;
    let b = register(&r, 3, 16).await;
    let c = register(&r, 2, 16).await;
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

// ------------------------------------------------------------------ internal API

#[tokio::test]
async fn internal_api_requires_the_token_and_known_ids() {
    let r = rig();
    let id = register(&r, 3, 16).await;
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
    assert_eq!(r.st.registry.results().len(), 1);
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

#[tokio::test]
async fn register_returns_the_ticket_key_fingerprint() {
    let r = rig();
    let (_, _, v) = call(
        &r.app,
        Method::POST,
        "/internal/matches/register",
        "9.9.9.9",
        Some(INTERNAL),
        json!({"name":"S","map":"m","mode":"c","region":"eu","version":"1","protocol_version":3,
               "data_hash":1,"cap":8,"addr":"a","ws_port":1,"wt_port":2}),
    )
    .await;
    assert_eq!(v["ticket_key_fingerprint"].as_str().unwrap().len(), 16);
    assert!(v["ticket_public_key"].as_str().unwrap().len() >= 43);
}

// ------------------------------------------------------------------ headers

#[tokio::test]
async fn responses_are_no_store_and_cors_is_origin_checked() {
    let r = rig();
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
        ]
    );
}
