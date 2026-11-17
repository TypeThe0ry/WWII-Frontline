//! `World`: owns the `bevy_ecs` world, the schedule and the match RNG state.

use std::borrow::Cow;

use bevy_ecs::prelude::{Entity, Schedule, World as EcsWorld};
use frontline_protocol::msg::input::InputCmd;
use glam::Vec3;

use crate::collision::CollisionData;
use crate::components::*;
use crate::data::GameData;
use crate::hash::HashWriter;
use crate::health::{Health, HitLog, LifeState, LifeStateView};
use crate::ids::{EntityAllocator, EntityId, PlayerId, Tick, MAX_PLAYER_ID};
use crate::movement::{MoveParams, PlayerAuthState};
use crate::projectile::{Projectile, Projectiles};
use crate::respawn::{RespawnPolicy, RespawnQueue, SpawnPoints};
use crate::schedule::build_schedule;
use crate::step::{SimEvent, StepOutput};
use crate::view::*;
use crate::weapon::WeaponState;

/// Role whose movement parameters every player uses until loadouts/roles are chosen (M0-012).
pub const DEFAULT_ROLE: &str = "rifleman";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Frontline,
    Assault,
    Skirmish,
    /// M0 sandbox (`--mode sandbox`).
    Sandbox,
}

#[derive(Debug, Clone, Copy)]
pub struct SimConfig {
    pub tick_hz: u8,
    pub match_seed: u64,
    pub map: MapId,
    pub mode: GameMode,
    pub data: &'static GameData,
}

#[derive(Debug, Clone, Copy)]
pub struct SpawnReq {
    pub player: PlayerId,
    pub team: Team,
    pub pos: Vec3,
    pub yaw: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnError {
    InvalidPlayerId,
    PlayerExists,
    NoFreeSlot,
}

pub struct World {
    cfg: SimConfig,
    ecs: EcsWorld,
    schedule: Schedule,
    alloc: EntityAllocator,
    players: Vec<PlayerView>,
    pending_events: Vec<SimEvent>,
}

/// Debug: panics unless ids are strictly ascending. Release: returns a sorted (stable, last
/// duplicate wins) copy when they are not.
pub(crate) fn normalize_inputs(inputs: &[(PlayerId, InputCmd)]) -> Cow<'_, [(PlayerId, InputCmd)]> {
    let ascending = inputs.windows(2).all(|w| w[0].0 < w[1].0);
    if ascending {
        return Cow::Borrowed(inputs);
    }
    if cfg!(debug_assertions) {
        panic!(
            "World::step inputs must be strictly ascending by PlayerId (got {:?})",
            inputs.iter().map(|(p, _)| *p).collect::<Vec<_>>()
        );
    }
    let mut v = inputs.to_vec();
    v.sort_by_key(|(p, _)| *p); // stable
    let mut out: Vec<(PlayerId, InputCmd)> = Vec::with_capacity(v.len());
    for item in v {
        match out.last_mut() {
            Some(last) if last.0 == item.0 => *last = item,
            _ => out.push(item),
        }
    }
    Cow::Owned(out)
}

