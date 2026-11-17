//! `kcc_step` (ARCHITECTURE 20.1-2: median single KCC move <= 20 us) and
//! `kcc_100_players_tick` (6.10 stage 2: 100 players one `World::step` <= 3 ms) on the M0 terrain.
//! Plain `Instant` harness like the other benches; writes `reports/baselines/sim-m0.json`
//! (merging its keys into an existing file). Needs the git-ignored terrain:
//! `PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0`.
//! Run with `cargo bench -p frontline-sim --bench kcc` (release, `enhanced-determinism`).
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant; // benches are outside the purity scan (src only)

use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::{
    step_player, CollisionChunk, CollisionData, GameData, GameMode, MapId, MoveParams,
    PlayerAuthState, SimConfig, SpawnReq, Team, Vec3, World,
};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn f32(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

fn load() -> Option<CollisionData> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/m0/chunks");
    let mut v = Vec::new();
    for bz in 0..4 {
        for bx in 0..4 {
            let b = std::fs::read(dir.join(format!("c_{bx}_{bz}.bin"))).ok()?;
            v.push(CollisionChunk::from_bytes(bx, bz, &b).ok()?);
        }
    }
    Some(CollisionData::from_chunks(v))
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn p99(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[(v.len() * 99 / 100).min(v.len() - 1)]
}

/// A random walking command: holds for ~1 s, mostly forward, sometimes jump/crouch/sprint.
fn random_cmd(r: &mut Rng) -> InputCmd {
    let b = r.next();
    let mut btn = 0;
    if b & 15 == 0 {
        btn |= buttons::JUMP;
    }
    if b >> 4 & 7 == 0 {
        btn |= buttons::CROUCH;
    }
    if b >> 7 & 3 == 0 {
        btn |= buttons::SPRINT;
    }
    InputCmd {
        seq: 0,
        tick: 0,
        move_x: ((r.next() % 101) as i32 - 50) as i8,
        move_z: (30 + r.next() % 97) as i8,
        yaw: (r.next() & 0xffff) as u16,
        pitch: 0,
        buttons: btn,
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    }
}

fn cfg() -> SimConfig {
    SimConfig {
        tick_hz: 30,
        match_seed: 1,
        map: MapId(0),
        mode: GameMode::Sandbox,
        data: GameData::embedded(),
    }
}

/// Spawn points spread over the central 160 m of the map.
fn spawn_xz(r: &mut Rng) -> (f32, f32) {
    (48.0 + r.f32() * 160.0, 48.0 + r.f32() * 160.0)
}

fn bench_kcc_step(col: &CollisionData) -> (f64, f64, f64) {
    let p = MoveParams::from_role(GameData::embedded().role("rifleman").unwrap(), 30).unwrap();
    let mut r = Rng(0x1234_5678_9abc_def1);
    // 200 independent walkers, 200 ticks each = 40 000 timed calls.
    let mut samples = Vec::with_capacity(40_000);
    let mut states = Vec::new();
    let mut cmds = Vec::new();
    for _ in 0..200 {
        let (x, z) = spawn_xz(&mut r);
        let y = col.height_at(x, z).unwrap() + 0.2;
        states.push(PlayerAuthState::spawn(Vec3::new(x, y, z), 0.0));
        cmds.push(random_cmd(&mut r));
    }
    for tick in 0..200 {
        for i in 0..states.len() {
            if tick % 30 == 0 {
                cmds[i] = random_cmd(&mut r);
            }
            let t0 = Instant::now();
            let n = step_player(black_box(&states[i]), black_box(&cmds[i]), &p, col);
            let dt = t0.elapsed().as_nanos() as f64 / 1000.0;
            states[i] = n;
            if tick >= 10 {
                samples.push(dt);
            }
        }
    }
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    (median(samples.clone()), p99(samples), mean)
}

fn bench_100_players(col: &CollisionData) -> (f64, f64, f64) {
    let mut r = Rng(0xfeed_beef_0000_0001);
    let mut w = World::new(cfg(), col);
    for p in 0..100u8 {
        let (x, z) = spawn_xz(&mut r);
        let y = col.height_at(x, z).unwrap() + 0.2;
        w.spawn_player(SpawnReq {
            player: p,
            team: if p % 2 == 0 { Team::Us } else { Team::Ger },
            pos: Vec3::new(x, y, z),
            yaw: 0.0,
        })
        .unwrap();
    }
    let mut cmds: Vec<InputCmd> = (0..100).map(|_| random_cmd(&mut r)).collect();
    let mut samples = Vec::with_capacity(600);
    for tick in 0..630 {
        if tick % 30 == 0 {
            for c in cmds.iter_mut() {
                *c = random_cmd(&mut r);
            }
        }
        let inputs: Vec<_> = cmds
            .iter()
            .enumerate()
            .map(|(i, c)| (i as u8, *c))
            .collect();
        let t0 = Instant::now();
        black_box(w.step(black_box(&inputs)));
        let dt = t0.elapsed().as_nanos() as f64 / 1e6;
        if tick >= 30 {
            samples.push(dt);
        }
    }
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    (median(samples.clone()), p99(samples), mean)
}

fn main() {
    let Some(col) = load() else {
        eprintln!(
            "kcc bench SKIPPED: data/maps/m0/chunks missing. Regenerate: PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0"
        );
        return;
    };
    let (med, p99u, mean) = bench_kcc_step(&col);
    println!(
        "kcc_step: median {med:.2} us, mean {mean:.2} us, p99 {p99u:.2} us (budget 20 us) -> {}",
        if med <= 20.0 { "PASS" } else { "FAIL" }
    );
    let (tmed, tp99, tmean) = bench_100_players(&col);
    println!(
        "kcc_100_players_tick: World::step median {tmed:.3} ms, mean {tmean:.3} ms, p99 {tp99:.3} ms (budget 3 ms) -> {}",
        if tmed <= 3.0 { "PASS" } else { "FAIL" }
    );
    let json = format!(
        "{{\n  \"kcc_step_median_us\": {med:.3},\n  \"kcc_step_mean_us\": {mean:.3},\n  \"kcc_step_p99_us\": {p99u:.3},\n  \"kcc_100_players_tick_median_ms\": {tmed:.4},\n  \"kcc_100_players_tick_mean_ms\": {tmean:.4},\n  \"kcc_100_players_tick_p99_ms\": {tp99:.4},\n  \"budget_kcc_step_us\": 20.0,\n  \"budget_kcc_100_players_tick_ms\": 3.0,\n  \"note\": \"M0-011, this machine (not the reference machine), release, enhanced-determinism, plain Instant harness\"\n}}\n"
    );
    if std::env::var_os("FRONTLINE_NO_BASELINE").is_none() {
        let dir = std::env::var_os("FRONTLINE_BASELINE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reports/baselines")
            });
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("sim-m0.json"), json);
    }
}
