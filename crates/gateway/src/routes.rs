//! HTTP routes (the table below is snapshot-tested):
//!
//! ```text
//! POST /v1/guest                         guest id + session token (optional renewal)
//! GET  /v1/servers                       live servers
//! POST /v1/servers/{match_id}/ticket     join ticket (Bearer session token)
//! GET  /v1/matches/{match_id}/result     stored match result (Bearer session token)
//! GET  /v1/me/settings                   read settings (Bearer session token)
//! PUT  /v1/me/settings                   replace settings (Bearer session token)
//! GET  /v1/oauth/{provider}/start        OAuth reservation (disabled by default)
//! GET  /v1/oauth/{provider}/callback     OAuth reservation (disabled by default)
//! POST /internal/matches/register        match server registers (Bearer internal token)
//! POST /internal/matches/{id}/heartbeat  match server heartbeat
//! POST /internal/matches/{id}/result     match result
//! GET  /healthz                          liveness
//! GET  /metrics                          Prometheus exposition
//! ```

use std::net::SocketAddr;
use std::time::Instant;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, DefaultBodyLimit, MatchedPath, Path, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use base64::Engine;
use ed25519_dalek::Signer;
use frontline_protocol::dto::TicketClaims;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::app::Shared;
use crate::auth::{ct_eq, ticket, token};
use crate::db::{Registration, SlotError};
use crate::nick::validate_nick;
use crate::registry::{from_hex, to_hex};

pub const ROUTE_TABLE: &[&str] = &[
    "POST /v1/guest",
    "GET /v1/servers",
    "POST /v1/servers/{match_id}/ticket",
    "GET /v1/matches/{match_id}/result",
    "GET /v1/me/settings",
    "PUT /v1/me/settings",
    "GET /v1/oauth/{provider}/start",
    "GET /v1/oauth/{provider}/callback",
    "POST /internal/matches/register",
    "POST /internal/matches/{id}/heartbeat",
    "POST /internal/matches/{id}/result",
    "GET /healthz",
    "GET /metrics",
];

const TICKET_PATH: &str = "/v1/servers/{match_id}/ticket";
const SETTINGS_MAX_BYTES: usize = 16 * 1024;

#[derive(Clone)]
pub struct ClientIp(pub String);

pub fn router(state: Shared) -> Router {
    Router::new()
        .route("/v1/guest", post(guest))
        .route("/v1/servers", get(servers))
        .route(TICKET_PATH, post(issue_ticket))
        .route("/v1/matches/{match_id}/result", get(match_result))
        .route(
            "/v1/me/settings",
            get(get_settings)
                .put(put_settings)
                .layer(DefaultBodyLimit::max(SETTINGS_MAX_BYTES)),
        )
        .route("/v1/oauth/{provider}/start", get(oauth_placeholder))
        .route("/v1/oauth/{provider}/callback", get(oauth_placeholder))
        .route("/internal/matches/register", post(register))
        .route("/internal/matches/{id}/heartbeat", post(heartbeat))
        .route("/internal/matches/{id}/result", post(result))
        .route("/healthz", get(|| async { "ok" }))
        .route("/metrics", get(metrics))
        .layer(middleware::from_fn_with_state(state.clone(), edge))
        .with_state(state)
}

/// Client IP, `Cache-Control: no-store`, CORS for the configured origins only.
async fn edge(State(st): State<Shared>, mut req: Request, next: Next) -> Response {
    let started = Instant::now();
    let route = req.extensions().get::<MatchedPath>().cloned();
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip().to_string());
    let fwd = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_owned());
    let ip = if st.cfg.trust_forwarded {
        fwd.or(peer)
    } else {
        peer
    }
    .unwrap_or_else(|| "unknown".into());
    req.extensions_mut().insert(ClientIp(ip));
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let preflight = req.method() == axum::http::Method::OPTIONS;
    let mut resp = if preflight {
        StatusCode::NO_CONTENT.into_response()
    } else {
        next.run(req).await
    };
    let h = resp.headers_mut();
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Some(o) = origin {
        if st.cfg.allowed_origins.contains(&o) {
            if let Ok(v) = HeaderValue::from_str(&o) {
                h.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, v);
                h.insert(header::VARY, HeaderValue::from_static("Origin"));
                h.insert(
                    header::ACCESS_CONTROL_ALLOW_HEADERS,
                    HeaderValue::from_static("authorization, content-type"),
                );
                h.insert(
                    header::ACCESS_CONTROL_ALLOW_METHODS,
                    HeaderValue::from_static("GET, POST, PUT, OPTIONS"),
                );
            }
        }
    }
    let route = route.as_ref().map_or("unmatched", MatchedPath::as_str);
    let elapsed = started.elapsed();
    let status = resp.status().as_u16();
    if route != "/metrics" {
        st.metrics.request(route, status, elapsed.as_secs_f64());
        let log_timer =
            (route == TICKET_PATH && status == 200).then(|| st.metrics.ticket_log.start_timer());
        tracing::debug!(
            route,
            status,
            elapsed_us = elapsed.as_micros(),
            "gateway request"
        );
        drop(log_timer);
    }
    resp
}

