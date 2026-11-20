//! Completed resource nodes and predetermined sites. Construction is handled by M2-010.

use std::collections::BTreeMap;

use bevy_ecs::prelude::Resource;
use glam::Vec3;

use super::objectives::Objectives;
use super::resources::ResourceKind;
use crate::components::Team;
use crate::data::defs::{ResourceNodeDef, ResourceNodeKind};
use crate::hash::HashWriter;
use crate::ids::PlayerId;

pub type ResourceSiteId = u16;
pub type ResourceNodeId = u32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResourceSite {
    pub id: ResourceSiteId,
    /// ID of the active strongpoint representing this sector's owner.
    pub sector: u16,
    pub kind: ResourceKind,
    pub pos: Vec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceNode {
    pub id: ResourceNodeId,
    pub site: ResourceSite,
    pub kind: ResourceKind,
    pub team: Team,
    pub hp: f32,
    pub builders: Vec<PlayerId>,
    pub overrun_ticks: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeSpawnError {
    Disabled,
    UnknownSite,
    WrongKind,
    WrongSectorOwner,
    Occupied,
    SectorKindLimit,
    KindLimit,
    TeamLimit,
    HostileNearby,
    IdExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeDestroyReason {
    Damage,
    Overrun,
    Demolished,
    SectorFlip(Team),
    SectorLost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeDamageKind {
    Bullet,
    Explosive,
}

#[derive(Resource, Debug, Default)]
pub(crate) struct ResourceNodes {
    pub(crate) sites: BTreeMap<ResourceSiteId, ResourceSite>,
    pub(crate) built: BTreeMap<ResourceNodeId, ResourceNode>,
    defs: &'static [ResourceNodeDef],
    next_id: ResourceNodeId,
    hz: u8,
}

impl ResourceNodes {
    pub(crate) fn new(defs: &'static [ResourceNodeDef], hz: u8) -> Self {
        Self {
            defs,
            hz,
            ..Self::default()
        }
    }

    pub(crate) fn def(&self, kind: ResourceKind) -> &'static ResourceNodeDef {
        definition(self.defs, kind)
    }

    pub(crate) fn counts(&self, team: Team) -> [i32; 3] {
        let mut counts = [0; 3];
        for node in self.built.values().filter(|node| node.team == team) {
            counts[node.kind as usize] += 1;
        }
        counts
    }

    pub(crate) fn site_for_build(
        &self,
        site_id: ResourceSiteId,
        kind: ResourceKind,
        team: Team,
        objectives: &Objectives,
    ) -> Result<ResourceSite, NodeSpawnError> {
        let site = *self
            .sites
            .get(&site_id)
            .ok_or(NodeSpawnError::UnknownSite)?;
        if site.kind != kind {
            return Err(NodeSpawnError::WrongKind);
        }
        if !objectives
            .0
            .iter()
            .any(|o| o.id == site.sector && o.owner == Some(team))
        {
            return Err(NodeSpawnError::WrongSectorOwner);
        }
        if self.built.values().any(|node| node.site.id == site.id) {
            return Err(NodeSpawnError::Occupied);
        }
        if self
            .built
            .values()
            .any(|node| node.site.sector == site.sector && node.kind == kind)
        {
            return Err(NodeSpawnError::SectorKindLimit);
        }
        let counts = self.counts(team);
        let def = self.def(kind);
        if counts.iter().sum::<i32>() as u32 >= def.max_per_team {
            return Err(NodeSpawnError::TeamLimit);
        }
        if counts[kind as usize] as u32 >= def.max_per_team_per_kind {
            return Err(NodeSpawnError::KindLimit);
        }
        Ok(site)
    }

    pub(crate) fn insert(
        &mut self,
        site: ResourceSite,
        team: Team,
        builders: &[PlayerId],
    ) -> Result<ResourceNodeId, NodeSpawnError> {
        let id = self
            .next_id
            .checked_add(1)
            .ok_or(NodeSpawnError::IdExhausted)?;
        self.next_id = id;
        let mut builders = builders.to_vec();
        builders.sort_unstable();
        builders.dedup();
        self.built.insert(
            id,
            ResourceNode {
                id,
                site,
                kind: site.kind,
                team,
                hp: self.def(site.kind).max_hp as f32,
                builders,
                overrun_ticks: 0,
            },
        );
        Ok(id)
    }

    pub(crate) fn tick(
        &mut self,
        objectives: &Objectives,
        mut presence: impl FnMut(Vec3, f32) -> [bool; 2],
        mut destroyed: impl FnMut(&ResourceNode, NodeDestroyReason),
    ) {
        let defs = self.defs;
        self.built.retain(|_, node| {
            match objectives
                .0
                .iter()
                .find(|o| o.id == node.site.sector)
                .and_then(|o| o.owner)
            {
                Some(owner) if owner != node.team => {
                    destroyed(node, NodeDestroyReason::SectorFlip(owner));
                    return false;
                }
                None => {
                    destroyed(node, NodeDestroyReason::SectorLost);
                    return false;
                }
                _ => {}
            }
            let def = definition(defs, node.kind);
            let radius = def.overrun_radius_m as f32;
            let nearby = presence(node.site.pos, radius * radius);
            let friendly = node.team as usize;
            if nearby[1 - friendly] && !nearby[friendly] {
                node.overrun_ticks += 1;
                if node.overrun_ticks >= libm::ceil(def.overrun_s * f64::from(self.hz)) as u32 {
                    destroyed(node, NodeDestroyReason::Overrun);
                    return false;
                }
            } else {
                node.overrun_ticks = 0;
            }
            true
        });
    }

    pub(crate) fn hash(&self, h: &mut HashWriter) {
        h.write_u32(self.next_id);
        h.write_u32(self.sites.len() as u32);
        for site in self.sites.values() {
            h.write_u16(site.id);
            h.write_u16(site.sector);
            h.write_u8(site.kind as u8);
            h.write_f32(site.pos.x);
            h.write_f32(site.pos.y);
            h.write_f32(site.pos.z);
        }
        h.write_u32(self.built.len() as u32);
        for node in self.built.values() {
            h.write_u32(node.id);
            h.write_u16(node.site.id);
            h.write_u8(node.team as u8);
            h.write_f32(node.hp);
            h.write_u32(node.overrun_ticks);
            h.write_u32(node.builders.len() as u32);
            h.write_bytes(&node.builders);
        }
    }
}

fn definition(defs: &[ResourceNodeDef], kind: ResourceKind) -> &ResourceNodeDef {
    let kind = match kind {
        ResourceKind::Manpower => ResourceNodeKind::Manpower,
        ResourceKind::Munitions => ResourceNodeKind::Munitions,
        ResourceKind::Fuel => ResourceNodeKind::Fuel,
    };
    defs.iter()
        .find(|def| def.kind == kind)
        .expect("one embedded definition per kind")
}
