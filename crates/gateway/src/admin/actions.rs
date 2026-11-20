use std::collections::HashMap;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::types::Json as SqlJson;
use sqlx::SqliteConnection;

use crate::app::Shared;
use crate::routes::{db_error, err, internal_ok};

use super::{active_match, decode, hex_id, reports::ChatMessage, ts, valid_text, DAY_MS};

#[derive(Clone, Copy)]
pub(super) enum Action {
    Kick,
    Mute,
    ForceTeam,
    Broadcast,
    MapChange,
}

impl Action {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Kick => "kick",
            Self::Mute => "mute",
            Self::ForceTeam => "force_team",
            Self::Broadcast => "broadcast",
            Self::MapChange => "map_change",
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Payload {
    #[serde(skip_serializing_if = "Option::is_none")]
    player_id: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    jti: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    team: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    map: Option<String>,
}

#[derive(Deserialize)]
struct Command {
    match_id: String,
    #[serde(flatten)]
    payload: Payload,
}

pub(super) async fn enqueue(
    st: &Shared,
    conn: &mut SqliteConnection,
    action: Action,
    body: &[u8],
    now: i64,
) -> Result<Response, sqlx::Error> {
    let Ok(mut command) = decode::<Command>(body) else {
        return Ok(err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body"));
    };
    let p = &command.payload;
    let target = p.player_id.is_some_and(|id| id < 128) && p.jti.as_deref().is_none_or(hex_id);
    let valid = match action {
        Action::Kick | Action::Mute => target,
        Action::ForceTeam => target && p.team.is_some_and(|team| team <= 1),
        Action::Broadcast => p.text.as_deref().is_some_and(|s| valid_text(s, 120)),
        Action::MapChange => p.map.as_deref().is_some_and(|map| {
            !map.is_empty()
                && map.len() <= 64
                && map
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        }),
    };
    if !valid || !hex_id(&command.match_id) {
        return Ok(err(StatusCode::UNPROCESSABLE_ENTITY, "bad_action"));
    }
    if !active_match(st, conn, &command.match_id, now).await? {
        return Ok(err(StatusCode::GONE, "match_unavailable"));
    }
    command.payload = match action {
        Action::Kick | Action::Mute | Action::ForceTeam => Payload {
            player_id: p.player_id,
            jti: p.jti.clone(),
            team: p.team.filter(|_| matches!(action, Action::ForceTeam)),
            ..Payload::default()
        },
        Action::Broadcast => Payload {
            text: p.text.clone(),
            ..Payload::default()
        },
        Action::MapChange => Payload {
            map: p.map.clone(),
            ..Payload::default()
        },
    };
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM moderation_actions WHERE match_id = ? AND result IS NULL AND created_at > ?")
        .bind(&command.match_id).bind(now.saturating_sub(ACTION_TTL_MS))
        .fetch_one(&mut *conn).await?;
    if count >= 64 {
        return Ok(err(StatusCode::TOO_MANY_REQUESTS, "action_queue_full"));
    }
    let id = sqlx::query("INSERT INTO moderation_actions (match_id, action, payload, created_at) VALUES (?, ?, ?, ?)")
        .bind(command.match_id).bind(action.name()).bind(SqlJson(command.payload)).bind(now)
        .execute(conn).await?.last_insert_rowid();
    Ok((
        StatusCode::ACCEPTED,
        axum::Extension(super::AuditRecordId(id)),
        Json(json!({"id":id})),
    )
        .into_response())
}

const ACTION_TTL_MS: i64 = 60_000;

#[derive(Serialize)]
struct Pending {
    id: i64,
    action: String,
    #[serde(flatten)]
    payload: Payload,
}

pub(super) async fn pending(
    State(st): State<Shared>,
    Path(match_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !internal_ok(&st, &headers) {
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    async fn query(st: &Shared, match_id: &str) -> Result<Response, sqlx::Error> {
        let mut conn = st.db.connection().await?;
        let now = ts(st.clock.now_ms());
        if !hex_id(match_id) || !active_match(st, &mut conn, match_id, now).await? {
            return Ok(err(StatusCode::GONE, "match_unavailable"));
        }
        let rows: Vec<(i64, String, SqlJson<Payload>)> = sqlx::query_as(
            "SELECT id, action, payload FROM moderation_actions WHERE match_id = ?
             AND result IS NULL AND created_at > ? ORDER BY id LIMIT 20",
        )
        .bind(match_id)
        .bind(now.saturating_sub(ACTION_TTL_MS))
        .fetch_all(&mut *conn)
        .await?;
        Ok(Json(
            rows.into_iter()
                .map(|(id, action, SqlJson(payload))| Pending {
                    id,
                    action,
                    payload,
                })
                .collect::<Vec<_>>(),
        )
        .into_response())
    }
    query(&st, &match_id).await.unwrap_or_else(|e| db_error(&e))
}

#[derive(Deserialize)]
struct Ack {
    result: String,
}

pub(super) async fn ack(
    State(st): State<Shared>,
    Path((match_id, id)): Path<(String, i64)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !internal_ok(&st, &headers) {
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let Ok(ack) = decode::<Ack>(&body) else {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body");
    };
    if !hex_id(&match_id)
        || !matches!(
            ack.result.as_str(),
            "applied" | "target_missing" | "unsupported"
        )
    {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_ack");
    }
    async fn update(
        st: &Shared,
        match_id: &str,
        id: i64,
        result: &str,
    ) -> Result<Response, sqlx::Error> {
        let changed = sqlx::query(
            "UPDATE moderation_actions SET result = COALESCE(result, ?) WHERE id = ? AND match_id = ?
             AND EXISTS (SELECT 1 FROM matches WHERE id = ? AND ticket_key = ?)",
        )
        .bind(result).bind(id).bind(match_id).bind(match_id)
        .bind(st.ticket_key.verifying_key().as_bytes().as_slice())
        .execute(&mut *st.db.connection().await?).await?;
        Ok(if changed.rows_affected() == 0 {
            err(StatusCode::NOT_FOUND, "no_such_action")
        } else {
            StatusCode::NO_CONTENT.into_response()
        })
    }
    update(&st, &match_id, id, &ack.result)
        .await
        .unwrap_or_else(|e| db_error(&e))
}

pub(super) async fn get_action(
    conn: &mut SqliteConnection,
    id: i64,
    now: i64,
) -> Result<Response, sqlx::Error> {
    let row: Option<(String, String, i64, Option<String>)> = sqlx::query_as(
        "SELECT match_id, action, created_at, result FROM moderation_actions WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?;
    Ok(match row {
        Some((match_id, action, created_at, result)) => {
            let result =
                result
                    .as_deref()
                    .unwrap_or(if created_at <= now.saturating_sub(ACTION_TTL_MS) {
                        "expired"
                    } else {
                        "pending"
                    });
            Json(json!({"id":id,"match_id":match_id,"action":action,"created_at":created_at,"result":result})).into_response()
        }
        None => err(StatusCode::NOT_FOUND, "no_such_action"),
    })
}

#[derive(Default)]
pub(crate) struct LiveChat(tokio::sync::Mutex<HashMap<String, (i64, Vec<ChatMessage>)>>);

impl LiveChat {
    pub(crate) async fn remove(&self, match_id: &str) {
        self.0.lock().await.remove(match_id);
    }

    pub(crate) async fn prune(&self, now: i64) {
        self.0
            .lock()
            .await
            .retain(|_, (updated, _)| *updated > now.saturating_sub(DAY_MS));
    }
}

#[derive(Deserialize)]
struct ChatUpdate {
    messages: Vec<ChatMessage>,
}

pub(super) async fn update_chat(
    State(st): State<Shared>,
    Path(match_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !internal_ok(&st, &headers) {
        return err(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let Ok(update) = decode::<ChatUpdate>(&body) else {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body");
    };
    if !hex_id(&match_id)
        || update.messages.len() > 100
        || !update.messages.iter().all(ChatMessage::valid)
    {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "bad_chat");
    }
    let now = ts(st.clock.now_ms());
    let mut conn = match st.db.connection().await {
        Ok(conn) => conn,
        Err(error) => return db_error(&error),
    };
    match active_match(&st, &mut conn, &match_id, now).await {
        Ok(true) => {
            // Keep lock order db -> chat, including the admin read path.
            st.live_chat
                .0
                .lock()
                .await
                .insert(match_id, (now, update.messages));
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => err(StatusCode::GONE, "match_unavailable"),
        Err(error) => db_error(&error),
    }
}

pub(super) async fn chat(
    st: &Shared,
    conn: &mut SqliteConnection,
    match_id: &str,
    now: i64,
) -> Result<Response, sqlx::Error> {
    if !active_match(st, conn, match_id, now).await? {
        return Ok(err(StatusCode::NOT_FOUND, "no_live_match"));
    }
    let chat = st.live_chat.0.lock().await;
    let messages = chat
        .get(match_id)
        .filter(|(updated, _)| *updated > now.saturating_sub(DAY_MS))
        .map_or(&[][..], |(_, messages)| messages.as_slice());
    Ok(Json(messages).into_response())
}
