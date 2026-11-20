//! Shared helpers for the collision integration tests.
#![allow(dead_code)]

use std::path::PathBuf;

use frontline_sim::{CollisionChunk, CollisionData};

pub const REGEN: &str =
    "PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0";

pub fn map_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/m0")
}

/// Real M0 terrain, or `None` (with a message naming the regeneration command) when the
/// git-ignored binaries are absent. Set `FRONTLINE_REQUIRE_TERRAIN=1` to turn that into a failure.
pub fn real_terrain(test: &str) -> Option<CollisionData> {
    let dir = map_dir().join("chunks");
    let mut chunks = Vec::new();
    for bz in 0..4 {
        for bx in 0..4 {
            let p = dir.join(format!("c_{bx}_{bz}.bin"));
            match std::fs::read(&p) {
                Ok(bytes) => chunks.push(
                    CollisionChunk::from_bytes(bx, bz, &bytes)
                        .unwrap_or_else(|e| panic!("{}: {e}", p.display())),
                ),
                Err(_) => {
                    let msg = format!(
                        "{test}: SKIPPED, {} missing. Regenerate with: {REGEN}",
                        p.display()
                    );
                    assert!(
                        std::env::var_os("FRONTLINE_REQUIRE_TERRAIN").is_none(),
                        "{msg}"
                    );
                    eprintln!("{msg}");
                    return None;
                }
            }
        }
    }
    Some(CollisionData::from_chunks(chunks))
}

/// Reference heights (metres) of `heightmap_1m.r16`: 257x257, row = z.
pub fn reference_heightmap(test: &str) -> Option<Vec<f64>> {
    let p = map_dir().join("heightmap_1m.r16");
    let Ok(bytes) = std::fs::read(&p) else {
        eprintln!(
            "{test}: SKIPPED, {} missing. Regenerate with: {REGEN}",
            p.display()
        );
        return None;
    };
    assert_eq!(bytes.len(), 257 * 257 * 2);
    // hmin/hmax from meta.json (height.hmin_m / hmax_m).
    let meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(map_dir().join("meta.json")).unwrap()).unwrap();
    let hmin = meta["height"]["hmin_m"].as_f64().unwrap();
    let hmax = meta["height"]["hmax_m"].as_f64().unwrap();
    Some(
        bytes
            .chunks_exact(2)
            .map(|p| hmin + u16::from_le_bytes([p[0], p[1]]) as f64 / 65535.0 * (hmax - hmin))
            .collect(),
    )
}

/// Synthetic chunk sampling `f(world_x, world_z)` at the 65x65 sample positions.
pub fn synth_chunk(bx: i32, bz: i32, f: &dyn Fn(f32, f32) -> f32) -> CollisionChunk {
    let mut h = Vec::with_capacity(65 * 65);
    for iz in 0..65 {
        for ix in 0..65 {
            h.push(f(
                bx as f32 * 64.0 + ix as f32,
                bz as f32 * 64.0 + iz as f32,
            ));
        }
    }
    CollisionChunk::new(bx, bz, h, vec![0; 64 * 64])
}

/// `nx` x `nz` chunk grid of a synthetic surface.
pub fn synth_data(nx: i32, nz: i32, f: &dyn Fn(f32, f32) -> f32) -> CollisionData {
    let mut v = Vec::new();
    for bz in 0..nz {
        for bx in 0..nx {
            v.push(synth_chunk(bx, bz, f));
        }
    }
    CollisionData::from_chunks(v)
}

/// Deterministic xorshift for tests (independent of the sim RNG).
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f32()
    }
}

// ---- movement helpers (M0-011) ----

use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::{step_player, GameData, MoveParams, PlayerAuthState, Vec3};

pub fn move_params() -> MoveParams {
    MoveParams::from_role(GameData::embedded().role("rifleman").unwrap(), 30).unwrap()
}

/// Command with the given move axes (-127..127), yaw code (0 = facing +z) and button mask.
pub fn cmd(move_x: i8, move_z: i8, yaw: u16, buttons: u16, aux: u8) -> InputCmd {
    InputCmd {
        seq: 0,
        tick: 0,
        move_x,
        move_z,
        yaw,
        pitch: 0,
        buttons,
        weapon_slot: 0,
        aux,
        view_tick: 0,
    }
}