impl World {
    pub fn new(cfg: SimConfig, collision: &CollisionData) -> Self {
        let mut ecs = EcsWorld::new();
        ecs.insert_resource(CurrentTick(Tick(0)));
        ecs.insert_resource(InputQueue::default());
        ecs.insert_resource(EntityTable::default());
        ecs.insert_resource(PlayerIndex::default());
        ecs.insert_resource(SimRng {
            match_seed: cfg.match_seed,
            draws: 0,
        });
        ecs.insert_resource(TestJitter::default());
        ecs.insert_resource(MoveParamsRes(
            cfg.data
                .role(DEFAULT_ROLE)
                .and_then(|r| MoveParams::from_role(r, cfg.tick_hz).ok()),
        ));
        ecs.insert_resource(CollisionRes(collision.clone()));
        ecs.insert_resource(SimCfgRes {
            data: cfg.data,
            hz: cfg.tick_hz,
        });
        ecs.insert_resource(WeaponTable::new(cfg.data, cfg.tick_hz));
        ecs.insert_resource(crate::components::MeleeTable::new(cfg.data, cfg.tick_hz));
        ecs.insert_resource(crate::throwables::ThrowableTable::new(
            cfg.data,
            cfg.tick_hz,
        ));
        ecs.insert_resource(crate::throwables::Throwables::default());
        ecs.insert_resource(crate::throwables::Smokes::default());
        ecs.insert_resource(Projectiles::default());
        ecs.insert_resource(EventBuf::default());
        ecs.insert_resource(HitLog::default());
        ecs.insert_resource(RespawnQueue::default());
        ecs.insert_resource(RespawnPolicyRes(RespawnPolicy::from_def(
            &cfg.data.rules.respawn,
        )));
        ecs.insert_resource(SpawnPoints {
            pos: [None, None],
            center: collision_center(collision),
        });
        World {
            cfg,
            ecs,
            schedule: build_schedule(),
            alloc: EntityAllocator::new(),
            players: Vec::new(),
            pending_events: Vec::new(),
        }
    }

    pub fn collision(&self) -> &CollisionData {
        &self.ecs.resource::<CollisionRes>().0
    }

    pub fn config(&self) -> &SimConfig {
        &self.cfg
    }

    /// Advances one tick. Inputs must be ascending by `PlayerId` (see [`normalize_inputs`]).
    pub fn step(&mut self, inputs: &[(PlayerId, InputCmd)]) -> StepOutput {
        let inputs = normalize_inputs(inputs);
        {
            let mut t = self.ecs.resource_mut::<CurrentTick>();
            t.0 = t.0.next();
        }
        {
            let mut q = self.ecs.resource_mut::<InputQueue>();
            q.0.clear();
            q.0.extend_from_slice(&inputs);
        }
        self.schedule.run(&mut self.ecs);
        self.refresh_views();
        let produced = std::mem::take(&mut self.ecs.resource_mut::<EventBuf>().0);
        self.pending_events.extend(produced);
        StepOutput {
            events: std::mem::take(&mut self.pending_events),
            rule_changes: Vec::new(),
        }
    }

    pub fn spawn_player(&mut self, req: SpawnReq) -> Result<EntityId, SpawnError> {
        if req.player > MAX_PLAYER_ID {
            return Err(SpawnError::InvalidPlayerId);
        }
        if self
            .ecs
            .resource::<PlayerIndex>()
            .map
            .contains_key(&req.player)
        {
            return Err(SpawnError::PlayerExists);
        }
        let id = self.alloc.alloc().ok_or(SpawnError::NoFreeSlot)?;
        let max_hp = self
            .cfg
            .data
            .role(DEFAULT_ROLE)
            .map_or(100.0, |r| r.health_hp as f32);
        let weapon = starting_weapon(self.cfg.data, DEFAULT_ROLE, req.team, self.cfg.tick_hz);
        let e = self
            .ecs
            .spawn((
                Player { id: req.player },
                SimId(id),
                req.team,
                Transform {
                    pos: req.pos,
                    yaw: req.yaw,
                    pitch: 0.0,
                },
                LastInput {
                    cmd: neutral_input(),
                },
                Motion::from_auth(&PlayerAuthState::spawn(req.pos, req.yaw)),
                Health::full(max_hp),
                LifeState::Alive,
                Home {
                    pos: req.pos,
                    yaw: req.yaw,
                },
            ))
            .id();
        self.ecs.entity_mut(e).insert((
            crate::weapon::MeleeState::default(),
            crate::throwables::Gadgets::default(),
            crate::combat::credit::RecentDamage::default(),
            crate::medical::ReviveHistory::default(),
            crate::medical::ReviveAction::default(),
            crate::medical::FireLock::default(),
            crate::medical::Vitals::default(),
            crate::medical::FallTracker::default(),
        ));
        if let Some(w) = weapon {
            self.ecs.entity_mut(e).insert(w);
        }
        self.ecs.resource_mut::<EntityTable>().insert(id, e);
        self.ecs
            .resource_mut::<PlayerIndex>()
            .map
            .insert(req.player, (id, e));
        self.pending_events.push(SimEvent::PlayerSpawned {
            player: req.player,
            entity: id,
        });
        self.refresh_views();
        Ok(id)
    }

