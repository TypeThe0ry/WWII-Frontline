//! Marker permissions, lifetimes and viewer scopes (GAMEDESIGN §16.3–4, §17.1).
//! The caller supplies current roster facts, authoritative positions and match ticks.

use std::collections::{BTreeMap, BTreeSet};

use crate::data::{defs::CommsRolesDef, CommsDef};
use crate::squad::{SquadId, SquadManager};
use crate::{EntityId, PlayerId, Team, Tick, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Member {
    pub player: PlayerId,
    pub team: Team,
    pub squad: Option<SquadId>,
    pub role: Option<&'static str>,
    pub lead: bool,
    pub joined: Tick,
}

impl Member {
    pub fn from_roster(roster: &SquadManager, player: PlayerId) -> Option<Self> {
        let entry = roster.entry(player).filter(|entry| entry.online)?;
        let squad = roster
            .squad_of(player)
            .filter(|squad| squad.team == entry.team && squad.has(player));
        Some(Self {
            player,
            team: entry.team,
            squad: squad.map(|squad| squad.id),
            // A deploy invalidation only takes effect at the next deployment.
            role: entry.role,
            lead: squad.is_some_and(|squad| squad.leader == player),
            joined: entry.joined,
        })
    }

    fn same_session(self, other: Self) -> bool {
        self.player == other.player && self.joined == other.joined
    }

    fn same_squad(self, other: Self) -> bool {
        self.team == other.team && self.squad.is_some() && self.squad == other.squad
    }
}

/// Gameplay kinds. Protocol conversion is the transport adapter's responsibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerKind {
    Attention,
    Enemy,
    NeedMedic,
    NeedAmmo,
    NeedEngineer,
    NeedAt,
    NeedSupplies,
    Thanks,
    OrderMove,
    OrderAttack,
    OrderDefend,
    OrderHold,
    OrderBuild,
    TeamInfantry,
    TeamVehicle,
    TeamArtillery,
    TeamAtGun,
    TeamGarrison,
    CommandAttack,
    CommandDefend,
    FireRequest,
    GarrisonRequest,
    SupplyRequest,
    FireMission,
    RallyMarker,
}

impl MarkerKind {
    pub const fn is_ping(self) -> bool {
        matches!(
            self,
            Self::Attention
                | Self::Enemy
                | Self::NeedMedic
                | Self::NeedAmmo
                | Self::NeedEngineer
                | Self::NeedAt
                | Self::NeedSupplies
                | Self::Thanks
        )
    }

    pub const fn is_enemy(self) -> bool {
        matches!(
            self,
            Self::Enemy
                | Self::TeamInfantry
                | Self::TeamVehicle
                | Self::TeamArtillery
                | Self::TeamAtGun
                | Self::TeamGarrison
        )
    }
}

pub type MarkerId = u32;

/// Backstop independent of tunable quotas. Expired entries count until `tick` drains them.
pub const MAX_ACTIVE_MARKERS_PER_TEAM: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marker {
    pub id: MarkerId,
    pub author: Member,
    pub kind: MarkerKind,
    /// Enemy positions are rounded to the nearest configured cell, with altitude stripped.
    pub pos: Vec3,
    /// Authoritative target for sim matching. Keep enemy identities out of outbound markers.
    pub target: Option<EntityId>,
    pub expires_at: Option<Tick>,
    scope: Scope,
}

impl Marker {
    fn live(&self, now: Tick) -> bool {
        self.expires_at.is_none_or(|end| now.lt(end))
    }
}

pub type MarkerExpired = Marker;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerErr {
    UnsupportedKind,
    NeedSquad,
    NotSquadLeader,
    RoleDenied,
    Cooldown,
    PlayerCap,
    TeamCap,
    InvalidPosition,
    NotFound,
    NotOwner,
    RefreshTooSoon,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    Squad,
    Role(NeedRole),
    Team,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NeedRole {
    Medic,
    Support,
    Engineer,
    AntiTank,
}

impl NeedRole {
    fn slug(self, roles: &CommsRolesDef) -> &str {
        match self {
            Self::Medic => &roles.medic,
            Self::Support => &roles.support,
            Self::Engineer => &roles.engineer,
            Self::AntiTank => &roles.anti_tank,
        }
    }
}

/// Quota and replacement identity. Needs replace per author, orders per squad and kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group {
    Ping,
    Need,
    SquadOrder,
    TeamMark,
    CommandOrder,
    FireRequest,
    GarrisonRequest,
    SupplyRequest,
}

