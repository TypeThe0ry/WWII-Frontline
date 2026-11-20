//! `validate_deploy`: first failing check wins, in the order of GAMEDESIGN 2.5.

use crate::components::Team;
use crate::data::defs::Faction;
use crate::deploy::SPAWN_HQ;
use crate::health::LifeState;
use crate::ids::PlayerId;
use crate::roles::quota::RoleError;
use crate::spawn::garrison::SpawnRequestError;
use crate::squad::SquadManager;
use crate::team::balance::elapsed;
use crate::world::World;

/// Role levels treated as unlocked while the game has no XP (M1).
pub const M1_UNLOCKED_LEVEL: u32 = 3;

/// Mirrors the wire `DeployRequest`: `role` and `loadout` index `GameData::{roles, loadouts}`,
/// `squad` is the squad id (0 = Unassigned), `spawn_point` a Garrison id or [`SPAWN_HQ`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeployReq {
    pub spawn_point: u16,
    pub role: u8,
    pub loadout: u8,
    pub squad: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeployOk {
    pub role: &'static str,
}

/// Result of [`World::deploy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeployDone {
    pub role: &'static str,
    /// 0 = at once (new body) or per the HQ respawn wave; otherwise the Garrison wait.
    pub spawn_in_ticks: u32,
}

/// First failing check; `code()` is the wire `DeployResult.reason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeployError {
    NotOnRoster,
    /// Alive or downed: deploy only from the DEPLOY screen or after KIA.
    NotDeployable,
    /// The request names a squad the player is not in (or does not exist).
    WrongSquad,
    UnknownRole,
    RoleFull,
    RoleTeamFull,
    SquadTooSmall,
    LeaderOnly,
    NotInArmor,
    WrongSquadKind,
    /// Unassigned players are Riflemen only.
    UnassignedRole,
    UnknownLoadout,
    LoadoutWrongRole,
    LoadoutWrongFaction,
    LoadoutLocked,
    /// Unassigned players may only spawn at the HQ.
    UnassignedNeedsHq,
    NoHq,
    NoSuchPoint,
    PointWrongTeam,
    PointNotActive,
    PointBlocked,
    PointNoSlots,
    /// Changed role less than `role_switch_cooldown_s` ago.
    RoleCooldown,
}

impl DeployError {
    pub const ALL: [DeployError; 23] = [
        DeployError::NotOnRoster,
        DeployError::NotDeployable,
        DeployError::WrongSquad,
        DeployError::UnknownRole,
        DeployError::RoleFull,
        DeployError::RoleTeamFull,
        DeployError::SquadTooSmall,
        DeployError::LeaderOnly,
        DeployError::NotInArmor,
        DeployError::WrongSquadKind,
        DeployError::UnassignedRole,
        DeployError::UnknownLoadout,
        DeployError::LoadoutWrongRole,
        DeployError::LoadoutWrongFaction,
        DeployError::LoadoutLocked,
        DeployError::UnassignedNeedsHq,
        DeployError::NoHq,
        DeployError::NoSuchPoint,
        DeployError::PointWrongTeam,
        DeployError::PointNotActive,
        DeployError::PointBlocked,
        DeployError::PointNoSlots,
        DeployError::RoleCooldown,
    ];

    /// Wire reason code (1-based; 0 is success). Fixed: never renumber.
    pub fn code(self) -> u8 {
        match self {
            DeployError::NotOnRoster => 1,
            DeployError::NotDeployable => 2,
            DeployError::WrongSquad => 3,
            DeployError::UnknownRole => 4,
            DeployError::RoleFull => 5,
            DeployError::RoleTeamFull => 6,
            DeployError::SquadTooSmall => 7,
            DeployError::LeaderOnly => 8,
            DeployError::NotInArmor => 9,
            DeployError::WrongSquadKind => 10,
            DeployError::UnassignedRole => 11,
            DeployError::UnknownLoadout => 12,
            DeployError::LoadoutWrongRole => 13,
            DeployError::LoadoutWrongFaction => 14,
            DeployError::LoadoutLocked => 15,
            DeployError::UnassignedNeedsHq => 16,
            DeployError::NoHq => 17,
            DeployError::NoSuchPoint => 18,
            DeployError::PointWrongTeam => 19,
            DeployError::PointNotActive => 20,
            DeployError::PointBlocked => 21,
            DeployError::PointNoSlots => 22,
            DeployError::RoleCooldown => 23,
        }
    }
}