    pub fn remove_player(&mut self, id: PlayerId) {
        let Some((eid, e)) = self.ecs.resource_mut::<PlayerIndex>().map.remove(&id) else {
            return;
        };
        self.ecs.resource_mut::<EntityTable>().remove(eid);
        self.ecs.despawn(e);
        self.ecs.resource_mut::<RespawnQueue>().remove_player(id);
        self.alloc.free(eid);
        self.pending_events.push(SimEvent::PlayerRemoved {
            player: id,
            entity: eid,
        });
        self.refresh_views();
    }

    /// Movement state of a player (what `self_state` / `Predictor.reset` carry).
    pub fn player_state(&self, id: PlayerId) -> Option<PlayerAuthState> {
        let (_, e) = *self.ecs.resource::<PlayerIndex>().map.get(&id)?;
        let er = self.ecs.entity(e);
        Some(er.get::<Motion>()?.to_auth(er.get::<Transform>()?))
    }

    /// Weapon state of a player (what `self_state` carries: ammo, fsm, recoil).
    pub fn weapon_state(&self, id: PlayerId) -> Option<WeaponState> {
        let (_, e) = *self.ecs.resource::<PlayerIndex>().map.get(&id)?;
        self.ecs.entity(e).get::<WeaponState>().copied()
    }

    /// Current hit points of a player.
    pub fn health(&self, id: PlayerId) -> Option<f32> {
        let (_, e) = *self.ecs.resource::<PlayerIndex>().map.get(&id)?;
        self.ecs.entity(e).get::<Health>().map(|h| h.hp)
    }

    /// Marks (or unmarks) a player as a medic, who can revive downed teammates (M1-016).
    pub fn set_medic(&mut self, id: PlayerId, medic: bool) {
        let Some(&(_, e)) = self.ecs.resource::<PlayerIndex>().map.get(&id) else {
            return;
        };
        let mut er = self.ecs.entity_mut(e);
        if medic {
            er.insert(crate::medical::Medic);
        } else {
            er.remove::<crate::medical::Medic>();
        }
    }

    /// Alive / dead state of a player (M0-013); `None` for an unknown player.
    pub fn player_life(&self, id: PlayerId) -> Option<LifeStateView> {
        let (_, e) = *self.ecs.resource::<PlayerIndex>().map.get(&id)?;
        self.ecs.entity(e).get::<LifeState>().copied()
    }

    /// Sets the fixed respawn point of a team (M0: the HQ point of `spawns.json`). Without one,
    /// players respawn where they joined.
    pub fn set_team_spawn(&mut self, team: Team, pos: Vec3) {
        self.ecs.resource_mut::<SpawnPoints>().pos[team as usize] = Some(pos);
    }

    /// Overrides the map centre respawned players face (default: centre of the loaded chunks).
    pub fn set_map_center(&mut self, x: f32, z: f32) {
        self.ecs.resource_mut::<SpawnPoints>().center = Some((x, z));
    }

    /// Players waiting to respawn.
    pub fn respawn_pending(&self) -> usize {
        self.ecs.resource::<RespawnQueue>().len()
    }

