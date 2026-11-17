//! Native reference trace for the wasm-vs-native comparison (`tests/node-smoke.mjs`,
//! `tests/native-vs-wasm.mjs`). Runs a scripted input sequence through the native `PredictSim`
//! on the real M0 terrain and writes, into the output directory (default
//! `target/wasm-trace`):
//!   cmds.bin   N x 18 B `InputCmd` (what `Predictor.step` takes)
//!   trace.bin  N x 72 B: `PlayerAuthState::to_bytes` (39) ++ `WeaponState::to_bytes` (33) after each tick
//!   meta.json  `{ticks, protocolVersion, dataHash, matchSeed, ...}`
//! Run: `cargo run --release -p frontline-sim-wasm --example trace [-- OUT_DIR [TICKS]]`.
use std::path::PathBuf;

use frontline_protocol::bitio::BitWriter;
use frontline_protocol::codec::Codec;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::predict::PredictSim;
use frontline_sim::weapon::{WeaponParams, WeaponState};
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, PlayerAuthState, SimConfig, Tick,
    Vec3,
};

const SEED: u64 = 0x5eed_0014;
const PLAYER: u8 = 3;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

/// Deterministic script: changes intent every 6 ticks; mixes walking, sprinting, jumping,
/// crouching, aiming, firing bursts and reloads.
fn script(n: usize) -> Vec<InputCmd> {
    let mut r = Lcg(0xfeed_0014);
    let mut cur = (0i8, 0i8, 0u16, 0i16, 0u16, 0u8);
    (0..n)
        .map(|i| {
            if i % 6 == 0 {
                let a = r.next();
                let mv = |v: u64| ((v % 255) as i32 - 127) as i8;
                let mut b = 0u16;
                if a & 7 == 0 {
                    b |= buttons::JUMP;
                }
                if a >> 3 & 7 == 0 {
                    b |= buttons::CROUCH;
                }
                if a >> 6 & 3 == 0 {
                    b |= buttons::SPRINT;
                }
                if a >> 8 & 3 != 0 {
                    b |= buttons::FIRE;
                }
                if a >> 10 & 15 == 0 {
                    b |= buttons::RELOAD;
                }
                if a >> 14 & 7 == 0 {
                    b |= buttons::AIM;
                }
                cur = (
                    mv(r.next()),
                    mv(r.next()),
                    (r.next() & 0xffff) as u16,
                    ((r.next() & 0xffff) as i32 - 32768) as i16 / 4,
                    b,
                    (a >> 20 & 1) as u8,
                );
            }
            InputCmd {
                seq: i as u16,
                tick: i as u32 + 1,
                move_x: cur.0,
                move_z: cur.1,
                yaw: cur.2,
                pitch: cur.3,
                buttons: cur.4,
                weapon_slot: 0,
                aux: cur.5,
                view_tick: 0,
            }
        })
        .collect()
}

fn terrain() -> CollisionData {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/m0/chunks");
    let mut chunks = Vec::new();
    for bz in 0..4 {
        for bx in 0..4 {
            let p = dir.join(format!("c_{bx}_{bz}.bin"));
            let bytes = std::fs::read(&p).unwrap_or_else(|e| {
                panic!(
                    "{}: {e} (regenerate terrain, see DEVELOPING.md)",
                    p.display()
                )
            });
            chunks.push(CollisionChunk::from_bytes(bx, bz, &bytes).unwrap());
        }
    }
    CollisionData::from_chunks(chunks)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/wasm-trace".into()));
    let n: usize = args.next().map_or(3000, |s| s.parse().unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let cfg = SimConfig {
        tick_hz: 30,
        match_seed: SEED,
        map: MapId(0),
        mode: GameMode::Sandbox,
        data: GameData::embedded(),
    };
    let mut sim = PredictSim::new(cfg, &terrain(), PLAYER);
    sim.reset_to(
        &PlayerAuthState::spawn(Vec3::new(128.0, 2.4, 56.0), 0.0),
        Tick(0),
    );
    let data = GameData::embedded();
    let idx = data
        .weapons
        .iter()
        .position(|w| w.id == "us_m1_garand")
        .unwrap();
    let mag = WeaponParams::from_def(&data.weapons[idx], 30)
        .unwrap()
        .mag_capacity;
    sim.reset_weapon(Some(WeaponState::new(idx as u16, mag, 400)));

    let cmds = script(n);
    let (mut cb, mut tb) = (Vec::new(), Vec::new());
    for c in &cmds {
        let mut w = BitWriter::new();
        c.encode(&mut w).unwrap();
        cb.extend_from_slice(&w.finish());
        sim.step_player(c);
        tb.extend_from_slice(&sim.state().to_bytes());
        tb.extend_from_slice(&sim.weapon().unwrap().to_bytes());
    }
    std::fs::write(out.join("cmds.bin"), cb).unwrap();
    std::fs::write(out.join("trace.bin"), tb).unwrap();
    std::fs::write(
        out.join("meta.json"),
        format!(
            "{{\"ticks\":{n},\"protocolVersion\":{},\"dataHash\":\"{}\",\"matchSeed\":\"{SEED}\",\"player\":{PLAYER},\"weaponIndex\":{idx},\"reserve\":400,\"spawn\":[128.0,2.4,56.0]}}\n",
            frontline_protocol::PROTOCOL_VERSION,
            frontline_sim::data::DATA_HASH
        ),
    )
    .unwrap();
    println!("wrote {n} ticks to {}", out.display());
}
