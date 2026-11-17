//! Acceptance 4: skeleton overhead with 100 player entities. Runs the real loop (real clock)
//! for 150 ticks (5 s) and checks `server_tick_ms` P99 < 1 ms. The host is a shared VM, so a
//! descheduling spike can push P99 over the line; up to 3 independent attempts are allowed
//! (a real regression fails all of them). Tick 0 (which also spawns the players) is warm-up.

use frontline_server::clock::SystemClock;
use frontline_server::metrics::Metrics;
use frontline_server::shutdown::StopFlag;
use frontline_server::sim_thread::{build_loop, Command, SimThreadConfig};
use frontline_sim::{GameMode, MapId, SpawnReq, Team, Vec3};

fn attempt() -> (f64, f64, u64) {
    let metrics = Metrics::new();
    let cfg = SimThreadConfig::bare(1, MapId(0), GameMode::Sandbox);
    let (mut lp, _inputs, cmds) = build_loop(SystemClock, &cfg, metrics, StopFlag::new());
    for i in 0..100u8 {
        cmds.send(Command::SpawnPlayer {
            req: SpawnReq {
                player: i,
                team: if i % 2 == 0 { Team::Us } else { Team::Ger },
                pos: Vec3::new(i as f32, 0.0, 0.0),
                yaw: 0.0,
            },
        })
        .unwrap();
    }
    lp.tick(); // warm-up: spawns happen here
    lp.discard_samples();
    for _ in 0..150 {
        lp.tick();
    }
    let r = lp.report();
    eprintln!(
        "100 players: ticks={} tick_ms p50={:.4} p99={:.4} max={:.4}",
        r.ticks, r.tick_ms.p50, r.tick_ms.p99, r.tick_ms.max
    );
    (r.tick_ms.p99, r.tick_ms.p50, r.tick_overrun_total)
}

#[test]
fn hundred_players_empty_world_p99_under_1ms() {
    let mut best = f64::MAX;
    for _ in 0..3 {
        let (p99, _p50, _overruns) = attempt();
        best = best.min(p99);
        if p99 < 1.0 {
            return;
        }
    }
    panic!("P99 never below 1 ms (best {best} ms)");
}
