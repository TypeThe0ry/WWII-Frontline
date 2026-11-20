use bevy_ecs::world::World as EcsWorld;

use crate::components::{EntityTable, Player, PlayerIndex, Transform};
use crate::health::LifeState;
use crate::rules::{self, MatchCfg, Objectives};
use crate::squad::SquadManager;
use crate::step::{RuleChange, StepOutput};
use crate::view::ObjectiveView;

use super::*;

#[derive(bevy_ecs::prelude::Component, Default)]
pub struct Afk(pub bool);

impl ScoreEngine {
    pub(crate) fn world_tick(
        &mut self,
        tick: u32,
        ecs: &EcsWorld,
        squads: &SquadManager,
        before: &[ObjectiveView],
        out: &mut StepOutput,
    ) {
        self.advance(tick);
        let index = &ecs.resource::<PlayerIndex>().map;
        for (&player, &(_, entity)) in index {
            let e = ecs.entity(entity);
            let Some(&team) = e.get::<Team>() else {
                continue;
            };
            let roster = squads.entry(player);
            let role = self.role(
                roster
                    .and_then(|r| r.role)
                    .unwrap_or(crate::world::DEFAULT_ROLE),
            );
            self.observe(
                player,
                Actor {
                    role,
                    team,
                    squad: roster.and_then(|r| r.squad),
                    alive: e.get::<LifeState>().is_some_and(LifeState::is_alive),
                    afk: e.get::<Afk>().is_some_and(|a| a.0),
                },
            );
        }
        for (p, r) in squads.roster() {
            if r.online && !index.contains_key(&p) {
                self.observe(
                    p,
                    Actor {
                        role: self.role(r.role.unwrap_or(crate::world::DEFAULT_ROLE)),
                        team: r.team,
                        squad: r.squad,
                        alive: false,
                        afk: false,
                    },
                );
            }
        }
        let table = ecs.resource::<EntityTable>();
        let player_for = |id| {
            table
                .get(id)
                .and_then(|e| ecs.get::<Player>(e))
                .map(|p| p.id)
        };
        let mut deltas = Vec::new();
        let mut blow = [(false, 0); 128];
        let mut eligible = [false; 128];
        let height = ecs.resource::<MatchCfg>().capture.height_tol_m;
        let objectives = &ecs.resource::<Objectives>().0;
        let mut emit = |d: Deltas| deltas.extend(d.into_iter().map(SimEvent::ScoreDelta));
        let mut one = Deltas::new();
        for e in &out.events {
            match *e {
                SimEvent::Shot { shooter, .. } => {
                    if let Some(l) = player_for(shooter).and_then(|p| self.ledger(p)) {
                        l.stats.shots += 1;
                    }
                }
                SimEvent::Hit {
                    shooter, target, ..
                } => {
                    let enemy = match (player_for(shooter), player_for(target)) {
                        (Some(s), Some(t)) => self
                            .actor(s)
                            .zip(self.actor(t))
                            .is_some_and(|(s, t)| s.team != t.team),
                        _ => false,
                    };
                    if let Some(l) = player_for(shooter)
                        .filter(|_| enemy)
                        .and_then(|p| self.ledger(p))
                    {
                        l.stats.hits += 1;
                    }
                }
                SimEvent::Downed {
                    victim,
                    headshot,
                    dist_m,
                    ..
                } => {
                    blow[victim as usize] = (headshot, (dist_m * 1000.0) as u32);
                    if let Some(l) = self.ledger(victim) {
                        l.streak.clear();
                    }
                }
                SimEvent::Kill {
                    victim,
                    headshot,
                    dist_m,
                    ..
                } => {
                    blow[victim as usize] = (headshot, (dist_m * 1000.0) as u32);
                    self.death(victim);
                }
                SimEvent::KillCredit {
                    victim,
                    killer: Some(killer),
                    ref assists,
                    kind,
                    team_kill,
                } => {
                    let (headshot, dist_mm) = blow[victim as usize];
                    let defense = index
                        .get(&killer)
                        .and_then(|&(_, e)| ecs.get::<Transform>(e))
                        .zip(self.actor(killer))
                        .is_some_and(|(t, a)| {
                            objectives.iter().any(|o| {
                                before.iter().find(|b| b.id == o.id).and_then(|b| b.owner)
                                    == Some(a.team)
                                    && within(t.pos, o, height)
                            })
                        });
                    eligible[victim as usize] = self.credit(
                        Kill {
                            killer,
                            victim,
                            team_kill,
                            kind,
                            assists,
                            headshot,
                            dist_mm,
                            defense,
                        },
                        &mut one,
                    );
                }
                SimEvent::SuppressionAssist { suppressor, victim } if eligible[victim as usize] => {
                    if let Some(p) = player_for(suppressor) {
                        self.award(p, K::SuppressionAssist, 1, &mut one);
                    }
                }
                SimEvent::Revive {
                    medic,
                    target_player,
                    ..
                } => {
                    if let Some(p) = player_for(medic) {
                        self.revive(p, target_player, &mut one);
                    }
                }
                SimEvent::Resupplied {
                    player, crate_id, ..
                } => {
                    if let Some(&owner) = self.ammo.get(&crate_id).filter(|&&owner| {
                        owner.player != player
                            && self.current(owner)
                            && self
                                .actor(owner.player)
                                .zip(self.actor(player))
                                .is_some_and(|(a, b)| a.team == b.team)
                    }) {
                        self.action(owner.player, Action::AmmoCrateUse, &mut one);
                    }
                }
                SimEvent::CrateDestroyed { id } => {
                    self.ammo.remove(&id);
                }
                SimEvent::SpawnPointState {
                    id,
                    state: crate::spawn::garrison::GarrisonState::Active,
                    ..
                } => {
                    if let Some(&(owner, _)) = self
                        .garrisons
                        .get(&id)
                        .filter(|&&(owner, _)| self.current(owner))
                    {
                        self.action(owner.player, Action::GarrisonBuilt, &mut one);
                    }
                }
                SimEvent::SpawnPointState {
                    id,
                    state: crate::spawn::garrison::GarrisonState::Destroyed,
                    ..
                } => {
                    self.garrisons.remove(&id);
                }
                _ => emit(self.on_event(e)),
            }
            emit(std::mem::take(&mut one));
        }

        for change in &out.rule_changes {
            if let RuleChange::ObjectiveOwner {
                objective, owner, ..
            } = *change
            {
                self.timeline.push(CaptureRecord {
                    tick,
                    objective,
                    owner: owner as u8,
                });
            }
        }
        for o in objectives {
            let Some(prev) = before.iter().find(|p| p.id == o.id) else {
                continue;
            };
            if o.locked {
                continue;
            }
            let mut present = SmallVec::<[(PlayerId, Team); 32]>::new();
            for (&p, &(_, e)) in index {
                if let Some(a) = self.actor(p).filter(|a| {
                    a.alive
                        && ecs
                            .get::<Transform>(e)
                            .is_some_and(|t| within(t.pos, o, height))
                }) {
                    present.push((p, a.team));
                }
            }
            let changed = prev.owner != o.owner;
            let moving = if o.p > prev.p {
                Some(Team::Us)
            } else if o.p < prev.p {
                Some(Team::Ger)
            } else {
                None
            };
            emit(self.objective(
                o.id,
                prev.owner,
                moving,
                o.p,
                o.owner.filter(|_| changed),
                &present,
            ));
        }
        out.events.extend(deltas);
    }

