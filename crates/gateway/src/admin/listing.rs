use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::types::Json as SqlJson;
use sqlx::{FromRow, SqliteConnection};

use crate::app::Shared;
use crate::db::Registration;
use crate::routes::{db_error, err};

use super::{decode, ts, valid_text};

#[derive(Deserialize)]
struct Visibility {
    listed: bool,
}

pub(super) async fn set_visibility(
    st: &Shared,
    conn: &mut SqliteConnection,
    id: &str,
    body: &[u8],
) -> Result<Response, sqlx::Error> {
    let Ok(visibility) = decode::<Visibility>(body) else {
        return Ok(err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body"));
    };
    let updated =
        sqlx::query("UPDATE matches SET listed = ? WHERE id = ? AND ticket_key = ? AND ended = 0")
            .bind(visibility.listed)
            .bind(id)
            .bind(st.ticket_key.verifying_key().as_bytes().as_slice())
            .execute(conn)
            .await?;
    Ok(if updated.rows_affected() == 0 {
        err(StatusCode::NOT_FOUND, "no_such_match")
    } else {
        StatusCode::NO_CONTENT.into_response()
    })
}

#[derive(Serialize, FromRow)]
struct ListedMatch {
    id: String,
    registration: SqlJson<Registration>,
    listed: bool,
    last_heartbeat_ms: i64,
}

pub(super) async fn servers(
    st: &Shared,
    conn: &mut SqliteConnection,
    now: i64,
) -> Result<Response, sqlx::Error> {
    let rows: Vec<ListedMatch> = sqlx::query_as(
        "SELECT id, registration, listed, last_heartbeat_ms FROM matches WHERE ended = 0 AND ticket_key = ?
         AND last_heartbeat_ms >= ? ORDER BY id",
    )
    .bind(st.ticket_key.verifying_key().as_bytes().as_slice())
    .bind(now.saturating_sub(ts(st.cfg.heartbeat_timeout_ms)))
    .fetch_all(conn).await?;
    Ok(Json(rows).into_response())
}

#[derive(Deserialize)]
struct AnnouncementRequest {
    text: String,
    duration_s: Option<u32>,
}

#[derive(Serialize, FromRow)]
struct Announcement {
    id: i64,
    text: String,
    created_at: i64,
    expires_at: i64,
}

pub(super) async fn publish(
    conn: &mut SqliteConnection,
    body: &[u8],
    now: i64,
) -> Result<Response, sqlx::Error> {
    let Ok(req) = decode::<AnnouncementRequest>(body) else {
        return Ok(err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body"));
    };
    let duration_s = req.duration_s.unwrap_or(86_400);
    if !valid_text(&req.text, 500) || !(1..=604_800).contains(&duration_s) {
        return Ok(err(StatusCode::UNPROCESSABLE_ENTITY, "bad_announcement"));
    }
    let expires_at = now.saturating_add(i64::from(duration_s) * 1000);
    let id =
        sqlx::query("INSERT INTO announcements (text, created_at, expires_at) VALUES (?, ?, ?)")
            .bind(req.text)
            .bind(now)
            .bind(expires_at)
            .execute(conn)
            .await?
            .last_insert_rowid();
    Ok((
        StatusCode::CREATED,
        Json(json!({"id":id,"expires_at":expires_at})),
    )
        .into_response())
}

pub(super) async fn retract(conn: &mut SqliteConnection, id: i64) -> Result<Response, sqlx::Error> {
    let removed = sqlx::query("DELETE FROM announcements WHERE id = ?")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(if removed.rows_affected() == 0 {
        err(StatusCode::NOT_FOUND, "no_such_announcement")
    } else {
        StatusCode::NO_CONTENT.into_response()
    })
}

pub(super) async fn announcements(
    axum::extract::State(st): axum::extract::State<Shared>,
) -> Response {
    async fn read(st: &Shared) -> Result<Vec<Announcement>, sqlx::Error> {
        sqlx::query_as("SELECT id, text, created_at, expires_at FROM announcements WHERE expires_at > ? ORDER BY id DESC LIMIT 20")
            .bind(ts(st.clock.now_ms())).fetch_all(&mut *st.db.connection().await?).await
    }
    match read(&st).await {
        Ok(rows) => Json(rows).into_response(),
        Err(error) => db_error(&error),
    }
}

pub(super) async fn player(conn: &mut SqliteConnection, id: &str) -> Result<Response, sqlx::Error> {
    let nick: Option<String> = sqlx::query_scalar("SELECT nick FROM guests WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?;
    let Some(nick) = nick else {
        return Ok(err(StatusCode::NOT_FOUND, "no_such_player"));
    };
    let (received, open, submitted): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM reports WHERE target_id = ?1),
                (SELECT COUNT(*) FROM reports WHERE target_id = ?1 AND status = 'open'),
                (SELECT COUNT(*) FROM reports WHERE reporter_id = ?1)",
    )
    .bind(id)
    .fetch_one(&mut *conn)
    .await?;
    // Older result rows identify only a match slot. Omit ambiguous recycled slots so a
    // player's score is never attributed to a different account by the gateway.
    let (matches, kills, deaths, score): (i64, i64, i64, i64) = sqlx::query_as(
        "WITH slots AS (SELECT DISTINCT match_id, player_id FROM moderation_tickets AS t WHERE guest_id = ?1
             AND NOT EXISTS (SELECT 1 FROM moderation_tickets AS other WHERE other.match_id = t.match_id
                             AND other.player_id = t.player_id AND other.guest_id <> ?1))
         SELECT COUNT(DISTINCT slots.match_id), COALESCE(SUM(json_extract(row.value, '$.kills')),0),
                COALESCE(SUM(json_extract(row.value, '$.deaths')),0), COALESCE(SUM(json_extract(row.value, '$.score')),0)
         FROM slots JOIN results USING (match_id), json_each(results.payload, '$.rows') AS row
         WHERE row.type = 'object' AND json_extract(row.value, '$.player') = slots.player_id",
    ).bind(id).fetch_one(conn).await?;
    Ok(Json(json!({"id":id,"nick":nick,
       "stats":{"matches":matches,"kills":kills,"deaths":deaths,"score":score},
       "suspicion":{"reports_received":received,"open_reports":open,"reports_submitted":submitted},
       "violations":null}))
    .into_response())
}
