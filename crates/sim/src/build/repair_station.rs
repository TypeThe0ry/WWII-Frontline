//! Automatic hull/module repair and ammunition service for completed Repair Stations.

use serde::Deserialize;

use crate::hash::HashWriter;
use crate::ids::EntityId;
use crate::supply::{Carrying, SupplyWorld, INTERACT_RANGE_M};
use crate::vehicle::{Vehicle, Vehicles};

use super::{BuildActor, BuildState, BuildSystem, ConstructionDef, StructureKind};

#[derive(Debug, Clone, Deserialize)]
pub struct RepairStationDef {
    pub id: String,
    pub construction: ConstructionDef,
    pub service: RepairServiceDef,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct RepairServiceDef {
    pub radius_m: f32,
    pub repair_hp_per_s: f32,
    pub hp_per_supply: f32,
    pub initial_supplies: u32,
    pub restock_supplies: u32,
    pub ammo_interval_s: f64,
    pub ammo_cost_supplies: u32,
}

impl RepairServiceDef {
    fn valid(self) -> bool {
        [self.radius_m, self.repair_hp_per_s, self.hp_per_supply]
            .iter()
            .all(|n| n.is_finite() && *n > 0.0)
            && self.ammo_interval_s.is_finite()
            && self.ammo_interval_s > 0.0
            && self.restock_supplies > 0
            && self.ammo_cost_supplies > 0
    }
}

#[derive(Debug, Clone)]
pub struct RepairStation {
    supplies: u32,
    // Pay in whole Supplies, then spend all the HP bought across ticks. Charging ceil(heal/5)
    // again on every sub-5 HP tick would spend 200 Supplies on only ~133 HP at 30 Hz.
    prepaid_hp: f64,
    ammo_ticks: u32,
}

impl RepairStation {
    fn new(def: RepairServiceDef) -> Self {
        Self {
            supplies: def.initial_supplies,
            prepaid_hp: 0.0,
            ammo_ticks: 0,
        }
    }

    pub fn supplies(&self) -> u32 {
        self.supplies
    }

    pub(super) fn hash_into(&self, hash: &mut HashWriter) {
        hash.write_u32(self.supplies);
        hash.write_u64(self.prepaid_hp.to_bits());
        hash.write_u32(self.ammo_ticks);
    }

    fn repair(&mut self, vehicle: &mut Vehicle, mut budget_hp: f64, hp_per_supply: f64) {
        budget_hp -= self.heal(
            &mut vehicle.hull_hp,
            vehicle.params.hull_hp,
            budget_hp,
            hp_per_supply,
        );
        // Use any remaining time on intact modules, following their existing data order.
        for (hp, spec) in vehicle
            .modules
            .hp
            .iter_mut()
            .zip(&vehicle.params.module_specs.0)
        {
            if budget_hp <= 0.0 {
                break;
            }
            budget_hp -= self.heal(hp, spec.max_hp, budget_hp, hp_per_supply);
        }
    }

    fn heal(&mut self, hp: &mut f32, max: f32, budget_hp: f64, hp_per_supply: f64) -> f64 {
        if !hp.is_finite() || *hp <= 0.0 || *hp >= max || !max.is_finite() {
            return 0.0;
        }
        let before = f64::from(*hp);
        let available = f64::from(self.supplies) * hp_per_supply + self.prepaid_hp;
        let target = (before + budget_hp.min(available)).min(f64::from(max));
        let mut after = target as f32;
        if f64::from(after) > target {
            after = after.next_down();
        }
        if after <= *hp {
            return 0.0;
        }
        let healed = f64::from(after) - before;
        let cost =
            (libm::ceil((healed - self.prepaid_hp) / hp_per_supply) as u32).min(self.supplies);
        self.supplies -= cost;
        self.prepaid_hp = (self.prepaid_hp + f64::from(cost) * hp_per_supply - healed).max(0.0);
        *hp = after;
        healed
    }

