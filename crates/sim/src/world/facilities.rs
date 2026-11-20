//! World-facing M2-011 construction and facility interactions.

use bevy_ecs::prelude::Mut;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_protocol::quant::{dequant_yaw, pitch_i16_to_rad};

use super::World;
use crate::build::{
    BuildActor, BuildError, BuildSite, BuildSystem, ConstructionDef, DamageOutcome, Structure,
    StructureDamage, StructureKind,
};
use crate::components::{EventBuf, PlayerIndex};
use crate::emplacement::at_gun::{Actor, Ammo, AtGunState, AtGuns, Seat};
use crate::supply::{Carrying, SupplyWorld};
use crate::vehicle::{seats::InVehicle, shells::Shells, Vehicles};
use crate::{EntityId, PlayerId, SimEvent, Team, Vec3};

impl World {
    /// Replace in-flight entities between stage-4 benchmark samples.
    #[cfg(feature = "test-util")]
    pub fn debug_seed_facility_flight(
        &mut self,
        projectiles: crate::projectile::Projectiles,
        shells: Shells,
    ) {
        self.ecs.insert_resource(projectiles);
        self.ecs.insert_resource(shells);
    }

    pub fn structures(&self) -> impl Iterator<Item = &Structure> {
        self.ecs.resource::<BuildSystem>().structures()
    }

