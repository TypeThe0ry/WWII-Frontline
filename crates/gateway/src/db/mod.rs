//! SQLite is the authoritative store for guest profiles, match registrations and results.

use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use frontline_protocol::dto::ServerInfo;
use frontline_protocol::PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::types::Json;
use sqlx::{ConnectOptions, Connection, FromRow, SqliteConnection};
use tokio::sync::{Mutex, MutexGuard};

use crate::metrics::Metrics;
use crate::registry::{from_hex, to_hex, MatchId};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./src/db/migrations");

mod ticket_queue;

pub struct Database {
    connection: Arc<Mutex<SqliteConnection>>,
    ticket_key: [u8; 32],
    tickets: ticket_queue::Queue,
}

/// Registration fields that do not change with heartbeats. JSON preserves `data_hash` as a
/// full unsigned 64-bit value (SQLite INTEGER is signed).
#[derive(Clone, Deserialize, Serialize)]
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

#[derive(FromRow, Serialize)]
pub(crate) struct StoredResult {
    match_id: String,
    map: String,
    mode: String,
    received_at_ms: Option<i64>,
    result: Json<Value>,
}

#[derive(Debug)]
pub enum SlotError {
    Unknown,
    Stale,
    Incompatible,
    Full,
    Banned(Option<i64>),
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
        // Keep one SQLite worker and statement cache for both memory and disk. A one-slot
        // SQLx pool adds a spawned return task and a worker ping on every handoff.
        let mut connection = options.connect().await?;
        MIGRATOR.run(&mut connection).await?;
        let connection = Arc::new(Mutex::new(connection));
        Ok(Self {
            tickets: ticket_queue::Queue::start(connection.clone(), ticket_key),
            connection,
            ticket_key,
        })
    }

    pub(crate) async fn connection(&self) -> Result<MutexGuard<'_, SqliteConnection>, sqlx::Error> {
        lock(&self.connection).await
    }

    pub async fn guest(&self, id: &str, suggested_nick: &str) -> Result<String, sqlx::Error> {
        // Backfill a valid guest session minted before the database migration, too.
        let mut conn = self.connection().await?;
        let mut txn = conn.begin().await?;
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
        .execute(&mut *self.connection().await?)
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
        .execute(&mut *self.connection().await?)
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
        let rows = sqlx::query_as::<_, ServerRow>(
            "SELECT id, registration, phase, time_left_s, humans, bots FROM matches
             WHERE ended = 0 AND listed = 1 AND ticket_key = ? AND last_heartbeat_ms >= ? ORDER BY id",
        )
        .bind(self.ticket_key.as_slice())
        .bind(timestamp(now_ms.saturating_sub(timeout_ms)))
        .fetch_all(&mut *self.connection().await?)
        .await?;
        rows.into_iter()
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

    pub(crate) async fn reserve_slot(
        &self,
        request: TicketReservation<'_>,
        metrics: &Metrics,
    ) -> Result<(Registration, u8), SlotError> {
        self.tickets.reserve(request, metrics).await
    }

    /// A result is terminal. Retrying a delivery is safe and preserves the first payload.
    pub async fn store_result(
        &self,
        id: &MatchId,
        payload: Value,
        now_ms: u64,
    ) -> Result<bool, sqlx::Error> {
        let id = to_hex(id);
        let mut conn = self.connection().await?;
        let mut txn = conn.begin().await?;
        let updated = sqlx::query("UPDATE matches SET ended = 1 WHERE id = ?")
            .bind(&id)
            .execute(&mut *txn)
            .await?;
        if updated.rows_affected() == 0 {
            return Ok(false);
        }
        sqlx::query(
            "INSERT INTO results (match_id, payload, received_at_ms) VALUES (?, ?, ?)
             ON CONFLICT (match_id) DO NOTHING",
        )
        .bind(&id)
        .bind(Json(payload))
        .bind(timestamp(now_ms))
        .execute(&mut *txn)
        .await?;
        txn.commit().await?;
        Ok(true)
    }

    pub(crate) async fn match_result(
        &self,
        id: &MatchId,
    ) -> Result<Option<StoredResult>, sqlx::Error> {
        sqlx::query_as(
            "SELECT r.match_id, json_extract(m.registration, '$.map') AS map,
                    json_extract(m.registration, '$.mode') AS mode,
                    r.received_at_ms, r.payload AS result
             FROM results r JOIN matches m ON m.id = r.match_id WHERE r.match_id = ?",
        )
        .bind(to_hex(id))
        .fetch_optional(&mut *self.connection().await?)
        .await
    }

    pub(crate) async fn settings(&self, guest_id: &str) -> Result<Map<String, Value>, sqlx::Error> {
        let settings: Option<Json<Map<String, Value>>> =
            sqlx::query_scalar("SELECT settings FROM player_settings WHERE guest_id = ?")
                .bind(guest_id)
                .fetch_optional(&mut *self.connection().await?)
                .await?;
        Ok(settings.map_or_else(Map::new, |Json(value)| value))
    }

    pub(crate) async fn replace_settings(
        &self,
        guest_id: &str,
        suggested_nick: &str,
        settings: &Map<String, Value>,
    ) -> Result<(), sqlx::Error> {
        let mut conn = self.connection().await?;
        let mut txn = conn.begin().await?;
        // Backfill sessions issued before guest profiles were stored. Keep existing nicknames.
        sqlx::query("INSERT INTO guests (id, nick) VALUES (?, ?) ON CONFLICT (id) DO NOTHING")
            .bind(guest_id)
            .bind(suggested_nick)
            .execute(&mut *txn)
            .await?;
        sqlx::query(
            "INSERT INTO player_settings (guest_id, settings) VALUES (?, ?)
             ON CONFLICT (guest_id) DO UPDATE SET settings = excluded.settings",
        )
        .bind(guest_id)
        .bind(Json(settings))
        .execute(&mut *txn)
        .await?;
        txn.commit().await
    }
}

