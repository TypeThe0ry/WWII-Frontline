//! Shared fixed-seed match for M2-029. Bot inputs alone drive the authoritative sim.

use std::path::Path;
use std::sync::Arc;

use frontline_bots::ctx::SoldierStatus;
use frontline_bots::perception::Sound;
use frontline_bots::scheduler::{BotRuntime, BotsMetrics};
use frontline_bots::{BotBrain, BotCtx, BotMap, Difficulty, SoldierBrain, Tuning};
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::rules::ObjectiveSpec;
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, LifeState, MapId, PlayerId, SimConfig,
    SimEvent, SpawnReq, Team, Vec3, World,
};

pub const SEED: u64 = 0x0295_ca1e;
pub const HZ: u8 = 30;
const MAP_SIZE: f32 = 256.0;

pub struct Terrain {
    pub name: &'static str,
    collision: CollisionData,
}

impl Terrain {
    pub fn load() -> Self {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/m0/chunks");
        let mut chunks = Vec::new();
        for z in 0..4 {
            for x in 0..4 {
                let file = dir.join(format!("c_{x}_{z}.bin"));
                match std::fs::read(&file) {
                    Ok(bytes) => chunks.push(CollisionChunk::from_bytes(x, z, &bytes).unwrap()),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => panic!("{}: {e}", file.display()),
                }
            }
        }
        if chunks.len() == 16 {
            return Self {
                name: "m0-terrain",
                collision: CollisionData::from_chunks(chunks),
            };
        }
        assert!(
            chunks.is_empty(),
            "incomplete M0 collision: {} of 16 chunks",
            chunks.len()
        );
        assert!(
            std::env::var_os("FRONTLINE_REQUIRE_TERRAIN").is_none(),
            "generate M0 collision with PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0"
        );
        Self::graybox()
    }

    pub fn graybox() -> Self {
        let chunks = (0..4).flat_map(|z| {
            (0..4).map(move |x| {
                let mut heights = vec![0.0; 65 * 65];
                for iz in 0..65 {
                    for ix in 0..65 {
                        let (wx, wz) = (64 * x + ix, 64 * z + iz);
                        // Two raised barriers leave an open crossing through the centre.
                        if (126..=130).contains(&wz)
                            && ((60..=101).contains(&wx) || (155..=196).contains(&wx))
                        {
                            heights[(iz * 65 + ix) as usize] = 3.0;
                        }
                    }
                }
                CollisionChunk::new(x, z, heights, vec![0; 64 * 64])
            })
        });
        Self {
            name: "graybox-two-barriers",
            collision: CollisionData::from_chunks(chunks),
        }
    }

    pub fn world(&self) -> World {
        World::new(
            SimConfig {
                tick_hz: HZ,
                match_seed: SEED,
                map: MapId(0),
                mode: GameMode::Sandbox,
                data: GameData::embedded(),
            },
            &self.collision,
        )
    }
}

pub fn ground(world: &World, x: f32, z: f32) -> Vec3 {
    Vec3::new(
        x,
        world.collision().height_at(x, z).expect("loaded ground") + 0.02,
        z,
    )
}

pub fn spawn(world: &mut World, player: PlayerId, team: Team, x: f32, z: f32) {
    world
        .spawn_player(SpawnReq {
            player,
            team,
            pos: ground(world, x, z),
            yaw: if team == Team::Us {
                0.0
            } else {
                std::f32::consts::PI
            },
        })
        .expect("spawn in loaded collision");
}

pub fn context<'a>(
    world: &World,
    bot_id: PlayerId,
    team: Team,
    tick: u32,
    hearing: &'a [Sound],
) -> BotCtx<'a> {
    let life = world.player_life(bot_id);
    BotCtx {
        bot_id,
        team,
        tick,
        // Keep the tuning identical so the comparison isolates the runtime change.
        difficulty: Difficulty::embedded_default(),
        rng_seed: SEED,
        self_weapon: (life == Some(LifeState::Alive))
            .then(|| world.weapon_state(bot_id))
            .flatten(),
        squad_order: None,
        squad_leader: None,
        hearing,
        survival: SoldierStatus {
            downed_since: match life {
                Some(LifeState::Downed { t_down, .. }) => Some(t_down.0),
                _ => None,
            },
            ..SoldierStatus::default()
        },
    }
}