impl MarkerKind {
    fn group(self) -> Option<Group> {
        use MarkerKind::*;
        Some(match self {
            Attention | Enemy | Thanks => Group::Ping,
            NeedMedic | NeedAmmo | NeedEngineer | NeedAt | NeedSupplies => Group::Need,
            OrderMove | OrderAttack | OrderDefend | OrderHold | OrderBuild => Group::SquadOrder,
            TeamInfantry | TeamVehicle | TeamArtillery | TeamAtGun | TeamGarrison => {
                Group::TeamMark
            }
            CommandAttack | CommandDefend => Group::CommandOrder,
            FireRequest => Group::FireRequest,
            GarrisonRequest => Group::GarrisonRequest,
            SupplyRequest => Group::SupplyRequest,
            FireMission | RallyMarker => return None,
        })
    }
}

#[derive(Clone, Copy)]
struct Policy {
    group: Group,
    scope: Scope,
    ttl_s: Option<u16>,
    cap: u8,
}

struct Active {
    marker: Marker,
    group: Group,
    changed: Tick,
    last_sent: Option<Tick>,
    dirty: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct MuteKey {
    team: Team,
    squad: SquadId,
    joined: Tick,
}

impl MuteKey {
    fn new(target: Member, squad: SquadId) -> Self {
        Self {
            team: target.team,
            squad,
            joined: target.joined,
        }
    }
}

struct Mutes {
    key: MuteKey,
    reporters: BTreeSet<PlayerId>,
    until: Option<Tick>,
}

pub struct MarkerRegistry {
    cfg: CommsDef,
    hz: u32,
    next_id: MarkerId,
    teams: [Vec<Active>; 2],
    cooldowns: BTreeMap<PlayerId, (Tick, Tick)>,
    mutes: BTreeMap<PlayerId, Mutes>,
    audience_changed: bool,
}

impl MarkerRegistry {
    pub fn new(cfg: &CommsDef, hz: u32) -> Self {
        // All u16 second intervals must fit Tick's signed wrapping comparison.
        assert!((1..=32_768).contains(&hz));
        Self {
            cfg: cfg.clone(),
            hz,
            next_id: 1,
            teams: Default::default(),
            cooldowns: BTreeMap::new(),
            mutes: BTreeMap::new(),
            audience_changed: false,
        }
    }

    /// Checks the role and squad permission matrix. Placement additionally checks live quotas,
    /// position, and time windows.
    pub fn can_place(&self, author: Member, kind: MarkerKind) -> Result<(), MarkerErr> {
        Self::policy(&self.cfg, author, kind).map(|_| ())
    }

    fn policy(cfg: &CommsDef, author: Member, kind: MarkerKind) -> Result<Policy, MarkerErr> {
        use MarkerKind::*;
        let group = kind.group().ok_or(MarkerErr::UnsupportedKind)?;
        let roles = &cfg.roles;
        let role = author.role.unwrap_or_default();
        let officer = role == roles.officer;
        let commander = role == roles.commander;
        let spotter = role == roles.spotter;
        let ping = &cfg.ping;
        let (scope, ttl, cap) = match group {
            Group::Ping => {
                let scope = if kind == Enemy && (officer || spotter) {
                    Scope::Team
                } else {
                    Scope::Squad
                };
                let ttl = match kind {
                    Enemy => ping.enemy_ttl_s,
                    Thanks => ping.thanks_ttl_s,
                    _ => ping.attention_ttl_s,
                };
                (scope, ttl, ping.max_active_markers)
            }
            Group::Need => {
                let role = match kind {
                    NeedMedic => NeedRole::Medic,
                    NeedAmmo | NeedSupplies => NeedRole::Support,
                    NeedEngineer => NeedRole::Engineer,
                    NeedAt => NeedRole::AntiTank,
                    _ => unreachable!("need kind"),
                };
                (Scope::Role(role), ping.need_ttl_s, 1)
            }
            Group::SquadOrder => {
                if author.squad.is_none() {
                    return Err(MarkerErr::NeedSquad);
                }
                if !author.lead {
                    return Err(MarkerErr::NotSquadLeader);
                }
                (
                    Scope::Squad,
                    cfg.squad_order.ttl_s,
                    cfg.squad_order.max_active_markers,
                )
            }
            Group::TeamMark => {
                let mark = &cfg.team_mark;
                let cap = if commander {
                    mark.commander_max_markers
                } else if officer {
                    mark.officer_max_markers
                } else if spotter {
                    mark.spotter_max_markers
                } else {
                    return Err(MarkerErr::RoleDenied);
                };
                let ttl = match kind {
                    TeamInfantry => mark.infantry_ttl_s,
                    TeamVehicle => mark.vehicle_ttl_s,
                    _ => mark.emplacement_ttl_s,
                };
                (Scope::Team, ttl, cap)
            }
            Group::CommandOrder if commander => {
                return Ok(Policy {
                    group,
                    scope: Scope::Team,
                    ttl_s: None,
                    cap: cfg.command_order_max_markers,
                });
            }
            Group::FireRequest if commander || officer => {
                let req = &cfg.fire_request;
                (
                    Scope::Team,
                    req.ttl_s,
                    if commander {
                        req.commander_max_markers
                    } else {
                        req.officer_max_markers
                    },
                )
            }
            Group::GarrisonRequest if commander || officer => {
                let req = &cfg.garrison_request;
                (Scope::Team, req.ttl_s, req.max_active_markers)
            }
            Group::SupplyRequest if commander || officer || role == roles.engineer => {
                let req = &cfg.supply_request;
                (Scope::Team, req.ttl_s, req.max_active_markers)
            }
            _ => return Err(MarkerErr::RoleDenied),
        };
        if scope == Scope::Squad && author.squad.is_none() {
            return Err(MarkerErr::NeedSquad);
        }
        Ok(Policy {
            group,
            scope,
            ttl_s: Some(ttl),
            cap,
        })
    }

