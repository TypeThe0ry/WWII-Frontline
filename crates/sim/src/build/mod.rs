//! Shared construction for M2-011 facilities. The World supplies authoritative player and
//! map facts and uses its existing entity allocator for the resulting structure IDs.

pub(crate) mod damage;
pub mod repair_station;
mod supply;

use std::collections::BTreeMap;

use bevy_ecs::prelude::Resource;
use glam::Vec3;
use serde::Deserialize;

use crate::components::Team;
use crate::hash::HashWriter;
use crate::ids::{EntityAllocator, EntityId, PlayerId};
use crate::supply::SupplyWorld;

use repair_station::RepairStation;

pub(crate) fn pay_facility_supplies(
    sw: &mut SupplyWorld,
    team: Team,
    pos: Vec3,
    amount: u32,
) -> bool {
    if !supply::can_pay(sw, team, pos, amount) {
        return false;
    }
    supply::pay(sw, team, pos, amount);
    true
}

pub const TEAM_BUILD_CAP: usize = 120;
pub const BUILD_RANGE_M: f32 = 3.0;
const MAX_BUILDERS: usize = 3;
const BASE_WORK: u32 = 10;
const ASSIST_WORK: u32 = 6;
const INITIAL_HP_FRACTION: f32 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StructureKind {
    RepairStation,
    AtGun,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConstructionDef {
    pub role: String,
    pub cost_supplies: u32,
    pub build_s: f64,
    pub hp: f32,
    pub team_cap: usize,
    #[serde(default)]
    pub sector_cap: Option<usize>,
    pub armor: StructureArmor,
}

impl ConstructionDef {
    fn required_work(&self, hz: u8) -> Result<u32, BuildError> {
        let ticks = libm::ceil(self.build_s * f64::from(hz));
        if !ticks.is_finite()
            || ticks < 1.0
            || ticks > f64::from(u32::MAX / BASE_WORK)
            || !self.hp.is_finite()
            || self.hp <= 0.0
            || self.team_cap == 0
            || self.sector_cap == Some(0)
            || self.role.is_empty()
            || !self.armor.valid()
        {
            return Err(BuildError::InvalidDefinition);
        }
        Ok((ticks as u32) * BASE_WORK)
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct StructureArmor {
    pub bullet: f32,
    pub explosion: f32,
    pub he: f32,
    pub heat: f32,
}

impl StructureArmor {
    fn coefficients(self) -> [f32; 4] {
        [self.bullet, self.explosion, self.he, self.heat]
    }

    fn valid(self) -> bool {
        self.coefficients()
            .iter()
            .all(|m| m.is_finite() && *m >= 0.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StructureDamage {
    Bullet,
    Explosion,
    He,
    Heat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageOutcome {
    Missing,
    Unchanged,
    Damaged,
    Destroyed,
}

/// Standing, living, dismounted player resolved by the authoritative caller.
#[derive(Debug, Clone, Copy)]
pub struct BuildActor<'a> {
    pub id: PlayerId,
    pub team: Team,
    pub role: &'a str,
    pub pos: Vec3,
}

/// Map-dependent placement facts. Compute these from the same snapped site used to start building.
/// The enemy check uses 15 m and the strongpoint exclusion uses R_sp + 15 m (GD 12.4).
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildSite {
    pub pos: Vec3,
    pub yaw: f32,
    pub sector: u16,
    pub friendly_or_contested: bool,
    pub inside_strongpoint_buffer: bool,
    pub enemy_nearby: bool,
    pub blocks_nav_critical: bool,
    pub slope_too_steep: bool,
    pub in_water: bool,
    pub overlap: bool,
    pub in_no_build_zone: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildError {
    WrongRole,
    WrongTeam,
    InvalidDefinition,
    InvalidPose,
    OutOfReach,
    NotOwnSector,
    TooCloseToStrongpoint,
    EnemyNearby,
    BlocksNavCritical,
    TeamCapReached,
    TypeCapReached,
    SectorCapReached,
    NoSupplies,
    SlopeTooSteep,
    InWater,
    Overlap,
    InNoBuildZone,
    NoEntitySlot,
    UnknownStructure,
    AlreadyDone,
    BuilderCapReached,
    AlreadyBuilding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildState {
    Building,
    Done,
}

#[derive(Debug, Clone)]
pub struct Structure {
    pub id: EntityId,
    pub kind: StructureKind,
    pub team: Team,
    pub sector: u16,
    pub pos: Vec3,
    pub yaw: f32,
    pub hp: f32,
    pub hp_max: f32,
    pub state: BuildState,
    builders: [Option<PlayerId>; MAX_BUILDERS],
    work: u32,
    required_work: u32,
    armor: StructureArmor,
    damage_taken: f32,
    repair_station: Option<RepairStation>,
}

impl Structure {
    pub fn progress(&self) -> f32 {
        self.work as f32 / self.required_work as f32
    }

    pub fn builders(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.builders.iter().flatten().copied()
    }

    /// Available after the first station service tick following construction.
    pub fn repair_station(&self) -> Option<&RepairStation> {
        self.repair_station.as_ref()
    }

    fn update_hp(&mut self) {
        let built_fraction = INITIAL_HP_FRACTION + (1.0 - INITIAL_HP_FRACTION) * self.progress();
        self.hp = (self.hp_max * built_fraction - self.damage_taken).max(0.0);
    }
}

#[derive(Debug, Default, Resource)]
pub struct BuildSystem {
    structures: BTreeMap<EntityId, Structure>,
}

impl BuildSystem {
    pub fn structures(&self) -> impl Iterator<Item = &Structure> {
        self.structures.values()
    }

    pub fn get(&self, id: EntityId) -> Option<&Structure> {
        self.structures.get(&id)
    }

    pub fn validate_placement(
        &self,
        actor: BuildActor<'_>,
        kind: StructureKind,
        site: BuildSite,
        def: &ConstructionDef,
        hz: u8,
        supplies: &SupplyWorld,
    ) -> Result<(), BuildError> {
        let expected_role = match kind {
            StructureKind::RepairStation => "engineer",
            StructureKind::AtGun => "anti_tank",
        };
        if def.role != expected_role {
            return Err(BuildError::InvalidDefinition);
        }
        if actor.role != def.role {
            return Err(BuildError::WrongRole);
        }
        def.required_work(hz)?;
        if !site.pos.is_finite() || !site.yaw.is_finite() || !actor.pos.is_finite() {
            return Err(BuildError::InvalidPose);
        }
        if !in_build_range(actor.pos, site.pos) {
            return Err(BuildError::OutOfReach);
        }
        if !site.friendly_or_contested {
            return Err(BuildError::NotOwnSector);
        }
        for (fails, reason) in [
            (
                site.inside_strongpoint_buffer,
                BuildError::TooCloseToStrongpoint,
            ),
            (site.enemy_nearby, BuildError::EnemyNearby),
            (site.blocks_nav_critical, BuildError::BlocksNavCritical),
            (site.slope_too_steep, BuildError::SlopeTooSteep),
            (site.in_water, BuildError::InWater),
            (site.overlap, BuildError::Overlap),
            (site.in_no_build_zone, BuildError::InNoBuildZone),
        ] {
            if fails {
                return Err(reason);
            }
        }
        let mut team_count = 0;
        let mut type_count = 0;
        let mut sector_count = 0;
        for structure in self.structures() {
            if structure.team == actor.team {
                team_count += 1;
                type_count += usize::from(structure.kind == kind);
            }
            sector_count += usize::from(structure.kind == kind && structure.sector == site.sector);
        }
        if team_count >= TEAM_BUILD_CAP {
            return Err(BuildError::TeamCapReached);
        }
        if type_count >= def.team_cap {
            return Err(BuildError::TypeCapReached);
        }
        if def.sector_cap.is_some_and(|cap| sector_count >= cap) {
            return Err(BuildError::SectorCapReached);
        }
        if self.assigned(actor.id) {
            return Err(BuildError::AlreadyBuilding);
        }
        if !supply::can_pay(supplies, actor.team, site.pos, def.cost_supplies) {
            return Err(BuildError::NoSupplies);
        }
        Ok(())
    }

    /// Central allocator and ground crate ledger are committed only after placement passes.
    #[allow(clippy::too_many_arguments)]
    pub fn start_build(
        &mut self,
        actor: BuildActor<'_>,
        kind: StructureKind,
        site: BuildSite,
        def: &ConstructionDef,
        hz: u8,
        supplies: &mut SupplyWorld,
        alloc: &mut EntityAllocator,
    ) -> Result<EntityId, BuildError> {
        self.validate_placement(actor, kind, site, def, hz, supplies)?;
        let required_work = def.required_work(hz)?;
        let id = alloc.alloc().ok_or(BuildError::NoEntitySlot)?;
        supply::pay(supplies, actor.team, site.pos, def.cost_supplies);
        let structure = Structure {
            id,
            kind,
            team: actor.team,
            sector: site.sector,
            pos: site.pos,
            yaw: site.yaw,
            hp: def.hp * INITIAL_HP_FRACTION,
            hp_max: def.hp,
            state: BuildState::Building,
            builders: [Some(actor.id), None, None],
            work: 0,
            required_work,
            armor: def.armor,
            damage_taken: 0.0,
            repair_station: None,
        };
        self.structures.insert(id, structure);
        Ok(id)
    }

    pub fn join_build(&mut self, id: EntityId, actor: BuildActor<'_>) -> Result<(), BuildError> {
        let building = self.get(id).ok_or(BuildError::UnknownStructure)?;
        if building.state == BuildState::Done {
            return Err(BuildError::AlreadyDone);
        }
        if actor.team != building.team {
            return Err(BuildError::WrongTeam);
        }
        if actor.role != "engineer"
            && !(building.kind == StructureKind::AtGun && actor.role == "anti_tank")
        {
            return Err(BuildError::WrongRole);
        }
        if !in_build_range(actor.pos, building.pos) {
            return Err(BuildError::OutOfReach);
        }
        if building.builders().any(|id| id == actor.id) {
            return Ok(());
        }
        if self.assigned(actor.id) {
            return Err(BuildError::AlreadyBuilding);
        }
        let building = self.structures.get_mut(&id).unwrap();
        let free = building.builders.iter_mut().find(|slot| slot.is_none());
        *free.ok_or(BuildError::BuilderCapReached)? = Some(actor.id);
        Ok(())
    }

    /// Stops this player's participation. An unfinished site remains available to resume.
    pub fn cancel_build(&mut self, id: EntityId, player: PlayerId) -> bool {
        let Some(site) = self.structures.get_mut(&id) else {
            return false;
        };
        Self::unassign(site, player)
    }

    /// Clear a departing/dead player's assignment before the player slot can be reused.
    pub fn leave_all_builds(&mut self, player: PlayerId) {
        for site in self.structures.values_mut() {
            Self::unassign(site, player);
        }
    }

    fn unassign(site: &mut Structure, player: PlayerId) -> bool {
        if let Some(slot) = site.builders.iter_mut().find(|slot| **slot == Some(player)) {
            *slot = None;
            return true;
        }
        false
    }

    fn assigned(&self, player: PlayerId) -> bool {
        self.structures().any(|s| s.builders().any(|p| p == player))
    }

    /// Active builder callback returns a position for a living, dismounted, still-eligible
    /// teammate who took no damage this tick. Missing and distant builders pause in place.
    /// Newly completed IDs are returned in ascending order, once each.
    pub fn tick_build(
        &mut self,
        mut builder_pos: impl FnMut(PlayerId, Team, StructureKind) -> Option<Vec3>,
    ) -> Vec<EntityId> {
        let mut completed = Vec::new();
        for site in self
            .structures
            .values_mut()
            .filter(|s| s.state == BuildState::Building)
        {
            let active = site
                .builders()
                .filter(|&id| {
                    builder_pos(id, site.team, site.kind)
                        .is_some_and(|p| in_build_range(p, site.pos))
                })
                .count() as u32;
            if active == 0 {
                continue;
            }
            let advance = BASE_WORK + (active - 1) * ASSIST_WORK;
            site.work = site.work.saturating_add(advance).min(site.required_work);
            site.update_hp();
            if site.work == site.required_work {
                site.state = BuildState::Done;
                site.builders.fill(None);
                completed.push(site.id);
            }
        }
        completed
    }

    /// Zero HP removes the gameplay state. The caller handles any associated snapshot,
    /// collision, seat teardown or destruction event for this ID.
    pub fn structure_damage(
        &mut self,
        id: EntityId,
        kind: StructureDamage,
        amount: f32,
        alloc: &mut EntityAllocator,
    ) -> DamageOutcome {
        let Some(site) = self.structures.get_mut(&id) else {
            return DamageOutcome::Missing;
        };
        if !amount.is_finite() || amount <= 0.0 {
            return DamageOutcome::Unchanged;
        }
        let effective = amount * site.armor.coefficients()[kind as usize];
        if effective == 0.0 {
            return DamageOutcome::Unchanged;
        }
        site.damage_taken += effective;
        site.update_hp();
        if site.hp > 0.0 {
            return DamageOutcome::Damaged;
        }
        self.structures.remove(&id);
        let freed = alloc.free(id);
        debug_assert!(freed, "structure IDs come from the World allocator");
        DamageOutcome::Destroyed
    }

    /// Empty legacy worlds keep their previous hash. Definitions are covered by the data hash.
    pub fn hash_into(&self, hash: &mut HashWriter) {
        if self.structures.is_empty() {
            return;
        }
        hash.write_bytes(b"build\0");
        hash.write_u16(self.structures.len() as u16);
        for site in self.structures() {
            hash.write_u16(site.id);
            hash.write_u8(site.kind as u8);
            hash.write_u8(site.team as u8);
            hash.write_u16(site.sector);
            for value in [
                site.pos.x,
                site.pos.y,
                site.pos.z,
                site.yaw,
                site.hp_max,
                site.damage_taken,
            ] {
                hash.write_f32(value);
            }
            for coefficient in site.armor.coefficients() {
                hash.write_f32(coefficient);
            }
            hash.write_u32(site.work);
            hash.write_u32(site.required_work);
            for builder in site.builders {
                hash.write_u16(builder.map_or(u16::MAX, u16::from));
            }
            hash.write_u8(u8::from(site.repair_station.is_some()));
            if let Some(station) = &site.repair_station {
                station.hash_into(hash);
            }
        }
    }
}

fn in_build_range(actor: Vec3, site: Vec3) -> bool {
    actor.distance_squared(site) <= BUILD_RANGE_M * BUILD_RANGE_M
}

#[cfg(test)]
mod tests;
