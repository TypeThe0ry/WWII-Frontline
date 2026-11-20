use std::net::IpAddr;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::types::Json as SqlJson;
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

pub(crate) fn fingerprint(token: &str) -> String {
    format!("{:x}", Sha256::digest(token))
}

pub(crate) struct BanSnapshot(Vec<BanRule>);

struct BanRule {
    kind: String,
    subject: Option<String>,
    cidr: Option<Cidr>,
    expiry: Option<i64>,
    created: i64,
}

impl BanSnapshot {
    /// Bound the query to identities in the reservation batch and currently active IP rules.
    pub(crate) async fn load(
        conn: &mut SqliteConnection,
        now: i64,
        ids: &[&str],
        fingerprints: &[&str],
        has_ip: bool,
    ) -> Result<Self, sqlx::Error> {
        type Row = (String, Option<String>, Option<String>, Option<i64>, i64);
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT subject_kind, subject_hash, ip_cidr, expires_at, created_at FROM bans
             WHERE (expires_at IS NULL OR expires_at > ?1) AND (
                (subject_kind = 'account' AND subject_hash IN (SELECT value FROM json_each(?2))) OR
                (subject_kind = 'guest' AND subject_hash IN (SELECT value FROM json_each(?3))) OR
                (?4 AND ip_cidr IS NOT NULL AND created_at > ?5))",
        )
        .bind(now)
        .bind(SqlJson(ids))
        .bind(SqlJson(fingerprints))
        .bind(has_ip)
        .bind(now.saturating_sub(30 * DAY_MS))
        .fetch_all(conn)
        .await?;
        Ok(Self(
            rows.into_iter()
                .map(|(kind, subject, cidr, expiry, created)| BanRule {
                    kind,
                    subject,
                    cidr: cidr.as_deref().and_then(Cidr::parse),
                    expiry,
                    created,
                })
                .collect(),
        ))
    }

    /// `Some(None)` is permanent. An attached CIDR is independent of the identity
    /// and stops matching after 30 days even if the subject ban stays active.
    pub(crate) fn decision(
        &self,
        now: i64,
        ip: Option<IpAddr>,
        account: Option<&str>,
        fingerprint: Option<&str>,
    ) -> Option<Option<i64>> {
        let mut longest = None;
        for rule in &self.0 {
            if rule.expiry.is_some_and(|end| end <= now) {
                continue;
            }
            let subject = match rule.kind.as_str() {
                "account" => account,
                "guest" => fingerprint,
                _ => None,
            };
            if subject.is_some() && subject == rule.subject.as_deref() {
                let Some(end) = rule.expiry else {
                    return Some(None);
                };
                longest = longest.max(Some(end));
            } else if let (Some(ip), Some(cidr)) = (ip, rule.cidr) {
                let retention = rule.created.saturating_add(30 * DAY_MS);
                if retention > now && cidr.contains(ip) {
                    longest = longest.max(Some(
                        rule.expiry.map_or(retention, |end| end.min(retention)),
                    ));
                }
            }
        }
        longest.map(Some)
    }
}

pub(crate) async fn check(
    st: &Shared,
    now: u64,
    ip: Option<IpAddr>,
    account: Option<&str>,
    token: Option<&str>,
) -> Result<Option<Response>, sqlx::Error> {
    let mut conn = st.db.connection().await?;
    let now = super::ts(now);
    let digest = token.map(fingerprint);
    let ids: Vec<_> = account.into_iter().collect();
    let fingerprints: Vec<_> = digest.as_deref().into_iter().collect();
    let rules = BanSnapshot::load(&mut conn, now, &ids, &fingerprints, ip.is_some()).await?;
    Ok(rules
        .decision(now, ip, account, digest.as_deref())
        .map(banned))
}

pub(crate) fn banned(expiry: Option<i64>) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({"error":"banned", "expires_at":expiry})),
    )
        .into_response()
}
