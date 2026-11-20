use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::types::Json as SqlJson;
use sqlx::{Connection, FromRow, SqliteConnection};

use crate::app::Shared;
use crate::routes::{db_error, err, internal_ok};

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct ChatMessage {
    player_id: u8,
    text: String,
}

impl ChatMessage {
    pub(super) fn valid(&self) -> bool {
        self.player_id < 128 && super::valid_text(&self.text, 500)
    }
}

#[derive(Deserialize)]
struct Submission {
    match_id: String,
    reporter_jti: String,
    target_jti: String,
    reason: u8,
    #[serde(default)]
    note: String,
    chat_excerpt: Option<Vec<ChatMessage>>,
}

impl Submission {
    fn valid(&self) -> bool {
        [&self.match_id, &self.reporter_jti, &self.target_jti]
            .into_iter()
            .all(|id| super::hex_id(id))
            && self.reporter_jti != self.target_jti
            && self.reason < 4
            && self.note.len() <= 120
            && !self.note.chars().any(char::is_control)
            && self.chat_excerpt.as_ref().is_none_or(|messages| {
                messages.len() <= 40 && messages.iter().all(ChatMessage::valid)
            })
    }
}

pub(super) async fn submit(State(st): State<Shared>, headers: HeaderMap, body: Bytes) -> Response {
    if !internal_ok(&st, &headers) {
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let Ok(req) = super::decode::<Submission>(&body) else {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body");
    };
    if !req.valid() {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_report");
    }
    store(&st, req).await.unwrap_or_else(|e| db_error(&e))
}

async fn store(st: &Shared, req: Submission) -> Result<Response, sqlx::Error> {
    let mut conn = st.db.connection().await?;
    let mut txn = conn.begin().await?;
    let now = super::ts(st.clock.now_ms());
    let subjects: Option<(String, String)> = sqlx::query_as(
        "SELECT reporter.guest_id, target.guest_id FROM moderation_tickets reporter
         JOIN moderation_tickets target ON target.match_id = reporter.match_id
         JOIN matches ON matches.id = reporter.match_id
         WHERE reporter.match_id = ? AND reporter.jti = ? AND target.jti = ?
           AND matches.ticket_key = ?",
    )
    .bind(&req.match_id)
    .bind(req.reporter_jti)
    .bind(req.target_jti)
    .bind(st.ticket_key.verifying_key().as_bytes().as_slice())
    .fetch_optional(&mut *txn)
    .await?;
    let Some((reporter, target)) = subjects.filter(|(a, b)| a != b) else {
        return Ok(err(
            StatusCode::UNPROCESSABLE_ENTITY,
            "unknown_report_participant",
        ));
    };
    let duplicate: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM reports WHERE match_id = ? AND reporter_id = ? AND target_id = ? AND reason = ?",
    )
    .bind(&req.match_id)
    .bind(&reporter)
    .bind(&target)
    .bind(req.reason)
    .fetch_optional(&mut *txn)
    .await?;
    if let Some(id) = duplicate {
        return Ok(Json(json!({"id":id})).into_response());
    }
    if !super::active_match(st, &mut txn, &req.match_id, now).await? {
        return Ok(err(StatusCode::GONE, "match_unavailable"));
    }
    let in_match: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM reports WHERE match_id = ? AND reporter_id = ?")
            .bind(&req.match_id)
            .bind(&reporter)
            .fetch_one(&mut *txn)
            .await?;
    if in_match >= 10 {
        return Ok(err(StatusCode::TOO_MANY_REQUESTS, "match_report_limit"));
    }
    let latest: Option<i64> = sqlx::query_scalar(
        "SELECT created_at FROM reports WHERE reporter_id = ? ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&reporter)
    .fetch_optional(&mut *txn)
    .await?;
    if let Some(wait) = latest
        .map(|t| t.saturating_add(60_000).saturating_sub(now))
        .filter(|w| *w > 0)
    {
        let mut response = err(StatusCode::TOO_MANY_REQUESTS, "report_rate_limit");
        if let Ok(value) = ((wait as u64).div_ceil(1000)).to_string().parse() {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        return Ok(response);
    }
    let id = sqlx::query(
        "INSERT INTO reports (match_id, reporter_id, target_id, reason, note, chat_excerpt, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(req.match_id)
    .bind(reporter)
    .bind(target)
    .bind(req.reason)
    .bind(req.note)
    .bind(req.chat_excerpt.filter(|messages| !messages.is_empty()).map(SqlJson))
    .bind(now)
    .execute(&mut *txn)
    .await?
    .last_insert_rowid();
    txn.commit().await?;
    Ok((StatusCode::CREATED, Json(json!({"id":id}))).into_response())
}

#[derive(Deserialize)]
struct ReplayConfirmation {
    replay_ref: String,
}

pub(super) async fn confirm_replay(
    State(st): State<Shared>,
    Path((match_id, id)): Path<(String, i64)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !internal_ok(&st, &headers) {
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let Ok(req) = super::decode::<ReplayConfirmation>(&body) else {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body");
    };
    if !super::hex_id(&match_id)
        || id < 1
        || req.replay_ref != format!("replays/{match_id}_{id}.bin")
    {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_replay_ref");
    }
    let mut conn = match st.db.connection().await {
        Ok(conn) => conn,
        Err(error) => return db_error(&error),
    };
    match sqlx::query(
        "UPDATE reports SET replay_ref = ? WHERE id = ? AND match_id = ?
         AND EXISTS (SELECT 1 FROM matches WHERE id = ? AND ticket_key = ?)",
    )
    .bind(req.replay_ref)
    .bind(id)
    .bind(&match_id)
    .bind(&match_id)
    .bind(st.ticket_key.verifying_key().as_bytes().as_slice())
    .execute(&mut *conn)
    .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => err(StatusCode::NOT_FOUND, "no_such_report"),
        Err(error) => db_error(&error),
    }
}

#[derive(Serialize, FromRow)]
struct ReportRow {
    id: i64,
    match_id: String,
    reporter_id: String,
    target_id: String,
    reason: u8,
    note: String,
    replay_ref: Option<String>,
    chat_excerpt: Option<SqlJson<Vec<ChatMessage>>>,
    status: String,
    created_at: i64,
}

#[derive(Deserialize, Default)]
pub(super) struct ReportQuery {
    status: Option<String>,
    before: Option<i64>,
    limit: Option<u32>,
}

pub(super) async fn list(
    conn: &mut SqliteConnection,
    query: ReportQuery,
    now: i64,
) -> Result<Response, sqlx::Error> {
    if !matches!(query.status.as_deref(), None | Some("open" | "resolved")) {
        return Ok(err(StatusCode::UNPROCESSABLE_ENTITY, "bad_status"));
    }
    let rows = sqlx::query_as::<_, ReportRow>(
        "SELECT id, match_id, reporter_id, target_id, reason, note, replay_ref,
           CASE WHEN created_at > ? THEN chat_excerpt END AS chat_excerpt, status, created_at
         FROM reports WHERE id < ? AND (? IS NULL OR status = ?) ORDER BY id DESC LIMIT ?",
    )
    .bind(now.saturating_sub(14 * super::DAY_MS))
    .bind(query.before.unwrap_or(i64::MAX))
    .bind(&query.status)
    .bind(&query.status)
    .bind(query.limit.unwrap_or(50).clamp(1, 100))
    .fetch_all(conn)
    .await?;
    Ok(Json(rows).into_response())
}

pub(super) async fn resolve(conn: &mut SqliteConnection, id: i64) -> Result<Response, sqlx::Error> {
    let res = sqlx::query("UPDATE reports SET status = 'resolved' WHERE id = ?")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(if res.rows_affected() != 0 {
        StatusCode::NO_CONTENT.into_response()
    } else {
        err(StatusCode::NOT_FOUND, "no_such_report")
    })
}
