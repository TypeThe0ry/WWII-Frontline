//! World entry points for resource consumers and the construction system.

use super::World;
use crate::components::{Player, Team, Transform};
use crate::health::{Health, LifeState};
use crate::ids::{EntityId, PlayerId};
use crate::rules::resource_node::{
    NodeDamageKind, NodeDestroyReason, NodeSpawnError, ResourceNode, ResourceNodeId, ResourceNodes,
    ResourceSite, ResourceSiteId,
};
use crate::rules::resources::{
    Cost, Insufficient, ResourceKind, Resources, TeamResources, UpkeepClass,
};
use crate::rules::Objectives;
use crate::supply::{limits, SupplyCfg, SupplyCrate, SupplyWorld};
use crate::SimEvent;

impl World {
    pub fn income_per_min(&self, team: Team) -> Resources {
        self.ecs.resource::<TeamResources>().income_per_min(
            team,
            self.ecs.resource::<Objectives>(),
            self.ecs.resource::<ResourceNodes>(),
        )
    }

    pub fn try_spend(&mut self, team: Team, cost: Cost) -> Result<(), Insufficient> {
        self.ecs
            .resource_mut::<TeamResources>()
            .try_spend(team, cost, &mut self.pending_events)
    }

    pub fn refund(&mut self, team: Team, cost: Cost, fraction: f64) {
        self.ecs.resource_mut::<TeamResources>().refund(
            team,
            cost,
            fraction,
            &mut self.pending_events,
        );
    }

    pub fn low_fuel(&self, team: Team) -> bool {
        self.ecs.resource::<TeamResources>().low_fuel(team)
    }

    /// The vehicle spawner supplies its authoritative team. Registering the same id replaces it.
    pub fn register_upkeep(&mut self, vehicle: EntityId, team: Team, class: UpkeepClass) {
        self.ecs
            .resource_mut::<TeamResources>()
            .register(vehicle, team, class);
    }

    pub fn unregister_upkeep(&mut self, vehicle: EntityId) {
        self.ecs.resource_mut::<TeamResources>().unregister(vehicle);
    }

    /// Install prevalidated map data before construction. Repeated site IDs are refused.
    pub fn install_resource_site(&mut self, site: ResourceSite) -> bool {
        if !self.ecs.resource::<TeamResources>().enabled
            || !site.pos.is_finite()
            || !self
                .objective_list()
                .iter()
                .any(|objective| objective.id == site.sector)
        {
            return false;
        }
        use std::collections::btree_map::Entry;
        if let Entry::Vacant(entry) = self
            .ecs
            .resource_mut::<ResourceNodes>()
            .sites
            .entry(site.id)
        {
            entry.insert(site);
            true
        } else {
            false
        }
    }

    pub fn resource_sites(&self) -> impl Iterator<Item = &ResourceSite> {
        self.ecs.resource::<ResourceNodes>().sites.values()
    }

    pub fn resource_nodes(&self) -> impl Iterator<Item = &ResourceNode> {
        self.ecs.resource::<ResourceNodes>().built.values()
    }

    fn resource_site_for_build(
        &self,
        site_id: ResourceSiteId,
        kind: ResourceKind,
        team: Team,
    ) -> Result<ResourceSite, NodeSpawnError> {
        if !self.ecs.resource::<TeamResources>().enabled {
            return Err(NodeSpawnError::Disabled);
        }
        self.ecs.resource::<ResourceNodes>().site_for_build(
            site_id,
            kind,
            team,
            self.ecs.resource::<Objectives>(),
        )
    }