    pub fn get(&self, id: MarkerId) -> Option<&Marker> {
        self.teams
            .iter()
            .flatten()
            .find(|entry| entry.marker.id == id)
            .map(|entry| &entry.marker)
    }

    pub fn place(
        &mut self,
        author: Member,
        kind: MarkerKind,
        pos: Vec3,
        target: Option<EntityId>,
        now: Tick,
    ) -> Result<Marker, MarkerErr> {
        let policy = Self::policy(&self.cfg, author, kind)?;
        let pos = self.position(kind, pos)?;
        if kind.is_ping()
            && self
                .cooldowns
                .get(&author.player)
                .is_some_and(|&(joined, end)| author.joined == joined && now.lt(end))
        {
            return Err(MarkerErr::Cooldown);
        }
        let ttl = policy
            .ttl_s
            .map(|seconds| add_seconds(now, self.hz, seconds));
        let team = author.team as usize;
        let mut count = 0;
        let mut replacement = None;
        for (index, active) in self.teams[team].iter().enumerate() {
            let marker = &active.marker;
            if active.group != policy.group || !marker.live(now) {
                continue;
            }
            let same_owner = author.same_session(marker.author);
            let included = match policy.group {
                Group::SquadOrder => author.same_squad(marker.author),
                Group::CommandOrder => true,
                Group::Need => same_owner && marker.kind == kind,
                _ => same_owner,
            };
            if !included {
                continue;
            }
            count += 1;
            if marker.kind == kind
                && (matches!(policy.group, Group::Need | Group::SquadOrder)
                    || (kind.is_enemy() && target.is_some() && marker.target == target))
            {
                replacement = Some(index);
            }
        }
        let marker = if let Some(index) = replacement {
            let active = &mut self.teams[team][index];
            check_refresh(active, now, self.hz, self.cfg.enemy_refresh_s)?;
            active.marker = Marker {
                id: active.marker.id,
                author,
                kind,
                pos,
                target,
                expires_at: ttl,
                scope: policy.scope,
            };
            active.changed = now;
            active.dirty = true;
            active.marker
        } else {
            if count >= usize::from(policy.cap) {
                return Err(MarkerErr::PlayerCap);
            }
            if self.teams[team].len() >= MAX_ACTIVE_MARKERS_PER_TEAM {
                return Err(MarkerErr::TeamCap);
            }
            let id = self.allocate_id();
            let marker = Marker {
                id,
                author,
                kind,
                pos,
                target,
                expires_at: ttl,
                scope: policy.scope,
            };
            self.teams[team].push(Active {
                marker,
                group: policy.group,
                changed: now,
                last_sent: None,
                dirty: true,
            });
            marker
        };
        if kind.is_ping() {
            self.cooldowns.insert(
                author.player,
                (
                    author.joined,
                    add_seconds(now, self.hz, self.cfg.ping.cooldown_s),
                ),
            );
        }
        Ok(marker)
    }