    fn facility_actor(&self, id: PlayerId) -> Result<BuildActor<'static>, BuildError> {
        let entry = self.squads.entry(id).ok_or(BuildError::WrongRole)?;
        if !self.player_life(id).is_some_and(|l| l.is_alive()) || self.seat_of(id).is_some() {
            return Err(BuildError::InvalidPose);
        }
        Ok(BuildActor {
            id,
            team: entry.team,
            role: entry.role.filter(|_| !entry.role_invalid).unwrap_or(""),
            pos: self.player_state(id).ok_or(BuildError::InvalidPose)?.pos,
        })
    }

    fn facility_def(&self, team: Team, kind: StructureKind) -> ConstructionDef {
        match kind {
            StructureKind::RepairStation => self.cfg.data.repair_station().construction.clone(),
            StructureKind::AtGun => {
                let def = self.cfg.data.at_gun(team);
                ConstructionDef {
                    role: "anti_tank".into(),
                    cost_supplies: u32::from(def.build.supplies),
                    build_s: f64::from(def.build.time_s),
                    hp: def.max_hp(),
                    team_cap: usize::from(def.build.team_limit),
                    sector_cap: Some(usize::from(def.build.sector_limit)),
                    armor: crate::build::StructureArmor {
                        bullet: 0.02,
                        explosion: 1.0,
                        he: 1.0,
                        heat: 1.0,
                    },
                }
            }
        }
    }

    fn facility_site(
        &self,
        actor: BuildActor<'_>,
        pos: Vec3,
        yaw: f32,
        preview: bool,
    ) -> Result<BuildSite, BuildError> {
        let y = self
            .collision()
            .height_at(pos.x, pos.z)
            .ok_or(BuildError::InvalidPose)?;
        let pos = Vec3::new(pos.x, y, pos.z);
        let objectives = self.objective_list();
        let distance = |other| crate::spawn::outpost::distance_sq(pos, other);
        let nearest = objectives
            .iter()
            .min_by(|a, b| distance(a.pos).total_cmp(&distance(b.pos)));
        Ok(BuildSite {
            pos,
            yaw,
            sector: nearest.map_or(u16::MAX, |o| o.id),
            friendly_or_contested: nearest
                .is_none_or(|o| o.owner.is_none_or(|t| t == actor.team) || o.contested),
            inside_strongpoint_buffer: objectives
                .iter()
                .any(|o| distance(o.pos) < (o.radius_m + 15.0).powi(2)),
            enemy_nearby: !preview
                && self.players.iter().any(|p| {
                    p.team != actor.team
                        && distance(p.pos) < 225.0
                        && self.player_life(p.player).is_some_and(|l| l.is_alive())
                }),
            blocks_nav_critical: false, // map connectivity service belongs to M2-010
            slope_too_steep: self
                .collision()
                .slope_deg_at(pos.x, pos.z)
                .is_none_or(|s| s > 30.0),
            in_water: self
                .collision()
                .material_at(pos.x, pos.z)
                .is_some_and(|m| Some(m) == crate::spawn::garrison::water_material(self.cfg.data)),
            overlap: self.structures().any(|s| distance(s.pos) < 4.0),
            in_no_build_zone: self.no_build_zones().contains(pos),
        })
    }

    pub fn preview_facility(
        &self,
        id: PlayerId,
        kind: StructureKind,
        pos: Vec3,
        yaw: f32,
    ) -> Result<u8, BuildError> {
        let actor = self.facility_actor(id)?;
        let def = self.facility_def(actor.team, kind);
        let site = self.facility_site(actor, pos, yaw, true)?;
        self.ecs.resource::<BuildSystem>().validate_placement(
            actor,
            kind,
            site,
            &def,
            self.cfg.tick_hz,
            self.supply_world(),
        )?;
        Ok(def.cost_supplies.min(255) as u8)
    }

    pub fn place_facility(
        &mut self,
        id: PlayerId,
        kind: StructureKind,
        pos: Vec3,
        yaw: f32,
    ) -> Result<EntityId, BuildError> {
        let actor = self.facility_actor(id)?;
        let def = self.facility_def(actor.team, kind);
        let site = self.facility_site(actor, pos, yaw, false)?;
        let hz = self.cfg.tick_hz;
        self.ecs.resource_scope(|ecs, mut build: Mut<BuildSystem>| {
            build.start_build(
                actor,
                kind,
                site,
                &def,
                hz,
                &mut ecs.resource_mut::<SupplyWorld>(),
                &mut self.alloc,
            )
        })
    }

    pub fn join_facility_build(
        &mut self,
        id: PlayerId,
        structure: EntityId,
    ) -> Result<(), BuildError> {
        let actor = self.facility_actor(id)?;
        self.ecs
            .resource_mut::<BuildSystem>()
            .join_build(structure, actor)
    }

    pub fn cancel_facility_build(&mut self, id: PlayerId, structure: EntityId) -> bool {
        self.ecs
            .resource_mut::<BuildSystem>()
            .cancel_build(structure, id)
    }

    pub fn damage_facility(
        &mut self,
        id: EntityId,
        kind: StructureDamage,
        amount: f32,
    ) -> DamageOutcome {
        let result = self.ecs.resource_mut::<BuildSystem>().structure_damage(
            id,
            kind,
            amount,
            &mut self.alloc,
        );
        if result == DamageOutcome::Damaged {
            let hp = self.ecs.resource::<BuildSystem>().get(id).unwrap().hp;
            if let Some(g) = self.ecs.resource_mut::<AtGuns>().list.get_mut(&id) {
                g.hp = g.hp.min(hp);
            }
        }
        if result == DamageOutcome::Destroyed {
            self.release_at_gun(id);
            self.pending_events
                .push(SimEvent::StructureDestroyed { id });
        }
        result
    }

    pub fn at_gun(&self, id: EntityId) -> Option<&AtGunState> {
        self.ecs.resource::<AtGuns>().list.get(&id)
    }

    pub fn nearest_at_gun(&self, player: PlayerId) -> Option<EntityId> {
        let actor = self.facility_actor(player).ok()?;
        self.ecs
            .resource::<AtGuns>()
            .list
            .values()
            .filter(|g| {
                g.team == actor.team && g.hp > 0.0 && (g.gunner.is_none() || g.loader.is_none())
            })
            .filter_map(|g| {
                let d = actor.pos.distance_squared(g.pos);
                (d <= crate::build::BUILD_RANGE_M.powi(2)).then_some((g.id, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }

    pub fn enter_at_gun(&mut self, player: PlayerId, id: EntityId, seat: Seat) -> bool {
        let Ok(actor) = self.facility_actor(player) else {
            return false;
        };
        let Some(g) = self.at_gun(id) else {
            return false;
        };
        if actor.pos.distance_squared(g.pos) > crate::build::BUILD_RANGE_M.powi(2) {
            return false;
        }
        let Some(g) = self
            .ecs
            .resource_mut::<AtGuns>()
            .into_inner()
            .list
            .get_mut(&id)
        else {
            return false;
        };
        if !g.occupy(
            Actor {
                player,
                team: actor.team,
                role_id: actor.role,
            },
            seat,
        ) {
            return false;
        }
        let (_, e) = self.ecs.resource::<PlayerIndex>().map[&player];
        self.ecs.entity_mut(e).insert(InVehicle(Some((
            id,
            if seat == Seat::Gunner { 0 } else { 1 },
        ))));
        self.ecs
            .resource_mut::<BuildSystem>()
            .leave_all_builds(player);
        true
    }

    pub fn exit_at_gun(&mut self, player: PlayerId) -> bool {
        let Some((id, _)) = self.seat_of(player) else {
            return false;
        };
        let Some(g) = self
            .ecs
            .resource_mut::<AtGuns>()
            .into_inner()
            .list
            .get_mut(&id)
        else {
            return false;
        };
        g.leave(player);
        self.clear_at_seat(player, id);
        true
    }

    fn clear_at_seat(&mut self, player: PlayerId, id: EntityId) {
        if self
            .seat_of(player)
            .is_some_and(|(seat_id, _)| seat_id == id)
        {
            let (_, e) = self.ecs.resource::<PlayerIndex>().map[&player];
            self.ecs.entity_mut(e).insert(InVehicle(None));
        }
    }

    fn release_at_gun(&mut self, id: EntityId) {
        if let Some(g) = self.ecs.resource_mut::<AtGuns>().list.remove(&id) {
            for p in [g.gunner, g.loader].into_iter().flatten() {
                self.clear_at_seat(p, id);
            }
        }
    }

    pub fn reorient_at_gun(&mut self, player: PlayerId, id: EntityId, yaw: f32) -> bool {
        let Ok(actor) = self.facility_actor(player) else {
            return false;
        };
        let Some(g) = self
            .ecs
            .resource_mut::<AtGuns>()
            .into_inner()
            .list
            .get_mut(&id)
        else {
            return false;
        };
        actor.pos.distance_squared(g.pos) <= crate::build::BUILD_RANGE_M.powi(2)
            && g.start_reorient(
                Actor {
                    player,
                    team: actor.team,
                    role_id: actor.role,
                },
                yaw,
                self.cfg.tick_hz,
            )
    }

    pub fn select_at_ammo(&mut self, player: PlayerId, ammo: Ammo) -> bool {
        let Some((id, _)) = self.seat_of(player) else {
            return false;
        };
        self.ecs
            .resource_mut::<AtGuns>()
            .list
            .get_mut(&id)
            .is_some_and(|g| g.select_ammo(player, ammo, self.cfg.tick_hz))
    }

    pub fn resupply_at_gun(&mut self, player: PlayerId, id: EntityId) -> u8 {
        let Ok(actor) = self.facility_actor(player) else {
            return 0;
        };
        if !matches!(actor.role, "support" | "engineer") {
            return 0;
        }
        self.ecs.resource_scope(|ecs, mut guns: Mut<AtGuns>| {
            let Some(g) = guns.list.get_mut(&id) else {
                return 0;
            };
            if g.team != actor.team
                || actor.pos.distance_squared(g.pos) > crate::build::BUILD_RANGE_M.powi(2)
            {
                return 0;
            }
            let mut supplies = ecs.resource_mut::<SupplyWorld>();
            let pos = g.pos;
            g.resupply_ap(|rounds, price| {
                if let Some(c) = supplies.ammo.values_mut().find(|c| {
                    c.team == actor.team
                        && c.points >= u32::from(rounds)
                        && c.pos.distance_squared(pos) <= 9.0
                }) {
                    c.points -= u32::from(rounds);
                    return true;
                }
                crate::build::pay_facility_supplies(
                    &mut supplies,
                    actor.team,
                    pos,
                    u32::from(price),
                )
            })
        })
    }

    pub(super) fn deposit_station_crate(&mut self, player: PlayerId) -> Option<u32> {
        let actor = self.facility_actor(player).ok()?;
        let (_, e) = self.ecs.resource::<PlayerIndex>().map[&player];
        let mut carrying = *self.ecs.get::<Carrying>(e)?;
        let crate_id = carrying.0?;
        let service = self.cfg.data.repair_station().service;
        self.ecs
            .resource_scope(|ecs, mut build: Mut<BuildSystem>| {
                build.deposit_nearby(
                    actor,
                    service,
                    &mut ecs.resource_mut::<SupplyWorld>(),
                    &mut carrying,
                )
            })?;
        self.ecs.entity_mut(e).insert(carrying);
        self.pending_events
            .push(SimEvent::CrateDestroyed { id: crate_id });
        Some(crate_id)
    }

    pub(super) fn tick_facilities(&mut self, inputs: &[(PlayerId, InputCmd)]) {
        let hits = std::mem::take(
            &mut self
                .ecs
                .resource_mut::<crate::build::damage::FacilityHits>()
                .0,
        );
        for (id, kind, damage) in hits {
            self.damage_facility(id, kind, damage);
        }
        if self.structures().next().is_none() {
            return;
        }
        let hz = self.cfg.tick_hz;
        let mut build = self
            .ecs
            .remove_resource::<BuildSystem>()
            .expect("installed");
        let completed = build.tick_build(|p, team, kind| {
            let a = self.facility_actor(p).ok()?;
            let (_, entity) = self.ecs.resource::<PlayerIndex>().map[&p];
            let entity_hit = self.ecs.resource::<PlayerIndex>().map[&p].0;
            let damaged = self.ecs.resource::<EventBuf>().0.iter().any(|ev| matches!(ev, SimEvent::Hit { target, damage_hp, .. } | SimEvent::MeleeHit { target, damage_hp, .. } if *target == entity_hit && *damage_hp > 0.0));
            (a.team == team && !damaged && self.ecs.get::<Carrying>(entity).is_none_or(|c| c.0.is_none())
                && (a.role == "engineer" || (kind == StructureKind::AtGun && a.role == "anti_tank")))
                .then_some(a.pos)
        });
        for id in completed {
            let s = build.get(id).unwrap();
            if s.kind == StructureKind::AtGun {
                let mut g =
                    AtGunState::new(id, s.sector, s.pos, s.yaw, self.cfg.data.at_gun(s.team));
                g.hp = s.hp;
                self.ecs.resource_mut::<AtGuns>().list.insert(id, g);
            }
        }
        build.tick_repair_stations(
            self.cfg.data.repair_station().service,
            hz,
            &mut self.ecs.resource_mut::<Vehicles>(),
        );
        self.ecs.insert_resource(build);

        if self.ecs.resource::<AtGuns>().list.is_empty() {
            return;
        }
        let mut guns = self.ecs.remove_resource::<AtGuns>().expect("installed");
        for (&id, gun) in &mut guns.list {
            if let Some(engineer) = gun.reorienting_engineer() {
                let still_nearby = self.facility_actor(engineer).is_ok_and(|a| {
                    a.team == gun.team
                        && a.role == "engineer"
                        && a.pos.distance_squared(gun.pos) <= crate::build::BUILD_RANGE_M.powi(2)
                });
                if !still_nearby {
                    gun.leave(engineer);
                }
            }
            for p in [gun.gunner, gun.loader].into_iter().flatten() {
                if !self.player_life(p).is_some_and(|l| l.is_alive())
                    || !self.squads.entry(p).is_some_and(|e| e.team == gun.team)
                {
                    gun.leave(p);
                    self.clear_at_seat(p, id);
                }
            }
            gun.tick();
            let Some(p) = gun.gunner else { continue };
            let Ok(i) = inputs.binary_search_by_key(&p, |(player, _)| *player) else {
                continue;
            };
            let c = inputs[i].1;
            gun.aim(p, dequant_yaw(c.yaw, 16), pitch_i16_to_rad(c.pitch));
            if c.buttons & buttons::FIRE != 0 {
                let loader = gun
                    .loader
                    .and_then(|p| self.squads.entry(p))
                    .and_then(|e| e.role)
                    .or(gun.loader.map(|_| ""));
                if let Some(flash) = gun.fire(p, loader, hz, &mut self.ecs.resource_mut::<Shells>())
                {
                    self.ecs
                        .resource_mut::<EventBuf>()
                        .0
                        .push(SimEvent::MuzzleFlash(flash));
                }
            }
        }
        let dead: Vec<_> = guns
            .list
            .values()
            .filter(|g| g.hp <= 0.0)
            .map(|g| g.id)
            .collect();
        self.ecs.insert_resource(guns);
        for id in dead {
            self.damage_facility(id, StructureDamage::Explosion, f32::MAX);
        }
    }
}
