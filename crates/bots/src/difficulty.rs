//! Difficulty tiers and behaviour tuning, read from `data/rules/bots.json` (never hard-coded).

use frontline_sim::data::defs::{BotDifficultyDef, BotsRulesDef};

/// Sim tick rate the bots are written for.
pub const TICK_HZ: f32 = 30.0;

/// One difficulty tier (GAMEDESIGN 20.6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Difficulty {
    /// Reaction delay range `(min, max)` in ms from recognising an enemy to the first shot.
    pub reaction_ms: (u16, u16),
    pub aim_error_mult: f32,
    /// How long a lost target is chased.
    pub memory_s: f32,
    /// Maximum distance at which an enemy can be seen.
    pub view_distance_m: f32,
}

impl Difficulty {
    pub fn from_def(d: &BotDifficultyDef) -> Self {
        Difficulty {
            reaction_ms: (d.reaction_ms[0] as u16, d.reaction_ms[1] as u16),
            aim_error_mult: d.aim_error_mult as f32,
            memory_s: d.memory_s as f32,
            view_distance_m: d.view_distance_m as f32,
        }
    }

    /// `recruit`, `regular` or `veteran`.
    pub fn named(rules: &BotsRulesDef, name: &str) -> Option<Self> {
        let d = &rules.difficulties;
        Some(Self::from_def(match name {
            "recruit" => &d.recruit,
            "regular" => &d.regular,
            "veteran" => &d.veteran,
            _ => return None,
        }))
    }

    /// The tier named by `default_difficulty` (M0: Regular).
    pub fn default_from(rules: &BotsRulesDef) -> Self {
        Self::named(rules, &rules.default_difficulty)
            .unwrap_or_else(|| Self::from_def(&rules.difficulties.regular))
    }

    /// The default tier of the embedded game data.
    pub fn embedded_default() -> Self {
        Self::default_from(&frontline_sim::GameData::embedded().rules.bots)
    }
}

/// All numeric behaviour parameters, converted to `f32` / ticks once.
#[derive(Debug, Clone, PartialEq)]
pub struct Tuning {
    pub think_every_ticks: u32,
    pub fov_deg: f32,
    pub view_distance_cap_m: f32,
    pub min_recognition: f32,
    pub cell_m: f32,
    pub max_slope_deg: f32,
    pub goal_radius_m: f32,
    pub repath_min_ticks: u32,
    pub blind_fire_max_m: f32,
    pub turn_rate_deg_s: f32,
    pub aim_error_base_deg: f32,
    pub aim_error_per_100m_deg: f32,
    pub fire_tolerance_deg: f32,
    pub crouch_min_m: f32,
    pub sprint_min_m: f32,
    pub approach_beyond_m: f32,
    pub stuck_window_ticks: u32,
    pub stuck_min_move_m: f32,
}

fn ms_to_ticks(ms: f64) -> u32 {
    ((ms * f64::from(TICK_HZ) / 1000.0).round() as u32).max(1)
}

impl Tuning {
    pub fn from_rules(r: &BotsRulesDef) -> Self {
        Tuning {
            think_every_ticks: ms_to_ticks(r.think_interval_ms),
            fov_deg: r.perception.fov_deg as f32,
            view_distance_cap_m: r.perception.view_distance_cap_m as f32,
            min_recognition: r.perception.min_recognition_mult as f32,
            cell_m: r.nav.cell_size_m as f32,
            max_slope_deg: r.nav.max_slope_deg as f32,
            goal_radius_m: r.nav.goal_radius_m as f32,
            repath_min_ticks: ms_to_ticks(r.nav.repath_min_interval_ms),
            blind_fire_max_m: r.combat.blind_fire_max_m as f32,
            turn_rate_deg_s: r.combat.turn_rate_deg_s as f32,
            aim_error_base_deg: r.combat.aim_error_base_deg as f32,
            aim_error_per_100m_deg: r.combat.aim_error_per_100m_deg as f32,
            fire_tolerance_deg: r.combat.fire_tolerance_deg as f32,
            crouch_min_m: r.combat.crouch_min_m as f32,
            sprint_min_m: r.combat.sprint_min_m as f32,
            approach_beyond_m: r.combat.approach_beyond_m as f32,
            stuck_window_ticks: ms_to_ticks(r.stuck.window_s * 1000.0),
            stuck_min_move_m: r.stuck.min_move_m as f32,
        }
    }

    pub fn embedded() -> Self {
        Self::from_rules(&frontline_sim::GameData::embedded().rules.bots)
    }
}