    /// Injects a projectile owned by `owner` (a player), flown by the rules of weapon `weapon`
    /// (index into `GameData::weapons`; drag, damage and falloff come from it). For benches,
    /// tests and later scripted fire (bots, artillery); normal shots come from the weapon stage.
    pub fn spawn_projectile(
        &mut self,
        owner: PlayerId,
        weapon: u16,
        origin: Vec3,
        vel: Vec3,
    ) -> Option<u32> {
        let (eid, e) = *self.ecs.resource::<PlayerIndex>().map.get(&owner)?;
        let team = *self.ecs.entity(e).get::<Team>()?;
        if usize::from(weapon) >= self.cfg.data.weapons.len() {
            return None;
        }
        Some(
            self.ecs
                .resource_mut::<Projectiles>()
                .spawn(eid, team, weapon, origin, vel),
        )
    }

    /// Projectiles in flight, in spawn order (for snapshots / debug draw).
    pub fn projectiles(&self) -> &[Projectile] {
        self.ecs.resource::<Projectiles>().as_slice()
    }

    pub fn view(&self) -> &dyn WorldView {
        self
    }

    pub fn tick(&self) -> Tick {
        self.ecs.resource::<CurrentTick>().0
    }

    pub fn data_hash(&self) -> u64 {
        self.cfg.data.hash
    }

    /// Test-only (feature `test-util`): enables the per-tick RNG jitter system.
    #[cfg(feature = "test-util")]
    pub fn set_test_jitter(&mut self, on: bool) {
        self.ecs.resource_mut::<TestJitter>().0 = on;
    }

    /// Test-only (feature `test-util`): sets a player's hit points.
    #[cfg(feature = "test-util")]
    pub fn debug_set_hp(&mut self, id: PlayerId, hp: f32) {
        if let Some(&(_, e)) = self.ecs.resource::<PlayerIndex>().map.get(&id) {
            if let Some(mut h) = self.ecs.entity_mut(e).get_mut::<Health>() {
                h.hp = hp;
            }
        }
    }

    /// Test-only (feature `test-util`): puts a player into the downed state (40 hp, bleeding).
    #[cfg(feature = "test-util")]
    pub fn debug_down(&mut self, id: PlayerId) {
        let t = self.tick();
        if let Some(&(_, e)) = self.ecs.resource::<PlayerIndex>().map.get(&id) {
            let mut er = self.ecs.entity_mut(e);
            if let Some(mut h) = er.get_mut::<Health>() {
                h.hp = crate::medical::DOWNED_HP;
            }
            if let Some(mut l) = er.get_mut::<LifeState>() {
                *l = LifeState::Downed {
                    t_down: t,
                    killer: None,
                    weapon: 0,
                    fatigue: 0,
                    give_up_ticks: 0,
                    stable: false,
                };
            }
        }
    }

    /// Test-only (feature `test-util`): moves a player to `pos` (e.g. onto a ledge to fall).
    #[cfg(feature = "test-util")]
    pub fn debug_teleport(&mut self, id: PlayerId, pos: Vec3) {
        if let Some(&(_, e)) = self.ecs.resource::<PlayerIndex>().map.get(&id) {
            if let Some(mut t) = self.ecs.entity_mut(e).get_mut::<Transform>() {
                t.pos = pos;
            }
        }
    }

    /// Stable hash key of the weapon at `idx` in `GameData::weapons`: FNV-1a of its string id
    /// (the index itself for an out-of-range value), independent of the data ordering.
    fn weapon_key(&self, idx: u16) -> u64 {
        match self.cfg.data.weapons.get(usize::from(idx)) {
            Some(w) => {
                let mut h = HashWriter::new();
                h.write_bytes(w.id.as_bytes());
                h.finish()
            }
            None => u64::from(idx),
        }
    }

