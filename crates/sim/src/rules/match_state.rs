//! Match state machine (GAMEDESIGN 2.2): `Idle -> Warmup -> Live -> Ended`, M1 win rule.

use bevy_ecs::prelude::*;

use super::capture::CaptureParams;
use super::objectives::Objectives;
use super::RuleChangeBuf;
use crate::components::{CurrentTick, SimCfgRes, Team};
use crate::step::RuleChange;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MatchPhaseKind {
    #[default]
    Idle = 0,
    Warmup = 1,
    Live = 2,
    /// Reserved (wire `Phase::Overtime`).
    Overtime = 3,
    Ended = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VictoryReason {
    /// One side holds every strongpoint.
    FullControl,
    /// Time ran out: more points, else larger `p` sum.
    TimeLimitLead,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchResult {
    pub reason: VictoryReason,
    pub winner: Option<Team>,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct MatchState {
    pub phase: MatchPhaseKind,
    pub phase_start: u32,
    /// Tick at which the phase ends (warmup / live); 0 = open ended.
    pub ends_at: u32,
    pub result: Option<MatchResult>,
    /// Connected human players (fed by the server; bots do not count).
    pub humans: u8,
}

/// Match rules in ticks plus the capture coefficients.
#[derive(Resource, Debug, Clone, Copy)]
pub struct MatchCfg {
    pub warmup_ticks: u32,
    pub early_start_players: u8,
    pub limit_ticks: u32,
    pub freeze_ticks: u32,
    pub ended_ticks: u32,
    pub repeat_ticks: u32,
    pub capture: CaptureParams,
}

impl MatchCfg {
    pub fn from_data(d: &crate::data::GameData, hz: u8) -> Self {
        let m = &d.rules.match_m1;
        let t = |s: f64| libm::ceil(s * f64::from(hz)) as u32;
        Self {
            warmup_ticks: t(m.warmup_s),
            early_start_players: m.early_start_players.min(255) as u8,
            limit_ticks: t(m.time_limit_s),
            freeze_ticks: t(m.ended_freeze_s),
            ended_ticks: t(m.ended_total_s),
            repeat_ticks: t(m.repeat_capture_s),
            capture: CaptureParams::from_def(&d.rules.capture),
        }
    }
}

/// Phase transitions and the win check (after the capture stage).
pub fn match_state(
    tick: Res<CurrentTick>,
    _cfg: Res<SimCfgRes>,
    mcfg: Res<MatchCfg>,
    mut ms: ResMut<MatchState>,
    objs: Res<Objectives>,
    mut out: ResMut<RuleChangeBuf>,
) {
    let now = tick.0 .0;
    match ms.phase {
        MatchPhaseKind::Idle | MatchPhaseKind::Overtime | MatchPhaseKind::Ended => {}
        MatchPhaseKind::Warmup => {
            if ms.humans >= mcfg.early_start_players || now >= ms.ends_at {
                ms.phase = MatchPhaseKind::Live;
                ms.phase_start = now;
                ms.ends_at = now + mcfg.limit_ticks;
                out.0.push(RuleChange::MatchPhase {
                    phase: MatchPhaseKind::Live,
                    ends_at: ms.ends_at,
                });
            }
        }
        MatchPhaseKind::Live => {
            let held = |t: Team| objs.0.iter().filter(|o| o.owner == Some(t)).count();
            let total = objs.0.len();
            let result = if total > 0 && held(Team::Us) == total {
                Some(MatchResult {
                    reason: VictoryReason::FullControl,
                    winner: Some(Team::Us),
                })
            } else if total > 0 && held(Team::Ger) == total {
                Some(MatchResult {
                    reason: VictoryReason::FullControl,
                    winner: Some(Team::Ger),
                })
            } else if now >= ms.ends_at {
                let (u, g) = (held(Team::Us), held(Team::Ger));
                let sum: f32 = objs.0.iter().map(|o| o.p).sum();
                let winner = if u != g {
                    Some(if u > g { Team::Us } else { Team::Ger })
                } else if sum != 0.0 {
                    Some(if sum > 0.0 { Team::Us } else { Team::Ger })
                } else {
                    None
                };
                Some(MatchResult {
                    reason: if winner.is_some() {
                        VictoryReason::TimeLimitLead
                    } else {
                        VictoryReason::Draw
                    },
                    winner,
                })
            } else {
                None
            };
            if let Some(r) = result {
                ms.phase = MatchPhaseKind::Ended;
                ms.phase_start = now;
                ms.ends_at = now + mcfg.ended_ticks;
                ms.result = Some(r);
                out.0.push(RuleChange::MatchPhase {
                    phase: MatchPhaseKind::Ended,
                    ends_at: ms.ends_at,
                });
                out.0.push(RuleChange::MatchEnded {
                    reason: r.reason,
                    winner: r.winner,
                });
            }
        }
    }
}

impl MatchState {
    /// Movement input is frozen for the first seconds after the match ended.
    pub fn input_frozen(&self, now: u32, freeze_ticks: u32) -> bool {
        self.phase == MatchPhaseKind::Ended && now.saturating_sub(self.phase_start) < freeze_ticks
    }
}
