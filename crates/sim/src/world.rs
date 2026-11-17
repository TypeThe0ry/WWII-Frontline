//! `World`: owns the `bevy_ecs` world, the schedule and the match RNG state.

use std::borrow::Cow;

use bevy_ecs::prelude::{Entity, Schedule, World as EcsWorld};
use frontline_protocol::msg::input::InputCmd;
use glam::Vec3;

use crate::collision::CollisionData;
use crate::components::*;
use crate::data::GameData;
use crate::hash::HashWriter;
use crate::ids::{EntityAllocator, EntityId, PlayerId, Tick, MAX_PLAYER_ID};
use crate::schedule::build_schedule;
use crate::step::{SimEvent, StepOutput};
use crate::view::*;

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
    collision: CollisionData,
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
        World {
            cfg,
            ecs,
            schedule: build_schedule(),
            alloc: EntityAllocator::new(),
            players: Vec::new(),
            pending_events: Vec::new(),
            collision: collision.clone(),
        }
    }

    pub fn collision(&self) -> &CollisionData {
        &self.collision
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
            ))
            .id();
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
        self.alloc.free(eid);
        self.pending_events.push(SimEvent::PlayerRemoved {
            player: id,
            entity: eid,
        });
        self.refresh_views();
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
            let (Some(p), Some(team), Some(t), Some(li)) = (
                er.get::<Player>(),
                er.get::<Team>(),
                er.get::<Transform>(),
                er.get::<LastInput>(),
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
        }
        h.finish()
    }

    fn refresh_views(&mut self) {
        self.players.clear();
        let table = self.ecs.resource::<EntityTable>();
        for (id, e) in table.sorted() {
            let er = self.ecs.entity(e);
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

fn neutral_input() -> InputCmd {
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
        Some(EntityStateView {
            entity: id,
            pos: t.pos,
            yaw: t.yaw,
            pitch: t.pitch,
        })
    }
    fn los(&self, from: Vec3, to: Vec3) -> bool {
        self.collision.los(from, to)
    }
    fn height_at(&self, x: f32, z: f32) -> Option<f32> {
        self.collision.height_at(x, z)
    }
    fn team_resources(&self, _team: Team) -> Resources {
        Resources::default() // TODO(M0-013)
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