async fn metrics(State(st): State<Shared>) -> Response {
    match st.metrics.encode() {
        Ok((content_type, body)) => ([(header::CONTENT_TYPE, content_type)], body).into_response(),
        Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "metrics_unavailable"),
    }
}

fn db_error(error: &sqlx::Error) -> Response {
    let kind = match error {
        sqlx::Error::PoolTimedOut => "pool_timeout",
        sqlx::Error::Database(_) => "sqlite",
        _ => "storage",
    };
    tracing::error!(kind, "gateway database operation failed");
    err(StatusCode::INTERNAL_SERVER_ERROR, "storage_unavailable")
}

fn err(code: StatusCode, msg: &'static str) -> Response {
    (code, Json(json!({ "error": msg }))).into_response()
}

fn too_many(retry_after_s: u64) -> Response {
    let mut r = err(StatusCode::TOO_MANY_REQUESTS, "rate_limited");
    if let Ok(v) = HeaderValue::from_str(&retry_after_s.to_string()) {
        r.headers_mut().insert(header::RETRY_AFTER, v);
    }
    r
}

fn bearer(h: &HeaderMap) -> Option<&str> {
    h.get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

fn guest_session(st: &Shared, headers: &HeaderMap) -> Option<token::Claims> {
    token::verify(&st.cfg.token_secret, bearer(headers)?, st.clock.now_s())
        .ok()
        .filter(|claims| from_hex(&claims.sub).is_some())
}

fn guest_nick(id: &str) -> String {
    format!("Guest-{}", &id[..4])
}

fn random_16() -> [u8; 16] {
    let mut b = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut b);
    b
}

// ------------------------------------------------------------------ guest

#[derive(Deserialize, Default)]
struct GuestReq {
    token: Option<String>,
}

#[derive(Serialize)]
struct GuestResp {
    guest_id: String,
    token: String,
    nick_suggestion: String,
    expires_in_s: u64,
}

async fn guest(
    State(st): State<Shared>,
    Extension(ip): Extension<ClientIp>,
    body: Bytes,
) -> Response {
    let now = st.clock.now_ms();
    if let Err(r) = st.ip_limiter.check(&ip.0, now) {
        st.metrics.auth("guest_login", "rate_limited");
        return too_many(r);
    }
    let req: GuestReq = serde_json::from_slice(&body).unwrap_or_default();
    // Invalid renewal still creates a new guest, but counts as an authentication failure.
    let renewed = req.token.as_deref().and_then(|t| {
        token::verify(&st.cfg.token_secret, t, now / 1000)
            .ok()
            .filter(|claims| from_hex(&claims.sub).is_some())
    });
    if req.token.is_some() {
        st.metrics.auth(
            "guest_renewal",
            if renewed.is_some() { "ok" } else { "invalid" },
        );
    }
    let guest_id = renewed.map_or_else(|| to_hex(&random_16()), |c| c.sub);
    let suggestion = guest_nick(&guest_id);
    let nick_suggestion = match st.db.guest(&guest_id, &suggestion).await {
        Ok(nick) => nick,
        Err(error) => {
            st.metrics.auth("guest_login", "storage_error");
            return db_error(&error);
        }
    };
    st.metrics.auth("guest_login", "ok");
    let tok = token::issue(&st.cfg.token_secret, &guest_id, now / 1000);
    Json(GuestResp {
        nick_suggestion,
        guest_id,
        token: tok,
        expires_in_s: token::TOKEN_TTL_S,
    })
    .into_response()
}

