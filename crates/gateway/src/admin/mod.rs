//! Moderation HTTP surface. The administrator credential is separate from both join tickets
//! and the match-server bearer. Mutations and their final audit status commit together.

mod actions;
mod audit;
mod auth;
pub(crate) mod bans;
mod listing;
mod reports;

pub(crate) use actions::LiveChat;
pub use auth::AdminConfig;

use std::net::IpAddr;
use std::time::Duration;

use axum::body::{to_bytes, Bytes};
use axum::extract::{Query, Request, State};
use axum::http::{Method, StatusCode};
use axum::response::Response;
use axum::routing::{any, get, post, put};
use axum::Router;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use sqlx::{Connection, SqliteConnection};

use crate::app::Shared;
use crate::routes::{db_error, err};

use actions::Action;
const DAY_MS: i64 = 86_400_000;
const MAX_MODERATION_BODY: usize = 64 * 1024;

#[derive(Clone, Copy)]
pub(crate) struct ModerationIp(pub Option<IpAddr>, pub bool);

#[derive(Clone)]
struct AuditRecordId(i64);

pub(crate) fn routes() -> Router<Shared> {
    Router::new()
        .route("/admin", any(dispatch))
        .route("/admin/", any(dispatch))
        .route("/admin/{*path}", any(dispatch))
        .route("/v1/announcements", get(listing::announcements))
        .route("/internal/report", post(reports::submit))
        .route(
            "/internal/matches/{id}/reports/{report_id}/replay",
            put(reports::confirm_replay),
        )
        .route("/internal/matches/{id}/actions", get(actions::pending))
        .route(
            "/internal/matches/{id}/actions/{action_id}/ack",
            post(actions::ack),
        )
        .route("/internal/matches/{id}/chat", put(actions::update_chat))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_MODERATION_BODY))
}

pub(crate) fn is_admin_path(path: &str) -> bool {
    path == "/admin" || path.starts_with("/admin/")
}

fn ts(ms: u64) -> i64 {
    i64::try_from(ms).unwrap_or(i64::MAX)
}

fn hex_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn valid_text(text: &str, max: usize) -> bool {
    !text.trim().is_empty() && text.len() <= max && !text.chars().any(char::is_control)
}

fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(body)
}

async fn active_match(
    st: &Shared,
    conn: &mut SqliteConnection,
    match_id: &str,
    now: i64,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM matches WHERE id = ? AND ended = 0 AND ticket_key = ? AND last_heartbeat_ms >= ?)",
    ).bind(match_id).bind(st.ticket_key.verifying_key().as_bytes().as_slice())
        .bind(now.saturating_sub(ts(st.cfg.heartbeat_timeout_ms))).fetch_one(conn).await
}

/// Start with a sweep so restarted gateways catch up before serving normal traffic.
/// The periodic task only holds a weak reference to application state.
pub(crate) async fn start_retention(st: &Shared) -> Result<(), sqlx::Error> {
    audit::prune(st).await?;
    let weak = std::sync::Arc::downgrade(st);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        interval.tick().await;
        loop {
            interval.tick().await;
            let Some(st) = weak.upgrade() else {
                break;
            };
            if audit::prune(&st).await.is_err() {
                tracing::error!("gateway moderation retention failed");
            }
        }
    });
    Ok(())
}

enum Endpoint {
    Reports,
    Resolve(i64),
    Ban,
    Unban(i64),
    Enqueue(Action),
    ActionStatus(i64),
    Chat(String),
    Player(String),
    Servers,
    Visibility(String),
    Publish,
    Retract(i64),
    Unknown,
}

impl Endpoint {
    fn from_request(method: &Method, path: &str) -> Self {
        let tail = path.strip_prefix("/admin/").unwrap_or_default();
        let (resource, rest) = tail.split_once('/').unwrap_or((tail, ""));
        let positive_id = |id: &str| id.parse::<i64>().ok().filter(|id| *id > 0);
        match (method, resource, rest) {
            (&Method::GET, "reports", "") => Self::Reports,
            (&Method::POST, "reports", tail) => tail
                .strip_suffix("/resolve")
                .and_then(positive_id)
                .map_or(Self::Unknown, Self::Resolve),
            (&Method::POST, "ban", "") => Self::Ban,
            (&Method::DELETE, "ban", id) => positive_id(id).map_or(Self::Unknown, Self::Unban),
            (&Method::POST, "kick", "") => Self::Enqueue(Action::Kick),
            (&Method::POST, "mute", "") => Self::Enqueue(Action::Mute),
            (&Method::POST, "force_team", "") => Self::Enqueue(Action::ForceTeam),
            (&Method::POST, "broadcast", "") => Self::Enqueue(Action::Broadcast),
            (&Method::POST, "map_change", "") => Self::Enqueue(Action::MapChange),
            (&Method::GET, "actions", id) => {
                positive_id(id).map_or(Self::Unknown, Self::ActionStatus)
            }
            (&Method::GET, "chat", id) if hex_id(id) => Self::Chat(id.into()),
            (&Method::GET, "players", id) if hex_id(id) => Self::Player(id.into()),
            (&Method::GET, "servers", "") => Self::Servers,
            (&Method::PUT, "servers", id) if hex_id(id) => Self::Visibility(id.into()),
            (&Method::POST, "announcements", "") => Self::Publish,
            (&Method::DELETE, "announcements", id) => {
                positive_id(id).map_or(Self::Unknown, Self::Retract)
            }
            _ => Self::Unknown,
        }
    }

