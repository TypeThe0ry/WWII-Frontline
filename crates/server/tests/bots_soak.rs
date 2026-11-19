//! M0-026 bots: soak, determinism and statistics, in process.
//!
//! * `bots_smoke_20s` / `bots_same_seed_same_inputs` run everywhere (terrain required).
//! * `bots_soak_5min` is the acceptance run:
//!   `cargo test --release -p frontline-server --test bots_soak -- --ignored --nocapture`
//!   (env `FRONTLINE_SOAK_SECS` shortens it, `FRONTLINE_SOAK_BOTS` changes the count).
//!
//! The loop runs with a stepping clock (as fast as the CPU allows); every tick is the real
//! `SimLoop::tick` including bot think, input validation and `World::step`.

use std::cell::Cell;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use frontline_protocol::msg::c0::{ClientMsgC0, Report};
use frontline_protocol::msg::input::{buttons, InputBundle, InputCmd};
use frontline_server::clock::Clock;
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::{Inbound, InboundQueue, Sessions};
use frontline_server::session::yaw_code;
use frontline_server::shutdown::StopFlag;
use frontline_server::sim_bridge::NetLink;
use frontline_server::sim_thread::{build_loop_with_bots, Command, EventTally, SimThreadConfig};
use frontline_server::world_loader::{self, LoadedMap};
use frontline_sim::rules::ObjectiveSpec;
use frontline_sim::{GameMode, LifeState, MapId, Team, Vec3};

struct StepClock {
    base: Instant,
    off_ns: Cell<u64>,
}

impl StepClock {
    fn new() -> Self {
        StepClock {
            base: Instant::now(),
            off_ns: Cell::new(0),
        }
    }
}

impl Clock for StepClock {
    fn now(&self) -> Instant {
        self.base + Duration::from_nanos(self.off_ns.get())
    }
    fn sleep_until(&self, deadline: Instant) {
        let t = deadline.duration_since(self.base).as_nanos() as u64;
        if t > self.off_ns.get() {
            self.off_ns.set(t);
        }
    }
}

fn terrain() -> Option<Arc<LoadedMap>> {
    static MAP: OnceLock<Option<Arc<LoadedMap>>> = OnceLock::new();
    MAP.get_or_init(|| {
        match world_loader::locate_data_dir(None, "m0")
            .and_then(|d| world_loader::load_map(&d, "m0"))
        {
            Ok(m) => Some(Arc::new(m)),
            Err(e) => {
                if std::env::var_os("FRONTLINE_REQUIRE_TERRAIN").is_some() {
                    panic!("terrain required but unavailable: {e}");
                }
                eprintln!("SKIPPED (no terrain): {e}");
                None
            }
        }
    })
    .clone()
}

#[derive(Debug, Default)]
struct Soak {
    ticks: u32,
    tally: EventTally,
    /// Per-tick hash of all bot commands.
    hashes: Vec<u64>,
    /// `World::state_hash` every 100 ticks.
    state_hashes: Vec<u64>,
    out_of_bounds: u32,
    stuck_events: u64,
    /// Bot think wall time per tick (ns), path time per tick (ns).
    think_ns: Vec<u64>,
    path_ns: Vec<u64>,
    tick_ms: Vec<f64>,
    /// Longest stretch (in ticks) in which an alive bot moved less than 1 m over 150 ticks.
    idle_windows: u32,
    bots_alive_min: usize,
    rejected_total: u64,
    server_bots_gauge: i64,
}