pub fn fwd(buttons: u16, aux: u8) -> InputCmd {
    cmd(0, 127, 0, buttons, aux)
}

pub const BTN_SPRINT: u16 = buttons::SPRINT;
pub const BTN_CROUCH: u16 = buttons::CROUCH;
pub const BTN_JUMP: u16 = buttons::JUMP;

/// Spawns a player 0.3 m above the terrain and idles until grounded.
pub fn settle(col: &CollisionData, p: &MoveParams, x: f32, z: f32) -> PlayerAuthState {
    let y = col.height_at(x, z).unwrap() + 0.3;
    let mut s = PlayerAuthState::spawn(Vec3::new(x, y, z), 0.0);
    let idle = cmd(0, 0, 0, 0, 0);
    for _ in 0..30 {
        s = step_player(&s, &idle, p, col);
    }
    assert!(s.grounded, "player did not settle: {s:?}");
    s
}

/// Flat terrain of `n` x `n` chunks at height `h`.
pub fn flat(n: i32, h: f32) -> CollisionData {
    synth_data(n, n, &move |_, _| h)
}

// ---- weapon helpers (M0-012) ----

use frontline_sim::weapon::{step_weapon, ShotCtx, WeaponOut, WeaponParams, WeaponState};
use frontline_sim::Tick;

pub const BTN_FIRE: u16 = buttons::FIRE;
pub const BTN_AIM: u16 = buttons::AIM;
pub const BTN_RELOAD: u16 = buttons::RELOAD;

pub fn weapon_index(id: &str) -> u16 {
    GameData::embedded()
        .weapons
        .iter()
        .position(|w| w.id == id)
        .unwrap_or_else(|| panic!("weapon {id}")) as u16
}

pub fn weapon_params(id: &str) -> WeaponParams {
    let def = &GameData::embedded().weapons[weapon_index(id) as usize];
    WeaponParams::from_def(def, 30).unwrap()
}

/// One standing player with a weapon, stepped directly through `step_weapon`.
pub struct Rig {
    pub w: WeaponState,
    pub wp: WeaponParams,
    pub mp: MoveParams,
    pub auth: PlayerAuthState,
    pub tick: u32,
    pub seed: u64,
    pub entity: u16,
}

impl Rig {
    /// Full magazine and `mags` spare clips.
    pub fn new(weapon: &str, mags: u32) -> Self {
        let wp = weapon_params(weapon);
        let reserve = (u32::from(wp.mag_capacity) * mags) as u16;
        let mut auth = PlayerAuthState::spawn(Vec3::new(10.0, 5.0, 10.0), 0.0);
        auth.grounded = true;
        Rig {
            w: WeaponState::new(weapon_index(weapon), wp.mag_capacity, reserve),
            wp,
            mp: move_params(),
            auth,
            tick: 0,
            seed: 99,
            entity: 3,
        }
    }

    /// Steps one tick with `buttons` and the given move axes.
    pub fn step_cmd(&mut self, c: frontline_protocol::msg::input::InputCmd) -> WeaponOut {
        self.tick += 1;
        let ctx = ShotCtx {
            match_seed: self.seed,
            tick: Tick(self.tick),
            entity: self.entity,
        };
        step_weapon(&mut self.w, &self.wp, &c, &self.auth, &self.mp, ctx)
    }

    pub fn step(&mut self, buttons: u16) -> WeaponOut {
        self.step_cmd(cmd(0, 0, 0, buttons, 0))
    }

