use serde::{Deserialize, Serialize};

use super::xp::MatchXp;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerStats {
    pub player: u8,
    /// Match-local incarnation of this slot; reconnects do not share a row or an XP balance.
    pub session: u32,
    pub team: u8,
    pub squad: Option<u16>,
    pub online_ticks: u32,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub kd_milli: Option<u64>,
    /// Combat, Offense, Defense, Support, respectively.
    pub categories: [i64; 4],
    pub total_score: i64,
    pub shots: u32,
    pub hits: u32,
    pub headshots: u32,
    pub hit_bps: u32,
    pub headshot_bps: u32,
    pub farthest_kill_mm: u32,
    pub longest_streak: u32,
    pub revives: u32,
    pub healed_hp: u64,
    pub build_supplies: u64,
    pub garrison_spawns: u32,
    pub repaired_hp: u64,
    pub spot_assists: u32,
    pub vehicles_destroyed: u32,
    pub vehicle_damage_hp: u64,
    pub driving_distance_mm: u64,
    pub role_xp: Vec<RoleXpDelta>,
    pub match_xp: MatchXp,
    /// Match-only level (zero prior XP until progression integration).
    pub account_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleXpDelta {
    pub role: u8,
    pub score: i64,
    /// Hundredths of XP; round only in the consumer, not once per score event.
    pub xp_hundredths: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerKey {
    pub player: u8,
    pub session: u32,
}

impl PlayerStats {
    pub fn key(&self) -> PlayerKey {
        PlayerKey {
            player: self.player,
            session: self.session,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquadRank {
    pub team: u8,
    pub squad: u16,
    pub score: i64,
    pub mvp: PlayerKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamRank {
    pub top: Vec<PlayerKey>,
    pub category_leaders: [Option<PlayerKey>; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRecord {
    pub tick: u32,
    pub objective: u16,
    pub owner: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceSample {
    pub tick: u32,
    /// US and GER, each ordered as manpower, munitions and fuel.
    pub teams: [[i32; 3]; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchSummary {
    pub map: u16,
    pub mode: String,
    pub tick_hz: u8,
    pub duration_ticks: u32,
    pub winner: Option<u8>,
    pub reason: Option<String>,
    pub players: Vec<PlayerStats>,
    pub timeline: Vec<CaptureRecord>,
    pub squad_ranking: Vec<SquadRank>,
    pub team_ranking: [TeamRank; 2],
    pub mvp: Option<PlayerKey>,
    pub resource_curve: Vec<ResourceSample>,
}
