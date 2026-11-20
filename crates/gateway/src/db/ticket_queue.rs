//! Batch only ready, unchanged-nickname ticket writes; do not wait to fill a batch.

use std::sync::Arc;
use std::time::Instant;

use prometheus::Histogram;
use sqlx::types::Json;
use sqlx::{Connection, SqliteConnection};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{timeout_at, Instant as Deadline};

use super::{lock, reserve_one, Registration, SlotError, WAIT_LIMIT};
use crate::metrics::Metrics;
use frontline_protocol::PROTOCOL_VERSION;

const QUEUE_SIZE: usize = 512;
const BATCH_SIZE: usize = 32;
const CURSOR_SIZE: usize = 128;

type Slot = (Registration, u8);

pub(super) struct Queue(mpsc::Sender<Request>);

struct Request {
    id: String,
    cutoff: i64,
    guest_id: String,
    nick: String,
    queued: Instant,
    wait: Histogram,
    transaction: Histogram,
    claimed: Option<oneshot::Sender<Result<(), sqlx::Error>>>,
    reply: oneshot::Sender<Result<Slot, SlotError>>,
}

impl Queue {
    pub(super) fn start(connection: Arc<Mutex<SqliteConnection>>, ticket_key: [u8; 32]) -> Self {
        let (tx, rx) = mpsc::channel(QUEUE_SIZE);
        tokio::spawn(run(connection, ticket_key, rx));
        Self(tx)
    }

    pub(super) async fn reserve(
        &self,
        id: String,
        cutoff: i64,
        guest_id: &str,
        nick: &str,
        metrics: &Metrics,
    ) -> Result<Slot, SlotError> {
        let (reply, response) = oneshot::channel();
        let (claim, claimed) = oneshot::channel();
        let deadline = Deadline::now() + WAIT_LIMIT;
        let request = Request {
            id,
            cutoff,
            guest_id: guest_id.to_owned(),
            nick: nick.to_owned(),
            queued: Instant::now(),
            wait: metrics.ticket_pool_wait.clone(),
            transaction: metrics.ticket_transaction.clone(),
            claimed: Some(claim),
            reply,
        };
        timeout_at(deadline, self.0.send(request))
            .await
            .map_err(|_| sqlx::Error::PoolTimedOut)?
            .map_err(|_| sqlx::Error::WorkerCrashed)?;
        // The 30 s limit covers queueing, not execution, just like pool acquisition.
        timeout_at(deadline, claimed)
            .await
            .map_err(|_| sqlx::Error::PoolTimedOut)?
            .map_err(|_| sqlx::Error::WorkerCrashed)??;
        response.await.map_err(|_| sqlx::Error::WorkerCrashed)?
    }
}