    pub fn idle(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.step(0);
        }
    }

    /// Input of the fastest possible clicker (hold the trigger, release it for exactly one tick
    /// right before the weapon is ready, press again): the rate cap is reached, not exceeded.
    pub fn max_rate_input(&self) -> u16 {
        let t = self.w.timer_units;
        let cooling = matches!(
            self.w.fsm,
            frontline_sim::weapon::Fsm::Cooldown | frontline_sim::weapon::Fsm::BoltCycle
        );
        if cooling
            && t > frontline_sim::weapon::TICK_UNITS
            && t <= 2 * frontline_sim::weapon::TICK_UNITS
        {
            0
        } else {
            BTN_FIRE
        }
    }

    /// Runs the max-rate clicker for `ticks`, topping the magazine up when it is empty (rate
    /// tests only); returns the 1-based ticks of the shots.
    pub fn max_rate(&mut self, ticks: u32) -> Vec<u32> {
        let mut shots = Vec::new();
        for _ in 0..ticks {
            if self.w.mag == 0 {
                self.w.mag = self.wp.mag_capacity;
                if self.w.fsm == frontline_sim::weapon::Fsm::Empty {
                    self.w.fsm = frontline_sim::weapon::Fsm::Ready;
                }
            }
            let b = self.max_rate_input();
            if self.step(b).shot.is_some() {
                shots.push(self.tick);
            }
        }
        shots
    }

    /// Presses fire on every other tick (press, release, press, ...) for `ticks` ticks; returns
    /// the ticks (1-based) at which a shot left the barrel.
    pub fn mash(&mut self, ticks: u32) -> Vec<u32> {
        let mut shots = Vec::new();
        for i in 0..ticks {
            let out = self.step(if i % 2 == 0 { BTN_FIRE } else { 0 });
            if out.shot.is_some() {
                shots.push(self.tick);
            }
        }
        shots
    }
}

// ---- world-level weapon helpers (M0-012) ----

use frontline_protocol::quant::rad_to_pitch_i16;
use frontline_sim::data::defs::WeaponDef;
use frontline_sim::weapon::ballistics::{substep, BallisticsParams, MUZZLE_OFFSET_M};
use frontline_sim::{GameMode, MapId, SimConfig, SimEvent, SpawnReq, Team, World};

/// `GameData::embedded()` with every weapon edited by `f` (e.g. zero spread for exact hits).
pub fn data_with(f: impl Fn(&mut WeaponDef)) -> &'static GameData {
    let e = GameData::embedded();
    let weapons: Vec<WeaponDef> = e
        .weapons
        .iter()
        .cloned()
        .map(|mut w| {
            f(&mut w);
            w
        })
        .collect();
    Box::leak(Box::new(GameData {
        weapons: Box::leak(weapons.into_boxed_slice()),
        explosives: e.explosives,
        melee: e.melee,
        ammo: e.ammo,
        materials: e.materials,
        gadgets: e.gadgets,
        roles: e.roles,
        loadouts: e.loadouts,
        vehicles: e.vehicles,
        rules: e.rules,
        hash: e.hash,
    }))
}

/// `no_spread()` with the respawn rules edited by `f` (M0-013).
pub fn with_respawn_rules(
    f: impl Fn(&mut frontline_sim::data::defs::RespawnRulesDef),
) -> &'static GameData {
    let e = no_spread();
    let mut rules = e.rules.clone();
    f(&mut rules.respawn);
    Box::leak(Box::new(GameData {
        weapons: e.weapons,
        explosives: e.explosives,
        melee: e.melee,
        ammo: e.ammo,
        materials: e.materials,
        gadgets: e.gadgets,
        roles: e.roles,
        loadouts: e.loadouts,
        vehicles: e.vehicles,
        rules: Box::leak(Box::new(rules)),
        hash: e.hash,
    }))
}

pub fn no_spread() -> &'static GameData {
    data_with(|w| {
        w.gameplay.spread.hip_deg = 0.0;
        w.gameplay.spread.ads_deg = 0.0;
    })
}

pub fn sim_cfg(data: &'static GameData, seed: u64) -> SimConfig {
    SimConfig {
        tick_hz: 30,
        match_seed: seed,
        map: MapId(0),
        mode: GameMode::Sandbox,
        data,
    }
}