    fn audit_fields(&self) -> (&'static str, Option<String>) {
        match self {
            Self::Reports => ("reports.list", None),
            Self::Resolve(id) => ("reports.resolve", Some(id.to_string())),
            Self::Ban => ("ban.create", None),
            Self::Unban(id) => ("ban.remove", Some(id.to_string())),
            Self::Enqueue(action) => (action.name(), None),
            Self::ActionStatus(id) => ("actions.get", Some(id.to_string())),
            Self::Chat(id) => ("chat.get", Some(id.clone())),
            Self::Player(id) => ("player.get", Some(id.clone())),
            Self::Servers => ("servers.list", None),
            Self::Visibility(id) => ("servers.visibility", Some(id.clone())),
            Self::Publish => ("announcements.publish", None),
            Self::Retract(id) => ("announcements.retract", Some(id.to_string())),
            Self::Unknown => ("unknown", None),
        }
    }

    fn safe_audit_params(&self, body: &[u8]) -> Option<Value> {
        let body: Value = decode(body).ok()?;
        match self {
            Self::Ban => {
                let kind = body.get("subject_kind")?.as_str()?;
                let duration = body.get("duration")?.as_str()?;
                (matches!(kind, "account" | "guest" | "ip")
                    && matches!(duration, "1h" | "24h" | "7d" | "perm"))
                .then(|| {
                    let subject =
                        body.get("subject_hash")
                            .and_then(Value::as_str)
                            .filter(|value| {
                                (kind == "account" && hex_id(value))
                                    || (kind == "guest" && auth::hex_hash(value).is_some())
                            });
                    json!({"subject_kind":kind,"duration":duration,"subject_hash":subject})
                })
            }
            Self::Enqueue(_) => {
                let id = body.get("match_id")?.as_str()?.to_owned();
                hex_id(&id).then(|| json!({"match_id":id,"player_id":body.get("player_id").and_then(Value::as_u64)}))
            }
            Self::Visibility(_) => Some(json!({"listed":body.get("listed")?.as_bool()?})),
            _ => None,
        }
    }

    async fn execute(
        &self,
        st: &Shared,
        conn: &mut SqliteConnection,
        now: i64,
        uri: &axum::http::Uri,
        body: &Bytes,
    ) -> Result<Response, sqlx::Error> {
        match self {
            Self::Reports => match Query::<reports::ReportQuery>::try_from_uri(uri) {
                Ok(Query(query)) => reports::list(conn, query, now).await,
                Err(_) => Ok(err(StatusCode::UNPROCESSABLE_ENTITY, "bad_query")),
            },
            Self::Resolve(id) => reports::resolve(conn, *id).await,
            Self::Ban => bans::create(conn, body, now).await,
            Self::Unban(id) => bans::remove(conn, *id).await,
            Self::Enqueue(action) => actions::enqueue(st, conn, *action, body, now).await,
            Self::ActionStatus(id) => actions::get_action(conn, *id, now).await,
            Self::Chat(id) => actions::chat(st, conn, id, now).await,
            Self::Player(id) => listing::player(conn, id).await,
            Self::Servers => listing::servers(st, conn, now).await,
            Self::Visibility(id) => listing::set_visibility(st, conn, id, body).await,
            Self::Publish => listing::publish(conn, body, now).await,
            Self::Retract(id) => listing::retract(conn, *id).await,
            Self::Unknown => Ok(err(StatusCode::NOT_FOUND, "no_such_admin_route")),
        }
    }
}

async fn dispatch(State(st): State<Shared>, req: Request) -> Response {
    handle(&st, req).await.unwrap_or_else(|e| db_error(&e))
}

async fn handle(st: &Shared, req: Request) -> Result<Response, sqlx::Error> {
    let now = ts(st.clock.now_ms());
    let ip = st.cfg.admin.client_ip(&req);
    let authorized = st.cfg.admin.authorized(req.headers(), ip);
    let endpoint = Endpoint::from_request(req.method(), req.uri().path());
    let (name, target) = endpoint.audit_fields();
    let audit_id = {
        let mut conn = st.db.connection().await?;
        audit::begin(&mut conn, now, authorized, name, target.as_deref(), ip).await?
    };
    if !authorized {
        return Ok(err(StatusCode::FORBIDDEN, "admin_forbidden"));
    }
    let (parts, body) = req.into_parts();
    let body =
        match tokio::time::timeout(Duration::from_secs(5), to_bytes(body, MAX_MODERATION_BODY))
            .await
        {
            Ok(Ok(body)) => body,
            failure => {
                let status = if failure.is_err() {
                    StatusCode::REQUEST_TIMEOUT
                } else {
                    StatusCode::BAD_REQUEST
                };
                let mut conn = st.db.connection().await?;
                audit::finish(&mut conn, audit_id, status.as_u16(), None).await?;
                return Ok(err(status, "bad_body"));
            }
        };
    let params = endpoint.safe_audit_params(&body);
    let mut conn = st.db.connection().await?;
    let mut txn = conn.begin().await?;
    match endpoint.execute(st, &mut txn, now, &parts.uri, &body).await {
        Ok(response) => {
            let mut params = params;
            if let Some(&AuditRecordId(id)) = response.extensions().get() {
                let mut payload = params.unwrap_or_else(|| json!({}));
                payload["record_id"] = json!(id);
                params = Some(payload);
            }
            audit::finish(&mut txn, audit_id, response.status().as_u16(), params).await?;
            txn.commit().await?;
            Ok(response)
        }
        Err(error) => {
            txn.rollback().await?;
            audit::finish(
                &mut conn,
                audit_id,
                StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                params,
            )
            .await?;
            Err(error)
        }
    }
}