fn run_soak(bots: u16, ticks: u32, seed: u64, map: Arc<LoadedMap>) -> Soak {
    let metrics = Metrics::new();
    let mut cfg = SimThreadConfig::bare(seed, MapId(0), GameMode::Sandbox);
    cfg.map_data = Some(map.clone());
    let (mut lp, _tx, _ctx) = build_loop_with_bots(
        StepClock::new(),
        &cfg,
        bots,
        metrics.clone(),
        StopFlag::new(),
    );
    lp.bot_host_mut().expect("bots").record_hashes(true);
    let mut out = Soak {
        bots_alive_min: usize::MAX,
        server_bots_gauge: metrics.bots.get(),
        ..Default::default()
    };
    let mut anchors: std::collections::BTreeMap<u8, ([f32; 2], u32)> = Default::default();
    for t in 0..ticks {
        let t0 = Instant::now();
        lp.tick();
        out.tick_ms.push(t0.elapsed().as_secs_f64() * 1e3);
        let view = lp.world().view();
        if t % 30 == 0 {
            out.bots_alive_min = out.bots_alive_min.min(view.players().len());
            for p in view.players() {
                let ok = p.pos.is_finite()
                    && (0.0..=map.size_m).contains(&p.pos.x)
                    && (0.0..=map.size_m).contains(&p.pos.z);
                if !ok {
                    out.out_of_bounds += 1;
                }
            }
        }
        if t % 150 == 0 {
            for p in view.players() {
                let a = anchors.entry(p.player).or_insert(([p.pos.x, p.pos.z], t));
                let moved = ((p.pos.x - a.0[0]).powi(2) + (p.pos.z - a.0[1]).powi(2)).sqrt();
                if t > a.1 && moved < 1.0 {
                    out.idle_windows += 1;
                }
                *a = ([p.pos.x, p.pos.z], t);
            }
        }
        if t % 100 == 0 {
            out.state_hashes.push(lp.world().state_hash());
        }
    }
    out.ticks = ticks;
    out.tally = lp.tally();
    let host = lp.bot_host().expect("bots");
    out.hashes = host.hashes().to_vec();
    out.stuck_events = host.stuck_total();
    out.think_ns = host.timings().iter().map(|t| t.total_ns).collect();
    out.path_ns = host.timings().iter().map(|t| t.path_ns).collect();
    out.rejected_total = frontline_server::validate::Reject::LABELS
        .iter()
        .map(|l| metrics.input_rejected_total.with_label_values(&[l]).get())
        .sum();
    out
}

fn pct(v: &mut [f64], q: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[(((v.len() - 1) as f64) * q).ceil() as usize]
}

fn report(label: &str, s: &Soak) {
    let mut think: Vec<f64> = s.think_ns.iter().map(|n| *n as f64 / 1e6).collect();
    let mut path: Vec<f64> = s.path_ns.iter().map(|n| *n as f64 / 1e6).collect();
    let mut tick = s.tick_ms.clone();
    let hit = if s.tally.shots > 0 {
        100.0 * s.tally.hits as f64 / s.tally.shots as f64
    } else {
        0.0
    };
    eprintln!(
        "[{label}] {} ticks ({:.0} s sim): shots {} hits {} ({hit:.1}% of shots) kills {} (headshots {}) spawns {}",
        s.ticks, s.ticks as f64 / 30.0, s.tally.shots, s.tally.hits, s.tally.kills, s.tally.headshots, s.tally.spawns
    );
    eprintln!(
        "[{label}] out_of_bounds {} stuck_events {} idle_windows(5 s, <1 m) {} min_alive {} rejected_inputs {}",
        s.out_of_bounds, s.stuck_events, s.idle_windows, s.bots_alive_min, s.rejected_total
    );
    eprintln!(
        "[{label}] bot step ms: P50 {:.3} P99 {:.3} max {:.3} | path ms: P50 {:.4} P99 {:.4} max {:.4} | server tick ms: P50 {:.3} P99 {:.3} max {:.3}{}",
        pct(&mut think, 0.5), pct(&mut think, 0.99), think.last().copied().unwrap_or(0.0),
        pct(&mut path, 0.5), pct(&mut path, 0.99), path.last().copied().unwrap_or(0.0),
        pct(&mut tick, 0.5), pct(&mut tick, 0.99), tick.last().copied().unwrap_or(0.0),
        if cfg!(debug_assertions) { " (debug build)" } else { " (release build)" }
    );
}

#[test]
fn bots_smoke_20s() {
    let Some(map) = terrain() else { return };
    let s = run_soak(10, 600, 7, map);
    report("smoke", &s);
    assert_eq!(
        s.server_bots_gauge, 10,
        "server_bots must equal the configured count"
    );
    assert_eq!(s.out_of_bounds, 0);
    assert_eq!(
        s.rejected_total, 0,
        "bots produced inputs the validator refused"
    );
}

#[test]
fn bots_same_seed_same_inputs() {
    let Some(map) = terrain() else { return };
    let a = run_soak(10, 5000, 42, map.clone());
    let b = run_soak(10, 5000, 42, map.clone());
    assert_eq!(a.hashes.len(), 5000);
    assert_eq!(
        a.hashes, b.hashes,
        "per-tick bot input hashes differ between identical runs"
    );
    assert_eq!(
        a.state_hashes, b.state_hashes,
        "world state differs between identical runs"
    );
    assert_eq!(a.tally, b.tally);
    // A different seed must change something (the hash is not constant).
    let c = run_soak(10, 600, 43, map);
    assert_ne!(
        a.hashes[..600],
        c.hashes[..],
        "different match seed, same bot behaviour"
    );
    eprintln!(
        "determinism: 5000 ticks x 2 runs identical (kills {} shots {}); last bot-input hash {:016x}",
        a.tally.kills, a.tally.shots, a.hashes[4999]
    );
}

