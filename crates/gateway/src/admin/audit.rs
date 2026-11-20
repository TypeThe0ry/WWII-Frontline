use std::net::IpAddr;

use serde_json::Value;
use sqlx::types::Json;
use sqlx::{Connection, SqliteConnection};

use crate::app::Shared;

use super::{ts, DAY_MS};

pub(super) async fn begin(
    conn: &mut SqliteConnection,
    now: i64,
    authorized: bool,
    action: &str,
    target: Option<&str>,
    ip: Option<IpAddr>,
) -> Result<i64, sqlx::Error> {
    Ok(sqlx::query(
        "INSERT INTO audit_log (ts, actor, action, target, result, ip) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(now)
    .bind(if authorized { "admin" } else { "anonymous" })
    .bind(action)
    .bind(target)
    .bind(if authorized { "pending" } else { "403" })
    .bind(ip.map(|addr| addr.to_string()))
    .execute(conn)
    .await?
    .last_insert_rowid())
}

pub(super) async fn finish(
    conn: &mut SqliteConnection,
    id: i64,
    status: u16,
    params: Option<Value>,
) -> Result<(), sqlx::Error> {
    let changed = sqlx::query("UPDATE audit_log SET result = ?, params_json = ? WHERE id = ?")
        .bind(status.to_string())
        .bind(params.map(Json))
        .bind(id)
        .execute(conn)
        .await?;
    if changed.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

pub(crate) async fn prune(st: &Shared) -> Result<(), sqlx::Error> {
    let now = ts(st.clock.now_ms());
    let mut conn = st.db.connection().await?;
    let mut txn = conn.begin().await?;
    sqlx::query(
        "UPDATE reports SET chat_excerpt = NULL WHERE chat_excerpt IS NOT NULL AND created_at <= ?",
    )
    .bind(now.saturating_sub(14 * DAY_MS))
    .execute(&mut *txn)
    .await?;
    let month = now.saturating_sub(30 * DAY_MS);
    sqlx::query("UPDATE audit_log SET ip = NULL WHERE ip IS NOT NULL AND ts <= ?")
        .bind(month)
        .execute(&mut *txn)
        .await?;
    sqlx::query("DELETE FROM bans WHERE subject_kind = 'ip' AND created_at <= ?")
        .bind(month)
        .execute(&mut *txn)
        .await?;
    sqlx::query("UPDATE bans SET ip_cidr = NULL WHERE ip_cidr IS NOT NULL AND created_at <= ?")
        .bind(month)
        .execute(&mut *txn)
        .await?;
    sqlx::query("DELETE FROM audit_log WHERE ts <= ?")
        .bind(now.saturating_sub(365 * DAY_MS))
        .execute(&mut *txn)
        .await?;
    txn.commit().await?;
    st.live_chat.prune(now).await;
    Ok(())
}
