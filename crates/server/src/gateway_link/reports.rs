//! Submit reports on a Tokio worker. The sim only enqueues bounded in-memory evidence.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};

use super::client::{bounded_json, GatewayClient, LinkError};
use super::{lower_hex, parse_lower_hex};

pub const QUEUE_CAPACITY: usize = 8;

#[derive(Clone, Debug, Serialize)]
pub struct ChatLine {
    /// Player id from the author's signed ticket.
    pub(crate) player_id: u8,
    pub(crate) text: String,
}

/// Full in-memory chat snapshots, coalesced before sending. A failed PUT retries the latest
/// snapshot at the next cadence. The gateway owns match-end eviction and retention.
pub async fn run_live_chat(
    client: GatewayClient,
    match_id: String,
    mut updates: watch::Receiver<Arc<Vec<ChatLine>>>,
    mut stop: watch::Receiver<bool>,
) {
    #[derive(Serialize)]
    struct Snapshot<'a> {
        messages: &'a [ChatLine],
    }
    if parse_lower_hex(&match_id).is_none() {
        return;
    }
    let path = format!("/internal/matches/{match_id}/chat");
    let upload = async {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut failed = false;
        loop {
            if !failed && updates.changed().await.is_err() {
                return;
            }
            interval.tick().await;
            let latest = updates.borrow_and_update().clone();
            match client.put(&path, &Snapshot { messages: &latest }).await {
                Ok(_) => failed = false,
                Err(error) => {
                    if !failed {
                        tracing::warn!(event = "live_chat_upload_failed", %error);
                    }
                    failed = true;
                }
            }
        }
    };
    tokio::select! {
        _ = stop.wait_for(|v| *v) => {},
        _ = upload => {},
    }
}

#[derive(Debug)]
pub struct ReportJob {
    pub(crate) reporter_jti: [u8; 16],
    pub(crate) target_jti: [u8; 16],
    pub(crate) reason: u8,
    pub(crate) note: String,
    pub(crate) replay: Option<Vec<u8>>,
    pub(crate) chat_excerpt: Vec<ChatLine>,
    pub(crate) following_chat: oneshot::Receiver<Vec<ChatLine>>,
}

#[derive(Serialize)]
struct ReportRequest<'a> {
    match_id: &'a str,
    reporter_jti: String,
    target_jti: String,
    reason: u8,
    note: &'a str,
    #[serde(skip_serializing_if = "<[ChatLine]>::is_empty")]
    chat_excerpt: &'a [ChatLine],
}

/// `spool_dir` is private server storage. References are relative to it and contain no player ids.
pub async fn run(
    client: GatewayClient,
    match_id: String,
    spool_dir: PathBuf,
    mut reports: mpsc::Receiver<ReportJob>,
) {
    if parse_lower_hex(&match_id).is_none() {
        tracing::error!(event = "report_worker_invalid_match");
        return;
    }
    #[derive(Deserialize)]
    struct Created {
        id: i64,
    }
    let mut contexts = tokio::task::JoinSet::new();
    while let Some(report) = reports.recv().await {
        while contexts.try_join_next().is_some() {}
        let request = ReportRequest {
            match_id: &match_id,
            reporter_jti: lower_hex(&report.reporter_jti),
            target_jti: lower_hex(&report.target_jti),
            reason: report.reason,
            note: &report.note,
            chat_excerpt: &report.chat_excerpt,
        };
        match client.post("/internal/report", &request).await {
            Ok(r) if r.status().as_u16() == 201 => {
                let id = match bounded_json::<Created>(r, 1024).await {
                    Ok(created) if created.id > 0 => created.id,
                    _ => {
                        tracing::warn!(event = "report_invalid_creation_response");
                        continue;
                    }
                };
                if let Some(bytes) = report.replay {
                    preserve_replay(&client, &match_id, &spool_dir, id, bytes).await;
                }
                if contexts.len() < 128 {
                    let client = client.clone();
                    let path = format!("/internal/matches/{match_id}/reports/{id}/chat");
                    contexts.spawn(async move {
                        #[derive(Serialize)]
                        struct ChatContext {
                            messages: Vec<ChatLine>,
                        }
                        let Ok(following) = report.following_chat.await else {
                            return;
                        };
                        if following.is_empty() {
                            return;
                        }
                        let mut messages = report.chat_excerpt;
                        messages.extend(following);
                        if let Err(error) = client.put(&path, &ChatContext { messages }).await {
                            tracing::warn!(event = "report_chat_context_upload_failed", %error);
                        }
                    });
                }
            }
            Ok(r) if r.status().as_u16() == 200 => {}
            Ok(r) => tracing::warn!(
                event = "report_unexpected_status",
                status = r.status().as_u16()
            ),
            Err(LinkError::Status(429)) => tracing::debug!(event = "report_gateway_rate_limited"),
            Err(e) => tracing::warn!(event = "report_delivery_failed", error = %e),
        }
    }
    while contexts.join_next().await.is_some() {}
}

async fn preserve_replay(
    client: &GatewayClient,
    match_id: &str,
    spool_dir: &Path,
    id: i64,
    bytes: Vec<u8>,
) {
    let replay_ref = format!("replays/{match_id}_{id}.bin");
    let path = spool_dir.join(&replay_ref);
    let result = tokio::task::spawn_blocking(move || persist(&path, &bytes)).await;
    if !matches!(result, Ok(Ok(()))) {
        tracing::warn!(event = "report_evidence_write_failed", report_id = id, error = ?result);
        return;
    }
    #[derive(Serialize)]
    struct Replay<'a> {
        replay_ref: &'a str,
    }
    let path = format!("/internal/matches/{match_id}/reports/{id}/replay");
    let body = Replay {
        replay_ref: &replay_ref,
    };
    for attempt in 0..3 {
        match client.put(&path, &body).await {
            Ok(_) => return,
            Err(LinkError::Http(_) | LinkError::Status(429 | 500..=599)) if attempt < 2 => {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            Err(error) => {
                tracing::warn!(event = "report_evidence_attach_failed", report_id = id, %error);
                return;
            }
        }
    }
}

fn persist(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path
        .parent()
        .expect("a replay always has a parent directory");
    std::fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist_noclobber(path).map_err(|e| e.error)?;
    Ok(())
}