    fn allocate_id(&mut self) -> MarkerId {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("marker id space exhausted in one match");
        id
    }

    fn position(&self, kind: MarkerKind, pos: Vec3) -> Result<Vec3, MarkerErr> {
        if !pos.is_finite() {
            return Err(MarkerErr::InvalidPosition);
        }
        if !kind.is_enemy() {
            return Ok(pos);
        }
        let cell = f64::from(self.cfg.enemy_cell_m);
        let quantize = |x| ((f64::from(x) / cell).round() * cell) as f32;
        let grid_pos = Vec3::new(quantize(pos.x), 0.0, quantize(pos.z));
        grid_pos
            .is_finite()
            .then_some(grid_pos)
            .ok_or(MarkerErr::InvalidPosition)
    }

    /// Move an existing marker without extending its life. A repeated target observation via
    /// `place` refreshes the TTL. Both operations respect the enemy update interval.
    pub fn move_marker(
        &mut self,
        author: Member,
        id: MarkerId,
        pos: Vec3,
        now: Tick,
    ) -> Result<Marker, MarkerErr> {
        let (index, marker) = self.owned(author, id)?;
        if !marker.live(now) {
            return Err(MarkerErr::NotFound);
        }
        let pos = self.position(marker.kind, pos)?;
        if pos == marker.pos {
            return Ok(marker);
        }
        let active = &mut self.teams[author.team as usize][index];
        check_refresh(active, now, self.hz, self.cfg.enemy_refresh_s)?;
        active.marker.pos = pos;
        active.changed = now;
        active.dirty = true;
        Ok(active.marker)
    }

    pub fn remove(&mut self, author: Member, id: MarkerId) -> Result<Marker, MarkerErr> {
        let (index, _) = self.owned(author, id)?;
        Ok(self.teams[author.team as usize].remove(index).marker)
    }

    fn owned(&self, author: Member, id: MarkerId) -> Result<(usize, Marker), MarkerErr> {
        let (index, active) = self.teams[author.team as usize]
            .iter()
            .enumerate()
            .find(|(_, active)| active.marker.id == id)
            .ok_or(MarkerErr::NotFound)?;
        let marker = active.marker;
        if !author.same_session(marker.author) || author.squad != marker.author.squad {
            return Err(MarkerErr::NotOwner);
        }
        let policy = Self::policy(&self.cfg, author, marker.kind)?;
        if policy.scope != marker.scope {
            return Err(MarkerErr::RoleDenied);
        }
        Ok((index, marker))
    }

    /// Remove expired entries and return each removal once, in team and insertion order.
    pub fn tick(&mut self, now: Tick) -> Vec<MarkerExpired> {
        self.cooldowns.retain(|_, (_, end)| now.lt(*end));
        let previous = self.mutes.len();
        self.mutes
            .retain(|_, state| state.until.is_none_or(|until| now.lt(until)));
        self.audience_changed |= self.mutes.len() != previous;
        self.retain(|active| active.marker.live(now))
    }

    /// Remove entries whose author left their roster session, switched team/squad, or lost
    /// the placement permission. Refresh `Member` from the live roster for each viewer.
    pub fn reconcile(&mut self, roster: &SquadManager) -> Vec<MarkerExpired> {
        self.cooldowns.retain(|&player, (joined, _)| {
            roster
                .entry(player)
                .is_some_and(|entry| entry.joined == *joined)
        });
        self.mutes.retain(|&player, state| {
            Member::from_roster(roster, player).is_some_and(|member| {
                member
                    .squad
                    .is_some_and(|squad| MuteKey::new(member, squad) == state.key)
            })
        });
        let cfg = &self.cfg;
        let mut removed = Vec::new();
        for team in &mut self.teams {
            team.retain(|entry| {
                let marker = entry.marker;
                let keep =
                    Member::from_roster(roster, marker.author.player).is_some_and(|member| {
                        member.same_session(marker.author)
                            && member.team == marker.author.team
                            && member.squad == marker.author.squad
                            && Self::policy(cfg, member, marker.kind)
                                .is_ok_and(|policy| policy.scope == marker.scope)
                    });
                if !keep {
                    removed.push(marker);
                }
                keep
            });
        }
        removed
    }

