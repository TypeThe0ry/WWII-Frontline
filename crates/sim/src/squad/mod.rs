//! Squads, roster and role bookkeeping (M1-021; GAMEDESIGN 2.4, 4.1, 4.4, 4.5).
//!
//! `SquadManager` is plain deterministic state with no ECS access: the server feeds it the
//! facts it cannot know (online, dead since, score) and asks it questions. All maps are
//! `BTreeMap` / `BTreeSet`, all time is `Tick`.

pub mod leader;
pub mod ops;

use std::collections::{BTreeMap, BTreeSet};

use crate::components::Team;
use crate::data::defs::SquadsRulesDef;
use crate::data::GameData;
use crate::hash::HashWriter;
use crate::ids::{PlayerId, Tick};
use crate::roles::quota::{role_available, RoleCtx, RoleError};
use crate::team::balance::{self, elapsed, TeamError};

pub use ops::{SquadError, SquadOp};

pub type SquadId = u16;

/// Per-player tick log for the rolling rate-limit windows.
pub(crate) type RateLog = BTreeMap<PlayerId, Vec<Tick>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SquadKind {
    Infantry,
    ArmorCrew,
    Recon,
}

/// Longest allowed squad name suffix, in characters (profanity filtering is M1-026).
pub const SUFFIX_MAX_CHARS: usize = 16;

const NATO: [&str; 26] = [
    "Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India", "Juliet",
    "Kilo", "Lima", "Mike", "November", "Oscar", "Papa", "Quebec", "Romeo", "Sierra", "Tango",
    "Uniform", "Victor", "Whiskey", "Xray", "Yankee", "Zulu",
];

/// Rule values in ticks / counts (from `data/rules/squads.json`).
#[derive(Debug, Clone)]
pub struct SquadCfg {
    pub team_max: u32,
    pub infantry_max: u32,
    pub armor_max: u32,
    pub armor_squads: u32,
    pub crewman_max: u32,
    pub recon_max: u32,
    pub recon_squads: u32,
    pub window: u32,
    pub create_limit: u32,
    pub kick_limit: u32,
    pub kick_ban: u32,
    pub handover_confirm: u32,
    pub leader_absent: u32,
    pub role_switch_cooldown: u32,
    pub balance_diff: u32,
    pub switch_min_after_join: u32,
    pub switch_interval: u32,
    pub armor_role: String,
}

impl SquadCfg {
    pub fn from_def(d: &SquadsRulesDef, hz: u8) -> Self {
        let t = |s: f64| (s * hz as f64).round() as u32;
        SquadCfg {
            team_max: d.team_max_players,
            infantry_max: d.infantry.max_players,
            armor_max: d.armor_crew.max_players,
            armor_squads: d.armor_crew.max_squads.unwrap_or(u32::MAX),
            crewman_max: d
                .armor_crew
                .crewman_max_players
                .unwrap_or(d.armor_crew.max_players),
            recon_max: d.recon.max_players,
            recon_squads: d.recon.max_squads.unwrap_or(u32::MAX),
            window: t(d.rate_window_s),
            create_limit: d.create_limit_squads,
            kick_limit: d.kick_limit_players,
            kick_ban: t(d.kick_ban_s),
            handover_confirm: t(d.handover_confirm_s),
            leader_absent: t(d.leader_absent_s),
            role_switch_cooldown: t(d.role_switch_cooldown_s),
            balance_diff: d.balance.max_diff_players,
            switch_min_after_join: t(d.balance.switch_min_after_join_s),
            switch_interval: t(d.balance.switch_interval_s),
            armor_role: d.m1_override.armor_crew_leader_role.clone(),
        }
    }

    pub fn max_players(&self, k: SquadKind) -> u32 {
        match k {
            SquadKind::Infantry => self.infantry_max,
            SquadKind::ArmorCrew => self.armor_max,
            SquadKind::Recon => self.recon_max,
        }
    }

