//! Shared by `examples/determinism.rs` (CI driver) and `tests/determinism_native.rs` (M0-029).
//! Included with `#[path]`, so it must not depend on anything but the crate's dependencies.
//!
//! * `gen_inputs(seed, n)`: fixed 10-minute-style random input sequence (own PCG, no `rand` dep).
//! * `run_predict`: one `PredictSim` run on the real M0 terrain; returns per-tick `state_hash`
//!   (same function as `Predictor.stateHash()`), positions and coverage statistics.
//! * `run_world`: 10-player `World` on the same terrain; per-tick `World::state_hash`.
#![allow(dead_code)]

use std::path::PathBuf;

use frontline_protocol::bitio::BitWriter;
use frontline_protocol::codec::Codec;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::hash::HashWriter;
use frontline_sim::predict::PredictSim;
use frontline_sim::weapon::{WeaponParams, WeaponState};
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, PlayerAuthState, SimConfig, SpawnReq,
    Team, Tick, Vec3, World,
};

pub const TICK_HZ: u8 = 30;
pub const PLAYER: u8 = 3;
/// Spawn (x, z) per seed: flat south field below the hills, foot of the southern slope (steepest
/// terrain of M0, up to ~39 deg), and the northern plateau interior. y comes from the height map.
pub const SPAWNS_XZ: [[f32; 2]; 3] = [[128.0, 100.0], [60.0, 118.0], [128.0, 170.0]];

pub fn spawn_for(seed: u64, col: &CollisionData) -> [f32; 3] {
    let [x, z] = SPAWNS_XZ[(seed as usize + 2) % 3];
    [
        x,
        col.height_at(x, z).expect("spawn inside the map") + 0.05,
        z,
    ]
}
pub const RESERVE: u16 = 60_000;
/// Ticks of a 10-minute run at 30 Hz.
pub const TEN_MIN_TICKS: usize = 18_000;

/// PCG-XSH-RR 64/32 (O'Neill); tiny, fixed, platform independent.
pub struct Pcg(u64);
impl Pcg {
    pub fn new(seed: u64) -> Self {
        let mut p = Pcg(0);
        p.next();
        p.0 = p.0.wrapping_add(seed ^ 0x9e37_79b9_7f4a_7c15);
        p.next();
        p
    }
    pub fn next(&mut self) -> u32 {
        let old = self.0;
        self.0 = old
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x = (((old >> 18) ^ old) >> 27) as u32;
        x.rotate_right((old >> 59) as u32)
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
}

/// Random inputs: intent segments of 6..=90 ticks. Mostly forward movement with strafing, view
/// yaw wandering (occasional large jumps) and pitch changes; sprint, crouch, jump, fire, reload,
/// aim all appear. The player roams the M0 map (slopes, chunk borders) for the whole run.
pub fn gen_inputs(seed: u64, n: usize) -> Vec<InputCmd> {
    let mut r = Pcg::new(seed);
    let mut yaw: u16 = (r.next() & 0xffff) as u16;
    let mut pitch: i16 = 0;
    let (mut mx, mut mz, mut b, mut aux, mut fire) = (0i8, 0i8, 0u16, 0u8, false);
    let mut left = 0u32;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if left == 0 {
            left = 6 + r.below(85);
            let a = r.next();
            mz = if a & 7 == 0 {
                -(r.below(128) as i32) as i8
            } else {
                40 + r.below(88) as i8
            };
            mx = (r.below(255) as i32 - 127) as i8 / 2;
            b = 0;
            if a >> 3 & 15 == 0 {
                b |= buttons::JUMP;
            }
            if a >> 7 & 7 == 0 {
                b |= buttons::CROUCH;
            }
            if a >> 10 & 3 == 0 {
                b |= buttons::SPRINT;
            }
            fire = a >> 12 & 3 != 0; // pulsed below: the rifle is semi-automatic (trigger edge)
            if a >> 14 & 15 == 0 {
                b |= buttons::RELOAD;
            }
            if a >> 18 & 7 == 0 {
                b |= buttons::AIM;
            }
            aux = (a >> 21 & 1) as u8;
            // Large view change per segment (any direction), small drift per tick below.
            yaw = yaw.wrapping_add(((r.below(0x6000)) as u16).wrapping_sub(0x3000));
            pitch = (r.below(0x2000) as i32 - 0x1000) as i16;
        }
        left -= 1;
        yaw = yaw.wrapping_add((r.below(129) as i32 - 64) as u16);
        out.push(InputCmd {
            seq: i as u16,
            tick: i as u32 + 1,
            move_x: mx,
            move_z: mz,
            yaw,
            pitch,
            buttons: if fire && left % 8 < 3 {
                b | buttons::FIRE
            } else {
                b
            },
            weapon_slot: 0,
            aux,
            view_tick: 0,
        });
    }
    out
}

