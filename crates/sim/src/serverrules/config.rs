use std::sync::OnceLock;

use crate::hash::HashWriter;
use serde::{Deserialize, Serialize};

/// Tunables in seconds, counts and integer percentages. No wall-clock time or floats enter sim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleConfig {
    pub balance: BalanceConfig,
    pub tk: TkConfig,
    pub vote: VoteConfig,
    pub afk: AfkConfig,
}

impl Default for RuleConfig {
    fn default() -> Self {
        static DEFAULT: OnceLock<RuleConfig> = OnceLock::new();
        *DEFAULT.get_or_init(|| {
            serde_json::from_str(include_str!("../../../../data/server_rules.json"))
                .expect("valid embedded server_rules.json")
        })
    }
}

impl RuleConfig {
    pub(super) fn hash_into(&self, w: &mut HashWriter) {
        macro_rules! fields {
            ($config:ident, $write:ident, $($field:ident),+ $(,)?) => {
                $(w.$write(self.$config.$field);)+
            };
        }
        fields!(
            balance,
            write_u8,
            team_max_players,
            join_max_difference,
            trigger_difference,
            volunteer_xp_bonus_percent,
            party_max_players
        );
        fields!(
            balance,
            write_u16,
            switch_min_after_join_s,
            switch_cooldown_s,
            check_period_s,
            persist_s,
            offer_s,
            recent_move_s
        );
        fields!(
            tk,
            write_u8,
            warning_strikes,
            rifleman_strikes,
            kick_match_strikes
        );
        fields!(
            tk,
            write_u16,
            history_s,
            prompt_s,
            strike_window_s,
            lock_s,
            kick_cooldown_s
        );
        fields!(
            vote,
            write_u8,
            kick_yes_percent,
            kick_min_yes,
            kick_no_multiplier,
            commander_min_officers,
            commander_yes_percent,
            mute_min_yes
        );
        fields!(
            vote,
            write_u16,
            kick_min_match_age_s,
            kick_start_cooldown_s,
            kick_duration_s,
            commander_duration_s,
            commander_min_tenure_s,
            mute_duration_s,
            map_duration_s
        );
        fields!(afk, write_u8, repeat_occurrences);
        fields!(
            afk,
            write_u16,
            warning_s,
            alive_s,
            leader_s,
            countdown_s,
            deploy_s,
            repeat_window_s
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BalanceConfig {
    pub team_max_players: u8,
    #[serde(rename = "join_max_difference_players")]
    pub join_max_difference: u8,
    pub switch_min_after_join_s: u16,
    pub switch_cooldown_s: u16,
    pub check_period_s: u16,
    #[serde(rename = "trigger_difference_players")]
    pub trigger_difference: u8,
    pub persist_s: u16,
    pub offer_s: u16,
    pub recent_move_s: u16,
    #[serde(rename = "volunteer_xp_bonus_pct")]
    pub volunteer_xp_bonus_percent: u8,
    pub party_max_players: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TkConfig {
    pub history_s: u16,
    pub prompt_s: u16,
    pub strike_window_s: u16,
    #[serde(rename = "warning_strike_count")]
    pub warning_strikes: u8,
    #[serde(rename = "rifleman_strike_count")]
    pub rifleman_strikes: u8,
    #[serde(rename = "kick_match_strike_count")]
    pub kick_match_strikes: u8,
    pub lock_s: u16,
    pub kick_cooldown_s: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoteConfig {
    pub kick_min_match_age_s: u16,
    pub kick_start_cooldown_s: u16,
    pub kick_duration_s: u16,
    #[serde(rename = "kick_yes_pct")]
    pub kick_yes_percent: u8,
    #[serde(rename = "kick_min_yes_players")]
    pub kick_min_yes: u8,
    #[serde(rename = "kick_no_mult")]
    pub kick_no_multiplier: u8,
    pub commander_duration_s: u16,
    pub commander_min_tenure_s: u16,
    #[serde(rename = "commander_min_officer_players")]
    pub commander_min_officers: u8,
    #[serde(rename = "commander_yes_pct")]
    pub commander_yes_percent: u8,
    pub mute_duration_s: u16,
    #[serde(rename = "mute_min_yes_players")]
    pub mute_min_yes: u8,
    pub map_duration_s: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AfkConfig {
    pub warning_s: u16,
    pub alive_s: u16,
    pub leader_s: u16,
    pub countdown_s: u16,
    pub deploy_s: u16,
    pub repeat_window_s: u16,
    #[serde(rename = "repeat_count")]
    pub repeat_occurrences: u8,
}
