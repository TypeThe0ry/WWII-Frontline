//! Role quotas (GAMEDESIGN 4.4). Pure: the caller counts who already holds the role.

use crate::data::defs::RoleDef;
use crate::squad::SquadKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RoleError {
    UnknownRole,
    /// The squad already holds `max_per_squad` of this role.
    Full,
    /// The team already holds `max_per_team` of this role.
    TeamFull,
    /// Squad has fewer players than `unlock.squad_size_min`.
    SquadTooSmall,
    /// Only the squad leader may pick this role.
    LeaderOnly,
    /// Crew role outside an Armor Crew.
    NotInArmor,
    /// Armor Crew / Recon seat that does not take this role.
    WrongSquadKind,
    /// Unassigned players are Riflemen only.
    Unassigned,
}

/// Who is asking and what the squad / team already hold (the asker excluded from counts).
#[derive(Debug, Clone, Copy)]
pub struct RoleCtx<'a> {
    pub squad_kind: Option<SquadKind>,
    /// Current squad size, the asker included.
    pub squad_size: u32,
    pub is_leader: bool,
    pub in_squad_of_role: u32,
    pub in_team_of_role: u32,
    pub crewman_cap: u32,
    /// M1 override: the role Armor Crew members take (`crewman`).
    pub armor_role: &'a str,
}

pub fn role_available(def: &RoleDef, ctx: &RoleCtx) -> Result<(), RoleError> {
    let id = def.id.as_str();
    let Some(kind) = ctx.squad_kind else {
        return if id == "rifleman" {
            Ok(())
        } else {
            Err(RoleError::Unassigned)
        };
    };
    let mut squad_cap = def.max_per_squad;
    match kind {
        SquadKind::ArmorCrew => {
            if id != ctx.armor_role {
                return Err(RoleError::WrongSquadKind);
            }
            squad_cap = Some(ctx.crewman_cap);
        }
        SquadKind::Recon => {
            if id != "rifleman" {
                return Err(RoleError::WrongSquadKind);
            }
        }
        SquadKind::Infantry => {
            if id == ctx.armor_role {
                return Err(RoleError::NotInArmor);
            }
        }
    }
    if let Some(u) = &def.unlock {
        if u.requires_squad_leader && !ctx.is_leader {
            return Err(RoleError::LeaderOnly);
        }
        if ctx.squad_size < u.squad_size_min {
            return Err(RoleError::SquadTooSmall);
        }
    }
    if squad_cap.is_some_and(|m| ctx.in_squad_of_role >= m) {
        return Err(RoleError::Full);
    }
    if def.max_per_team.is_some_and(|m| ctx.in_team_of_role >= m) {
        return Err(RoleError::TeamFull);
    }
    Ok(())
}
