//! Leader succession and the per-tick squad upkeep (GAMEDESIGN 4.1).

use super::{RosterDelta, SquadId, SquadManager};
use crate::ids::{PlayerId, Tick};

impl SquadManager {
    pub(crate) fn relinquish_leader(&mut self, p: PlayerId, now: Tick) -> Vec<RosterDelta> {
        let mut out = Vec::new();
        if let Some(s) = self.squad_of(p).filter(|s| s.leader == p) {
            self.leave_squad_for_absence(s.id, p, now, &mut out);
        }
        out
    }

    /// Best successor to the lead of `sid`, never `excluding`: Officer first, then the
    /// highest match score, then the earliest joiner (then the lowest id).
    pub fn pick_successor(&self, sid: SquadId, excluding: PlayerId) -> Option<PlayerId> {
        let s = self.squad(sid)?;
        s.members
            .iter()
            .filter(|m| m.player != excluding)
            .filter_map(|m| self.entry(m.player).map(|e| (m, e)))
            .min_by_key(|(m, e)| {
                (
                    e.role != Some("officer"),
                    std::cmp::Reverse(e.score),
                    // Join order is the member order, so the index is the tie-break.
                    s.members.iter().position(|x| x.player == m.player),
                    m.player,
                )
            })
            .map(|(m, _)| m.player)
    }

    /// Removes `p` from its squad: hands the lead on or disbands, flags invalid roles.
    pub(crate) fn leave_squad(&mut self, p: PlayerId, _now: Tick, out: &mut Vec<RosterDelta>) {
        let Some(sid) = self.entry(p).and_then(|e| e.squad) else {
            return;
        };
        let was_leader = self.squad(sid).is_some_and(|s| s.leader == p);
        let successor = if was_leader {
            self.pick_successor(sid, p)
        } else {
            None
        };
        {
            let s = self.squad_mut(sid).expect("exists");
            s.members.retain(|m| m.player != p);
            if s.handover.is_some_and(|h| h.target == p) {
                s.handover = None;
            }
        }
        let e = self.entry_mut(p).expect("roster");
        e.squad = None;
        out.push(RosterDelta::Left {
            player: p,
            squad: sid,
        });
        if self.squad(sid).expect("exists").members.is_empty() {
            self.squads_map().remove(&sid);
            out.push(RosterDelta::SquadDisbanded { squad: sid });
        } else {
            if was_leader {
                let next = successor.expect("non-empty squad has a successor");
                let s = self.squad_mut(sid).expect("exists");
                s.leader = next;
                s.locked = false;
                s.invites.clear();
                s.handover = None;
                s.absent_since = None;
                out.push(RosterDelta::LeaderChanged {
                    squad: sid,
                    leader: next,
                });
            }
            let rest: Vec<PlayerId> = self
                .squad(sid)
                .expect("exists")
                .members
                .iter()
                .map(|m| m.player)
                .collect();
            self.recheck_roles(&rest, out);
        }
        // The leaver is Unassigned now (Rifleman only).
        self.recheck_roles(&[p], out);
    }

    /// Per-tick upkeep: expires handover offers and replaces a leader who has been offline
    /// or dead (not redeployed) for `leader_absent_s`.
    pub fn update(&mut self, now: Tick) -> Vec<RosterDelta> {
        let mut out = Vec::new();
        let ids: Vec<SquadId> = self.squads().map(|s| s.id).collect();
        let absent_limit = self.cfg.leader_absent;
        for sid in ids {
            let (leader, expired) = {
                let s = self.squad(sid).expect("listed");
                (s.leader, s.handover.is_some_and(|h| !now.lt(h.expires)))
            };
            let present = self
                .entry(leader)
                .is_some_and(|e| e.online && e.dead_since.is_none());
            let s = self.squad_mut(sid).expect("listed");
            if expired {
                s.handover = None;
            }
            if present {
                s.absent_since = None;
                continue;
            }
            let since = *s.absent_since.get_or_insert(now);
            if SquadManager::since(now, since) < absent_limit {
                continue;
            }
            // Lead passes on; a lone absentee's squad is dissolved.
            self.leave_squad_for_absence(sid, leader, now, &mut out);
        }
        out
    }

    fn leave_squad_for_absence(
        &mut self,
        sid: SquadId,
        leader: PlayerId,
        now: Tick,
        out: &mut Vec<RosterDelta>,
    ) {
        match self.pick_successor(sid, leader) {
            Some(next) => {
                let s = self.squad_mut(sid).expect("listed");
                s.leader = next;
                s.locked = false;
                s.invites.clear();
                s.handover = None;
                s.absent_since = None;
                out.push(RosterDelta::LeaderChanged {
                    squad: sid,
                    leader: next,
                });
                let all: Vec<PlayerId> = s.members.iter().map(|m| m.player).collect();
                self.recheck_roles(&all, out);
            }
            None => self.leave_squad(leader, now, out),
        }
    }
}