#[test]
#[ignore = "5 minute soak; run with --ignored (release recommended)"]
fn bots_soak_5min() {
    let Some(map) = terrain() else { return };
    let secs: u32 = std::env::var("FRONTLINE_SOAK_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    let bots: u16 = std::env::var("FRONTLINE_SOAK_BOTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let s = run_soak(bots, secs * 30, 1, map);
    report(&format!("soak {bots} bots"), &s);
    assert_eq!(s.server_bots_gauge, i64::from(bots));
    assert_eq!(s.out_of_bounds, 0, "bots left the map");
    if bots >= 10 && secs >= 300 {
        assert!(s.tally.kills >= 10, "kills {} < 10", s.tally.kills);
        // Every death is followed by a respawn except those still waiting at the end.
        assert!(
            s.tally.spawns + u64::from(bots) >= s.tally.kills,
            "spawns {} + in-flight {} < kills {}",
            s.tally.spawns,
            bots,
            s.tally.kills
        );
    }
}

/// Two scripted humans enter through the real Join/C0 command queue and C1 bundle queue.
/// No sockets, handshake, or snapshot clients are started by this test.
fn tactical_soak(mut map: LoadedMap, ticks: u32, mixed: bool) -> EventTally {
    let metrics = Metrics::new();
    let net = InboundQueue::new();
    let mut cfg = SimThreadConfig::bare(39, MapId(0), GameMode::Sandbox);
    if mixed {
        let (x, z) = map.spawns.center;
        map.objectives = vec![ObjectiveSpec {
            id: 1,
            pos: Vec3::new(x, map.collision.height_at(x, z).unwrap(), z),
            radius_m: 25.0,
            owner: None,
        }];
    }
    assert!(
        !map.objectives.is_empty(),
        "tactical soak requires objectives"
    );
    let humans_per_team = u8::from(mixed);
    let bots_per_team = 10 - usize::from(humans_per_team);
    let label = if mixed { "M0 mixed" } else { "Carentan 10v10" };
    let size = map.size_m;
    cfg.map_data = Some(Arc::new(map));
    cfg.net = Some(NetLink {
        inbound: net.rx,
        sessions: Sessions::new(metrics.clone()),
    });
    let (mut lp, _, commands) =
        build_loop_with_bots(StepClock::new(), &cfg, 0, metrics.clone(), StopFlag::new());
    if mixed {
        for (player, team) in [(0, Team::Us), (1, Team::Ger)] {
            commands.send(Command::Join { player, team }).unwrap();
        }
    }
    commands.send(Command::SetBotFill(10)).unwrap();
    let start = Instant::now();
    let mut wall_ticks = Vec::with_capacity(ticks as usize);
    let mut closest = f32::INFINITY;
    for tick in 1..=ticks {
        for player in 0..2 * humans_per_team {
            let yaw = lp
                .world()
                .player_state(player)
                .map_or(0, |p| yaw_code(p.yaw));
            let cmd = InputCmd {
                seq: tick as u16,
                tick,
                yaw,
                move_x: 0,
                move_z: if tick % 120 < 30 { 40 } else { 0 },
                pitch: 0,
                buttons: if tick.is_multiple_of(19) {
                    buttons::FIRE
                } else {
                    0
                },
                weapon_slot: 0,
                aux: 0,
                view_tick: 0,
            };
            net.tx
                .send(Inbound {
                    player,
                    bundle: InputBundle {
                        ack_snapshot_tick: 0,
                        pred_hash: 0,
                        cmds: std::iter::once(cmd).collect(),
                        fire: Default::default(),
                    },
                })
                .unwrap();
        }
        // Benign synthetic C0 traffic; leaves deploy metrics bot-only.
        if mixed && tick == 120 {
            commands
                .send(Command::C0 {
                    player: 0,
                    msg: ClientMsgC0::Report(Report {
                        target: 1,
                        reason: 0,
                        note: "scripted soak traffic".into(),
                    }),
                })
                .unwrap();
        }
        let tick_start = Instant::now();
        lp.tick();
        wall_ticks.push(tick_start.elapsed().as_secs_f64() * 1e3);
        if tick == 30 {
            for team in [Team::Us, Team::Ger] {
                assert_eq!(lp.bot_host().unwrap().count_team(team), bots_per_team);
                assert_eq!(
                    lp.world().squads().team_count(team) as usize,
                    bots_per_team + usize::from(humans_per_team),
                );
                if mixed {
                    let human = lp
                        .world()
                        .squads()
                        .entry(team as u8)
                        .expect("human roster row");
                    assert_eq!(human.team, team);
                    assert!(human.online);
                }
            }
            let world = lp.world();
            for id in lp.bot_host().unwrap().players() {
                assert_eq!(world.player_life(id), Some(LifeState::Alive));
                assert!(world.squads().entry(id).unwrap().loadout.is_some());
            }
            for role in ["officer", "medic", "support"] {
                assert!(
                    world.squads().roster().any(|(_, e)| e.role == Some(role)),
                    "missing {role}"
                );
            }
        }
        if tick.is_multiple_of(30) {
            let view = lp.world().view();
            for a in view.players().iter().filter(|p| p.team == Team::Us) {
                for b in view.players().iter().filter(|p| p.team == Team::Ger) {
                    closest = closest.min(frontline_sim::supply::dist_xz(a.pos, b.pos));
                }
            }
            assert!(
                view.players().iter().all(|p| {
                    p.pos.is_finite()
                        && (0.0..=size).contains(&p.pos.x)
                        && (0.0..=size).contains(&p.pos.z)
                }),
                "a player left the map at {tick}"
            );
        }
    }
    let wall = start.elapsed().as_secs_f64();
    let tally = lp.tally();
    let accepted = metrics
        .deploy_requests_total
        .with_label_values(&["ok"])
        .get();
    let rejected = metrics
        .deploy_requests_total
        .with_label_values(&["rejected"])
        .get();
    let kicks = metrics.session_kicked_total.get();
    let bot_host = lp.bot_host().unwrap();
    let mut think: Vec<_> = bot_host
        .timings()
        .iter()
        .map(|s| s.total_ns as f64 / 1e6)
        .collect();
    eprintln!("M1-039c {label}: {ticks} ticks ({:.0} s sim), {wall:.3} s wall, tick P99 {:.3} ms, host P95 {:.3} ms / P99 {:.3} ms, {} C1 client bundles sent, {} C0 handled, nearest opponents {closest:.1} m, shots {}, kills {}, respawns {}, deploy ok {accepted} rejected {rejected}, kicks {kicks}",
        f64::from(ticks) / 30.0, pct(&mut wall_ticks, 0.99), pct(&mut think, 0.95), pct(&mut think, 0.99), u32::from(2 * humans_per_team) * ticks,
        metrics.reports_total.get(), tally.shots, tally.kills, tally.spawns);
    assert_eq!(bot_host.len(), 2 * bots_per_team);
    for team in ["us", "ger"] {
        assert_eq!(
            metrics.bots_active.with_label_values(&[team]).get(),
            bots_per_team as i64
        );
        assert_eq!(
            metrics.players.with_label_values(&[team]).get(),
            i64::from(humans_per_team)
        );
    }
    assert_eq!(metrics.reports_total.get(), u64::from(humans_per_team));
    assert_eq!(
        metrics
            .input_rejected_total
            .with_label_values(&["no_session"])
            .get(),
        0
    );
    if mixed {
        assert!(tally.shots > 0, "no simulated combat traffic");
    }
    assert!(accepted >= 2 * bots_per_team as u64 && 100 * rejected < accepted + rejected);
    assert_eq!(kicks, 0);
    tally
}

#[test]
fn fill_20_players() {
    let Some(map) = terrain() else { return };
    tactical_soak((*map).clone(), 120, true);
}

#[test]
#[ignore = "measured 300-s tactical run; release recommended"]
fn tactical_mixed_soak_5min() {
    let map = terrain().expect("M0 terrain fixture is required for tactical soak");
    tactical_soak((*map).clone(), 9_000, true);
}

#[test]
#[ignore = "measured 300-s tactical run; requires raw Carentan collision fixture"]
fn tactical_carentan_10v10_5min() {
    let mut map = world_loader::locate_data_dir(None, "carentan/m1")
        .and_then(|d| world_loader::load_map(&d, "carentan/m1"))
        .expect("Carentan M1 fixture must contain raw c_*.bin collision chunks");
    // With all objectives active, the two sides can stay at separate near-HQ objectives.
    // Use one authored objective so the scenario actually tests contested movement.
    let [us, ger] = map.spawns.hq;
    let center = Vec3::new((us.x + ger.x) * 0.5, 0.0, (us.z + ger.z) * 0.5);
    let objective = map
        .objectives
        .iter()
        .min_by(|a, b| {
            frontline_sim::supply::dist_xz(a.pos, center)
                .total_cmp(&frontline_sim::supply::dist_xz(b.pos, center))
        })
        .copied()
        .expect("Carentan objective data");
    eprintln!(
        "Carentan soak objective {} at {:?}",
        objective.id, objective.pos
    );
    map.objectives = vec![objective];
    let tally = tactical_soak(map, 9_000, false);
    assert!(
        tally.kills > 0,
        "Carentan 10v10 did not demonstrate combat kills"
    );
}