pub struct Battle {
    pub world: World,
    pub bots: Vec<(PlayerId, Team, SoldierBrain)>,
    pub shots: usize,
    pub kills: usize,
    pub spawns: usize,
    commands: Vec<(PlayerId, InputCmd)>,
    sounds: Vec<Sound>,
}

impl Battle {
    pub fn new(terrain: &Terrain, bots: u8) -> Self {
        assert!(bots > 0 && bots.is_multiple_of(2));
        let mut world = terrain.world();
        world.set_map_center(128.0, 128.0);
        world.set_team_spawn(Team::Us, ground(&world, 128.0, 56.0));
        world.set_team_spawn(Team::Ger, ground(&world, 128.0, 200.0));
        world.set_objectives(&[ObjectiveSpec {
            id: 1,
            pos: ground(&world, 128.0, 128.0),
            radius_m: 24.0,
            owner: None,
        }]);
        let map = Arc::new(BotMap::build(
            world.view(),
            (MAP_SIZE, MAP_SIZE),
            vec![
                [72.0, 96.0],
                [120.0, 104.0],
                [184.0, 96.0],
                [72.0, 160.0],
                [136.0, 152.0],
                [184.0, 160.0],
            ],
            Tuning::embedded(),
        ));
        let brains = (0..bots)
            .map(|id| {
                let team = if id % 2 == 0 { Team::Us } else { Team::Ger };
                let row = id / 2 / 10;
                let x = 74.0 + f32::from(id / 2 % 10) * 12.0 + f32::from(row) * 0.75;
                let offset = 4.0 * f32::from(row);
                let z = if team == Team::Us {
                    109.0 - offset
                } else {
                    147.0 + offset
                };
                spawn(&mut world, id, team, x, z);
                (id, team, SoldierBrain::with_tactics(map.clone()))
            })
            .collect();
        Self {
            world,
            bots: brains,
            shots: 0,
            kills: 0,
            spawns: 0,
            commands: Vec::with_capacity(bots.into()),
            sounds: Vec::with_capacity(64),
        }
    }

    /// Inspect the complete command set before the sim advances. None drives the legacy path.
    pub fn step(
        &mut self,
        mut runtime: Option<&mut BotRuntime>,
        inspect: impl FnOnce(&World, &[(PlayerId, Team, SoldierBrain)], &mut [(PlayerId, InputCmd)]),
    ) -> Option<BotsMetrics> {
        let tick = self.world.tick().0 + 1;
        let view = self.world.view();
        if let Some(rt) = &mut runtime {
            rt.begin_tick(view, tick);
            let ids: Vec<_> = self.bots.iter().map(|(id, _, _)| *id).collect();
            rt.plan(&ids);
        }
        self.commands.clear();
        for (id, team, brain) in &mut self.bots {
            let ctx = context(&self.world, *id, *team, tick, &self.sounds);
            let cmd = match &mut runtime {
                Some(rt) => rt.think(brain, view, &ctx),
                None => {
                    let cmd = brain.think(view, &ctx);
                    brain.take_stats();
                    cmd
                }
            };
            self.commands.push((*id, cmd));
        }
        let metrics = runtime.map(|rt| rt.finish_tick());
        inspect(&self.world, &self.bots, &mut self.commands);
        self.sounds.retain(|s| tick.saturating_sub(s.tick) <= 120);
        for event in self.world.step(&self.commands).events {
            match event {
                SimEvent::Shot { muzzle, .. } => {
                    self.shots += 1;
                    if self.sounds.len() == 64 {
                        self.sounds.remove(0);
                    }
                    self.sounds.push(Sound {
                        pos: muzzle,
                        tick,
                        range_m: 120.0,
                    });
                }
                SimEvent::Kill { .. } => self.kills += 1,
                SimEvent::PlayerSpawned { .. } => self.spawns += 1,
                _ => {}
            }
        }
        metrics
    }
}