    /// Hash of all simulation state: tick, seed, RNG draw count, then every entity in
    /// ascending `EntityId` order (id, player, team, transform, last input).
    pub fn state_hash(&self) -> u64 {
        let mut h = HashWriter::new();
        h.write_u32(self.tick().0);
        let rng = self.ecs.resource::<SimRng>();
        h.write_u64(rng.match_seed);
        h.write_u64(rng.draws);
        let table = self.ecs.resource::<EntityTable>();
        for (id, e) in table.sorted() {
            let er = self.ecs.entity(e);
            h.write_u16(id);
            let (Some(p), Some(team), Some(t), Some(li), Some(mo)) = (
                er.get::<Player>(),
                er.get::<Team>(),
                er.get::<Transform>(),
                er.get::<LastInput>(),
                er.get::<Motion>(),
            ) else {
                continue;
            };
            h.write_u8(p.id);
            h.write_u8(*team as u8);
            h.write_f32(t.pos.x);
            h.write_f32(t.pos.y);
            h.write_f32(t.pos.z);
            h.write_f32(t.yaw);
            h.write_f32(t.pitch);
            let c = &li.cmd;
            h.write_u16(c.seq);
            h.write_u32(c.tick);
            h.write_i8(c.move_x);
            h.write_i8(c.move_z);
            h.write_u16(c.yaw);
            h.write_i16(c.pitch);
            h.write_u16(c.buttons);
            h.write_u8(c.weapon_slot);
            h.write_u8(c.aux);
            h.write_u16(c.view_tick);
            h.write_f32(mo.vel.x);
            h.write_f32(mo.vel.y);
            h.write_f32(mo.vel.z);
            h.write_u8(mo.stance as u8);
            h.write_f32(mo.stance_blend);
            h.write_u8(mo.stance_timer);
            h.write_u8(mo.stance_from as u8);
            h.write_f32(mo.stamina);
            h.write_u8(mo.stamina_delay);
            h.write_u8(u8::from(mo.sprint_locked));
            h.write_u8(u8::from(mo.grounded) | (u8::from(mo.blocked) << 1));
            // M1-011b: written only while leaning / vaulting, so other scenarios keep their hashes.
            if mo.lean != 0.0 || mo.vault.is_some() {
                h.write_f32(mo.lean);
                if let Some(v) = &mo.vault {
                    h.write_u8(v.ticks_left);
                    h.write_u8(v.total);
                    h.write_u8(u8::from(v.climb));
                    for f in [
                        v.start.x, v.start.y, v.start.z, v.target.x, v.target.y, v.target.z, v.top,
                    ] {
                        h.write_f32(f);
                    }
                }
            }
            // M0-012: health and weapon state.
            if let Some(hp) = er.get::<Health>() {
                h.write_f32(hp.hp);
            }
            // M0-013: written only while dead, so scenarios without deaths keep their hashes.
            if let Some(LifeState::Downed {
                t_down,
                killer,
                weapon,
                fatigue,
                give_up_ticks,
                stable,
            }) = er.get::<LifeState>()
            {
                h.write_u8(0xD4);
                h.write_u32(t_down.0);
                h.write_u16(killer.map_or(0xFFFF, u16::from));
                h.write_u64(self.weapon_key(u16::from(*weapon)));
                h.write_u8(*fatigue);
                h.write_u16(*give_up_ticks);
                h.write_u8(u8::from(*stable));
            }
            if let Some(ra) = er.get::<crate::medical::ReviveAction>() {
                if let Some(a) = ra.0 {
                    h.write_u8(0xD6);
                    h.write_u16(a.target);
                    h.write_u16(a.ticks);
                }
            }
            if let Some(LifeState::Dead {
                t_kia,
                killer,
                weapon,
                headshot,
                respawn_tick,
            }) = er.get::<LifeState>()
            {
                h.write_u8(0xD0 | u8::from(*headshot));
                h.write_u32(t_kia.0);
                h.write_u16(killer.map_or(0xFFFF, u16::from));
                h.write_u64(self.weapon_key(u16::from(*weapon)));
                h.write_u32(respawn_tick.0);
            }
            // M1-015: grenade counts and cooking, written only once a grenade was used.
            if let Some(g) = er.get::<crate::throwables::Gadgets>() {
                use crate::throwables::system::{START_FRAG, START_SMOKE};
                if g.frag != START_FRAG || g.smoke != START_SMOKE || g.cook.is_some() {
                    h.write_u8(0xF0);
                    h.write_u8(g.frag);
                    h.write_u8(g.smoke);
                    if let Some(c) = g.cook {
                        h.write_u8(c.kind as u8);
                        h.write_u16(c.ticks);
                    }
                }
            }
            // M1-013b: written only while a melee action runs.
            if let Some(ms) = er.get::<crate::weapon::MeleeState>() {
                if ms.executing() {
                    h.write_u8(0xE8);
                    h.write_u16(ms.exec_target);
                    h.write_u16(ms.exec_ticks);
                }
                if !ms.is_idle() {
                    h.write_u8(0xE0 | ms.phase as u8 | (u8::from(ms.lunging) << 3));
                    h.write_u16(ms.ticks_left);
                }
            }
            if let Some(w) = er.get::<WeaponState>() {
                // The weapon is hashed by id, not by its index in `GameData::weapons`, so adding or
                // reordering weapon data does not shift every golden hash.
                let b = w.to_bytes();
                h.write_u64(self.weapon_key(w.def_id));
                h.write_bytes(&b[2..]);
            }
        }
        let th = self.ecs.resource::<crate::throwables::Throwables>();
        if th.next_id > 0 {
            h.write_u32(th.next_id);
            for g in &th.list {
                h.write_u32(g.id);
                h.write_u16(g.owner);
                h.write_u8(g.kind as u8);
                h.write_u16(g.fuse_left);
                for f in [
                    g.flight.pos.x,
                    g.flight.pos.y,
                    g.flight.pos.z,
                    g.flight.vel.x,
                    g.flight.vel.y,
                    g.flight.vel.z,
                ] {
                    h.write_f32(f);
                }
            }
        }
        let sm = self.ecs.resource::<crate::throwables::Smokes>();
        for c in &sm.clouds {
            h.write_u32(c.start.0);
            h.write_f32(c.pos.x);
            h.write_f32(c.pos.y);
            h.write_f32(c.pos.z);
        }
        let projs = self.ecs.resource::<Projectiles>();
        h.write_u32(projs.next_id());
        h.write_u32(projs.len() as u32);
        for p in projs.as_slice() {
            h.write_u32(p.id);
            h.write_u16(p.owner);
            h.write_u64(self.weapon_key(p.weapon));
            for v in [p.origin, p.pos, p.vel] {
                h.write_f32(v.x);
                h.write_f32(v.y);
                h.write_f32(v.z);
            }
            h.write_u32(p.substeps);
        }
        h.finish()
    }

