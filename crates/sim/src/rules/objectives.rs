//! Strongpoints: effective head count, capture progress, regression, ownership changes.

use bevy_ecs::prelude::*;
use glam::Vec3;

use super::capture::{capture_rate, CaptureParams};
use super::match_state::{MatchCfg, MatchPhaseKind, MatchState};
use super::{side, CaptureReason, RuleChangeBuf};
use crate::components::{CurrentTick, EntityTable, SimCfgRes, Team, Transform};
use crate::health::{Health, LifeState};
use crate::step::RuleChange;

/// Placement of a strongpoint (`zones.json`, M1-034, or a test fixture).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectiveSpec {
    pub id: u16,
    pub pos: Vec3,
    pub radius_m: f32,
    /// `None` = neutral, never owned.
    pub owner: Option<Team>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Objective {
    pub id: u16,
    pub pos: Vec3,
    pub radius_m: f32,
    /// Capture progress in `[-100, +100]`; US is positive.
    pub p: f32,
    pub owner: Option<Team>,
    /// Team that held it last (the regression target); `None` for a never-captured point.
    pub last_owner: Option<Team>,
    pub locked: bool,
    pub contested: bool,
    /// Tick of the last ownership change (anti-farm `repeat` flag).
    pub last_change: Option<u32>,
}

impl Objective {
    pub fn new(s: ObjectiveSpec) -> Self {
        Self {
            id: s.id,
            pos: s.pos,
            radius_m: s.radius_m,
            p: s.owner.map_or(0.0, |t| 100.0 * side(t)),
            owner: s.owner,
            last_owner: s.owner,
            locked: false,
            contested: false,
            last_change: None,
        }
    }
}

#[derive(Resource, Debug, Default)]
pub struct Objectives(pub Vec<Objective>);

/// Capture stage (runs after the combat stages): counts valid players per point and advances `p`.
/// Valid = alive, not downed, within the radius and the height tolerance (vehicles: M2).
#[allow(clippy::too_many_arguments)]
pub fn objectives(
    tick: Res<CurrentTick>,
    cfg: Res<SimCfgRes>,
    mcfg: Res<MatchCfg>,
    ms: Res<MatchState>,
    table: Res<EntityTable>,
    mut objs: ResMut<Objectives>,
    mut out: ResMut<RuleChangeBuf>,
    q: Query<(&Transform, &Team, &LifeState, &Health)>,
) {
    if objs.0.is_empty() {
        return;
    }
    let now = tick.0 .0;
    let dt = 1.0 / f32::from(cfg.hz);
    let params: CaptureParams = mcfg.capture;
    let frozen = matches!(ms.phase, MatchPhaseKind::Warmup | MatchPhaseKind::Ended);
    // (position, team) of the valid players, ascending entity id.
    let mut valid: Vec<(Vec3, Team)> = Vec::new();
    for (_, e) in table.sorted() {
        if let Ok((t, team, life, hp)) = q.get(e) {
            if life.is_alive() && hp.hp > 0.0 {
                valid.push((t.pos, *team));
            }
        }
    }
    for o in objs.0.iter_mut() {
        let (mut us, mut ger) = (0i32, 0i32);
        for &(pos, team) in &valid {
            let dx = pos.x - o.pos.x;
            let dz = pos.z - o.pos.z;
            if dx * dx + dz * dz <= o.radius_m * o.radius_m
                && (pos.y - o.pos.y).abs() <= params.height_tol_m
            {
                match team {
                    Team::Us => us += 1,
                    Team::Ger => ger += 1,
                }
            }
        }
        o.contested = us > 0 && ger > 0;
        if frozen || o.locked {
            continue;
        }
        let d = us - ger;
        if d != 0 {
            o.p = (o.p + capture_rate(d, &params) * dt).clamp(-100.0, 100.0);
        } else if us == 0 && ger == 0 && o.p.abs() < 100.0 {
            // Nobody around: drift back to the last owner.
            if let Some(last) = o.last_owner {
                let target = 100.0 * side(last);
                let step = params.regress * dt;
                o.p = if o.p < target {
                    (o.p + step).min(target)
                } else {
                    (o.p - step).max(target)
                };
            }
        }
        // Ownership changes exactly when p reaches an end.
        let reached = if o.p >= 100.0 {
            Some(Team::Us)
        } else if o.p <= -100.0 {
            Some(Team::Ger)
        } else {
            None
        };
        if let Some(team) = reached {
            if o.owner != Some(team) {
                let repeat = o
                    .last_change
                    .is_some_and(|t| now.saturating_sub(t) <= mcfg.repeat_ticks);
                o.owner = Some(team);
                o.last_owner = Some(team);
                o.last_change = Some(now);
                out.0.push(RuleChange::ObjectiveOwner {
                    objective: o.id,
                    owner: team,
                    reason: CaptureReason::Captured,
                    repeat,
                });
            }
        }
    }
}
