//! Deterministic per-team strategic economy. Supplies are handled in `crate::supply`.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;

use super::objectives::Objectives;
use super::resource_node::{NodeDestroyReason, ResourceNodes};
use crate::components::{CurrentTick, EventBuf, Player, SimCfgRes, Team, Transform};
use crate::data::defs::EconomyDef;
use crate::hash::HashWriter;
use crate::health::{Health, LifeState};
use crate::ids::EntityId;
use crate::step::{SimEvent, StepProfiler};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Resources {
    pub manpower: i32,
    pub munitions: i32,
    pub fuel: i32,
}

impl Resources {
    pub const fn new(manpower: i32, munitions: i32, fuel: i32) -> Self {
        Self {
            manpower,
            munitions,
            fuel,
        }
    }

    fn as_array(self) -> [i32; 3] {
        [self.manpower, self.munitions, self.fuel]
    }
}

pub type Cost = Resources;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResourceKind {
    Manpower = 0,
    Munitions = 1,
    Fuel = 2,
}

impl ResourceKind {
    pub const ALL: [Self; 3] = [Self::Manpower, Self::Munitions, Self::Fuel];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResourceReason {
    Income = 0,
    Spend = 1,
    Refund = 2,
    Upkeep = 3,
    Spoils = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Insufficient {
    Disabled,
    InvalidCost,
    Resource(ResourceKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UpkeepClass {
    Light,
    Medium,
    Heavy,
    None,
}

/// A minute-boundary snapshot, in stable team order (US, GER).
pub type ResourceHistory = [Resources; 2];

#[derive(Resource, Debug)]
pub struct TeamResources {
    economy: &'static EconomyDef,
    pub(crate) enabled: bool,
    pools: [[i32; 3]; 2],
    upkeep: BTreeMap<EntityId, (Team, UpkeepClass)>,
    pending_overflow: [u64; 3],
    history: Option<ResourceHistory>,
}

impl TeamResources {
    pub(crate) fn new(enabled: bool, economy: &'static EconomyDef) -> Self {
        let initial = if enabled {
            [
                economy.manpower.initial,
                economy.munitions.initial,
                economy.fuel.initial,
            ]
        } else {
            [0; 3]
        };
        Self {
            economy,
            enabled,
            pools: [initial; 2],
            upkeep: BTreeMap::new(),
            pending_overflow: [0; 3],
            history: None,
        }
    }

    pub(crate) fn get(&self, team: Team) -> Resources {
        let [m, u, f] = self.pools[team as usize];
        Resources::new(m, u, f)
    }

    pub(crate) fn low_fuel(&self, team: Team) -> bool {
        self.enabled && self.get(team).fuel == 0
    }

    pub(crate) fn register(&mut self, vehicle: EntityId, team: Team, class: UpkeepClass) {
        if self.enabled {
            self.upkeep.insert(vehicle, (team, class));
        }
    }

    pub(crate) fn unregister(&mut self, vehicle: EntityId) {
        self.upkeep.remove(&vehicle);
    }

    pub(crate) fn try_spend(
        &mut self,
        team: Team,
        cost: Cost,
        events: &mut Vec<SimEvent>,
    ) -> Result<(), Insufficient> {
        if !self.enabled {
            return Err(Insufficient::Disabled);
        }
        let costs = cost.as_array();
        if costs.iter().any(|&cost| cost < 0) {
            return Err(Insufficient::InvalidCost);
        }
        for kind in ResourceKind::ALL {
            let i = kind as usize;
            if self.pools[team as usize][i] < costs[i] {
                return Err(Insufficient::Resource(kind));
            }
        }
        for kind in ResourceKind::ALL {
            self.apply(
                team,
                kind,
                -costs[kind as usize],
                ResourceReason::Spend,
                events,
            );
        }
        Ok(())
    }

    pub(crate) fn refund(
        &mut self,
        team: Team,
        cost: Cost,
        fraction: f64,
        events: &mut Vec<SimEvent>,
    ) {
        let costs = cost.as_array();
        if !self.enabled || !(0.0..=1.0).contains(&fraction) || costs.iter().any(|&cost| cost < 0) {
            return;
        }
        for kind in ResourceKind::ALL {
            let amount = (f64::from(costs[kind as usize]) * fraction) as i32;
            self.apply(team, kind, amount, ResourceReason::Refund, events);
        }
    }

    fn apply(
        &mut self,
        team: Team,
        kind: ResourceKind,
        amount: i32,
        reason: ResourceReason,
        events: &mut Vec<SimEvent>,
    ) {
        let caps = [
            self.economy.manpower.cap,
            self.economy.munitions.cap,
            self.economy.fuel.cap,
        ];
        let i = kind as usize;
        let pool = &mut self.pools[team as usize][i];
        let raw = i64::from(*pool) + i64::from(amount);
        self.pending_overflow[i] += (raw - i64::from(caps[i])).max(0) as u64;
        let next = raw.clamp(0, i64::from(caps[i])) as i32;
        let value = next - *pool;
        *pool = next;
        if value != 0 {
            events.push(SimEvent::ResourceDelta {
                team,
                kind,
                value,
                reason,
            });
        }
    }

    pub(crate) fn income_per_min(
        &self,
        team: Team,
        objectives: &Objectives,
        nodes: &ResourceNodes,
    ) -> Resources {
        if !self.enabled {
            return Resources::default();
        }
        let sectors = objectives
            .0
            .iter()
            .filter(|o| o.owner == Some(team))
            .count() as i32;
        let counts = nodes.counts(team);
        let rates = [
            &self.economy.manpower,
            &self.economy.munitions,
            &self.economy.fuel,
        ];
        let [m, u, f] = std::array::from_fn(|i| {
            let rate = rates[i];
            rate.base + sectors * rate.sector_rate + counts[i] * rate.node_rate
        });
        Resources::new(m, u, f)
    }

    fn income_tick(
        &mut self,
        team: Team,
        objectives: &Objectives,
        nodes: &ResourceNodes,
        events: &mut Vec<SimEvent>,
    ) {
        let income = self.income_per_min(team, objectives, nodes).as_array();
        for kind in ResourceKind::ALL {
            self.apply(
                team,
                kind,
                income[kind as usize],
                ResourceReason::Income,
                events,
            );
        }
        let upkeep = self
            .upkeep
            .values()
            .filter(|(owner, _)| *owner == team)
            .map(|(_, class)| match class {
                UpkeepClass::Light => self.economy.upkeep.light,
                UpkeepClass::Medium => self.economy.upkeep.medium,
                UpkeepClass::Heavy => self.economy.upkeep.heavy,
                UpkeepClass::None => 0,
            })
            .sum::<i32>();
        self.apply(
            team,
            ResourceKind::Fuel,
            -upkeep,
            ResourceReason::Upkeep,
            events,
        );
    }

    pub(crate) fn take_output(&mut self) -> (Option<ResourceHistory>, [u64; 3]) {
        (
            self.history.take(),
            std::mem::take(&mut self.pending_overflow),
        )
    }

    pub(crate) fn hash(&self, h: &mut HashWriter) {
        for pool in self.pools {
            for value in pool {
                h.write_u32(value as u32);
            }
        }
        h.write_u32(self.upkeep.len() as u32);
        for (&vehicle, &(team, class)) in &self.upkeep {
            h.write_u16(vehicle);
            h.write_u8(team as u8);
            h.write_u8(class as u8);
        }
    }
}

pub(crate) fn resources_tick(
    (tick, cfg): (Res<CurrentTick>, Res<SimCfgRes>),
    objectives: Res<Objectives>,
    mut pools: ResMut<TeamResources>,
    mut nodes: ResMut<ResourceNodes>,
    mut events: ResMut<EventBuf>,
    players: Query<(&Transform, &Team, &Health, &LifeState), With<Player>>,
    mut profiler: Option<ResMut<StepProfiler>>,
) {
    if !pools.enabled {
        return;
    }
    let _sample = StepProfiler::measure(&mut profiler, |t| &mut t.resources);
    // This runs immediately after capture. An already-built node cannot pay out on the
    // minute on which it is lost, and an overrun cannot double-pay the flip reward.
    nodes.tick(
        &objectives,
        |pos, radius_sq| {
            let mut nearby = [false; 2];
            for (transform, team, hp, life) in &players {
                if life.is_alive()
                    && hp.hp > 0.0
                    && transform.pos.distance_squared(pos) <= radius_sq
                {
                    nearby[*team as usize] = true;
                    if nearby == [true; 2] {
                        break;
                    }
                }
            }
            nearby
        },
        |node, reason| {
            events.0.push(SimEvent::ResourceNodeDestroyed {
                id: node.id,
                reason,
            });
            if let NodeDestroyReason::SectorFlip(winner) = reason {
                if node.kind == ResourceKind::Manpower {
                    let spoils = pools.economy.spoils_manpower;
                    pools.apply(
                        winner,
                        ResourceKind::Manpower,
                        spoils,
                        ResourceReason::Spoils,
                        &mut events.0,
                    );
                }
            }
        },
    );
    let minute = 60 * u32::from(cfg.hz);
    let now = tick.0 .0;
    if minute != 0 && now != 0 && now.is_multiple_of(minute) {
        for team in [Team::Us, Team::Ger] {
            pools.income_tick(team, &objectives, &nodes, &mut events.0);
        }
        pools.history = Some([pools.get(Team::Us), pools.get(Team::Ger)]);
    }
}
