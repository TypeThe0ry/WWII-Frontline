//! Test arena: the real M0 terrain, a `World`, and soldier brains driven directly (no server).
//! Commands go straight into `World::step`; validation is covered by the server tests.
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use frontline_bots::{BotBrain, BotCtx, BotMap, Difficulty, SoldierBrain, Tuning};
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, PlayerId, SimConfig, SimEvent,
    SpawnReq, Team, Vec3, World,
};

pub const MAP_SIZE: f32 = 256.0;

fn data_dir() -> Option<PathBuf> {
    let mut d = std::env::current_dir().ok()?;
    loop {
        if d.join("data/maps/m0/meta.json").is_file() {
            return Some(d.join("data"));
        }
        if !d.pop() {
            return None;
        }
    }
}

/// The M0 terrain, or `None` after printing the regeneration command
/// (`FRONTLINE_REQUIRE_TERRAIN=1` makes that a failure).
pub fn terrain() -> Option<CollisionData> {
    let dir = data_dir().map(|d| d.join("maps/m0/chunks"));
    let mut chunks = Vec::new();
    if let Some(dir) = dir.as_ref().and_then(|d| std::fs::read_dir(d).ok()) {
        for e in dir.filter_map(|e| e.ok()) {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(rest) = name.strip_prefix("c_").and_then(|r| r.strip_suffix(".bin")) else {
                continue;
            };
            let Some((bx, bz)) = rest.split_once('_') else {
                continue;
            };
            let (Ok(bx), Ok(bz)) = (bx.parse::<i32>(), bz.parse::<i32>()) else {
                continue;
            };
            let bytes = std::fs::read(e.path()).expect("read chunk");
            chunks.push(CollisionChunk::from_bytes(bx, bz, &bytes).expect("chunk"));
        }
    }
    if chunks.is_empty() {
        if std::env::var_os("FRONTLINE_REQUIRE_TERRAIN").is_some() {
            panic!("terrain required but missing");
        }
        eprintln!(
            "SKIPPED (no terrain): PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0"
        );
        return None;
    }
    Some(CollisionData::from_chunks(chunks))
}

/// Patrol points of `data/maps/m0/spawns.json`.
pub fn patrol_points() -> Vec<[f32; 2]> {
    let d = data_dir().expect("data dir");
    let v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(d.join("maps/m0/spawns.json")).unwrap()).unwrap();
    v["patrol_points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            [
                p["pos"][0].as_f64().unwrap() as f32,
                p["pos"][1].as_f64().unwrap() as f32,
            ]
        })
        .collect()
}

pub fn new_world(col: &CollisionData, seed: u64) -> World {
    let mut w = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: seed,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        col,
    );
    // Team respawn points as the server installs them (HQ points of spawns.json).
    w.set_team_spawn(Team::Us, Vec3::new(128.0, 2.3, 56.0));
    w.set_team_spawn(Team::Ger, Vec3::new(128.0, 10.3, 200.0));
    w.set_map_center(MAP_SIZE / 2.0, MAP_SIZE / 2.0);
    w
}

pub fn spawn(w: &mut World, player: PlayerId, team: Team, x: f32, z: f32, yaw: f32) {
    let y = w.collision().height_at(x, z).unwrap_or(0.0) + 0.02;
    w.spawn_player(SpawnReq {
        player,
        team,
        pos: Vec3::new(x, y, z),
        yaw,
    })
    .expect("spawn");
}

pub struct Arena {
    pub world: World,
    pub brains: Vec<(PlayerId, Team, SoldierBrain)>,
    pub seed: u64,
    pub difficulty: Difficulty,
    pub kills: u32,
    pub shots: u32,
    pub hits: u32,
    /// Wall time of all `think` calls of each tick (ns) and the part spent in path queries.
    pub think_ns: Vec<u64>,
    pub path_ns: Vec<u64>,
}