    fn refill_one(&mut self, vehicle: &mut Vehicle, def: RepairServiceDef, interval: u32) {
        if self.ammo_ticks < interval || self.supplies < def.ammo_cost_supplies {
            return;
        }
        let missing = vehicle
            .gun
            .ammo
            .iter_mut()
            .zip(&vehicle.params.gun.ammo)
            .find(|(n, max)| **n < **max);
        if let Some((ammo, _)) = missing {
            *ammo += 1;
            self.ammo_ticks = 0;
            self.supplies -= def.ammo_cost_supplies;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestockError {
    UnknownStation,
    NotBuilt,
    WrongRole,
    WrongTeam,
    OutOfReach,
    NoCrate,
    InvalidDefinition,
    StockOverflow,
}

impl BuildSystem {
    pub(crate) fn deposit_nearby(
        &mut self,
        actor: BuildActor<'_>,
        def: RepairServiceDef,
        supplies: &mut SupplyWorld,
        carrying: &mut Carrying,
    ) -> Option<u32> {
        let id = self
            .structures()
            .find(|s| {
                s.kind == StructureKind::RepairStation
                    && s.state == BuildState::Done
                    && s.team == actor.team
                    && s.pos.distance_squared(actor.pos) <= INTERACT_RANGE_M.powi(2)
            })?
            .id;
        self.restock_repair_station(id, actor, def, supplies, carrying)
            .ok()
    }

    /// Call once each tick after advancing construction and resolving vehicle damage. Repair
    /// has priority over refill for each vehicle; all iteration is in stable entity ID order.
    pub fn tick_repair_stations(&mut self, def: RepairServiceDef, hz: u8, vehicles: &mut Vehicles) {
        let interval = libm::ceil(def.ammo_interval_s * f64::from(hz));
        if !def.valid() || hz == 0 || interval > f64::from(u32::MAX) {
            return;
        }
        let interval = (interval as u32).max(1);
        let budget = f64::from(def.repair_hp_per_s) / f64::from(hz);
        let radius_sq = def.radius_m * def.radius_m;
        for structure in self
            .structures
            .values_mut()
            .filter(|s| s.kind == StructureKind::RepairStation && s.state == BuildState::Done)
        {
            let station = structure
                .repair_station
                .get_or_insert_with(|| RepairStation::new(def));
            station.ammo_ticks = station.ammo_ticks.saturating_add(1).min(interval);
            if station.supplies == 0 && station.prepaid_hp == 0.0 {
                continue;
            }
            for vehicle in vehicles.list.values_mut().filter(|v| {
                v.team == structure.team
                    && !v.gone
                    && !v.knocked_out
                    && v.hull_hp > 0.0
                    && v.pos.distance_squared(structure.pos) <= radius_sq
            }) {
                station.repair(vehicle, budget, f64::from(def.hp_per_supply));
                station.refill_one(vehicle, def, interval);
            }
        }
    }

    /// Commit a Support crate deposit after the existing USE hold or a dedicated interaction.
    /// The actual carried crate supplies the stock. A partially spent crate deposits only its
    /// remainder, and a failed deposit leaves both ledgers untouched.
    pub fn restock_repair_station(
        &mut self,
        id: EntityId,
        actor: BuildActor<'_>,
        def: RepairServiceDef,
        supplies: &mut SupplyWorld,
        carrying: &mut Carrying,
    ) -> Result<u32, RestockError> {
        let structure = self
            .structures
            .get_mut(&id)
            .filter(|s| s.kind == StructureKind::RepairStation)
            .ok_or(RestockError::UnknownStation)?;
        if structure.state != BuildState::Done {
            return Err(RestockError::NotBuilt);
        }
        if actor.role != "support" {
            return Err(RestockError::WrongRole);
        }
        if actor.team != structure.team {
            return Err(RestockError::WrongTeam);
        }
        if !actor.pos.is_finite()
            || actor.pos.distance_squared(structure.pos) > INTERACT_RANGE_M * INTERACT_RANGE_M
        {
            return Err(RestockError::OutOfReach);
        }
        if !def.valid() {
            return Err(RestockError::InvalidDefinition);
        }
        let crate_id = carrying.0.ok_or(RestockError::NoCrate)?;
        let source = supplies
            .crates
            .get_mut(&crate_id)
            .filter(|c| {
                c.carried_by == Some(actor.id)
                    && c.team == actor.team
                    && c.hp > 0.0
                    && c.supplies > 0
            })
            .ok_or(RestockError::NoCrate)?;
        let deposit = source.supplies.min(def.restock_supplies);
        let stock = structure
            .repair_station
            .as_ref()
            .map_or(def.initial_supplies, RepairStation::supplies)
            .checked_add(deposit)
            .ok_or(RestockError::StockOverflow)?;
        let station = structure
            .repair_station
            .get_or_insert_with(|| RepairStation::new(def));
        station.supplies = stock;
        source.supplies -= deposit;
        if source.supplies == 0 {
            supplies.crates.remove(&crate_id);
            carrying.0 = None;
        }
        Ok(deposit)
    }
}