    /// Award capture/defense points from one authoritative objective update. `present` contains
    /// only valid bodies in this objective's zone, in deterministic player order.
    pub fn objective(
        &mut self,
        id: u16,
        owner: Option<Team>,
        moving: Option<Team>,
        progress: f32,
        captured: Option<Team>,
        present: &[(PlayerId, Team)],
    ) -> Deltas {
        let periodic = self.ticks(self.rules.periodic_s).max(1);
        let participation = self.ticks(self.rules.capture_participation_s);
        let cooldown = self.ticks(self.rules.capture_cooldown_s);
        let state = self.points.entry(id).or_default();
        let cooldown_active = state
            .last_capture
            .is_some_and(|t| self.now.wrapping_sub(t) <= cooldown);
        let enemy_in_zone = owner.is_some_and(|t| present.iter().any(|&(_, team)| team != t));
        if owner.is_some_and(|own| {
            progress * rules::side(own) < -(self.rules.repel_threshold_pct as f32)
        }) {
            state.threatened = true;
        }
        let repel = state.threatened && owner.is_some_and(|t| progress * rules::side(t) >= 0.0);
        if repel {
            state.threatened = false;
        }

        for p in state.players.values_mut() {
            p.present = false;
        }
        let mut wanted = SmallVec::<[(PlayerId, K); 16]>::new();
        for &(p, team) in present {
            let ps = state.players.entry(p).or_default();
            ps.present = true;
            let afk = self
                .players
                .get(p as usize)
                .and_then(Option::as_ref)
                .and_then(|l| l.actor)
                .is_none_or(|a| a.afk);
            if afk {
                ps.attack = 0;
                ps.defend = 0;
                continue;
            }
            if Some(team) != owner {
                ps.participation += 1;
            }
            if Some(team) != owner && moving == Some(team) && !cooldown_active {
                ps.attack += 1;
                if ps.attack == periodic {
                    ps.attack = 0;
                    wanted.push((p, K::CaptureAdvance));
                }
            } else {
                ps.attack = 0;
            }
            if Some(team) == owner && enemy_in_zone {
                ps.defend += 1;
                if ps.defend == periodic {
                    ps.defend = 0;
                    wanted.push((p, K::ContestedDefense));
                }
            } else {
                ps.defend = 0;
            }
            if repel && Some(team) == owner {
                wanted.push((p, K::Repel));
            }
            if !cooldown_active && captured == Some(team) && ps.participation >= participation {
                wanted.push((p, K::Capture));
            }
        }
        if captured.is_some() {
            state.last_capture = Some(self.now);
            state.players.clear();
            state.threatened = false;
        } else {
            for p in state.players.values_mut().filter(|p| !p.present) {
                p.attack = 0;
                p.defend = 0;
            }
        }
        let mut result = Deltas::new();
        for (p, k) in wanted {
            self.award(p, k, 1, &mut result);
        }
        result
    }
}

fn within(pos: glam::Vec3, point: &rules::Objective, height_tol: f32) -> bool {
    let d = pos - point.pos;
    d.x * d.x + d.z * d.z <= point.radius_m * point.radius_m && d.y.abs() <= height_tol
}