impl Arena {
    pub fn new(col: &CollisionData, seed: u64) -> Self {
        let w = new_world(col, seed);
        Arena {
            world: w,
            brains: Vec::new(),
            seed,
            difficulty: Difficulty::embedded_default(),
            kills: 0,
            shots: 0,
            hits: 0,
            think_ns: Vec::new(),
            path_ns: Vec::new(),
        }
    }

    pub fn bot_map(&self) -> Arc<BotMap> {
        Arc::new(BotMap::build(
            self.world.view(),
            (MAP_SIZE, MAP_SIZE),
            patrol_points(),
            Tuning::embedded(),
        ))
    }

    pub fn add_bot(
        &mut self,
        map: &Arc<BotMap>,
        player: PlayerId,
        team: Team,
        x: f32,
        z: f32,
        yaw: f32,
    ) {
        spawn(&mut self.world, player, team, x, z, yaw);
        self.brains
            .push((player, team, SoldierBrain::new(map.clone())));
    }

    /// Runs one tick: every brain thinks, then `World::step`. Returns the commands used.
    pub fn step(&mut self) -> Vec<(PlayerId, InputCmd)> {
        let tick = self.world.tick().0 + 1;
        let mut cmds = Vec::new();
        let t0 = std::time::Instant::now();
        let mut path_ns = 0u64;
        for (p, team, brain) in &mut self.brains {
            let ctx = BotCtx {
                bot_id: *p,
                team: *team,
                tick,
                difficulty: self.difficulty,
                rng_seed: self.seed,
                self_weapon: self.world.weapon_state(*p),
                squad_order: None,
                squad_leader: None,
                hearing: &[],
                survival: Default::default(),
            };
            cmds.push((*p, brain.think(self.world.view(), &ctx)));
            path_ns += brain.take_stats().path_ns;
        }
        self.think_ns.push(t0.elapsed().as_nanos() as u64);
        self.path_ns.push(path_ns);
        cmds.sort_by_key(|c| c.0);
        let out = self.world.step(&cmds);
        for e in &out.events {
            match e {
                SimEvent::Shot { .. } => self.shots += 1,
                SimEvent::Hit { .. } => self.hits += 1,
                SimEvent::Kill { .. } => self.kills += 1,
                _ => {}
            }
        }
        cmds
    }

    /// `n` bots per team on a golden-angle spiral around each HQ (like the server's join spawn).
    pub fn add_squads(&mut self, map: &Arc<BotMap>, n_per_team: u8) {
        for i in 0..n_per_team {
            for (k, (team, hx, hz)) in [(Team::Us, 128.0f32, 56.0f32), (Team::Ger, 128.0, 200.0)]
                .into_iter()
                .enumerate()
            {
                let id = i * 2 + k as u8;
                let n = f32::from(i) + 1.0;
                let r = (1.6 * n.sqrt()).min(20.0);
                let a = n * 2.399_963_1;
                let yaw = if team == Team::Us {
                    0.0
                } else {
                    std::f32::consts::PI
                };
                self.add_bot(
                    map,
                    id,
                    team,
                    hx + r * libm::cosf(a),
                    hz + r * libm::sinf(a),
                    yaw,
                );
            }
        }
    }

    pub fn pos_of(&self, p: PlayerId) -> Option<Vec3> {
        self.world
            .view()
            .players()
            .iter()
            .find(|v| v.player == p)
            .map(|v| v.pos)
    }
}

/// Flat synthetic terrain (256 x 256 m, height 0) with an optional wall: `wall` is
/// `(x0, x1, z0, z1, height)`; inside the box the ground jumps to `height` over one metre.
pub fn synth(wall: Option<(f32, f32, f32, f32, f32)>) -> CollisionData {
    let mut chunks = Vec::new();
    for bz in 0..4i32 {
        for bx in 0..4i32 {
            let mut h = vec![0.0f32; 65 * 65];
            for iz in 0..65 {
                for ix in 0..65 {
                    let (x, z) = ((bx * 64 + ix as i32) as f32, (bz * 64 + iz as i32) as f32);
                    if let Some((x0, x1, z0, z1, hh)) = wall {
                        if x >= x0 && x <= x1 && z >= z0 && z <= z1 {
                            h[iz * 65 + ix] = hh;
                        }
                    }
                }
            }
            chunks.push(CollisionChunk::new(bx, bz, h, vec![0; 64 * 64]));
        }
    }
    CollisionData::from_chunks(chunks)
}