    fn refresh_views(&mut self) {
        self.players.clear();
        let table = self.ecs.resource::<EntityTable>();
        for (id, e) in table.sorted() {
            let er = self.ecs.entity(e);
            if er.get::<LifeState>().is_some_and(|l| !l.is_alive()) {
                continue; // `players()` lists the living only
            }
            if let (Some(p), Some(team), Some(t)) =
                (er.get::<Player>(), er.get::<Team>(), er.get::<Transform>())
            {
                self.players.push(PlayerView {
                    player: p.id,
                    entity: id,
                    team: *team,
                    pos: t.pos,
                    yaw: t.yaw,
                    pitch: t.pitch,
                });
            }
        }
    }
}

/// Centre `(x, z)` of the loaded collision chunks (64 m each), or `None` when empty.
fn collision_center(col: &CollisionData) -> Option<(f32, f32)> {
    let mut it = col.chunks();
    let first = it.next()?;
    let (mut x0, mut x1, mut z0, mut z1) = (first.bx, first.bx, first.bz, first.bz);
    for c in it {
        x0 = x0.min(c.bx);
        x1 = x1.max(c.bx);
        z0 = z0.min(c.bz);
        z1 = z1.max(c.bz);
    }
    Some(((x0 + x1 + 1) as f32 * 32.0, (z0 + z1 + 1) as f32 * 32.0))
}

