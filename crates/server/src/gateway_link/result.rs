//! End-of-match report (M1 basics; scoreboard rows and persistence are M2).

use std::{
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::client::GatewayClient;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerResult {
    pub player: u8,
    pub kills: u16,
    pub deaths: u16,
    pub score: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatchResult {
    /// Humans that were connected at the end.
    pub players: u16,
    pub bots: u16,
    /// Capture points in the map (0 on the sandbox map).
    pub objectives: u16,
    pub duration_s: u32,
    pub ticks: u64,
    pub rows: Vec<PlayerResult>,
}

fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Publishes a complete JSON file, so interruption before rename leaves no partial result
/// eligible for replay.
pub fn spool(dir: &Path, match_id: &str, result: &MatchResult) -> io::Result<PathBuf> {
    if !valid_id(match_id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid match id",
        ));
    }
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("{match_id}.json"));
    let mut pending = tempfile::NamedTempFile::new_in(dir)?;
    serde_json::to_writer_pretty(&mut pending, result).map_err(io::Error::other)?;
    pending.as_file().sync_all()?;
    pending.persist(&path).map_err(|e| e.error)?;
    Ok(path)
}

/// Retry the previous match IDs against the same gateway. A failed POST (including an
/// unknown ID after a gateway restart) or malformed file leaves the result on disk.
pub async fn replay_spool(dir: &Path, client: &GatewayClient) {
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return,
        Err(e) => {
            tracing::warn!(event = "result_spool_unreadable", error = %e, "cannot scan result spool");
            return;
        }
    };
    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => return,
            Err(e) => {
                tracing::warn!(event = "result_spool_unreadable", error = %e, "cannot continue scanning result spool");
                return;
            }
        };
        let name = entry.file_name();
        let Some(id) = name
            .to_str()
            .and_then(|s| s.strip_suffix(".json"))
            .filter(|s| valid_id(s))
        else {
            continue;
        };
        let path = entry.path();
        let retry = async {
            if !entry.file_type().await?.is_file() {
                return Ok::<_, Box<dyn std::error::Error + Send + Sync>>(false);
            }
            let result: MatchResult = serde_json::from_slice(&tokio::fs::read(&path).await?)?;
            client.submit_result(id, &result).await?;
            tokio::fs::remove_file(&path).await?;
            Ok(true)
        }
        .await;
        match retry {
            Ok(true) => tracing::info!(
                event = "result_replayed",
                gateway_match_id = id,
                "delivered spooled result"
            ),
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(event = "result_replay_failed", gateway_match_id = id, error = %e, "could not replay or remove spooled result")
            }
        }
    }
}