    pub fn max_squads(&self, k: SquadKind) -> u32 {
        match k {
            SquadKind::Infantry => u32::MAX,
            SquadKind::ArmorCrew => self.armor_squads,
            SquadKind::Recon => self.recon_squads,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Member {
    pub player: PlayerId,
    pub joined: Tick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handover {
    pub target: PlayerId,
    pub expires: Tick,
}

#[derive(Debug, Clone)]
pub struct Squad {
    pub id: SquadId,
    pub team: Team,
    pub kind: SquadKind,
    pub name_idx: u8,
    pub suffix: String,
    pub leader: PlayerId,
    /// Join order.
    pub members: Vec<Member>,
    pub locked: bool,
    pub invites: BTreeSet<PlayerId>,
    /// Kicked player -> may rejoin from this tick on.
    pub kick_bans: BTreeMap<PlayerId, Tick>,
    pub handover: Option<Handover>,
    /// Since when the leader has been offline or dead.
    pub absent_since: Option<Tick>,
}

impl Squad {
    pub fn size(&self) -> u32 {
        self.members.len() as u32
    }

    pub fn has(&self, p: PlayerId) -> bool {
        self.members.iter().any(|m| m.player == p)
    }

    /// `Alpha`, `Armor-1`, `Recon-2`, plus the optional suffix.
    pub fn name(&self) -> String {
        let base = match self.kind {
            SquadKind::Infantry => NATO
                .get(self.name_idx as usize)
                .map_or_else(|| format!("Squad-{}", self.name_idx + 1), |s| s.to_string()),
            SquadKind::ArmorCrew => format!("Armor-{}", self.name_idx + 1),
            SquadKind::Recon => format!("Recon-{}", self.name_idx + 1),
        };
        if self.suffix.is_empty() {
            base
        } else {
            format!("{base} {}", self.suffix)
        }
    }
}

#[derive(Debug, Clone)]
pub struct RosterEntry {
    pub team: Team,
    pub squad: Option<SquadId>,
    pub role: Option<&'static str>,
    pub score: i32,
    pub online: bool,
    /// Set while the player is dead; cleared on respawn.
    pub dead_since: Option<Tick>,
    pub joined: Tick,
    pub last_switch: Option<Tick>,
    pub last_role_change: Option<Tick>,
    /// The current role no longer fits the squad: it must change on the next deploy.
    pub role_invalid: bool,
    /// Loadout index chosen at the last deploy (`GameData::loadouts`).
    pub loadout: Option<u8>,
}

/// What changed in the roster (carried by `SimEvent::Roster`, feeds `RosterDelta`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RosterDelta {
    TeamJoined {
        player: PlayerId,
        team: Team,
    },
    TeamLeft {
        player: PlayerId,
    },
    TeamSwitched {
        player: PlayerId,
        team: Team,
    },
    SquadCreated {
        squad: SquadId,
        team: Team,
        kind: SquadKind,
        leader: PlayerId,
    },
    SquadDisbanded {
        squad: SquadId,
    },
    Joined {
        player: PlayerId,
        squad: SquadId,
    },
    Left {
        player: PlayerId,
        squad: SquadId,
    },
    Kicked {
        player: PlayerId,
        squad: SquadId,
    },
    LeaderChanged {
        squad: SquadId,
        leader: PlayerId,
    },
    HandoverOffered {
        squad: SquadId,
        target: PlayerId,
    },
    Locked {
        squad: SquadId,
        locked: bool,
    },
    Renamed {
        squad: SquadId,
    },
    RoleChanged {
        player: PlayerId,
        role: Option<&'static str>,
    },
    /// The role no longer fits; takes effect at the next deploy (GD 4.4).
    RoleInvalidated {
        player: PlayerId,
    },
}

/// Permission queries (GD 4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    PlaceSquadMarker,
    PlaceGarrison,
    Kick,
    Lock,
    Invite,
    Promote,
    Rename,
}

pub struct SquadManager {
    pub cfg: SquadCfg,
    data: &'static GameData,
    roster: BTreeMap<PlayerId, RosterEntry>,
    squads: BTreeMap<SquadId, Squad>,
    next_id: SquadId,
    create_log: BTreeMap<PlayerId, Vec<Tick>>,
    kick_log: BTreeMap<PlayerId, Vec<Tick>>,
}

impl SquadManager {
    pub fn new(data: &'static GameData, hz: u8) -> Self {
        SquadManager {
            cfg: SquadCfg::from_def(&data.rules.squads, hz),
            data,
            roster: BTreeMap::new(),
            squads: BTreeMap::new(),
            next_id: 1,
            create_log: BTreeMap::new(),
            kick_log: BTreeMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.roster.is_empty()
    }

    pub fn entry(&self, p: PlayerId) -> Option<&RosterEntry> {
        self.roster.get(&p)
    }

    pub fn roster(&self) -> impl Iterator<Item = (PlayerId, &RosterEntry)> {
        self.roster.iter().map(|(p, e)| (*p, e))
    }

    pub fn squad(&self, id: SquadId) -> Option<&Squad> {
        self.squads.get(&id)
    }

    pub fn squads(&self) -> impl Iterator<Item = &Squad> {
        self.squads.values()
    }

    pub fn squad_of(&self, p: PlayerId) -> Option<&Squad> {
        self.roster
            .get(&p)
            .and_then(|e| e.squad)
            .and_then(|s| self.squads.get(&s))
    }

    pub fn team_count(&self, team: Team) -> u32 {
        self.roster.values().filter(|e| e.team == team).count() as u32
    }

    fn other(team: Team) -> Team {
        match team {
            Team::Us => Team::Ger,
            Team::Ger => Team::Us,
        }
    }

    // ---- roster / teams ----

    pub fn can_join_team(&self, team: Team) -> Result<(), TeamError> {
        balance::can_join(
            self.team_count(team),
            self.team_count(Self::other(team)),
            self.cfg.team_max,
            self.cfg.balance_diff,
        )
    }

    pub fn roster_add(
        &mut self,
        p: PlayerId,
        team: Team,
        now: Tick,
    ) -> Result<Vec<RosterDelta>, TeamError> {
        if self.roster.contains_key(&p) {
            return Err(TeamError::AlreadyOnRoster);
        }
        self.can_join_team(team)?;
        self.admit(p, team, now)
    }

    /// Insert after the authoritative human-only admission check. Bots still use `roster_add`.
    pub(crate) fn admit(
        &mut self,
        p: PlayerId,
        team: Team,
        now: Tick,
    ) -> Result<Vec<RosterDelta>, TeamError> {
        if self.roster.contains_key(&p) {
            return Err(TeamError::AlreadyOnRoster);
        }
        if self.team_count(team) >= self.cfg.team_max {
            return Err(TeamError::Full);
        }
        self.roster.insert(
            p,
            RosterEntry {
                team,
                squad: None,
                role: None,
                score: 0,
                online: true,
                dead_since: None,
                joined: now,
                last_switch: None,
                last_role_change: None,
                role_invalid: false,
                loadout: None,
            },
        );
        Ok(vec![RosterDelta::TeamJoined { player: p, team }])
    }

    pub fn roster_remove(&mut self, p: PlayerId, now: Tick) -> Vec<RosterDelta> {
        let mut out = Vec::new();
        if !self.roster.contains_key(&p) {
            return out;
        }
        self.leave_squad(p, now, &mut out);
        self.roster.remove(&p);
        self.create_log.remove(&p);
        self.kick_log.remove(&p);
        out.push(RosterDelta::TeamLeft { player: p });
        out
    }

    pub fn can_switch_team(&self, p: PlayerId, now: Tick) -> Result<(), TeamError> {
        let e = self.roster.get(&p).ok_or(TeamError::Unknown)?;
        balance::can_switch(
            self.team_count(e.team),
            self.team_count(Self::other(e.team)),
            self.cfg.team_max,
            self.cfg.balance_diff,
            now,
            e.joined,
            e.last_switch,
            self.cfg.switch_min_after_join,
            self.cfg.switch_interval,
        )
    }

    /// Moves `p` to the other team (leaves the squad; the role is cleared).
    pub fn switch_team(&mut self, p: PlayerId, now: Tick) -> Result<Vec<RosterDelta>, TeamError> {
        self.can_switch_team(p, now)?;
        self.move_team(p, now)
    }

    /// Applies a previously authorized move; the hard cap still applies.
    pub(crate) fn move_team(
        &mut self,
        p: PlayerId,
        now: Tick,
    ) -> Result<Vec<RosterDelta>, TeamError> {
        let team = Self::other(self.entry(p).ok_or(TeamError::Unknown)?.team);
        if self.team_count(team) >= self.cfg.team_max {
            return Err(TeamError::Full);
        }
        let mut out = Vec::new();
        self.leave_squad(p, now, &mut out);
        let e = self.roster.get_mut(&p).expect("checked");
        e.team = Self::other(e.team);
        e.last_switch = Some(now);
        e.role = None;
        e.role_invalid = false;
        e.loadout = None;
        e.last_role_change = None;
        e.dead_since = None;
        out.push(RosterDelta::TeamSwitched {
            player: p,
            team: e.team,
        });
        Ok(out)
    }

    // ---- facts fed by the server ----

    pub fn set_online(&mut self, p: PlayerId, online: bool) {
        if let Some(e) = self.roster.get_mut(&p) {
            e.online = online;
        }
    }

    pub fn set_dead_since(&mut self, p: PlayerId, t: Option<Tick>) {
        if let Some(e) = self.roster.get_mut(&p) {
            e.dead_since = t;
        }
    }

    pub fn set_loadout(&mut self, p: PlayerId, loadout: Option<u8>) {
        if let Some(e) = self.roster.get_mut(&p) {
            e.loadout = loadout;
        }
    }

    pub fn set_score(&mut self, p: PlayerId, score: i32) {
        if let Some(e) = self.roster.get_mut(&p) {
            e.score = score;
        }
    }

    // ---- roles ----

    /// Would `role` be allowed for `p` right now (the asker's own current role is ignored)?
    pub fn role_available(&self, p: PlayerId, role: &str) -> Result<(), RoleError> {
        let e = self.roster.get(&p).ok_or(RoleError::Unassigned)?;
        let def = self.data.role(role).ok_or(RoleError::UnknownRole)?;
        let squad = e.squad.and_then(|s| self.squads.get(&s));
        let holds = |q: &PlayerId, r: &RosterEntry| *q != p && r.role == Some(def.id.as_str());
        let in_team = self
            .roster
            .iter()
            .filter(|(q, r)| r.team == e.team && holds(q, r))
            .count() as u32;
        let in_squad = squad.map_or(0, |s| {
            s.members
                .iter()
                .filter(|m| {
                    self.roster
                        .get(&m.player)
                        .is_some_and(|r| holds(&m.player, r))
                })
                .count() as u32
        });
        role_available(
            def,
            &RoleCtx {
                squad_kind: squad.map(|s| s.kind),
                squad_size: squad.map_or(0, |s| s.size()),
                is_leader: squad.is_some_and(|s| s.leader == p),
                in_squad_of_role: in_squad,
                in_team_of_role: in_team,
                crewman_cap: self.cfg.crewman_max,
                armor_role: &self.cfg.armor_role,
            },
        )
    }

    /// Takes `role` after the quota check; `None` clears it.
    pub fn set_role(
        &mut self,
        p: PlayerId,
        role: Option<&str>,
        now: Tick,
    ) -> Result<Vec<RosterDelta>, RoleError> {
        let id: Option<&'static str> = match role {
            None => None,
            Some(r) => {
                self.role_available(p, r)?;
                Some(self.data.role(r).ok_or(RoleError::UnknownRole)?.id.as_str())
            }
        };
        let e = self.roster.get_mut(&p).ok_or(RoleError::Unassigned)?;
        let changed = e.role != id;
        e.role = id;
        e.role_invalid = false;
        if changed {
            e.last_role_change = Some(now);
        }
        Ok(if changed {
            vec![RosterDelta::RoleChanged {
                player: p,
                role: id,
            }]
        } else {
            Vec::new()
        })
    }

    /// Flags members whose role no longer fits their squad (GD 4.4); nobody is forced to switch.
    pub(crate) fn recheck_roles(&mut self, players: &[PlayerId], out: &mut Vec<RosterDelta>) {
        for &p in players {
            let Some(role) = self.roster.get(&p).and_then(|e| e.role) else {
                continue;
            };
            let bad = self.role_available(p, role).is_err();
            let e = self.roster.get_mut(&p).expect("exists");
            if bad && !e.role_invalid {
                out.push(RosterDelta::RoleInvalidated { player: p });
            }
            e.role_invalid = bad;
        }
    }

    // ---- permissions ----

    pub fn can(&self, p: PlayerId, action: Action) -> bool {
        let Some(e) = self.roster.get(&p) else {
            return false;
        };
        let squad = self.squad_of(p);
        let leader = squad.is_some_and(|s| s.leader == p);
        match action {
            Action::PlaceSquadMarker
            | Action::Kick
            | Action::Lock
            | Action::Invite
            | Action::Promote
            | Action::Rename => leader,
            Action::PlaceGarrison => e.role == Some("officer"),
        }
    }

    // ---- state hash (block 0xE1) ----

    pub fn hash_into(&self, h: &mut HashWriter) {
        h.write_u8(0xE1);
        for (p, e) in &self.roster {
            h.write_u8(*p);
            h.write_u8(e.team as u8);
            h.write_u16(e.squad.unwrap_or(0));
            h.write_bytes(e.role.unwrap_or("").as_bytes());
            h.write_u8(e.role_invalid as u8);
            h.write_u8(e.loadout.map_or(0xFF, |l| l));
            h.write_u32(e.score as u32);
        }
        for s in self.squads.values() {
            h.write_u16(s.id);
            h.write_u8(s.team as u8);
            h.write_u8(s.kind as u8);
            h.write_u8(s.leader);
            h.write_u8(s.locked as u8);
            h.write_bytes(s.suffix.as_bytes());
            for m in &s.members {
                h.write_u8(m.player);
            }
        }
    }

    // ---- internals shared with ops / leader ----

    pub(crate) fn squad_mut(&mut self, id: SquadId) -> Option<&mut Squad> {
        self.squads.get_mut(&id)
    }

    pub(crate) fn entry_mut(&mut self, p: PlayerId) -> Option<&mut RosterEntry> {
        self.roster.get_mut(&p)
    }

    pub(crate) fn squads_map(&mut self) -> &mut BTreeMap<SquadId, Squad> {
        &mut self.squads
    }

    pub(crate) fn alloc_id(&mut self) -> SquadId {
        loop {
            // Ids fit the wire's `squad:u8`; 0 means Unassigned.
            let id = self.next_id;
            self.next_id = if id >= 255 { 1 } else { id + 1 };
            if !self.squads.contains_key(&id) {
                return id;
            }
        }
    }

    pub(crate) fn logs(&mut self) -> (&mut RateLog, &mut RateLog) {
        (&mut self.create_log, &mut self.kick_log)
    }

    /// Ticks since `t`.
    pub(crate) fn since(now: Tick, t: Tick) -> u32 {
        elapsed(now, t)
    }
}
