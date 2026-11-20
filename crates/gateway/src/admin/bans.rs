use std::net::IpAddr;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::SqliteConnection;

use super::auth::{hex_hash, Cidr};
use super::{decode, DAY_MS};
use crate::app::Shared;
use crate::routes::err;

#[derive(Deserialize)]
struct Ban {
    subject_kind: String,
    subject_hash: Option<String>,
    ip_cidr: Option<String>,
    reason: String,
    duration: String,
}

pub(super) async fn create(
    conn: &mut SqliteConnection,
    body: &[u8],
    now: i64,
) -> Result<Response, sqlx::Error> {
    let Ok(mut ban) = decode::<Ban>(body) else {
        return Ok(err(StatusCode::UNPROCESSABLE_ENTITY, "bad_body"));
    };
    let expires_at = match ban.duration.as_str() {
        "1h" => Some(now.saturating_add(3_600_000)),
        "24h" => Some(now.saturating_add(DAY_MS)),
        "7d" => Some(now.saturating_add(7 * DAY_MS)),
        "perm" if ban.subject_kind != "ip" => None,
        _ => return Ok(err(StatusCode::UNPROCESSABLE_ENTITY, "bad_duration")),
    };
    let cidr = match ban.ip_cidr.as_deref() {
        None => None,
        Some(s) => match Cidr::parse(s) {
            Some(cidr) => Some(cidr.canonical()),
            None => return Ok(err(StatusCode::UNPROCESSABLE_ENTITY, "bad_cidr")),
        },
    };
    let valid_subject = match (ban.subject_kind.as_str(), ban.subject_hash.as_deref()) {
        ("account", Some(id)) => super::hex_id(id),
        ("guest", Some(hash)) => hex_hash(hash).is_some(),
        ("ip", None) => cidr.is_some(),
        _ => false,
    };
    if !valid_subject || !super::valid_text(&ban.reason, 500) {
        return Ok(err(StatusCode::UNPROCESSABLE_ENTITY, "bad_ban"));
    }
    if let Some(subject) = &mut ban.subject_hash {
        subject.make_ascii_lowercase();
    }
    let id = sqlx::query(
        "INSERT INTO bans (subject_kind, subject_hash, ip_cidr, reason, expires_at, created_by, created_at)
         VALUES (?, ?, ?, ?, ?, 'admin', ?)",
    )
    .bind(ban.subject_kind)
    .bind(ban.subject_hash)
    .bind(cidr)
    .bind(ban.reason)
    .bind(expires_at)
    .bind(now)
    .execute(conn)
    .await?
    .last_insert_rowid();
    Ok((
        StatusCode::CREATED,
        axum::Extension(super::AuditRecordId(id)),
        Json(json!({"id": id, "expires_at": expires_at})),
    )
        .into_response())
}

pub(super) async fn remove(conn: &mut SqliteConnection, id: i64) -> Result<Response, sqlx::Error> {
    let removed = sqlx::query("DELETE FROM bans WHERE id = ?")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(if removed.rows_affected() > 0 {
        StatusCode::NO_CONTENT.into_response()
    } else {
        err(StatusCode::NOT_FOUND, "no_such_ban")
    })
}

/// `Some(None)` means permanent. An attached network is an independent match condition and
/// stops matching after 30 days, even if the identity ban remains active.
pub(super) async fn lookup(
    conn: &mut SqliteConnection,
    now: i64,
    ip: Option<IpAddr>,
    account: Option<&str>,
    token: Option<&str>,
) -> Result<Option<Option<i64>>, sqlx::Error> {
    let fingerprint = token.map(|token| format!("{:x}", Sha256::digest(token)));
    let identity: Option<(Option<i64>,)> = sqlx::query_as(
        "SELECT expires_at FROM bans
         WHERE ((subject_kind = 'account' AND subject_hash = ?) OR (subject_kind = 'guest' AND subject_hash = ?))
           AND (expires_at IS NULL OR expires_at > ?)
         ORDER BY expires_at IS NOT NULL, expires_at DESC LIMIT 1",
    )
    .bind(account)
    .bind(fingerprint)
    .bind(now)
    .fetch_optional(&mut *conn)
    .await?;
    let ideal = identity.map(|(expiry,)| expiry);
    let Some(ip) = ip.filter(|_| ideal != Some(None)) else {
        return Ok(ideal);
    };
    let ranges: Vec<(String, Option<i64>, i64)> = sqlx::query_as(
        "SELECT ip_cidr, expires_at, created_at FROM bans WHERE ip_cidr IS NOT NULL
           AND (expires_at IS NULL OR expires_at > ?) AND created_at > ?",
    )
    .bind(now)
    .bind(now.saturating_sub(30 * DAY_MS))
    .fetch_all(conn)
    .await?;
    let expiry = ranges
        .into_iter()
        .filter(|(cidr, _, _)| Cidr::parse(cidr).is_some_and(|c| c.contains(ip)))
        .map(|(_, expiry, created)| {
            let retention = created.saturating_add(30 * DAY_MS);
            expiry.map_or(retention, |end| end.min(retention))
        })
        .chain(ideal.flatten())
        .max();
    Ok(expiry.map(Some))
}

pub(crate) async fn check(
    st: &Shared,
    now: u64,
    ip: Option<IpAddr>,
    account: Option<&str>,
    token: Option<&str>,
) -> Result<Option<Response>, sqlx::Error> {
    let mut conn = st.db.connection().await?;
    Ok(lookup(&mut conn, super::ts(now), ip, account, token)
        .await?
        .map(banned))
}

pub(super) fn banned(expiry: Option<i64>) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({"error":"banned", "expires_at":expiry})),
    )
        .into_response()
}
