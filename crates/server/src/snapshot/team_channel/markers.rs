//! Server-owned manual map marks. Placement uses only roster permissions and the submitted
//! map coordinate, never an enemy target, raycast, or other source of authoritative intel.

use std::collections::BTreeMap;

use frontline_protocol::msg::c0::MarkerPlace;
use frontline_protocol::msg::team::{
    TeamMarker, TeamMarkerKind, MARKER_CELL_M, MAX_MARKER_CELL, MAX_TEAM_MARKERS,
};
use frontline_sim::squad::{Action, SquadId, SquadManager};
use frontline_sim::{PlayerId, Team, Tick};

use super::MapFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerReject {
    NotOnRoster,
    UnsupportedKind,
    InvalidMap,
    NeedSquad,
    NotSquadLeader,
    Cooldown,
    PlayerCap,
    TeamCap,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RosterMember {
    pub team: Team,
    squad: Option<SquadId>,
    joined: Tick,
    role: Option<&'static str>,
    leader: bool,
}

impl RosterMember {
    pub fn get(roster: &SquadManager, player: PlayerId) -> Option<Self> {
        let e = roster.entry(player).filter(|e| e.online)?;
        let squad = roster
            .squad_of(player)
            .filter(|s| s.team == e.team && s.has(player));
        Some(Self {
            team: e.team,
            squad: squad.map(|s| s.id),
            joined: e.joined,
            role: e.role.filter(|_| !e.role_invalid),
            leader: squad.is_some() && roster.can(player, Action::PlaceSquadMarker),
        })
    }
}

#[derive(Clone, Copy, Debug)]
enum Scope {
    Squad,
    Role(&'static str),
    SquadOrder,
}

#[derive(Clone, Debug)]
struct Active {
    author: PlayerId,
    joined: Tick,
    squad: Option<SquadId>,
    scope: Scope,
    placed: u32,
    marker: TeamMarker,
}

impl Active {
    fn remaining(&self, tick: u32, hz: u32) -> u32 {
        (u32::from(self.marker.ttl_s) * hz).saturating_sub(tick.wrapping_sub(self.placed))
    }

    fn visible_to(&self, viewer: RosterMember) -> bool {
        let same_squad = self.squad.is_some() && self.squad == viewer.squad;
        match self.scope {
            Scope::Squad | Scope::SquadOrder => same_squad,
            Scope::Role(role) => viewer.role == Some(role) || (same_squad && viewer.leader),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct Markers {
    by_team: [Vec<Active>; 2],
    // At most one entry per PlayerId, even if a session repeatedly changes squads.
    last_attention: BTreeMap<PlayerId, u32>,
}

impl Markers {
    pub fn forget(&mut self, player: PlayerId) {
        for active in &mut self.by_team {
            active.retain(|m| m.author != player);
        }
        self.last_attention.remove(&player);
    }

    pub fn prune(&mut self, roster: &SquadManager, tick: u32, hz: u32) {
        self.last_attention
            .retain(|_, placed| tick.wrapping_sub(*placed) < 2 * hz);
        for team in [Team::Us, Team::Ger] {
            self.by_team[team as usize].retain(|m| {
                m.remaining(tick, hz) > 0
                    && RosterMember::get(roster, m.author).is_some_and(|author| {
                        author.team == team
                            && author.joined == m.joined
                            && author.squad == m.squad
                            && (!matches!(m.scope, Scope::SquadOrder) || author.leader)
                    })
            });
        }
    }

    pub fn place(
        &mut self,
        roster: &SquadManager,
        player: PlayerId,
        request: MarkerPlace,
        tick: u32,
        hz: u32,
        map: MapFrame,
    ) -> Result<(), MarkerReject> {
        let author = RosterMember::get(roster, player).ok_or(MarkerReject::NotOnRoster)?;
        let kind = TeamMarkerKind::from_u8(request.kind).ok_or(MarkerReject::UnsupportedKind)?;
        let (scope, ttl_s) = match kind {
            TeamMarkerKind::Attention => (Scope::Squad, 8),
            TeamMarkerKind::NeedMedic => (Scope::Role("medic"), 20),
            TeamMarkerKind::NeedAmmo => (Scope::Role("support"), 20),
            TeamMarkerKind::OrderAttack
            | TeamMarkerKind::OrderDefend
            | TeamMarkerKind::OrderMove => (Scope::SquadOrder, 180),
        };
        if !matches!(scope, Scope::Role(_)) && author.squad.is_none() {
            return Err(MarkerReject::NeedSquad);
        }
        if matches!(scope, Scope::SquadOrder) && !author.leader {
            return Err(MarkerReject::NotSquadLeader);
        }
        let [x, z] = marker_cells(map, request.pos).ok_or(MarkerReject::InvalidMap)?;
        self.prune(roster, tick, hz);
        let active = &mut self.by_team[author.team as usize];
        let replacement = match scope {
            Scope::Squad => {
                if self.last_attention.contains_key(&player) {
                    return Err(MarkerReject::Cooldown);
                }
                if active
                    .iter()
                    .filter(|m| m.author == player && m.marker.kind == kind)
                    .count()
                    >= 3
                {
                    return Err(MarkerReject::PlayerCap);
                }
                None
            }
            Scope::Role(_) => active
                .iter()
                .position(|m| m.marker.kind == kind && m.author == player),
            Scope::SquadOrder => active
                .iter()
                .position(|m| m.marker.kind == kind && m.squad == author.squad),
        };
        let marker = Active {
            author: player,
            joined: author.joined,
            squad: author.squad,
            scope,
            placed: tick,
            marker: TeamMarker { kind, x, z, ttl_s },
        };
        if let Some(i) = replacement {
            active[i] = marker;
        } else if active.len() < MAX_TEAM_MARKERS {
            active.push(marker);
        } else {
            return Err(MarkerReject::TeamCap);
        }
        if kind == TeamMarkerKind::Attention {
            self.last_attention.insert(player, tick);
        }
        Ok(())
    }

    /// `prune` must have run at this tick; do not read another team's storage.
    pub fn visible_to(&self, viewer: RosterMember, tick: u32, hz: u32) -> Vec<TeamMarker> {
        self.by_team[viewer.team as usize]
            .iter()
            .filter(|m| m.visible_to(viewer))
            .map(|m| TeamMarker {
                ttl_s: m.remaining(tick, hz).div_ceil(hz) as u8,
                ..m.marker
            })
            .collect()
    }
}

/// Normalize inside the playable map square, then take the lower edge on the 5 m grid.
fn marker_cells(map: MapFrame, pos: [u16; 2]) -> Option<[u16; 2]> {
    let extent_cells = f64::from(map.extent_m) / f64::from(MARKER_CELL_M);
    if !map.origin_x.is_finite()
        || !map.origin_z.is_finite()
        || !(map.origin_x + map.extent_m).is_finite()
        || !(map.origin_z + map.extent_m).is_finite()
        || !extent_cells.is_finite()
        || extent_cells <= 0.0
        || extent_cells >= f64::from(MAX_MARKER_CELL) + 1.0
    {
        return None;
    }
    Some(pos.map(|code| (f64::from(code) * extent_cells / f64::from(u16::MAX)).floor() as u16))
}
