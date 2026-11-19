//! SQLite is the authoritative store for guest profiles, match registrations and results.

use std::str::FromStr;
use std::time::Duration;

use frontline_protocol::dto::ServerInfo;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::types::Json;
use sqlx::{ConnectOptions, FromRow, SqlitePool};

use crate::registry::{from_hex, to_hex, MatchId};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./src/db/migrations");

pub struct Database {
    pool: SqlitePool,
    ticket_key: [u8; 32],
}

/// Registration fields that do not change with heartbeats. JSON preserves `data_hash` as a
/// full unsigned 64-bit value (SQLite INTEGER is signed).
#[derive(Deserialize, Serialize)]
pub struct Registration {
    pub name: String,
    pub map: String,
    pub mode: String,
    pub region: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub version: String,
    pub protocol_version: u16,
    pub data_hash: u64,
    pub cap: u8,
    pub addr: String,
    pub ws_port: u16,
    pub wt_port: u16,
}

#[derive(FromRow)]
struct ServerRow {
    id: String,
    registration: Json<Registration>,
    phase: String,
    time_left_s: u32,
    humans: u8,
    bots: u8,
}

#[derive(Debug)]
pub enum SlotError {
    Unknown,
    Stale,
    Full,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for SlotError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

fn timestamp(ms: u64) -> i64 {
    i64::try_from(ms).unwrap_or(i64::MAX)
}

impl Database {
    pub async fn open(url: &str, ticket_key: [u8; 32]) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(url)?
            .disable_statement_logging()
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            // WAL NORMAL avoids a disk flush for each ticket. SQLite protects consistency
            // across power loss, but recent commits may be lost before a checkpoint.
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5));
        // One connection keeps in-memory fixtures and on-disk writes under the same semantics.
        // SQLx runs SQLite on its own blocking worker and queues concurrent async callers.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .test_before_acquire(false)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await?;
        MIGRATOR.run(&pool).await?;
        Ok(Self { pool, ticket_key })
    }

    pub async fn guest(&self, id: &str, suggested_nick: &str) -> Result<String, sqlx::Error> {
        // Backfill a valid guest session minted before the database migration, too.
        let mut txn = self.pool.begin().await?;
        sqlx::query("INSERT INTO guests (id, nick) VALUES (?, ?) ON CONFLICT (id) DO NOTHING")
            .bind(id)
            .bind(suggested_nick)
            .execute(&mut *txn)
            .await?;
        let nick = sqlx::query_scalar("SELECT nick FROM guests WHERE id = ?")
            .bind(id)
            .fetch_one(&mut *txn)
            .await?;
        txn.commit().await?;
        Ok(nick)
    }

    pub async fn register(
        &self,
        id: &MatchId,
        registration: Registration,
        now_ms: u64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO matches (id, registration, ticket_key, cap, last_heartbeat_ms)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(to_hex(id))
        .bind(Json(&registration))
        .bind(self.ticket_key.as_slice())
        .bind(registration.cap)
        .bind(timestamp(now_ms))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn heartbeat(
        &self,
        id: &MatchId,
        now_ms: u64,
        humans: u8,
        bots: u8,
        phase: &str,
        time_left_s: u32,
    ) -> Result<bool, sqlx::Error> {
        let changed = sqlx::query(
            "UPDATE matches SET humans = ?, bots = ?, phase = ?, time_left_s = ?, last_heartbeat_ms = ?
             WHERE id = ? AND ended = 0 AND ticket_key = ?",
        )
        .bind(humans)
        .bind(bots)
        .bind(phase)
        .bind(time_left_s)
        .bind(timestamp(now_ms))
        .bind(to_hex(id))
        .bind(self.ticket_key.as_slice())
        .execute(&self.pool)
        .await?;
        Ok(changed.rows_affected() != 0)
    }

    /// The cutoff is applied on every read, including immediately after a restart.
    pub async fn list(
        &self,
        now_ms: u64,
        timeout_ms: u64,
        protocol: u16,
    ) -> Result<Vec<ServerInfo>, sqlx::Error> {
        sqlx::query_as::<_, ServerRow>(
            "SELECT id, registration, phase, time_left_s, humans, bots FROM matches
             WHERE ended = 0 AND ticket_key = ? AND last_heartbeat_ms >= ? ORDER BY id",
        )
        .bind(self.ticket_key.as_slice())
        .bind(timestamp(now_ms.saturating_sub(timeout_ms)))
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            let match_id =
                from_hex(&row.id).ok_or(sqlx::Error::Protocol("invalid match id".into()))?;
            let reg = row.registration.0;
            Ok(ServerInfo {
                match_id,
                name: reg.name,
                map: reg.map,
                mode: reg.mode,
                phase: row.phase,
                time_left_s: row.time_left_s,
                humans: row.humans,
                bots: row.bots,
                cap: reg.cap,
                region: reg.region,
                tags: reg.tags,
                version: reg.version,
                compatible: reg.protocol_version == protocol,
            })
        })
        .collect()
    }

    pub async fn reserve_slot(
        &self,
        id: &MatchId,
        now_ms: u64,
        timeout_ms: u64,
        guest_id: &str,
        nick: &str,
    ) -> Result<(Registration, u8), SlotError> {
        let id = to_hex(id);
        let cutoff = timestamp(now_ms.saturating_sub(timeout_ms));
        let mut txn = self.pool.begin().await?;
        let slot: Option<(Json<Registration>, u8)> = sqlx::query_as(
            "UPDATE matches SET next_player = (next_player + 1) % 128
             WHERE id = ? AND ended = 0 AND ticket_key = ? AND last_heartbeat_ms >= ?
                 AND humans + bots < cap
             RETURNING registration, (next_player + 127) % 128",
        )
        .bind(&id)
        .bind(self.ticket_key.as_slice())
        .bind(cutoff)
        .fetch_optional(&mut *txn)
        .await?;
        let Some((Json(registration), player_id)) = slot else {
            let eligible: Option<bool> = sqlx::query_scalar(
                "SELECT ended = 0 AND ticket_key = ? AND last_heartbeat_ms >= ?
                 FROM matches WHERE id = ?",
            )
            .bind(self.ticket_key.as_slice())
            .bind(cutoff)
            .bind(&id)
            .fetch_optional(&mut *txn)
            .await?;
            return Err(match eligible {
                None => SlotError::Unknown,
                Some(false) => SlotError::Stale,
                Some(true) => SlotError::Full,
            });
        };
        sqlx::query(
            "INSERT INTO guests (id, nick) VALUES (?, ?)
             ON CONFLICT (id) DO UPDATE SET nick = excluded.nick WHERE guests.nick <> excluded.nick",
        )
        .bind(guest_id)
        .bind(nick)
        .execute(&mut *txn)
        .await?;
        txn.commit().await?;
        Ok((registration, player_id))
    }

    /// A result is terminal. Retrying a delivery is safe and preserves the first payload.
    pub async fn store_result(
        &self,
        id: &MatchId,
        payload: serde_json::Value,
    ) -> Result<bool, sqlx::Error> {
        let id = to_hex(id);
        let mut txn = self.pool.begin().await?;
        let updated = sqlx::query("UPDATE matches SET ended = 1 WHERE id = ?")
            .bind(&id)
            .execute(&mut *txn)
            .await?;
        if updated.rows_affected() == 0 {
            return Ok(false);
        }
        sqlx::query("INSERT INTO results (match_id, payload) VALUES (?, ?) ON CONFLICT DO NOTHING")
            .bind(&id)
            .bind(Json(payload))
            .execute(&mut *txn)
            .await?;
        txn.commit().await?;
        Ok(true)
    }
}