/// Command of a player that does nothing (also the idle state after a respawn).
pub(crate) fn neutral_input() -> InputCmd {
    InputCmd {
        seq: 0,
        tick: 0,
        move_x: 0,
        move_z: 0,
        yaw: 0,
        pitch: 0,
        buttons: 0,
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    }
}

impl WorldView for World {
    fn players(&self) -> &[PlayerView] {
        &self.players
    }
    fn vehicles(&self) -> &[VehicleView] {
        &[]
    }
    fn entity_state(&self, id: EntityId) -> Option<EntityStateView> {
        if !self.alloc.is_alive(id) {
            return None;
        }
        let e: Entity = self.ecs.resource::<EntityTable>().get(id)?;
        let t = self.ecs.get::<Transform>(e)?;
        let (hp, alive) = self
            .ecs
            .get::<Health>(e)
            .map_or((0.0, true), |h| (h.hp, h.hp > 0.0));
        let max_hp = self.ecs.get::<Health>(e).map_or(100.0, |h| h.max_hp);
        Some(EntityStateView {
            entity: id,
            pos: t.pos,
            yaw: t.yaw,
            pitch: t.pitch,
            health_q: crate::health::quant_health(hp, max_hp, 8),
            alive,
        })
    }
    fn health(&self, id: EntityId) -> Option<f32> {
        if !self.alloc.is_alive(id) {
            return None;
        }
        let e = self.ecs.resource::<EntityTable>().get(id)?;
        self.ecs.get::<Health>(e).map(|h| h.hp)
    }
    fn alive(&self, id: EntityId) -> bool {
        self.alloc.is_alive(id)
            && self
                .ecs
                .resource::<EntityTable>()
                .get(id)
                .and_then(|e| self.ecs.get::<LifeState>(e))
                .is_some_and(LifeState::is_alive)
    }
    fn los(&self, from: Vec3, to: Vec3) -> bool {
        let now = self.ecs.resource::<CurrentTick>().0;
        self.collision().los(from, to)
            && !crate::throwables::smoke_blocks_los(
                &self.ecs.resource::<crate::throwables::Smokes>().clouds,
                now,
                from,
                to,
            )
    }
    fn height_at(&self, x: f32, z: f32) -> Option<f32> {
        self.collision().height_at(x, z)
    }
    fn slope_deg_at(&self, x: f32, z: f32) -> Option<f32> {
        self.collision().slope_deg_at(x, z)
    }
    fn team_resources(&self, _team: Team) -> Resources {
        Resources::default() // resources arrive with M1 scoring
    }
    fn objectives(&self) -> &[ObjectiveView] {
        &[]
    }
    fn nav_hint(&self) -> &NavData {
        static NAV: NavData = NavData {};
        &NAV
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        let cfg = SimConfig {
            tick_hz: 30,
            match_seed: 5,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        };
        let mut w = World::new(cfg, &CollisionData::empty());
        w.spawn_player(SpawnReq {
            player: 3,
            team: Team::Ger,
            pos: Vec3::new(1.0, 2.0, 3.0),
            yaw: 0.0,
        })
        .unwrap();
        w
    }

    #[test]
    fn step_advances_tick_and_applies_view_angles() {
        let mut w = world();
        let mut cmd = neutral_input();
        cmd.yaw = 16384; // quarter turn
        cmd.pitch = i16::MAX;
        let out = w.step(&[(3, cmd)]);
        assert_eq!(w.tick(), Tick(1));
        assert_eq!(out.events.len(), 1, "spawn event delivered on next step");
        let p = w.view().players()[0];
        assert!((p.yaw - core::f32::consts::FRAC_PI_2).abs() < 1e-4);
        assert!(p.pitch > 1.5);
        let before = w.state_hash();
        w.step(&[(3, cmd)]);
        assert_ne!(before, w.state_hash(), "tick is part of the hash");
    }
}
