//! Squad operations (GAMEDESIGN 4.1): create / join / leave / kick / promote / lock / ...

use super::{
    Handover, Member, RosterDelta, Squad, SquadId, SquadKind, SquadManager, SUFFIX_MAX_CHARS,
};
use crate::ids::{PlayerId, Tick};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SquadOp {
    Create {
        kind: SquadKind,
        suffix: String,
    },
    Join {
        squad: SquadId,
    },
    Leave,
    Kick {
        target: PlayerId,
    },
    /// Offer the lead; the target has `handover_confirm_s` to `AcceptLead`.
    Promote {
        target: PlayerId,
    },
    AcceptLead,
    Rename {
        suffix: String,
    },
    Lock,
    Unlock,
    Invite {
        target: PlayerId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SquadError {
    NotOnRoster,
    AlreadyInSquad,
    NotInSquad,
    NoSuchSquad,
    WrongTeam,
    /// `max_squads` of this kind reached for the team.
    KindFull,
    SquadFull,
    Locked,
    /// Kicked less than `kick_ban_s` ago.
    Banned,
    RateLimited,
    NotLeader,
    TargetNotInSquad,
    TargetUnavailable,
    CannotTargetSelf,
    NoHandover,
    HandoverExpired,
    NameInvalid,
}

fn valid_suffix(s: &str) -> bool {
    s.chars().count() <= SUFFIX_MAX_CHARS && !s.chars().any(|c| c.is_control())
}

/// Records an action in a rolling window; `false` when the limit is already reached.
fn take_slot(log: &mut Vec<Tick>, now: Tick, window: u32, limit: u32) -> bool {
    log.retain(|t| SquadManager::since(now, *t) < window);
    if log.len() as u32 >= limit {
        return false;
    }
    log.push(now);
    true
}

impl SquadManager {
    pub fn apply(
        &mut self,
        p: PlayerId,
        op: SquadOp,
        now: Tick,
    ) -> Result<Vec<RosterDelta>, SquadError> {
        let entry = self.entry(p).ok_or(SquadError::NotOnRoster)?;
        let (team, my_squad) = (entry.team, entry.squad);
        let mut out = Vec::new();
        match op {
            SquadOp::Create { kind, suffix } => {
                if my_squad.is_some() {
                    return Err(SquadError::AlreadyInSquad);
                }
                if !valid_suffix(&suffix) {
                    return Err(SquadError::NameInvalid);
                }
                let same: Vec<u8> = self
                    .squads()
                    .filter(|s| s.team == team && s.kind == kind)
                    .map(|s| s.name_idx)
                    .collect();
                if same.len() as u32 >= self.cfg.max_squads(kind) {
                    return Err(SquadError::KindFull);
                }
                let (window, limit) = (self.cfg.window, self.cfg.create_limit);
                let log = self.logs().0.entry(p).or_default();
                if !take_slot(log, now, window, limit) {
                    return Err(SquadError::RateLimited);
                }
                let name_idx = (0u8..=u8::MAX)
                    .find(|i| !same.contains(i))
                    .unwrap_or(u8::MAX);
                let id = self.alloc_id();
                self.squads_map().insert(
                    id,
                    Squad {
                        id,
                        team,
                        kind,
                        name_idx,
                        suffix,
                        leader: p,
                        members: vec![Member {
                            player: p,
                            joined: now,
                        }],
                        locked: false,
                        invites: Default::default(),
                        kick_bans: Default::default(),
                        handover: None,
                        absent_since: None,
                    },
                );
                self.entry_mut(p).expect("roster").squad = Some(id);
                out.push(RosterDelta::SquadCreated {
                    squad: id,
                    team,
                    kind,
                    leader: p,
                });
                self.recheck_roles(&[p], &mut out);
            }
            SquadOp::Join { squad } => {
                if my_squad.is_some() {
                    return Err(SquadError::AlreadyInSquad);
                }
                let max = {
                    let s = self.squad(squad).ok_or(SquadError::NoSuchSquad)?;
                    if s.team != team {
                        return Err(SquadError::WrongTeam);
                    }
                    if s.kick_bans.get(&p).is_some_and(|until| now.lt(*until)) {
                        return Err(SquadError::Banned);
                    }
                    if s.locked && !s.invites.contains(&p) {
                        return Err(SquadError::Locked);
                    }
                    self.cfg.max_players(s.kind)
                };
                let s = self.squad_mut(squad).expect("exists");
                if s.size() >= max {
                    return Err(SquadError::SquadFull);
                }
                s.kick_bans.remove(&p);
                s.invites.remove(&p);
                s.members.push(Member {
                    player: p,
                    joined: now,
                });
                let all: Vec<PlayerId> = s.members.iter().map(|m| m.player).collect();
                self.entry_mut(p).expect("roster").squad = Some(squad);
                out.push(RosterDelta::Joined { player: p, squad });
                // A bigger squad can only unlock roles, but the newcomer's own role may no
                // longer fit (e.g. Rifleman kept across a join into a full Rifleman squad).
                self.recheck_roles(&all, &mut out);
            }
            SquadOp::Leave => {
                if my_squad.is_none() {
                    return Err(SquadError::NotInSquad);
                }
                self.leave_squad(p, now, &mut out);
            }
            SquadOp::Kick { target } => {
                let sid = self.require_leader(p)?;
                if target == p {
                    return Err(SquadError::CannotTargetSelf);
                }
                if !self.squad(sid).expect("leader").has(target) {
                    return Err(SquadError::TargetNotInSquad);
                }
                let (window, limit) = (self.cfg.window, self.cfg.kick_limit);
                let log = self.logs().1.entry(p).or_default();
                if !take_slot(log, now, window, limit) {
                    return Err(SquadError::RateLimited);
                }
                let until = Tick(now.0.wrapping_add(self.cfg.kick_ban));
                self.leave_squad(target, now, &mut out);
                if let Some(s) = self.squad_mut(sid) {
                    s.kick_bans.insert(target, until);
                }
                out.push(RosterDelta::Kicked {
                    player: target,
                    squad: sid,
                });
            }
            SquadOp::Promote { target } => {
                let sid = self.require_leader(p)?;
                if target == p {
                    return Err(SquadError::CannotTargetSelf);
                }
                let confirm = self.cfg.handover_confirm;
                let s = self.squad_mut(sid).expect("leader");
                if !s.has(target) {
                    return Err(SquadError::TargetNotInSquad);
                }
                s.handover = Some(Handover {
                    target,
                    expires: Tick(now.0.wrapping_add(confirm)),
                });
                out.push(RosterDelta::HandoverOffered { squad: sid, target });
            }
            SquadOp::AcceptLead => {
                let sid = my_squad.ok_or(SquadError::NotInSquad)?;
                let s = self.squad_mut(sid).expect("exists");
                match s.handover {
                    Some(h) if h.target == p => {
                        s.handover = None;
                        if !now.lt(h.expires) {
                            return Err(SquadError::HandoverExpired);
                        }
                        s.leader = p;
                        s.absent_since = None;
                        out.push(RosterDelta::LeaderChanged {
                            squad: sid,
                            leader: p,
                        });
                        let all: Vec<PlayerId> = s.members.iter().map(|m| m.player).collect();
                        self.recheck_roles(&all, &mut out);
                    }
                    _ => return Err(SquadError::NoHandover),
                }
            }
            SquadOp::Rename { suffix } => {
                let sid = self.require_leader(p)?;
                if !valid_suffix(&suffix) {
                    return Err(SquadError::NameInvalid);
                }
                self.squad_mut(sid).expect("leader").suffix = suffix;
                out.push(RosterDelta::Renamed { squad: sid });
            }
            SquadOp::Lock | SquadOp::Unlock => {
                let sid = self.require_leader(p)?;
                let locked = matches!(op, SquadOp::Lock);
                let s = self.squad_mut(sid).expect("leader");
                if s.locked != locked {
                    s.locked = locked;
                    out.push(RosterDelta::Locked { squad: sid, locked });
                }
            }
            SquadOp::Invite { target } => {
                let sid = self.require_leader(p)?;
                if target == p {
                    return Err(SquadError::CannotTargetSelf);
                }
                let t = self.entry(target).ok_or(SquadError::TargetUnavailable)?;
                if t.team != team || t.squad.is_some() {
                    return Err(SquadError::TargetUnavailable);
                }
                self.squad_mut(sid).expect("leader").invites.insert(target);
            }
        }
        Ok(out)
    }

    fn require_leader(&self, p: PlayerId) -> Result<SquadId, SquadError> {
        let s = self.squad_of(p).ok_or(SquadError::NotInSquad)?;
        if s.leader != p {
            return Err(SquadError::NotLeader);
        }
        Ok(s.id)
    }
}