impl DeployError {
    pub(crate) fn from_spawn_request(e: SpawnRequestError) -> Self {
        e.into()
    }
}

impl From<RoleError> for DeployError {
    fn from(e: RoleError) -> Self {
        match e {
            RoleError::UnknownRole => DeployError::UnknownRole,
            RoleError::Full => DeployError::RoleFull,
            RoleError::TeamFull => DeployError::RoleTeamFull,
            RoleError::SquadTooSmall => DeployError::SquadTooSmall,
            RoleError::LeaderOnly => DeployError::LeaderOnly,
            RoleError::NotInArmor => DeployError::NotInArmor,
            RoleError::WrongSquadKind => DeployError::WrongSquadKind,
            RoleError::Unassigned => DeployError::UnassignedRole,
        }
    }
}

impl From<SpawnRequestError> for DeployError {
    fn from(e: SpawnRequestError) -> Self {
        match e {
            // `NotDead` cannot happen here: the point check never looks at the player.
            SpawnRequestError::NotDead | SpawnRequestError::NoSuchPoint => DeployError::NoSuchPoint,
            SpawnRequestError::WrongTeam => DeployError::PointWrongTeam,
            SpawnRequestError::NotActive => DeployError::PointNotActive,
            SpawnRequestError::Blocked => DeployError::PointBlocked,
            SpawnRequestError::NoSlots => DeployError::PointNoSlots,
        }
    }
}

fn faction_team(f: Faction) -> Team {
    match f {
        Faction::Us => Team::Us,
        Faction::Ger => Team::Ger,
    }
}

impl World {
    /// Authoritative deploy check (ARCHITECTURE 7.3): player state -> squad -> role quota ->
    /// loadout -> spawn point -> role-switch cooldown.
    pub fn validate_deploy(&self, p: PlayerId, req: &DeployReq) -> Result<DeployOk, DeployError> {
        let sq: &SquadManager = self.squads();
        let entry = sq.entry(p).ok_or(DeployError::NotOnRoster)?;
        let team = entry.team;
        let data = self.config().data;
        // 1. Player state.
        if let Some(life) = self.player_life(p) {
            if !matches!(life, LifeState::Dead { .. }) {
                return Err(DeployError::NotDeployable);
            }
        }
        // 2. Squad.
        if u16::from(req.squad) != entry.squad.unwrap_or(0) {
            return Err(DeployError::WrongSquad);
        }
        // 3. Role quota / unlock.
        let role = data
            .roles
            .get(usize::from(req.role))
            .ok_or(DeployError::UnknownRole)?;
        if self.rules_role_locked(p, &role.id) {
            return Err(DeployError::RoleCooldown);
        }
        sq.role_available(p, &role.id)?;
        // 4. Loadout: belongs to the role and the faction, unlocked.
        let lo = data
            .loadouts
            .get(usize::from(req.loadout))
            .ok_or(DeployError::UnknownLoadout)?;
        if lo.role != role.id {
            return Err(DeployError::LoadoutWrongRole);
        }
        if faction_team(lo.faction) != team {
            return Err(DeployError::LoadoutWrongFaction);
        }
        if lo.unlock_level > M1_UNLOCKED_LEVEL {
            return Err(DeployError::LoadoutLocked);
        }
        // 5. Spawn point.
        if req.spawn_point == SPAWN_HQ {
            if self.deployment_hq(team).is_none() {
                return Err(DeployError::NoHq);
            }
        } else {
            if entry.squad.is_none() {
                return Err(DeployError::UnassignedNeedsHq);
            }
            self.check_garrison_point(team, req.spawn_point)?;
        }
        // 6. Role-switch cooldown (changing role at most once per window).
        if entry.role.is_some_and(|r| r != role.id) {
            if let Some(t) = entry.last_role_change {
                if elapsed(self.tick(), t) < sq.cfg.role_switch_cooldown {
                    return Err(DeployError::RoleCooldown);
                }
            }
        }
        Ok(DeployOk {
            role: data.role(&role.id).expect("listed").id.as_str(),
        })
    }
}