async fn run(
    connection: Arc<Mutex<SqliteConnection>>,
    ticket_key: [u8; 32],
    mut rx: mpsc::Receiver<Request>,
) {
    let mut pending = Vec::with_capacity(BATCH_SIZE);
    while rx.recv_many(&mut pending, BATCH_SIZE).await != 0 {
        let mut conn = match lock(&connection).await {
            Ok(conn) => conn,
            Err(_) => {
                for mut req in pending.drain(..) {
                    req.wait.observe(req.queued.elapsed().as_secs_f64());
                    if let Some(claim) = req.claimed.take() {
                        let _ = claim.send(Err(sqlx::Error::PoolTimedOut));
                    }
                }
                continue;
            }
        };
        pending.retain_mut(|req| {
            let waited = req.queued.elapsed();
            req.wait.observe(waited.as_secs_f64());
            let Some(claim) = req.claimed.take() else {
                return false;
            };
            if waited >= WAIT_LIMIT {
                let _ = claim.send(Err(sqlx::Error::PoolTimedOut));
                return false;
            }
            !req.reply.is_closed() && claim.send(Ok(())).is_ok()
        });
        let started = Instant::now();
        let slots = if pending.len() > 1 {
            let inputs: Vec<_> = pending
                .iter()
                .map(|req| {
                    (
                        req.id.as_str(),
                        req.cutoff,
                        req.guest_id.as_str(),
                        req.nick.as_str(),
                    )
                })
                .collect();
            match reserve_batch(&mut conn, &ticket_key, &inputs).await {
                Ok(slots) => slots,
                Err(err) => {
                    for req in pending.drain(..) {
                        req.transaction.observe(started.elapsed().as_secs_f64());
                        let _ = req
                            .reply
                            .send(Err(sqlx::Error::Protocol(err.to_string()).into()));
                    }
                    continue;
                }
            }
        } else {
            vec![None; pending.len()]
        };
        // Reply to committed reservations before potentially slower fallback writes.
        let batch_elapsed = started.elapsed().as_secs_f64();
        let mut misses = Vec::new();
        for (req, slot) in pending.drain(..).zip(slots) {
            if let Some(slot) = slot {
                req.transaction.observe(batch_elapsed);
                let _ = req.reply.send(Ok(slot));
            } else {
                misses.push(req);
            }
        }
        for req in misses {
            if req.reply.is_closed() {
                continue;
            }
            let result = reserve_one(
                &mut conn,
                &ticket_key,
                &req.id,
                req.cutoff,
                &req.guest_id,
                &req.nick,
            )
            .await;
            req.transaction.observe(started.elapsed().as_secs_f64());
            let _ = req.reply.send(result);
        }
    }
}

