//! Squad-owned, finite-capacity spawn points (M2-011).

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use glam::Vec3;

use crate::components::{EntityTable, LastInput, Team, Transform};
use crate::data::defs::OutpostDef;
use crate::hash::HashWriter;
use crate::respawn::ms_to_ticks;
use crate::squad::SquadId;
use crate::{EntityId, LifeState, PlayerId, SimEvent};

// Keep the existing deploy wire format and reserve a disjoint range from Garrison/HQ.
pub const ID_START: u16 = 0x8000;
pub const ID_END: u16 = u16::MAX - 1;

pub fn is_outpost(id: u16) -> bool {
    (ID_START..=ID_END).contains(&id)
}

pub(crate) fn distance_sq(a: Vec3, b: Vec3) -> f32 {
    let x = a.x - b.x;
    let z = a.z - b.z;
    x * x + z * z
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutpostError {
    NeedSquadmates,
    EnemyNearby,
    TooCloseFriendly,
    TooCloseEnemySP,
    Slope,
    InVehicle,
    Cooldown,
    WrongRole,
    NotAlive,
    Unreachable,
    AlreadyPlacing,
    IdExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutpostState {
    pub id: u16,
    pub team: Team,
    pub squad: SquadId,
    pub pos: Vec3,
    pub hp: f32,
    pub spawns_left: u8,
    pub expires_tick: u32,
    pub blocked: bool,
    pub(crate) overrun_ticks: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutpostStatus {
    Active,
    Blocked,
    Destroyed,
}

impl OutpostState {
    pub(crate) fn event(self, status: OutpostStatus) -> SimEvent {
        SimEvent::OutpostChanged {
            point: self,
            status,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OutpostPlace {
    pub builder: PlayerId,
    pub entity: EntityId,
    pub team: Team,
    pub squad: SquadId,
    pub pos: Vec3,
    pub anchor: Vec3,
    pub complete_at: u32,
    pub last_hp: f32,
}

#[derive(Resource, Default)]
pub(crate) struct Outposts {
    pub list: BTreeMap<u16, OutpostState>,
    pub placing: BTreeMap<SquadId, OutpostPlace>,
    pub cooldown: BTreeMap<SquadId, u32>,
    pub choice: BTreeMap<PlayerId, u16>,
    pub next: u16,
}

impl Outposts {
    pub fn remove(&mut self, id: u16, now: u32, def: &OutpostDef, hz: u8) -> Option<SimEvent> {
        let point = self.list.remove(&id)?;
        self.cooldown
            .insert(point.squad, now + ms_to_ticks(def.cooldown_ms, hz));
        self.placing.remove(&point.squad);
        Some(point.event(OutpostStatus::Destroyed))
    }

    pub fn damage(
        &mut self,
        id: u16,
        raw: f32,
        explosive: bool,
        now: u32,
        def: &OutpostDef,
        hz: u8,
    ) -> Option<SimEvent> {
        let point = self.list.get_mut(&id)?;
        let mult = if explosive {
            def.explosion_mult
        } else {
            def.bullet_mult
        };
        point.hp -= raw.max(0.0) * mult;
        if point.hp <= 0.0 {
            return self.remove(id, now, def, hz);
        }
        None
    }

    pub fn usable(&self, id: u16, now: u32) -> bool {
        self.list
            .get(&id)
            .is_some_and(|p| !p.blocked && p.spawns_left > 0 && now < p.expires_tick)
    }

    pub fn hash_into(&self, h: &mut HashWriter) {
        if self.next == 0 && self.placing.is_empty() && self.choice.is_empty() {
            return;
        }
        h.write_u8(0xD4);
        h.write_u16(self.next);
        h.write_u32(self.list.len() as u32);
        for p in self.list.values() {
            h.write_u16(p.id);
            h.write_u8(p.team as u8);
            h.write_u16(p.squad);
            h.write_u8(p.spawns_left);
            h.write_u8(p.blocked as u8);
            h.write_u32(p.expires_tick);
            h.write_u32(p.overrun_ticks);
            h.write_f32(p.hp);
            for n in p.pos.to_array() {
                h.write_f32(n);
            }
        }
        h.write_u32(self.placing.len() as u32);
        for p in self.placing.values() {
            h.write_u8(p.builder);
            h.write_u16(p.entity);
            h.write_u8(p.team as u8);
            h.write_u16(p.squad);
            h.write_u32(p.complete_at);
            h.write_f32(p.last_hp);
            for n in p.pos.to_array().into_iter().chain(p.anchor.to_array()) {
                h.write_f32(n);
            }
        }
        h.write_u32(self.cooldown.len() as u32);
        for (&squad, &until) in &self.cooldown {
            h.write_u16(squad);
            h.write_u32(until);
        }
        h.write_u32(self.choice.len() as u32);
        for (&player, &point) in &self.choice {
            h.write_u8(player);
            h.write_u16(point);
        }
    }
}

/// Server-only occupancy is omitted from preview, to avoid using it as a proximity detector.
pub(crate) struct PlaceRules<'a> {
    pub pos: Vec3,
    pub nearby_mates: usize,
    pub enemy_nearby: bool,
    pub friendly_spawns: &'a [Vec3],
    pub enemy_points: &'a [(Vec3, f32)],
    pub slope: Option<f32>,
    pub mounted: bool,
    pub now: u32,
    pub cooldown_until: u32,
}

pub(crate) fn check_place(def: &OutpostDef, rules: &PlaceRules<'_>) -> Result<(), OutpostError> {
    use OutpostError::*;
    if rules.mounted {
        return Err(InVehicle);
    }
    if rules.now < rules.cooldown_until {
        return Err(Cooldown);
    }
    if rules.nearby_mates < usize::from(def.required_squadmates) {
        return Err(NeedSquadmates);
    }
    if rules.enemy_nearby {
        return Err(EnemyNearby);
    }
    if rules
        .friendly_spawns
        .iter()
        .any(|&p| distance_sq(p, rules.pos) < def.friendly_spawn_spacing_m.powi(2))
    {
        return Err(TooCloseFriendly);
    }
    if rules.enemy_points.iter().any(|&(p, radius)| {
        distance_sq(p, rules.pos) < (radius + def.enemy_strongpoint_buffer_m).powi(2)
    }) {
        return Err(TooCloseEnemySP);
    }
    if !rules.slope.is_some_and(|s| s <= def.placement_slope_deg) {
        return Err(Slope);
    }
    Ok(())
}

pub(crate) fn alive(life: &LifeState) -> bool {
    !life.is_dead() && !life.is_downed()
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn tick_outposts(
    tick: Res<crate::components::CurrentTick>,
    cfg: Res<crate::components::SimCfgRes>,
    table: Res<EntityTable>,
    mut outposts: ResMut<Outposts>,
    mut events: ResMut<crate::components::EventBuf>,
    objectives: Res<crate::rules::Objectives>,
    changes: Res<crate::rules::RuleChangeBuf>,
    mut profiler: Option<ResMut<crate::step::StepProfiler>>,
    players: Query<(
        &Transform,
        &Team,
        &LifeState,
        &LastInput,
        &crate::health::Health,
        Option<&crate::vehicle::seats::InVehicle>,
    )>,
) {
    if outposts.list.is_empty() && outposts.placing.is_empty() {
        return;
    }
    let _sample = crate::step::StepProfiler::measure(&mut profiler, |t| &mut t.outpost_rules);
    let now = tick.0 .0;
    let def = cfg.data.outpost();
    let hz = cfg.hz;
    let mut remove = Vec::new();
    for point in outposts.list.values_mut() {
        let mut enemy = false;
        let mut near_enemy = false;
        let mut near_friendly = false;
        for (_, e) in table.sorted() {
            let Ok((t, team, life, _, _, _)) = players.get(e) else {
                continue;
            };
            if !alive(life) {
                continue;
            }
            let d = distance_sq(t.pos, point.pos);
            if *team == point.team {
                near_friendly |= d <= def.overrun_radius_m.powi(2);
            } else {
                enemy |= d <= def.blocked_radius_m.powi(2);
                near_enemy |= d <= def.overrun_radius_m.powi(2);
            }
        }
        point.overrun_ticks = if near_enemy && !near_friendly {
            point.overrun_ticks + 1
        } else {
            0
        };
        // M2-008 sector polygons are not installed yet. The capture hook receives the
        // nearest objective's ownership transition, the same partition used by M1 placement.
        let sector = objectives
            .0
            .iter()
            .min_by(|a, b| distance_sq(a.pos, point.pos).total_cmp(&distance_sq(b.pos, point.pos)));
        let lost = sector.is_some_and(|o| {
            distance_sq(o.pos, point.pos) < def.sector_loss_radius_m.powi(2)
                && changes.0.iter().any(|c| matches!(c, crate::RuleChange::ObjectiveOwner { objective, owner, .. } if *objective == o.id && *owner != point.team))
        });
        if now >= point.expires_tick
            || point.overrun_ticks >= ms_to_ticks(def.overrun_ms, hz)
            || lost
        {
            remove.push(point.id);
        } else if point.blocked != enemy {
            point.blocked = enemy;
            events.0.push(point.event(if enemy {
                OutpostStatus::Blocked
            } else {
                OutpostStatus::Active
            }));
        }
    }
    for id in remove {
        events.0.extend(outposts.remove(id, now, def, hz));
    }
    let mut completed = Vec::new();
    outposts.placing.retain(|_, site| {
        let Some(e) = table.get(site.entity) else {
            return false;
        };
        let Ok((t, _, life, last, hp, seat)) = players.get(e) else {
            return false;
        };
        let moved = last.cmd.move_x != 0
            || last.cmd.move_z != 0
            || t.pos.distance_squared(site.anchor) > 0.01;
        if !alive(life) || moved || hp.hp < site.last_hp || seat.is_some_and(|s| s.0.is_some()) {
            return false;
        }
        site.last_hp = hp.hp;
        if now >= site.complete_at {
            completed.push(*site);
            return false;
        }
        true
    });
    for site in completed {
        // Completion must still respect physical spacing if another squad finished first.
        if outposts.list.values().any(|p| {
            p.team == site.team
                && p.squad != site.squad
                && distance_sq(p.pos, site.pos) < def.friendly_spawn_spacing_m.powi(2)
        }) {
            continue;
        }
        let Some(id) = ID_START
            .checked_add(outposts.next)
            .filter(|id| *id <= ID_END)
        else {
            continue;
        };
        outposts.next += 1;
        if let Some(old) = outposts
            .list
            .values()
            .find(|p| p.squad == site.squad)
            .map(|p| p.id)
        {
            events.0.extend(outposts.remove(old, now, def, hz));
        }
        let blocked = table.sorted().any(|(_, e)| {
            players.get(e).is_ok_and(|(t, team, life, _, _, _)| {
                *team != site.team
                    && alive(life)
                    && distance_sq(t.pos, site.pos) <= def.blocked_radius_m.powi(2)
            })
        });
        let point = OutpostState {
            id,
            team: site.team,
            squad: site.squad,
            pos: site.pos,
            hp: def.hp,
            spawns_left: def.capacity,
            expires_tick: now + ms_to_ticks(def.lifetime_ms, hz),
            blocked,
            overrun_ticks: 0,
        };
        events.0.push(point.event(if blocked {
            OutpostStatus::Blocked
        } else {
            OutpostStatus::Active
        }));
        outposts.list.insert(id, point);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GameData;

    #[test]
    fn outpost_place_rules_spatial_boundaries() {
        let def = GameData::embedded().outpost();
        let mut rules = PlaceRules {
            pos: Vec3::ZERO,
            nearby_mates: 2,
            enemy_nearby: false,
            friendly_spawns: &[],
            enemy_points: &[],
            slope: Some(30.0),
            mounted: false,
            now: 0,
            cooldown_until: 0,
        };
        assert_eq!(check_place(def, &rules), Ok(()));
        rules.mounted = true;
        assert_eq!(check_place(def, &rules), Err(OutpostError::InVehicle));
        rules.mounted = false;
        let too_close = [Vec3::new(99.99, 0.0, 0.0)];
        rules.friendly_spawns = &too_close;
        assert_eq!(
            check_place(def, &rules),
            Err(OutpostError::TooCloseFriendly)
        );
        let boundary = [Vec3::new(100.0, 0.0, 0.0)];
        rules.friendly_spawns = &boundary;
        rules.slope = Some(30.01);
        assert_eq!(check_place(def, &rules), Err(OutpostError::Slope));
    }
}
