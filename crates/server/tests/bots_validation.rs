//! M0-026 acceptance 3 and 7: bot input takes the same validation path as a human's, bots are
//! named `[BOT]...`, the team split is even, `server_bots` equals the configured count, and the
//! `bots` crate cannot reach the server or a mutable world.

use frontline_bots::{BotBrain, BotCtx, BrainStats, Difficulty};
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_server::bot_host::{bot_nickname, bot_slot, BotHost, BOT_NAME_PREFIX};
use frontline_server::metrics::Metrics;
use frontline_server::sim_bridge::SimBridge;
use frontline_server::validate::{Reject, ValidateCfg};
use frontline_sim::{CollisionData, GameData, GameMode, MapId, SimConfig, Team, World, WorldView};

/// A deliberately misbehaving bot: oversize move vector, reserved bits, `i8::MIN`, replayed seq.
struct Rogue {
    n: u32,
}

impl BotBrain for Rogue {
    fn think(&mut self, _view: &dyn WorldView, ctx: &BotCtx) -> InputCmd {
        self.n += 1;
        let mut c = InputCmd {
            seq: self.n as u16,
            tick: ctx.tick,
            move_x: 127,
            move_z: 127, // length 179.6 > 128: must be clamped
            yaw: 0,
            pitch: 0,
            buttons: buttons::FIRE | buttons::SPRINT,
            weapon_slot: 0,
            aux: 0,
            view_tick: 0,
        };
        match self.n % 4 {
            1 => {}
            2 => c.buttons |= 0x4000, // reserved bit 14: rejected
            3 => c.move_x = i8::MIN,  // not representable: rejected
            _ => c.seq = 1,           // replay of an old seq: rejected
        }
        c
    }
    fn take_stats(&mut self) -> BrainStats {
        BrainStats::default()
    }
}

fn world() -> World {
    World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 1,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        &CollisionData::empty(),
    )
}

fn count(m: &Metrics, r: Reject) -> u64 {
    m.input_rejected_total.with_label_values(&[r.label()]).get()
}

#[test]
fn bot_input_goes_through_validation() {
    let m = Metrics::new();
    let mut w = world();
    let mut bridge = SimBridge::new(m.clone(), None, None, ValidateCfg::default());
    bridge.join(&mut w, 5, Team::Us).unwrap();
    let mut host = BotHost::new(1, Difficulty::embedded_default(), m.clone());
    host.add(5, Team::Us, bot_nickname(0), Box::new(Rogue { n: 0 }));

    let mut max_len = 0.0f32;
    for _ in 0..40 {
        let now = w.tick().0 + 1;
        host.tick(&w, &mut bridge);
        let cmd = bridge.resolve(now)[0].1;
        max_len = max_len.max((cmd.move_x as f32).hypot(cmd.move_z as f32));
        assert_eq!(
            cmd.buttons & 0xC000,
            0,
            "reserved bits must never reach the sim"
        );
        assert_ne!(cmd.move_x, i8::MIN);
        w.step(&[(5, cmd)]);
    }
    // The oversize vector was scaled down, never passed on as is.
    assert!(
        max_len <= 127.0 + 1e-3,
        "move vector of length {max_len} reached the sim"
    );
    assert!(
        count(&m, Reject::MoveClamped) > 0,
        "MoveClamped not counted"
    );
    assert!(
        count(&m, Reject::ReservedBits) > 0,
        "ReservedBits not counted"
    );
    assert!(count(&m, Reject::BadMove) > 0, "BadMove not counted");
    assert!(count(&m, Reject::Duplicate) > 0, "replayed seq not counted");
    // A clean command from the same bot is accepted and counted as accepted.
    assert!(m.input_accepted_total.get() > 0);
}

/// The host holds `&World` only (compile-time: `BotHost::tick` takes `&World`), and the `bots`
/// crate depends on neither `server` nor anything that could hand it a mutable world.
#[test]
fn bots_crate_does_not_depend_on_server() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../bots/Cargo.toml"))
            .unwrap();
    assert!(
        !manifest.contains("frontline-server"),
        "bots must not depend on server"
    );
    let deps: Vec<&str> = manifest
        .lines()
        .filter(|l| l.starts_with("frontline-"))
        .collect();
    assert_eq!(deps.len(), 2, "{deps:?}");
    // Source-level check: the bots crate never names a mutable world API.
    let src = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../bots/src")).unwrap();
    for f in src.filter_map(|e| e.ok()) {
        let text = std::fs::read_to_string(f.path()).unwrap();
        for forbidden in [
            "&mut World",
            "World::step",
            "spawn_player",
            "remove_player",
            "spawn_projectile",
        ] {
            assert!(
                !text.contains(forbidden),
                "{} mentions {forbidden}",
                f.path().display()
            );
        }
    }
    // And the host ingests only through the bridge's validating entry point.
    let host =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bot_host.rs")).unwrap();
    assert!(host.contains("bridge.ingest_cmd("));
    assert!(
        !host.contains("&mut World") || host.contains("fn with_soldiers"),
        "only the joiner may touch a mutable world"
    );
    assert!(!host.contains("world.step("));
}

#[test]
fn bots_are_named_split_evenly_and_counted() {
    let m = Metrics::new();
    let mut w = world();
    let mut bridge = SimBridge::new(m.clone(), None, None, ValidateCfg::default());
    let host = BotHost::with_soldiers(10, 7, &mut w, &mut bridge, None, None, m.clone());
    assert_eq!(host.len(), 10);
    assert_eq!(m.bots.get(), 10, "server_bots must equal --bots");
    assert_eq!(host.count_team(Team::Us), 5);
    assert_eq!(host.count_team(Team::Ger), 5);
    let mut ids: Vec<u8> = host.players().collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 10);
    for (i, p) in host.players().enumerate() {
        assert_eq!(bot_slot(i as u16).0, p);
        let n = host.nickname(p).unwrap();
        assert!(n.starts_with(BOT_NAME_PREFIX), "{n}");
        assert!(n.starts_with("[BOT]"));
    }
    assert_eq!(w.view().players().len(), 10);
    // Odd counts differ by at most one.
    let mut w2 = world();
    let mut b2 = SimBridge::new(m.clone(), None, None, ValidateCfg::default());
    let h2 = BotHost::with_soldiers(7, 7, &mut w2, &mut b2, None, None, m.clone());
    assert_eq!((h2.count_team(Team::Us), h2.count_team(Team::Ger)), (4, 3));
}