pub fn encode_cmd(c: &InputCmd) -> Vec<u8> {
    let mut w = BitWriter::new();
    c.encode(&mut w).expect("encode InputCmd");
    w.finish()
}

/// File format `[u32 LE count][18 B x count]` (full-width `InputCmd`, the `Predictor.step` input).
pub fn inputs_to_bytes(cmds: &[InputCmd]) -> Vec<u8> {
    let mut v = (cmds.len() as u32).to_le_bytes().to_vec();
    for c in cmds {
        v.extend_from_slice(&encode_cmd(c));
    }
    v
}

pub fn inputs_from_bytes(b: &[u8]) -> Result<Vec<InputCmd>, String> {
    use frontline_protocol::bitio::BitReader;
    let n = u32::from_le_bytes(b.get(0..4).ok_or("short header")?.try_into().unwrap()) as usize;
    if b.len() != 4 + 18 * n {
        return Err(format!("length {} does not match count {n}", b.len()));
    }
    (0..n)
        .map(|i| {
            let mut r = BitReader::new(&b[4 + 18 * i..4 + 18 * (i + 1)]);
            InputCmd::decode(&mut r).map_err(|e| format!("cmd {i}: {e}"))
        })
        .collect()
}

pub fn terrain() -> Option<CollisionData> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/m0/chunks");
    let mut chunks = Vec::new();
    for bz in 0..4 {
        for bx in 0..4 {
            let bytes = std::fs::read(dir.join(format!("c_{bx}_{bz}.bin"))).ok()?;
            chunks.push(CollisionChunk::from_bytes(bx, bz, &bytes).ok()?);
        }
    }
    Some(CollisionData::from_chunks(chunks))
}

/// Terrain or skip (prints the regeneration command); `FRONTLINE_REQUIRE_TERRAIN=1` makes it fatal.
pub fn terrain_or_skip() -> Option<CollisionData> {
    let t = terrain();
    if t.is_none() {
        let msg = "M0 terrain missing: PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0";
        assert!(
            std::env::var_os("FRONTLINE_REQUIRE_TERRAIN").is_none(),
            "{msg}"
        );
        eprintln!("SKIP: {msg}");
    }
    t
}

fn cfg(seed: u64) -> SimConfig {
    SimConfig {
        tick_hz: TICK_HZ,
        match_seed: seed,
        map: MapId(0),
        mode: GameMode::Sandbox,
        data: GameData::embedded(),
    }
}

/// Same fold as `Predictor.stateHash()` in sim-wasm: auth bytes, weapon presence flag, weapon bytes.
pub fn predict_hash(sim: &PredictSim) -> u64 {
    let mut h = HashWriter::new();
    h.write_bytes(&sim.state().to_bytes());
    match sim.weapon() {
        Some(w) => {
            h.write_u8(1);
            h.write_bytes(&w.to_bytes());
        }
        None => h.write_u8(0),
    }
    h.finish()
}

pub struct PredictRun {
    pub spawn: [f32; 3],
    pub hashes: Vec<u64>,
    /// x, y, z per tick (f32).
    pub pos: Vec<[f32; 3]>,
    pub blocked_ticks: usize,
    pub grounded_ticks: usize,
    pub chunks_visited: usize,
    pub shots: usize,
    pub max_speed: f32,
    pub elapsed_s: f64,
}