/// World with the given players `(id, team, x, z, yaw code)` on flat terrain at y = 5 and
/// already settled on the ground.
pub fn duel_world(
    data: &'static GameData,
    col: &CollisionData,
    seed: u64,
    players: &[(u8, Team, f32, f32, u16)],
) -> World {
    let mut w = World::new(sim_cfg(data, seed), col);
    for &(id, team, x, z, _) in players {
        w.spawn_player(SpawnReq {
            player: id,
            team,
            pos: Vec3::new(x, 5.02, z),
            yaw: 0.0,
        })
        .unwrap();
    }
    for _ in 0..10 {
        let inputs: Vec<_> = players
            .iter()
            .map(|p| (p.0, cmd(0, 0, p.4, 0, 0)))
            .collect();
        w.step(&inputs);
    }
    for p in players {
        assert!(w.player_state(p.0).unwrap().grounded);
    }
    w
}

/// Pitch (radians) that makes a shot fired by a standing player at the origin pass `target_y`
/// metres above the shooter's feet at a horizontal distance `dist`, with the sim integrator.
pub fn solve_pitch(bp: &BallisticsParams, eye_h: f32, dist: f32, target_y: f32) -> f32 {
    let height_at = |pitch: f32| {
        let dir = Vec3::new(libm::cosf(pitch), libm::sinf(pitch), 0.0);
        let (mut p, mut v) = (
            Vec3::new(0.0, eye_h, 0.0) + dir * MUZZLE_OFFSET_M,
            dir * bp.muzzle_velocity_mps,
        );
        let h = 1.0 / 120.0;
        loop {
            let (p0, _) = (p, v);
            substep(&mut p, &mut v, bp.drag_k_per_m, bp.gravity_mps2, h);
            if p.x >= dist {
                let f = (dist - p0.x) / (p.x - p0.x);
                return p0.y + f * (p.y - p0.y);
            }
        }
    };
    let (mut lo, mut hi) = (-0.2f32, 0.3f32);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if height_at(mid) < target_y {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Command that fires along +x (yaw code 16384) with the given pitch (radians).
pub fn fire_cmd(pitch: f32, buttons: u16) -> frontline_protocol::msg::input::InputCmd {
    let mut c = cmd(0, 0, 16384, buttons, 0);
    c.pitch = rad_to_pitch_i16(pitch).code;
    c
}

/// Folds events into an order-sensitive hash (floats by bit pattern).
pub fn event_hash(events: &[SimEvent]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let mut put = |b: &[u8]| {
        for &x in b {
            h = (h ^ u64::from(x)).wrapping_mul(0x0100_0000_01b3);
        }
    };
    for e in events {
        match e {
            SimEvent::Shot {
                shooter,
                weapon,
                muzzle,
                dir,
            } => {
                put(&[1]);
                put(&shooter.to_le_bytes());
                put(&weapon.to_le_bytes());
                for v in [muzzle.x, muzzle.y, muzzle.z, dir.x, dir.y, dir.z] {
                    put(&v.to_bits().to_le_bytes());
                }
            }
            SimEvent::Hit {
                shooter,
                target,
                zone,
                damage_hp,
                dist_m,
            } => {
                put(&[2]);
                put(&shooter.to_le_bytes());
                put(&target.to_le_bytes());
                put(&[*zone as u8]);
                put(&damage_hp.to_bits().to_le_bytes());
                put(&dist_m.to_bits().to_le_bytes());
            }
            SimEvent::Impact {
                pos,
                normal,
                material,
            } => {
                put(&[3, *material]);
                for v in [pos.x, pos.y, pos.z, normal.x, normal.y, normal.z] {
                    put(&v.to_bits().to_le_bytes());
                }
            }
            SimEvent::DryFire { shooter } => {
                put(&[4]);
                put(&shooter.to_le_bytes());
            }
            SimEvent::Kill {
                victim,
                killer,
                weapon,
                headshot,
                dist_m,
            } => {
                put(&[5, *weapon, u8::from(*headshot)]);
                put(&[*victim]);
                put(&killer.map_or([0xFF, 0xFF], |k| [k, 0]));
                put(&dist_m.to_bits().to_le_bytes());
            }
            SimEvent::Spawn { player, pos } => {
                put(&[6, *player]);
                for v in [pos.x, pos.y, pos.z] {
                    put(&v.to_bits().to_le_bytes());
                }
            }
            _ => put(&[9]),
        }
    }
    h
}