const WAIT_LIMIT: Duration = Duration::from_secs(30);

async fn lock(
    conn: &Mutex<SqliteConnection>,
) -> Result<MutexGuard<'_, SqliteConnection>, sqlx::Error> {
    // Retain the previous pool's default wait limit. SQLx queues a dropped transaction's
    // rollback on the same worker before the next command from this connection.
    tokio::time::timeout(WAIT_LIMIT, conn.lock())
        .await
        .map_err(|_| sqlx::Error::PoolTimedOut)
}

pub(crate) struct TicketReservation<'a> {
    pub(crate) id: &'a MatchId,
    pub(crate) now_ms: u64,
    pub(crate) timeout_ms: u64,
    pub(crate) guest_id: &'a str,
    pub(crate) nick: &'a str,
    pub(crate) jti: &'a [u8; 16],
    pub(crate) ip: Option<IpAddr>,
    pub(crate) fingerprint: Option<String>,
}

async fn reserve_one(
    conn: &mut SqliteConnection,
    ticket_key: &[u8; 32],
    id: &str,
    cutoff: i64,
    guest_id: &str,
    nick: &str,
    jti: &str,
) -> Result<(Registration, u8), SlotError> {
    // Both nickname backfills and moderation identity associations commit with the slot.
    let mut txn = conn.begin().await?;
    let Some(slot) = advance_slot(&mut txn, ticket_key, id, cutoff).await? else {
        let status: Option<(bool, bool)> = sqlx::query_as(
            "SELECT ended = 0 AND ticket_key = ? AND last_heartbeat_ms >= ?,
                        json_extract(registration, '$.protocol_version') = ?
                 FROM matches WHERE id = ?",
        )
        .bind(ticket_key.as_slice())
        .bind(cutoff)
        .bind(PROTOCOL_VERSION)
        .bind(id)
        .fetch_optional(&mut *txn)
        .await?;
        return Err(match status {
            None => SlotError::Unknown,
            Some((false, _)) => SlotError::Stale,
            Some((true, false)) => SlotError::Incompatible,
            Some((true, true)) => SlotError::Full,
        });
    };
    sqlx::query(
        "INSERT INTO guests (id, nick) SELECT ?1, ?2 WHERE NOT EXISTS (
                SELECT 1 FROM guests WHERE id = ?1 AND nick = ?2)
             ON CONFLICT (id) DO UPDATE SET nick = excluded.nick",
    )
    .bind(guest_id)
    .bind(nick)
    .execute(&mut *txn)
    .await?;
    sqlx::query(
        "INSERT INTO moderation_tickets (match_id, jti, player_id, guest_id) VALUES (?, ?, ?, ?)",
    )
    .bind(id)
    .bind(jti)
    .bind(slot.1)
    .bind(guest_id)
    .execute(&mut *txn)
    .await?;
    txn.commit().await?;
    Ok(slot)
}

async fn advance_slot(
    conn: &mut SqliteConnection,
    ticket_key: &[u8; 32],
    id: &str,
    cutoff: i64,
) -> Result<Option<(Registration, u8)>, sqlx::Error> {
    let mut rows: Vec<(Json<Registration>, u8)> = sqlx::query_as(
        "UPDATE matches SET next_player = (next_player + 1) % 128
             WHERE id = ?1 AND ended = 0 AND ticket_key = ?2 AND last_heartbeat_ms >= ?3
                 AND json_extract(registration, '$.protocol_version') = ?4 AND humans + bots < cap
             RETURNING registration, (next_player + 127) % 128",
    )
    .bind(id)
    .bind(ticket_key.as_slice())
    .bind(cutoff)
    .bind(PROTOCOL_VERSION)
    // Drain RETURNING through statement completion.
    .fetch_all(conn)
    .await?;
    Ok(rows
        .pop()
        .map(|(Json(registration), player_id)| (registration, player_id)))
}