/// Test hook: lets a caller corrupt the second run (fault injection for acceptance 5).
pub type Fault<'a> = Option<&'a mut dyn FnMut(usize, &mut InputCmd)>;

pub fn run_predict(
    match_seed: u64,
    col: &CollisionData,
    cmds: &[InputCmd],
    mut fault: Fault,
) -> PredictRun {
    let mut sim = PredictSim::new(cfg(match_seed), col, PLAYER);
    let sp = spawn_for(match_seed, col);
    sim.reset_to(
        &PlayerAuthState::spawn(Vec3::new(sp[0], sp[1], sp[2]), 0.0),
        Tick(0),
    );
    let data = GameData::embedded();
    let idx = data
        .weapons
        .iter()
        .position(|w| w.id == "us_m1_garand")
        .unwrap();
    let mag = WeaponParams::from_def(&data.weapons[idx], TICK_HZ)
        .unwrap()
        .mag_capacity;
    sim.reset_weapon(Some(WeaponState::new(idx as u16, mag, RESERVE)));
    let mut run = PredictRun {
        spawn: sp,
        hashes: Vec::with_capacity(cmds.len()),
        pos: Vec::with_capacity(cmds.len()),
        blocked_ticks: 0,
        grounded_ticks: 0,
        chunks_visited: 0,
        shots: 0,
        max_speed: 0.0,
        elapsed_s: 0.0,
    };
    let mut seen = [false; 16];
    let t0 = std::time::Instant::now();
    for (i, c) in cmds.iter().enumerate() {
        let mut c = *c;
        if let Some(f) = fault.as_mut() {
            f(i, &mut c);
        }
        let s = sim.step_player(&c);
        run.hashes.push(predict_hash(&sim));
        run.pos.push([s.pos.x, s.pos.y, s.pos.z]);
        run.blocked_ticks += usize::from(s.blocked);
        run.grounded_ticks += usize::from(s.grounded);
        run.shots += usize::from(sim.last_weapon_out().shot.is_some());
        run.max_speed = run.max_speed.max(s.vel.length());
        let (bx, bz) = (
            (s.pos.x / 64.0).floor() as i32,
            (s.pos.z / 64.0).floor() as i32,
        );
        if (0..4).contains(&bx) && (0..4).contains(&bz) {
            seen[(bz * 4 + bx) as usize] = true;
        }
    }
    run.elapsed_s = t0.elapsed().as_secs_f64();
    run.chunks_visited = seen.iter().filter(|b| **b).count();
    run
}

/// 10-player `World` run (players spawn in a row at the spawn point, each with its own input
/// stream derived from `seed`); returns the per-tick `World::state_hash`.
pub fn run_world(seed: u64, col: &CollisionData, ticks: usize) -> (Vec<u64>, f64) {
    let mut w = World::new(cfg(seed), col);
    let streams: Vec<Vec<InputCmd>> = (0..10u64)
        .map(|p| gen_inputs(seed * 1000 + p, ticks))
        .collect();
    for p in 0..10u8 {
        let sp = spawn_for(seed, col);
        let x = sp[0] - 10.0 + f32::from(p) * 2.0;
        let y = col.height_at(x, sp[2]).unwrap_or(sp[1]) + 0.05;
        w.spawn_player(SpawnReq {
            player: p,
            team: if p % 2 == 0 { Team::Us } else { Team::Ger },
            pos: Vec3::new(x, y, sp[2]),
            yaw: 0.0,
        })
        .unwrap();
    }
    let t0 = std::time::Instant::now();
    let mut hashes = Vec::with_capacity(ticks);
    for t in 0..ticks {
        let inputs: Vec<_> = streams
            .iter()
            .enumerate()
            .map(|(p, s)| (p as u8, s[t]))
            .collect();
        w.step(&inputs);
        hashes.push(w.state_hash());
    }
    (hashes, t0.elapsed().as_secs_f64())
}

/// First index at which two hash sequences differ (or a length mismatch).
pub fn first_diff(a: &[u64], b: &[u64]) -> Option<usize> {
    a.iter()
        .zip(b)
        .position(|(x, y)| x != y)
        .or_else(|| (a.len() != b.len()).then(|| a.len().min(b.len())))
}