/// A world on synthetic terrain with a bot map of no patrol points and the given nav.
pub fn synth_world(col: &CollisionData, seed: u64) -> World {
    new_world(col, seed)
}

// ---------------------------------------------------------------- scripted world view

use frontline_sim::view::{
    EntityStateView, NavData, ObjectiveView, PlayerView, Resources, VehicleView,
};
use frontline_sim::{EntityId, WeaponState, WorldView};

/// A hand-scripted read-only world: flat ground, clear sight lines, players put in by the test.
pub struct FakeView {
    pub players: Vec<PlayerView>,
    pub nav: NavData,
    pub blocked: bool,
    pub objectives: Vec<ObjectiveView>,
}

impl FakeView {
    pub fn new() -> Self {
        FakeView {
            players: Vec::new(),
            nav: NavData::default(),
            blocked: false,
            objectives: Vec::new(),
        }
    }

    pub fn put(&mut self, player: PlayerId, team: Team, x: f32, z: f32, yaw: f32) {
        self.players.retain(|p| p.player != player);
        self.players.push(PlayerView {
            player,
            entity: player as EntityId + 1,
            team,
            pos: Vec3::new(x, 0.0, z),
            yaw,
            pitch: 0.0,
        });
        self.players.sort_by_key(|p| p.entity);
    }

    pub fn remove(&mut self, player: PlayerId) {
        self.players.retain(|p| p.player != player);
    }
}

impl WorldView for FakeView {
    fn players(&self) -> &[PlayerView] {
        &self.players
    }
    fn vehicles(&self) -> &[VehicleView] {
        &[]
    }
    fn entity_state(&self, _id: EntityId) -> Option<EntityStateView> {
        None
    }
    fn los(&self, _from: Vec3, _to: Vec3) -> bool {
        !self.blocked
    }
    fn height_at(&self, x: f32, z: f32) -> Option<f32> {
        ((-1.0..=300.0).contains(&x) && (-1.0..=300.0).contains(&z)).then_some(0.0)
    }
    fn team_resources(&self, _team: Team) -> Resources {
        Resources::default()
    }
    fn objectives(&self) -> &[ObjectiveView] {
        &self.objectives
    }
    fn nav_hint(&self) -> &NavData {
        &self.nav
    }
}

pub fn weapon_by_id(id: &str, mag: u16, reserve: u16) -> WeaponState {
    let idx = GameData::embedded()
        .weapons
        .iter()
        .position(|w| w.id == id)
        .expect("weapon");
    let mut w = WeaponState::new(idx as u16, mag, reserve);
    // The scripted world never advances the weapon: report the sights as fully up.
    w.ads_units = 1_000_000;
    w
}

pub fn ctx(bot: PlayerId, team: Team, tick: u32, w: Option<WeaponState>) -> BotCtx<'static> {
    BotCtx {
        bot_id: bot,
        team,
        tick,
        difficulty: Difficulty::embedded_default(),
        rng_seed: 99,
        self_weapon: w,
        squad_order: None,
        squad_leader: None,
        hearing: &[],
        survival: Default::default(),
    }
}

pub fn fake_brain(view: &FakeView, patrol: Vec<[f32; 2]>) -> SoldierBrain {
    SoldierBrain::new(Arc::new(BotMap::build(
        view,
        (MAP_SIZE, MAP_SIZE),
        patrol,
        Tuning::embedded(),
    )))
}
