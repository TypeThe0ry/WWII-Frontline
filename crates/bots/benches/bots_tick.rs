//! `cargo bench -p frontline-bots --bench bots_tick [-- bots_tick_100]`.
//! Criterion measures bot time only. The JSON is a fixed middle-of-match window of genuine
//! per-tick samples, not percentiles of Criterion's batched means.
//! Set `FRONTLINE_BOTS_BASELINE_PATH` for an exact path or `FRONTLINE_BASELINE_DIR` for a directory.

#[path = "../tests/scale_support/mod.rs"]
mod scale_support;

use std::path::{Path, PathBuf};
use std::time::Duration;

use criterion::Criterion;
use frontline_bots::scheduler::{BotRuntime, BotsMetrics};
use scale_support::{Battle, Terrain, HZ, SEED};
use serde_json::{json, Value};

const WARMUP: usize = 450;
const TICKS: usize = 900;
const BASELINE: &str = "bots_tick_baseline.json";

struct Window {
    battle: Battle,
    runtime: BotRuntime,
    ticks: usize,
    shots_at_start: usize,
    kills_at_start: usize,
    spawns_at_start: usize,
}

impl Window {
    fn new(terrain: &Terrain, bots: u8) -> Self {
        let mut runtime = BotRuntime::default();
        let mut battle = Battle::new(terrain, bots);
        for _ in 0..WARMUP {
            battle.step(Some(&mut runtime), |_, _, _| {});
        }
        Self {
            shots_at_start: battle.shots,
            kills_at_start: battle.kills,
            spawns_at_start: battle.spawns,
            battle,
            runtime,
            ticks: 0,
        }
    }

    fn sample(&mut self) -> BotsMetrics {
        self.ticks += 1;
        self.battle
            .step(Some(&mut self.runtime), |_, _, _| {})
            .unwrap()
    }
}

fn percentiles(values: impl Iterator<Item = u64>) -> Value {
    let mut sorted: Vec<_> = values.collect();
    sorted.sort_unstable();
    let n = sorted.len();
    let p50 = (sorted[(n - 1) / 2] as f64 + sorted[n / 2] as f64) * 0.5;
    let p99 = sorted[(99 * n).div_ceil(100) - 1];
    json!({"p50": p50 / 1e6, "p99": p99 as f64 / 1e6, "max": sorted[n - 1] as f64 / 1e6})
}

fn measure(c: &mut Criterion, terrain: &Terrain, bots: u8, report: bool) -> Option<Value> {
    let name = format!("bots_tick_{bots}");
    let mut window = None::<Window>;
    let mut fixed = Vec::with_capacity(TICKS);
    let mut combat = (0, 0, 0);
    c.bench_function(&name, |b| {
        b.iter_custom(|iters| {
            let mut ns = 0;
            for _ in 0..iters {
                if window.as_ref().is_none_or(|w| w.ticks == TICKS) {
                    window = Some(Window::new(terrain, bots));
                }
                let w = window.as_mut().unwrap();
                let m = w.sample();
                ns += m.total_ns;
                if report && fixed.len() < TICKS {
                    fixed.push(m);
                    combat = (
                        w.battle.shots - w.shots_at_start,
                        w.battle.kills - w.kills_at_start,
                        w.battle.spawns - w.spawns_at_start,
                    );
                }
            }
            Duration::from_nanos(ns)
        });
    });
    if !report || fixed.is_empty() {
        return None; // A filtered-out benchmark or Criterion smoke/list mode.
    }
    // `--quick` can finish estimation before a complete raw window has been captured.
    let w = window.as_mut().unwrap();
    while fixed.len() < TICKS {
        fixed.push(w.sample());
        combat = (
            w.battle.shots - w.shots_at_start,
            w.battle.kills - w.kills_at_start,
            w.battle.spawns - w.spawns_at_start,
        );
    }
    let total = percentiles(fixed.iter().map(|m| m.total_ns));
    let path = percentiles(fixed.iter().map(|m| m.path_ns));
    let p50_limit = if bots == 60 { 3.0 } else { 5.0 };
    println!(
        "{name} PI-109 workstation/non-reference: tick ms P50 {:.3}, P99 {:.3}, max {:.3} ({} measured ticks; {} shots)",
        total["p50"].as_f64().unwrap(),
        total["p99"].as_f64().unwrap(),
        total["max"].as_f64().unwrap(),
        fixed.len(),
        combat.0,
    );
    Some(json!({
        "benchmark": name,
        "bots": bots,
        "warmup_ticks": WARMUP,
        "measured_ticks": fixed.len(),
        "bots_tick_ms": total,
        "bots_path_ms": path,
        "bots_perception_ms": percentiles(fixed.iter().map(|m| m.perception_ns)),
        "bots_tactical_ms": percentiles(fixed.iter().map(|m| m.tactical_ns)),
        "deferred": fixed.iter().map(|m| m.deferred).sum::<u64>(),
        "timeouts": fixed.iter().map(|m| m.timeouts).sum::<u64>(),
        "stuck": fixed.iter().map(|m| m.stuck).sum::<u64>(),
        "shots": combat.0,
        "kills": combat.1,
        "spawns": combat.2,
        "limits_ms": {"p50": p50_limit, "p99": (bots == 100).then_some(8.0)},
    }))
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let report = args.iter().any(|a| a == "--bench")
        && !args
            .iter()
            .any(|a| a == "--test" || a == "--list" || a.starts_with("--profile-time"));
    let mut c = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2))
        .configure_from_args();
    let terrain = Terrain::load();
    let cases: Vec<_> = [60, 100]
        .into_iter()
        .filter_map(|n| measure(&mut c, &terrain, n, report))
        .collect();
    c.final_summary();
    let path = std::env::var_os("FRONTLINE_BOTS_BASELINE_PATH")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("FRONTLINE_BASELINE_DIR").map(|dir| PathBuf::from(dir).join(BASELINE))
        });
    if cases.is_empty() || (cases.len() < 2 && path.is_none()) {
        return; // A filtered run must not overwrite the canonical two-case baseline.
    }
    let path = path.unwrap_or_else(|| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../reports/bench")
            .join(BASELINE)
    });
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).expect("create baseline directory");
    }
    let data = json!({
        "schema_version": 1,
        "environment": "PI-109 workstation/non-reference (default label; confirm hardware separately)",
        "reference_machine": "unmeasured",
        "terrain": terrain.name,
        "seed": SEED,
        "tick_hz": HZ,
        "profile": if cfg!(debug_assertions) { "debug" } else { "optimized" },
        "difficulty": "legacy Regular",
        "timing": "BotRuntime::default, no sim step or server transport; P50 midpoint, P99 nearest rank",
        "cases": cases,
    });
    std::fs::write(&path, serde_json::to_string_pretty(&data).unwrap() + "\n")
        .expect("write baseline JSON");
    println!("wrote {}", path.display());
}
