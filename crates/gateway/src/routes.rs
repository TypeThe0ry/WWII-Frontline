//! HTTP routes (the table below is snapshot-tested):
//!
//! ```text
//! POST /v1/guest                         guest id + session token (optional renewal)
//! GET  /v1/servers                       live servers
//! POST /v1/servers/{match_id}/ticket     join ticket (Bearer session token)
//! POST /internal/matches/register        match server registers (Bearer internal token)
//! POST /internal/matches/{id}/heartbeat  match server heartbeat
//! POST /internal/matches/{id}/result     match result
//! GET  /healthz                          liveness
//! ```

use std::net::SocketAddr;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use base64::Engine;
use frontline_protocol::dto::TicketClaims;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::app::Shared;
use crate::auth::{ct_eq, ticket, token};
use crate::nick::validate_nick;
use crate::registry::{from_hex, to_hex, MatchEntry, SlotError};

pub const ROUTE_TABLE: &[&str] = &[
    "POST /v1/guest",
    "GET /v1/servers",
    "POST /v1/servers/{match_id}/ticket",
    "POST /internal/matches/register",
    "POST /internal/matches/{id}/heartbeat",
    "POST /internal/matches/{id}/result",
    "GET /healthz",
];

#[derive(Clone)]
pub struct ClientIp(pub String);

pub fn router(state: Shared) -> Router {
    Router::new()
        .route("/v1/guest", post(guest))
        .route("/v1/servers", get(servers))
        .route("/v1/servers/{match_id}/ticket", post(issue_ticket))
        .route("/internal/matches/register", post(register))
        .route("/internal/matches/{id}/heartbeat", post(heartbeat))
        .route("/internal/matches/{id}/result", post(result))
        .route("/healthz", get(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(state.clone(), edge))
        .with_state(state)
}

/// Client IP, `Cache-Control: no-store`, CORS for the configured origins only.
async fn edge(State(st): State<Shared>, mut req: Request, next: Next) -> Response {
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
                    HeaderValue::from_static("GET, POST, OPTIONS"),
                );
            }
        }
    }
    resp
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
        return too_many(r);
    }
    let req: GuestReq = serde_json::from_slice(&body).unwrap_or_default();
    // A valid old token renews the same guest; anything else makes a new one.
    let guest_id = req
        .token
        .as_deref()
        .and_then(|t| token::verify(&st.cfg.token_secret, t, now / 1000).ok())
        .map_or_else(|| to_hex(&random_16()), |c| c.sub);
    let tok = token::issue(&st.cfg.token_secret, &guest_id, now / 1000);
    Json(GuestResp {
        nick_suggestion: format!("Guest-{}", &guest_id[..4]),
        guest_id,
        token: tok,
        expires_in_s: token::TOKEN_TTL_S,
    })
    .into_response()
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
    let list = st.registry.list(
        st.clock.now_ms(),
        st.cfg.heartbeat_timeout_ms,
        st.cfg.protocol_version,
    );
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
    let now_ms = st.clock.now_ms();
    if let Err(r) = st.ip_limiter.check(&ip.0, now_ms) {
        return too_many(r);
    }
    let Some(claims) =
        bearer(&headers).and_then(|t| token::verify(&st.cfg.token_secret, t, now_ms / 1000).ok())
    else {
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    if let Err(r) = st.account_limiter.check(&claims.sub, now_ms) {
        return too_many(r);
    }
    let Ok(req) = serde_json::from_slice::<TicketReq>(&body) else {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body");
    };
    let Ok(nick) = validate_nick(&req.nick) else {
        return err(StatusCode::BAD_REQUEST, "bad_nick");
    };
    let Some(id) = from_hex(&match_id) else {
        return err(StatusCode::NOT_FOUND, "no_such_match");
    };
    let (m, player_id) = match st
        .registry
        .reserve_slot(&id, now_ms, st.cfg.heartbeat_timeout_ms)
    {
        Ok(x) => x,
        Err(SlotError::Unknown) => return err(StatusCode::NOT_FOUND, "no_such_match"),
        Err(SlotError::Stale) => return err(StatusCode::GONE, "match_unavailable"),
        Err(SlotError::Full) => return err(StatusCode::CONFLICT, "match_full"),
    };
    let t = ticket::issue(
        &st.ticket_key,
        TicketClaims {
            player_id,
            match_id: id,
            nick,
            exp: 0,
            jti: random_16(),
            is_admin: false,
        },
        now_ms / 1000,
    );
    Json(TicketResp {
        ticket: t,
        addr: m.addr,
        wt_port: m.wt_port,
        ws_port: m.ws_port,
        expires_in_s: frontline_protocol::dto::TICKET_TTL_S,
    })
    .into_response()
}

// ------------------------------------------------------------------ internal

fn internal_ok(st: &Shared, h: &HeaderMap) -> bool {
    bearer(h).is_some_and(|t| ct_eq(t.as_bytes(), st.cfg.internal_token.as_bytes()))
}

#[derive(Deserialize)]
struct RegisterReq {
    name: String,
    map: String,
    mode: String,
    region: String,
    #[serde(default)]
    tags: Vec<String>,
    version: String,
    protocol_version: u16,
    data_hash: u64,
    cap: u8,
    addr: String,
    ws_port: u16,
    wt_port: u16,
}

async fn register(State(st): State<Shared>, headers: HeaderMap, body: Bytes) -> Response {
    if !internal_ok(&st, &headers) {
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let Ok(r) = serde_json::from_slice::<RegisterReq>(&body) else {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body");
    };
    let id = random_16();
    st.registry.register(MatchEntry {
        id,
        name: r.name,
        map: r.map,
        mode: r.mode,
        region: r.region,
        tags: r.tags,
        version: r.version,
        protocol_version: r.protocol_version,
        data_hash: r.data_hash,
        cap: r.cap,
        addr: r.addr,
        ws_port: r.ws_port,
        wt_port: r.wt_port,
        phase: "lobby".into(),
        time_left_s: 0,
        humans: 0,
        bots: 0,
        last_heartbeat_ms: st.clock.now_ms(),
        ended: false,
        next_player: 0,
    });
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
    let ok = st.registry.heartbeat(&id, st.clock.now_ms(), |e| {
        e.humans = r.humans;
        e.bots = r.bots;
        e.phase = r.phase;
        e.time_left_s = r.time_left_s;
    });
    if ok {
        StatusCode::NO_CONTENT.into_response()
    } else {
        err(StatusCode::NOT_FOUND, "no_such_match")
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
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body");
    };
    let Some(id) = from_hex(&id) else {
        return err(StatusCode::NOT_FOUND, "no_such_match");
    };
    if st.registry.store_result(&id, v) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        err(StatusCode::NOT_FOUND, "no_such_match")
    }
}
