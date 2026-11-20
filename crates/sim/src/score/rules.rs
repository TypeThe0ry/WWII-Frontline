use serde::Deserialize;
use std::collections::BTreeMap;

use super::xp::XpRules;

/// Stable C4 kind assignments: high 2 bits are the category, low 6 the reason.
macro_rules! score_kinds {
    ($($kind:ident = $id:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[repr(u8)]
        pub enum ScoreKind { $($kind = $id),+ }
        impl ScoreKind {
            pub const ALL: &'static [Self] = &[$(Self::$kind),+];
        }
    };
}

score_kinds! {
    Kill = 0, Headshot = 1, Assist = 2, Finish = 3, LightVehicle = 4, MediumVehicle = 5,
    HeavyVehicle = 6, SoftVehicle = 7, CrewMember = 8, GunDestroyed = 9, KillStreak = 10,
    SuppressionAssist = 11, TeamKill = 12, Revenge = 13,
    CaptureAdvance = 64, Capture = 65, AssaultCapture = 66, ForwardGarrisonSpawn = 67,
    GarrisonDestroyed = 68, NodeDestroyed = 69,
    ContestedDefense = 128, DefenseKill = 129, Repel = 130, GarrisonDefense = 131,
    Revive = 192, RepeatRevive = 193, Heal = 194, Resupply = 195, AmmoCrateUse = 196,
    Build = 197, NodeBuilt = 198, GarrisonBuild = 199, GarrisonBuilt = 200,
    GarrisonSpawn = 201, OutpostSpawn = 202, SupplyDelivery = 203, Repair = 204,
    ModuleRepair = 205, SpotAssist = 206, SpotterAssist = 207, ReconFlight = 208,
    ArtilleryKill = 209, ArtilleryLoad = 210, ArtilleryObserve = 211,
    CommanderStrike = 212, CommanderRally = 213, CommanderSupply = 214,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Category {
    Combat,
    Offense,
    Defense,
    Support,
}

impl ScoreKind {
    pub const fn category(self) -> Category {
        match self as u8 >> 6 {
            0 => Category::Combat,
            1 => Category::Offense,
            2 => Category::Defense,
            _ => Category::Support,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Award {
    #[serde(rename = "value_points")]
    pub points: i16,
    pub enabled: bool,
    #[serde(default = "one")]
    pub units: u32,
    pub action_cap_points: Option<u16>,
    pub minute_cap_points: Option<u16>,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Deserialize)]
pub struct ScoreRules {
    pub awards: BTreeMap<ScoreKind, Award>,
    pub periodic_s: u32,
    pub cap_window_s: u32,
    pub capture_participation_s: u32,
    pub repel_threshold_pct: u32,
    pub duplicate_kill_s: u32,
    pub duplicate_kill_count: usize,
    pub capture_cooldown_s: u32,
    pub revive_repeat_s: u32,
    pub streak_s: u32,
    pub streak_count: usize,
    pub revenge_s: u32,
    pub assist_s: u32,
    pub assist_min_hp: f32,
    pub xp: XpRules,
    pub xp_role_bonus_mult: f64,
    pub xp_role_bonus: BTreeMap<String, Vec<ScoreKind>>,
}

impl ScoreRules {
    pub fn embedded() -> &'static Self {
        static RULES: std::sync::OnceLock<ScoreRules> = std::sync::OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(include_str!("../../../../data/scoring.json"))
                .expect("validated data/scoring.json")
        })
    }
}
