//! Match rules (M1-018): strongpoint capture (GAMEDESIGN 3.2) and the match state machine
//! (GAMEDESIGN 2.2) with the M1 simplified win rule (PI-059).

pub mod capture;
pub mod match_state;
pub mod objectives;

pub use capture::{capture_rate, eff, eff_with, CaptureParams};
pub use match_state::{
    match_state, MatchCfg, MatchPhaseKind, MatchResult, MatchState, VictoryReason,
};
pub use objectives::{objectives, Objective, ObjectiveSpec, Objectives};

use bevy_ecs::prelude::*;

use crate::components::Team;

/// Why an objective changed hands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureReason {
    /// `|p|` reached 100.
    Captured,
}

/// Rule outputs of a tick, drained into `StepOutput::rule_changes`.
#[derive(Resource, Debug, Default)]
pub struct RuleChangeBuf(pub Vec<crate::step::RuleChange>);

/// Sign of `p` for a team: US is the positive side.
pub fn side(team: Team) -> f32 {
    match team {
        Team::Us => 1.0,
        Team::Ger => -1.0,
    }
}