async fn reserve_batch(
    conn: &mut SqliteConnection,
    key: &[u8; 32],
    inputs: &[(&str, i64, &str, &str)],
) -> Result<Vec<Option<Slot>>, sqlx::Error> {
    let mut txn = conn.begin().await?;
    // Materialize request eligibility before moving any cursor. Each returned index is
    // a request ordinal; the old cursor follows from the new cursor and the group size.
    let rows: Vec<(Json<Registration>, u8, Json<Vec<usize>>)> = sqlx::query_as(
        "WITH eligible AS MATERIALIZED (
            SELECT request.key AS ordinal, m.id AS id
            FROM json_each(?1) AS request
            JOIN matches AS m ON m.id = json_extract(request.value, '$[0]')
            JOIN guests AS g ON g.id = json_extract(request.value, '$[2]')
            WHERE m.ended = 0 AND m.ticket_key = ?2
                AND m.last_heartbeat_ms >= json_extract(request.value, '$[1]')
                AND json_extract(m.registration, '$.protocol_version') = ?3
                AND m.humans + m.bots < m.cap AND g.nick = json_extract(request.value, '$[3]')
        ), counts AS MATERIALIZED (
            SELECT id, count(*) AS n FROM eligible GROUP BY id
        )
        UPDATE matches SET next_player = (next_player + counts.n) % 128
        FROM counts WHERE matches.id = counts.id
        RETURNING registration, next_player, (SELECT json_group_array(ordinal ORDER BY ordinal)
            FROM eligible WHERE eligible.id = matches.id)",
    )
    .bind(Json(inputs))
    .bind(key.as_slice())
    .bind(PROTOCOL_VERSION)
    .fetch_all(&mut *txn)
    .await?;

    let mut slots = vec![None; inputs.len()];
    for (Json(registration), end, Json(ordinals)) in rows {
        let start = (usize::from(end) + CURSOR_SIZE - ordinals.len()) % CURSOR_SIZE;
        for (offset, ordinal) in ordinals.into_iter().enumerate() {
            slots[ordinal] = Some((registration.clone(), ((start + offset) % CURSOR_SIZE) as u8));
        }
    }
    txn.commit().await?;
    Ok(slots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[tokio::test]
    async fn cancelled_queued_request_does_not_advance_cursor() {
        use std::future::{poll_fn, Future};
        use std::task::Poll;

        let db = Database::open("sqlite::memory:", [7; 32]).await.unwrap();
        let match_id = [7; 16];
        let guest = "0".repeat(32);
        db.guest(&guest, "Guest").await.unwrap();
        db.register(&match_id, registration("a", PROTOCOL_VERSION), 100)
            .await
            .unwrap();
        let held = db.connection().await.unwrap();
        let metrics = Metrics::default();
        {
            let mut cancelled =
                std::pin::pin!(db.reserve_slot(&match_id, 100, 5, &guest, "Guest", &metrics,));
            poll_fn(|cx| {
                assert!(cancelled.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
        }
        drop(held);
        let (_, id) = db
            .reserve_slot(&match_id, 100, 5, &guest, "Guest", &metrics)
            .await
            .unwrap();
        assert_eq!(id, 0);
    }

    fn registration(name: &str, version: u16) -> Registration {
        Registration {
            name: name.into(),
            map: "m0".into(),
            mode: "sandbox".into(),
            region: "local".into(),
            tags: vec![],
            version: "test".into(),
            protocol_version: version,
            data_hash: 1,
            cap: 16,
            addr: "127.0.0.1".into(),
            ws_port: 8081,
            wt_port: 8082,
        }
    }

    #[tokio::test]
    async fn batch_advances_only_eligible_requests_with_per_match_wrap() {
        let key = [7; 32];
        let db = Database::open("sqlite::memory:", key).await.unwrap();
        let guest = "0".repeat(32);
        db.guest(&guest, "Guest").await.unwrap();
        let mut conn = db.connection().await.unwrap();
        for (id, version, ticket_key, humans, ended) in [
            ('a', PROTOCOL_VERSION, key, 0, false),
            ('b', PROTOCOL_VERSION, key, 0, false),
            ('c', PROTOCOL_VERSION, key, 16, false),
            ('d', PROTOCOL_VERSION + 1, key, 0, false),
            ('e', PROTOCOL_VERSION, [8; 32], 0, false),
            ('f', PROTOCOL_VERSION, key, 0, true),
        ] {
            let registration = registration(&id.to_string(), version);
            sqlx::query(
                "INSERT INTO matches (id, registration, ticket_key, cap, last_heartbeat_ms,
                 humans, ended, next_player) VALUES (?, ?, ?, 16, 100, ?, ?, 127)",
            )
            .bind(id.to_string().repeat(32))
            .bind(Json(registration))
            .bind(ticket_key.as_slice())
            .bind(humans)
            .bind(ended)
            .execute(&mut *conn)
            .await
            .unwrap();
        }
        let inputs: Vec<_> = [
            ('a', 100, "Guest"),
            ('b', 100, "Guest"),
            ('a', 101, "Guest"),
            ('a', 100, "Renamed"),
            ('a', 100, "Guest"),
            ('c', 100, "Guest"),
            ('d', 100, "Guest"),
            ('e', 100, "Guest"),
            ('f', 100, "Guest"),
            ('0', 100, "Guest"),
        ]
        .into_iter()
        .map(|(id, cutoff, nick)| (id.to_string().repeat(32), cutoff, nick))
        .collect();
        let inputs: Vec<_> = inputs
            .iter()
            .map(|(id, cutoff, nick)| (id.as_str(), *cutoff, guest.as_str(), *nick))
            .collect();
        let slots: Vec<_> = reserve_batch(&mut conn, &key, &inputs)
            .await
            .unwrap()
            .into_iter()
            .map(|slot| slot.map(|(reg, player)| (reg.name, player)))
            .collect();
        assert_eq!(
            slots,
            [
                Some(("a".into(), 127)),
                Some(("b".into(), 127)),
                None,
                None,
                Some(("a".into(), 0)),
                None,
                None,
                None,
                None,
                None
            ]
        );
        let cursors: Vec<u8> = sqlx::query_scalar("SELECT next_player FROM matches ORDER BY id")
            .fetch_all(&mut *conn)
            .await
            .unwrap();
        assert_eq!(cursors, [1, 0, 127, 127, 127, 127]);
    }
}