// ------------------------------------------------------------------ M2 player settings, match results, and reserved OAuth

async fn get_settings(State(st): State<Shared>, headers: HeaderMap) -> Response {
    let Some(claims) = guest_session(&st, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    match st.db.settings(&claims.sub).await {
        Ok(settings) => Json(settings).into_response(),
        Err(error) => db_error(&error),
    }
}

async fn put_settings(State(st): State<Shared>, headers: HeaderMap, body: Bytes) -> Response {
    let Some(claims) = guest_session(&st, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    let Ok(settings) = serde_json::from_slice::<Map<String, Value>>(&body) else {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body");
    };
    match st
        .db
        .replace_settings(&claims.sub, &guest_nick(&claims.sub), &settings)
        .await
    {
        Ok(()) => Json(settings).into_response(),
        Err(error) => db_error(&error),
    }
}

async fn match_result(
    State(st): State<Shared>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if guest_session(&st, &headers).is_none() {
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let Some(id) = from_hex(&id) else {
        return err(StatusCode::NOT_FOUND, "no_such_result");
    };
    match st.db.match_result(&id).await {
        Ok(Some(result)) => Json(result).into_response(),
        Ok(None) => err(StatusCode::NOT_FOUND, "no_such_result"),
        Err(error) => db_error(&error),
    }
}

async fn oauth_placeholder(State(st): State<Shared>, Path(provider): Path<String>) -> Response {
    if !st.cfg.oauth_enabled || !matches!(provider.as_str(), "discord" | "google") {
        return err(StatusCode::NOT_FOUND, "not_found");
    }
    err(StatusCode::NOT_IMPLEMENTED, "oauth_not_implemented")
}

// ------------------------------------------------------------------ server list

#[derive(Serialize)]
struct ServerJson {
    match_id: String,
    name: String,
    map: String,
    mode: String,
    phase: String,
    time_left_s: u32,
    humans: u8,
    bots: u8,
    cap: u8,
    region: String,
    tags: Vec<String>,
    version: String,
    compatible: bool,
}

async fn servers(State(st): State<Shared>) -> Response {
    let list = match st
        .db
        .list(
            st.clock.now_ms(),
            st.cfg.heartbeat_timeout_ms,
            st.cfg.protocol_version,
        )
        .await
    {
        Ok(list) => list,
        Err(error) => return db_error(&error),
    };
    Json(
        list.into_iter()
            .map(|s| ServerJson {
                match_id: to_hex(&s.match_id),
                name: s.name,
                map: s.map,
                mode: s.mode,
                phase: s.phase,
                time_left_s: s.time_left_s,
                humans: s.humans,
                bots: s.bots,
                cap: s.cap,
                region: s.region,
                tags: s.tags,
                version: s.version,
                compatible: s.compatible,
            })
            .collect::<Vec<_>>(),
    )
    .into_response()
}

// ------------------------------------------------------------------ ticket

#[derive(Deserialize)]
struct TicketReq {
    nick: String,
}

#[derive(Serialize)]
struct TicketResp {
    ticket: String,
    addr: String,
    wt_port: u16,
    ws_port: u16,
    expires_in_s: u64,
}

async fn issue_ticket(
    State(st): State<Shared>,
    Extension(ip): Extension<ClientIp>,
    Path(match_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let auth_timer = st.metrics.ticket_auth.start_timer();
    let now_ms = st.clock.now_ms();
    if let Err(r) = st.ip_limiter.check(&ip.0, now_ms) {
        return too_many(r);
    }
    let Some(claims) = bearer(&headers)
        .and_then(|t| token::verify(&st.cfg.token_secret, t, now_ms / 1000).ok())
        .filter(|claims| from_hex(&claims.sub).is_some())
    else {
        st.metrics.auth("ticket", "invalid");
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    st.metrics.auth("ticket", "ok");
    if let Err(r) = st.account_limiter.check(&claims.sub, now_ms) {
        return too_many(r);
    }
    auth_timer.observe_duration();
    let parse_timer = st.metrics.ticket_parse.start_timer();
    let Ok(req) = serde_json::from_slice::<TicketReq>(&body) else {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body");
    };
    let Ok(nick) = validate_nick(&req.nick) else {
        return err(StatusCode::BAD_REQUEST, "bad_nick");
    };
    let Some(id) = from_hex(&match_id) else {
        return err(StatusCode::NOT_FOUND, "no_such_match");
    };
    parse_timer.observe_duration();
    let (m, player_id) = match st
        .db
        .reserve_slot(
            &id,
            now_ms,
            st.cfg.heartbeat_timeout_ms,
            &claims.sub,
            &nick,
            &st.metrics,
        )
        .await
    {
        Ok(x) => x,
        Err(SlotError::Unknown) => return err(StatusCode::NOT_FOUND, "no_such_match"),
        Err(SlotError::Stale) => return err(StatusCode::GONE, "match_unavailable"),
        Err(SlotError::Incompatible) => return err(StatusCode::CONFLICT, "protocol_mismatch"),
        Err(SlotError::Full) => return err(StatusCode::CONFLICT, "match_full"),
        Err(SlotError::Database(error)) => return db_error(&error),
    };
    let claims = TicketClaims {
        player_id,
        match_id: id,
        nick,
        exp: 0,
        jti: random_16(),
        is_admin: false,
    };
    let mut sign_elapsed = std::time::Duration::ZERO;
    let serialization_started = Instant::now();
    let t = ticket::issue_with_signer(claims, now_ms / 1000, |message| {
        let started = Instant::now();
        let sig = st.ticket_key.sign(message);
        sign_elapsed = started.elapsed();
        sig
    });
    let resp = Json(TicketResp {
        ticket: t,
        addr: m.addr,
        wt_port: m.wt_port,
        ws_port: m.ws_port,
        expires_in_s: frontline_protocol::dto::TICKET_TTL_S,
    })
    .into_response();
    st.metrics.ticket_serialization.observe(
        serialization_started
            .elapsed()
            .saturating_sub(sign_elapsed)
            .as_secs_f64(),
    );
    st.metrics.ticket_sign.observe(sign_elapsed.as_secs_f64());
    resp
}

// ------------------------------------------------------------------ internal

fn internal_ok(st: &Shared, h: &HeaderMap) -> bool {
    bearer(h).is_some_and(|t| ct_eq(t.as_bytes(), st.cfg.internal_token.as_bytes()))
}

async fn register(State(st): State<Shared>, headers: HeaderMap, body: Bytes) -> Response {
    if !internal_ok(&st, &headers) {
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let Ok(r) = serde_json::from_slice::<Registration>(&body) else {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body");
    };
    let id = random_16();
    if let Err(error) = st.db.register(&id, r, st.clock.now_ms()).await {
        return db_error(&error);
    }
    let pk = st.ticket_key.verifying_key();
    let fp = Sha256::digest(pk.as_bytes());
    Json(json!({
        "match_id": to_hex(&id),
        "ticket_public_key": crate::auth::b64().encode(pk.as_bytes()),
        "ticket_key_fingerprint": fp.iter().take(8).map(|b| format!("{b:02x}")).collect::<String>(),
    }))
    .into_response()
}

#[derive(Deserialize)]
struct HeartbeatReq {
    humans: u8,
    bots: u8,
    phase: String,
    time_left_s: u32,
}

async fn heartbeat(
    State(st): State<Shared>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !internal_ok(&st, &headers) {
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let Ok(r) = serde_json::from_slice::<HeartbeatReq>(&body) else {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body");
    };
    let Some(id) = from_hex(&id) else {
        return err(StatusCode::NOT_FOUND, "no_such_match");
    };
    match st
        .db
        .heartbeat(
            &id,
            st.clock.now_ms(),
            r.humans,
            r.bots,
            &r.phase,
            r.time_left_s,
        )
        .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => err(StatusCode::NOT_FOUND, "no_such_match"),
        Err(error) => db_error(&error),
    }
}

async fn result(
    State(st): State<Shared>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !internal_ok(&st, &headers) {
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let Ok(result) = serde_json::from_slice::<Value>(&body) else {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body");
    };
    let Some(id) = from_hex(&id) else {
        return err(StatusCode::NOT_FOUND, "no_such_match");
    };
    match st.db.store_result(&id, result, st.clock.now_ms()).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => err(StatusCode::NOT_FOUND, "no_such_match"),
        Err(error) => db_error(&error),
    }
}
