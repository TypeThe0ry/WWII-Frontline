//! Registered match servers (heartbeat-tracked) and reported results, in memory. SQLite
//! persistence of guests / matches / results is M1-028b; the gateway keeps no real-time state.

use std::collections::BTreeMap;
use std::sync::Mutex;

use frontline_protocol::dto::ServerInfo;

pub type MatchId = [u8; 16];

pub fn to_hex(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn from_hex(s: &str) -> Option<MatchId> {
    if s.len() != 32 || !s.is_ascii() {
        return None;
    }
    let mut out = [0u8; 16];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

#[derive(Debug, Clone)]
pub struct MatchEntry {
    pub id: MatchId,
    pub name: String,
    pub map: String,
    pub mode: String,
    pub region: String,
    pub tags: Vec<String>,
    pub version: String,
    pub protocol_version: u16,
    pub data_hash: u64,
    pub cap: u8,
    pub addr: String,
    pub ws_port: u16,
    pub wt_port: u16,
    pub phase: String,
    pub time_left_s: u32,
    pub humans: u8,
    pub bots: u8,
    pub last_heartbeat_ms: u64,
    pub ended: bool,
    /// Next `player_id` offered in a ticket (wraps inside the 128-slot id space).
    pub next_player: u8,
}

#[derive(Default)]
pub struct Registry {
    matches: Mutex<BTreeMap<MatchId, MatchEntry>>,
    results: Mutex<Vec<(MatchId, serde_json::Value)>>,
}

impl Registry {
    pub fn register(&self, e: MatchEntry) {
        self.matches
            .lock()
            .unwrap_or_else(|x| x.into_inner())
            .insert(e.id, e);
    }

    /// Updates a match; `false` when the id is unknown.
    pub fn heartbeat(&self, id: &MatchId, now_ms: u64, f: impl FnOnce(&mut MatchEntry)) -> bool {
        let mut g = self.matches.lock().unwrap_or_else(|x| x.into_inner());
        match g.get_mut(id) {
            Some(e) => {
                f(e);
                e.last_heartbeat_ms = now_ms;
                true
            }
            None => false,
        }
    }

    pub fn exists(&self, id: &MatchId) -> bool {
        self.matches
            .lock()
            .unwrap_or_else(|x| x.into_inner())
            .contains_key(id)
    }

    /// Servers that are live: not ended and heartbeat within `timeout_ms`.
    pub fn list(&self, now_ms: u64, timeout_ms: u64, protocol: u16) -> Vec<ServerInfo> {
        let g = self.matches.lock().unwrap_or_else(|x| x.into_inner());
        g.values()
            .filter(|e| !e.ended && now_ms.saturating_sub(e.last_heartbeat_ms) <= timeout_ms)
            .map(|e| ServerInfo {
                match_id: e.id,
                name: e.name.clone(),
                map: e.map.clone(),
                mode: e.mode.clone(),
                phase: e.phase.clone(),
                time_left_s: e.time_left_s,
                humans: e.humans,
                bots: e.bots,
                cap: e.cap,
                region: e.region.clone(),
                tags: e.tags.clone(),
                version: e.version.clone(),
                compatible: e.protocol_version == protocol,
            })
            .collect()
    }

    /// The live match `id` and its next ticket `player_id`, if it can take another player.
    pub fn reserve_slot(
        &self,
        id: &MatchId,
        now_ms: u64,
        timeout_ms: u64,
    ) -> Result<(MatchEntry, u8), SlotError> {
        let mut g = self.matches.lock().unwrap_or_else(|x| x.into_inner());
        let e = g.get_mut(id).ok_or(SlotError::Unknown)?;
        if e.ended || now_ms.saturating_sub(e.last_heartbeat_ms) > timeout_ms {
            return Err(SlotError::Stale);
        }
        if u16::from(e.humans) + u16::from(e.bots) >= u16::from(e.cap) {
            return Err(SlotError::Full);
        }
        let p = e.next_player % 128;
        e.next_player = (p + 1) % 128;
        Ok((e.clone(), p))
    }

    pub fn store_result(&self, id: &MatchId, v: serde_json::Value) -> bool {
        let mut m = self.matches.lock().unwrap_or_else(|x| x.into_inner());
        let Some(e) = m.get_mut(id) else {
            return false;
        };
        e.ended = true;
        drop(m);
        self.results
            .lock()
            .unwrap_or_else(|x| x.into_inner())
            .push((*id, v));
        true
    }

    pub fn results(&self) -> Vec<(MatchId, serde_json::Value)> {
        self.results
            .lock()
            .unwrap_or_else(|x| x.into_inner())
            .clone()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SlotError {
    Unknown,
    Stale,
    Full,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let id = [0xAB; 16];
        assert_eq!(from_hex(&to_hex(&id)), Some(id));
        assert_eq!(from_hex("zz"), None);
        assert_eq!(from_hex(&"é".repeat(16)), None);
    }
}
