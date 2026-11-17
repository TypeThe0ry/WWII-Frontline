//! Acceptance 4: skeleton overhead with 100 player entities. Runs the real loop (real clock)
//! for 5 s and checks `server_tick_ms` P99 < 1 ms.

use frontline_server::clock::SystemClock;
use frontline_server::metrics::Metrics;
use frontline_server::shutdown::StopFlag;
use frontline_server::sim_thread::{build_loop, Command, SimThreadConfig};
use frontline_sim::{GameMode, MapId, SpawnReq, Team, Vec3};

#[test]
fn hundred_players_empty_world_p99_under_1ms() {
    let metrics = Metrics::new();
    let stop = StopFlag::new();
    let cfg = SimThreadConfig {
        match_seed: 1,
        map: MapId(0),
        mode: GameMode::Sandbox,
        pin_core: None,
    };
    let (mut lp, _inputs, cmds) = build_loop(SystemClock, &cfg, metrics, stop);
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
    for _ in 0..150 {
        lp.tick();
    }
    let r = lp.report();
    eprintln!(
        "100 players: ticks={} tick_ms p50={:.4} p99={:.4} max={:.4}",
        r.ticks, r.tick_ms.p50, r.tick_ms.p99, r.tick_ms.max
    );
    assert_eq!(r.ticks, 150);
    assert!(r.tick_ms.p99 < 1.0, "P99 {} ms", r.tick_ms.p99);
    assert_eq!(r.tick_overrun_total, 0);
}