    /// Check the resource-specific placement constraints, including the 40 m enemy exclusion.
    /// Construction handles Engineer permission, the Supplies debit and build duration.
    pub fn check_resource_node_placement(
        &self,
        site_id: ResourceSiteId,
        kind: ResourceKind,
        team: Team,
    ) -> Result<ResourceSite, NodeSpawnError> {
        let site = self.resource_site_for_build(site_id, kind, team)?;
        let radius = self
            .ecs
            .resource::<ResourceNodes>()
            .def(kind)
            .hostile_build_radius_m as f32;
        for (_, entity) in self
            .ecs
            .resource::<crate::components::EntityTable>()
            .sorted()
        {
            let entity = self.ecs.entity(entity);
            if entity.contains::<Player>()
                && entity.get::<Team>().is_some_and(|other| *other != team)
                && entity
                    .get::<LifeState>()
                    .is_some_and(|life| life.is_alive())
                && entity.get::<Health>().is_some_and(|hp| hp.hp > 0.0)
                && entity.get::<Transform>().is_some_and(|transform| {
                    transform.pos.distance_squared(site.pos) < radius * radius
                })
            {
                return Err(NodeSpawnError::HostileNearby);
            }
        }
        Ok(site)
    }

    /// Called after authorized construction finishes. An enemy approaching after placement does
    /// not cancel completion, but starts the overrun timer according to normal rules.
    pub fn spawn_node(
        &mut self,
        site_id: ResourceSiteId,
        kind: ResourceKind,
        team: Team,
        builder_ids: &[PlayerId],
    ) -> Result<ResourceNodeId, NodeSpawnError> {
        let site = self.resource_site_for_build(site_id, kind, team)?;
        self.ecs
            .resource_mut::<ResourceNodes>()
            .insert(site, team, builder_ids)
    }

    /// Called by the combat/Engineer action after it authorizes the removal. Automatic removals
    /// are handled in the resource tick. Demolition returns Supplies via a ground crate.
    pub fn destroy_node(&mut self, id: ResourceNodeId, reason: NodeDestroyReason) -> bool {
        if !matches!(
            reason,
            NodeDestroyReason::Damage | NodeDestroyReason::Demolished
        ) {
            return false;
        }
        let mut nodes = self.ecs.resource_mut::<ResourceNodes>();
        let Some(node) = nodes.built.remove(&id) else {
            return false;
        };
        if reason == NodeDestroyReason::Demolished {
            let def = nodes.def(node.kind);
            let supplies =
                (u64::from(def.build_cost_supplies) * u64::from(def.refund_percent) / 100) as u32;
            if supplies != 0 {
                let cfg = *self.ecs.resource::<SupplyCfg>();
                let mut world = self.ecs.resource_mut::<SupplyWorld>();
                let id = world.next_id;
                world.next_id += 1;
                let seq = world.next_seq;
                world.next_seq += 1;
                let pos = node.site.pos;
                let team = node.team;
                world.crates.insert(
                    id,
                    SupplyCrate {
                        id,
                        team,
                        pos,
                        supplies,
                        hp: cfg.crate_hp,
                        carried_by: None,
                        seq,
                    },
                );
                self.pending_events
                    .push(SimEvent::CratePlaced { id, team, pos });
                for id in limits::evict(&mut world, team, cfg.ground_max) {
                    self.pending_events.push(SimEvent::CrateDestroyed { id });
                }
            }
        }
        self.pending_events
            .push(SimEvent::ResourceNodeDestroyed { id, reason });
        true
    }

    pub fn damage_resource_node(
        &mut self,
        id: ResourceNodeId,
        raw_hp: f32,
        kind: NodeDamageKind,
    ) -> Option<f32> {
        if !raw_hp.is_finite() || raw_hp <= 0.0 {
            return None;
        }
        let mut nodes = self.ecs.resource_mut::<ResourceNodes>();
        let def = nodes.def(nodes.built.get(&id)?.kind);
        let multiplier = match kind {
            NodeDamageKind::Bullet => def.bullet_mult,
            NodeDamageKind::Explosive => def.explosive_mult,
        } as f32;
        let node = nodes.built.get_mut(&id)?;
        node.hp = (node.hp - raw_hp * multiplier).max(0.0);
        let hp = node.hp;
        if hp == 0.0 {
            self.destroy_node(id, NodeDestroyReason::Damage);
        }
        Some(hp)
    }

    pub(super) fn take_resource_output(
        &mut self,
    ) -> (Option<crate::rules::resources::ResourceHistory>, [u64; 3]) {
        self.ecs.resource_mut::<TeamResources>().take_output()
    }
}