    /// Drop every marker and per-session state for a departing or reused player slot.
    pub fn forget(&mut self, player: PlayerId) -> Vec<MarkerExpired> {
        self.cooldowns.remove(&player);
        self.mutes.remove(&player);
        for state in self.mutes.values_mut() {
            state.reporters.remove(&player);
        }
        self.retain(|entry| entry.marker.author.player != player)
    }

    /// Consume a notification that mute suppression changed the published audience.
    pub fn take_audience_changed(&mut self) -> bool {
        std::mem::take(&mut self.audience_changed)
    }

    fn retain(&mut self, keep: impl Fn(&Active) -> bool) -> Vec<MarkerExpired> {
        let mut removed = Vec::new();
        for team in &mut self.teams {
            team.retain(|entry| {
                if keep(entry) {
                    true
                } else {
                    removed.push(entry.marker);
                    false
                }
            });
        }
        removed
    }

    /// Drain new and changed markers. Enemy updates for an ID are sent at most once per
    /// configured interval, including when a previous drain ran late.
    pub fn take_due(&mut self, now: Tick) -> Vec<Marker> {
        let mut due = Vec::new();
        let refresh = u32::from(self.cfg.enemy_refresh_s) * self.hz;
        for entry in self.teams.iter_mut().flatten() {
            if entry.dirty
                && entry.marker.live(now)
                && (!entry.marker.kind.is_enemy()
                    || entry
                        .last_sent
                        .is_none_or(|last| now.diff(last) >= refresh as i32))
            {
                entry.dirty = false;
                entry.last_sent = Some(now);
                due.push(entry.marker);
            }
        }
        due
    }

    #[inline]
    pub fn visible_to(&self, marker: &Marker, viewer: Member, now: Tick) -> bool {
        marker.live(now) && self.audience_allows(marker, viewer, now)
    }

    /// Audience policy without the lifetime check, for expired/removal notifications.
    #[inline]
    pub fn audience_allows(&self, marker: &Marker, viewer: Member, now: Tick) -> bool {
        if marker.author.team != viewer.team {
            return false;
        }
        let same_squad = marker.author.same_squad(viewer);
        let scoped = match marker.scope {
            Scope::Team => true,
            Scope::Squad => same_squad,
            Scope::Role(role) => {
                viewer.role == Some(role.slug(&self.cfg.roles)) || (same_squad && viewer.lead)
            }
        };
        scoped
            && !(same_squad
                && !self.mutes.is_empty()
                && marker.kind.is_ping()
                && viewer.squad.is_some_and(|squad| {
                    self.mutes
                        .get(&marker.author.player)
                        .filter(|state| state.key == MuteKey::new(marker.author, squad))
                        .and_then(|state| state.until)
                        .is_some_and(|until| now.lt(until))
                }))
    }

    pub fn visible(&self, viewer: Member, now: Tick) -> impl Iterator<Item = &Marker> {
        self.teams[viewer.team as usize]
            .iter()
            .map(|entry| &entry.marker)
            .filter(move |marker| self.visible_to(marker, viewer, now))
    }

    /// Accept one report per distinct squad member. Suppress only pings visible to that
    /// squad, without affecting the same team's other squads. Returns true on activation.
    pub fn report_squad_mute(&mut self, by: Member, target: Member, now: Tick) -> bool {
        let Some(squad) = target.squad else {
            return false;
        };
        if by.player == target.player || !by.same_squad(target) {
            return false;
        }
        let key = MuteKey::new(target, squad);
        let empty = || Mutes {
            key,
            reporters: BTreeSet::new(),
            until: None,
        };
        let state = self.mutes.entry(target.player).or_insert_with(empty);
        if state.key != key {
            *state = empty();
        }
        if state.until.is_some_and(|until| now.lt(until)) {
            return false;
        }
        state.until = None;
        state.reporters.insert(by.player);
        if state.reporters.len() < usize::from(self.cfg.squad_mute.count) {
            return false;
        }
        state.reporters.clear();
        state.until = Some(add_seconds(now, self.hz, self.cfg.squad_mute.duration_s));
        self.audience_changed = true;
        true
    }
}

fn add_seconds(now: Tick, hz: u32, seconds: u16) -> Tick {
    Tick(now.0.wrapping_add(u32::from(seconds) * hz))
}

fn check_refresh(active: &Active, now: Tick, hz: u32, seconds: u16) -> Result<(), MarkerErr> {
    if active.marker.kind.is_enemy() && now.lt(add_seconds(active.changed, hz, seconds)) {
        return Err(MarkerErr::RefreshTooSoon);
    }
    Ok(())
}
