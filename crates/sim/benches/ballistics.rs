//! `ballistics_step` (ARCHITECTURE 6.10 stage 4 "weapons and projectiles": <= 2.5 ms for 400
//! active projectiles x 4 sub-steps) on the M0 terrain with 20 players.
//! Plain `Instant` harness like the other benches. The cost of the projectile stage is the
//! `World::step` time with 400 projectiles in flight minus the same step without projectiles.
//! Also measures the weapon stage (20 players firing). Needs the git-ignored terrain:
//! `PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0`.
//! Run with `cargo bench -p frontline-sim --bench ballistics`; writes
//! `reports/baselines/sim-m0-weapon.json` (set `FRONTLINE_NO_BASELINE` to skip).
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant; // benches are outside the purity scan (src only)

use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, PlayerId, SimConfig, SpawnReq, Team,
    Vec3, World,
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

fn idle(yaw: u16) -> InputCmd {
    InputCmd {
        seq: 0,
        tick: 0,
        move_x: 0,
        move_z: 0,
        yaw,
        pitch: 0,
        buttons: 0,
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    }
}

fn world(col: &CollisionData, r: &mut Rng) -> World {
    let mut w = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 1,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        col,
    );
    for p in 0..20u8 {
        let (x, z) = (48.0 + r.f32() * 160.0, 48.0 + r.f32() * 160.0);
        let y = col.height_at(x, z).unwrap() + 0.1;
        w.spawn_player(SpawnReq {
            player: p,
            team: if p % 2 == 0 { Team::Us } else { Team::Ger },
            pos: Vec3::new(x, y, z),
            yaw: 0.0,
        })
        .unwrap();
    }
    w
}

fn inputs() -> Vec<(PlayerId, InputCmd)> {
    (0..20u8).map(|p| (p, idle(0))).collect()
}

/// `World::step` time (ms) of tick 2 (everyone already on the ground), with `n` projectiles
/// injected before it, flying horizontally 3 m above the ground at the start (inside the relief band, so every sub-step
/// runs a real terrain ray; some end early on a hill).
fn timed_step(col: &CollisionData, seed: u64, n: usize) -> f64 {
    let mut r = Rng(seed);
    let mut w = world(col, &mut r);
    let inp = inputs();
    w.step(&inp);
    for i in 0..n {
        let (x, z) = (20.0 + r.f32() * 216.0, 20.0 + r.f32() * 216.0);
        let a = r.f32() * core::f32::consts::TAU;
        let (sx, sz) = (libm::sinf(a), libm::cosf(a));
        let weapon = (i % 2) as u16; // both M0 weapons, alternating
        let _ = w.spawn_projectile(
            (i % 20) as u8,
            weapon,
            Vec3::new(x, col.height_at(x, z).unwrap() + 3.0, z),
            Vec3::new(sx * 800.0, 0.0, sz * 800.0),
        );
    }
    let t0 = Instant::now();
    black_box(w.step(black_box(&inp)));
    t0.elapsed().as_nanos() as f64 / 1e6
}

/// Shooters: all 20 players hold fire / mash the trigger for 30 ticks; time of the whole step.
fn firing_ticks(col: &CollisionData) -> Vec<f64> {
    let mut r = Rng(77);
    let mut w = world(col, &mut r);
    let mut samples = Vec::new();
    for tick in 0..230 {
        let mut inp = inputs();
        if tick >= 10 {
            for (_, c) in inp.iter_mut() {
                c.buttons = if tick % 2 == 0 { buttons::FIRE } else { 0 };
                c.yaw = (tick as u16).wrapping_mul(997);
            }
        }
        let t0 = Instant::now();
        black_box(w.step(black_box(&inp)));
        if tick >= 20 {
            samples.push(t0.elapsed().as_nanos() as f64 / 1e6);
        }
    }
    samples
}

fn main() {
    let Some(col) = load() else {
        eprintln!(
            "ballistics bench SKIPPED: data/maps/m0/chunks missing. Regenerate: PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0"
        );
        return;
    };
    let reps = 60;
    // Warm-up.
    for s in 0..5 {
        timed_step(&col, 900 + s, 400);
    }
    let base: Vec<f64> = (0..reps).map(|s| timed_step(&col, 1000 + s, 0)).collect();
    let with400: Vec<f64> = (0..reps).map(|s| timed_step(&col, 1000 + s, 400)).collect();
    let (b, m) = (median(base.clone()), median(with400.clone()));
    let stage: Vec<f64> = with400.iter().zip(&base).map(|(a, b)| a - b).collect();
    let (smed, sp99) = (median(stage.clone()), p99(stage));
    println!(
        "ballistics_step: World::step with 400 projectiles x 4 sub-steps: median {m:.3} ms (p99 {:.3}); without {b:.3} ms; projectile stage = {smed:.3} ms median (p99 {sp99:.3}) (budget 2.5 ms) -> {}",
        p99(with400.clone()),
        if smed <= 2.5 { "PASS" } else { "FAIL" }
    );
    let fire = firing_ticks(&col);
    let (fmed, fp99) = (median(fire.clone()), p99(fire.clone()));
    println!("20 players mashing fire: World::step median {fmed:.3} ms, p99 {fp99:.3} ms");
    let json = format!(
        "{{\n  \"ballistics_400_projectiles_stage_median_ms\": {smed:.4},\n  \"ballistics_400_projectiles_stage_p99_ms\": {sp99:.4},\n  \"world_step_20_players_with_400_projectiles_median_ms\": {m:.4},\n  \"world_step_20_players_no_projectiles_median_ms\": {b:.4},\n  \"world_step_20_players_firing_median_ms\": {fmed:.4},\n  \"world_step_20_players_firing_p99_ms\": {fp99:.4},\n  \"budget_ballistics_stage_ms\": 2.5,\n  \"note\": \"M0-012, this machine (not the reference machine), release, enhanced-determinism, plain Instant harness\"\n}}\n"
    );
    if std::env::var_os("FRONTLINE_NO_BASELINE").is_none() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reports/baselines");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("sim-m0-weapon.json"), json);
    }
}
