//! End-of-match report (M1 basics; scoreboard rows and persistence are M2).

use serde::{Deserialize, Serialize};

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

/// Writes the result to `<dir>/<match_id>.json` for a retry on the next start. Returns the path.
pub fn spool(
    dir: &std::path::Path,
    match_id: &str,
    r: &MatchResult,
) -> std::io::Result<std::path::PathBuf> {
    std::fs::create_dir_all(dir)?;
    let safe: String = match_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    let path = dir.join(format!("{safe}.json"));
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(r).map_err(std::io::Error::other)?,
    )?;
    Ok(path)
}
