//! M0-018 acceptance 4: the session flow end to end. Real WebSocket + WebTransport listeners feed
//! a real `SimLoop` that the test drives tick by tick (lock-step, deterministic): join, team
//! balance, input -> position equality with an offline `World`, graded input disconnect.
//! Needs the generated terrain (see DEVELOPING.md); skipped with a message when absent
//! (`FRONTLINE_REQUIRE_TERRAIN=1` makes that a failure).

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use common::sim_harness::*;
use common::*;
use frontline_protocol::msg::c0::{decode_c0_server, Disconnect, ServerMsgC0};
use frontline_protocol::msg::input::InputBundle;
use frontline_protocol::Channel;
use frontline_server::anticheat::DisconnectReason;
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::{InboundQueue, Sessions};
use frontline_server::shutdown::StopFlag;
use frontline_server::sim_bridge::NetLink;
use frontline_server::sim_thread::{build_loop, SimLoop, SimThreadConfig};
use frontline_server::world_loader::LoadedMap;
use frontline_sim::{GameData, GameMode, MapId, PlayerId, SimConfig, SpawnReq, Team, World};

const SEED: u64 = 0x5EED;

struct Rig {
    lp: SimLoop<StepClock>,
    srv: TestServer,
    map: Arc<LoadedMap>,
}

async fn rig(wt: bool, reuse_delay: Duration) -> Option<Rig> {
    let map = terrain()?;
    let metrics = Metrics::new();
    let sessions = Sessions::with_reuse_delay(
        metrics.clone(),
        128,
        frontline_server::net::limits::MAX_CONNECTIONS,
        reuse_delay,
    );
    let inbound = InboundQueue::new();
    let cfg = SimThreadConfig {
        map_data: Some(map.clone()),
        net: Some(NetLink {
            inbound: inbound.rx.clone(),
            sessions: sessions.clone(),
        }),
        ..SimThreadConfig::bare(SEED, MapId(0), GameMode::Sandbox)
    };
    let (lp, _inputs, commands) =
        build_loop(StepClock::new(), &cfg, metrics.clone(), StopFlag::new());
    let srv = start_listeners(metrics, sessions, &inbound, commands, wt).await;
    Some(Rig { lp, srv, map })
}

fn tick(lp: &mut SimLoop<StepClock>) {
    tokio::task::block_in_place(|| lp.tick());
}

fn mirror(map: &LoadedMap, players: &[(PlayerId, Team)]) -> World {
    let cfg = SimConfig {
        tick_hz: 30,
        match_seed: SEED,
        map: MapId(0),
        mode: GameMode::Sandbox,
        data: GameData::embedded(),
    };
    let mut w = World::new(cfg, &map.collision);
    map.apply_to(&mut w);
    for &(p, t) in players {
        let (pos, yaw) = map.join_spawn(p, t);
        w.spawn_player(SpawnReq {
            player: p,
            team: t,
            pos,
            yaw,
        })
        .unwrap();
    }
    w
}

async fn wait_c1(m: &Metrics, n: u64) {
    let ok = wait_until(Duration::from_secs(10), || {
        m.net_c1_inbound_total.get() >= n
    })
    .await;
    assert!(
        ok,
        "only {} of {n} bundles arrived",
        m.net_c1_inbound_total.get()
    );
}

fn rejected(m: &Metrics, reason: &str) -> u64 {
    m.input_rejected_total.with_label_values(&[reason]).get()
}

fn team_of(t: u8) -> Team {
    if t == 0 {
        Team::Us
    } else {
        Team::Ger
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_ws_two_wt_clients_join_balanced_move_like_the_offline_world_and_leave() {
    let Some(mut r) = rig(true, Duration::ZERO).await else {
        return;
    };
    let m = r.srv.metrics.clone();

    // ---- handshakes: WS, WT, WS, WT ----
    let kinds = [Kind::Ws, Kind::Wt, Kind::Ws, Kind::Wt];
    let mut clients = Vec::new();
    let mut joined: Vec<(PlayerId, Team)> = Vec::new();
    for k in kinds {
        let (c, w) = Conn::handshake(r.srv.target(k)).await;
        joined.push((w.player_id, team_of(w.team)));
        clients.push(c);
    }
    let ids: Vec<PlayerId> = joined.iter().map(|j| j.0).collect();
    assert_eq!(ids, vec![0, 1, 2, 3]);
    let teams: Vec<u8> = joined.iter().map(|j| j.1 as u8).collect();
    assert_eq!(teams, vec![0, 1, 0, 1], "teams alternate US, GER, US, GER");
    assert_eq!(r.srv.sessions.team_counts(), [2, 2]);

    // Pre-send ticks 1..=3 so every player has real input from the very first tick.
    let mut sent = 0u64;
    for k in 1..=3u32 {
        for (i, c) in clients.iter_mut().enumerate() {
            c.send(Channel::C1, &encode(&bundle_upto(ids[i], k))).await;
            sent += 1;
        }
    }
    wait_c1(&m, sent).await;

    // ---- 90 ticks (3 s) of lock-step play ----
    const N: u32 = 90;
    let mut off = mirror(&r.map, &joined);
    let mag0 = off.weapon_state(0).unwrap().mag;
    for t in 1..=N {
        for (i, c) in clients.iter_mut().enumerate() {
            c.send(Channel::C1, &encode(&bundle_upto(ids[i], t + 3)))
                .await;
            sent += 1;
        }
        wait_c1(&m, sent).await;
        tick(&mut r.lp);
        if t == 1 {
            // Everybody is in the world, on their own team, at the HQ.
            let view = r.lp.world().view();
            assert_eq!(view.players().len(), 4);
            for pv in view.players() {
                assert_eq!(pv.team, joined[pv.player as usize].1);
            }
            assert_eq!(m.players.with_label_values(&["us"]).get(), 2);
            assert_eq!(m.players.with_label_values(&["ger"]).get(), 2);
        }
        let inputs: Vec<_> = ids.iter().map(|&p| (p, gen_cmd(p, t))).collect();
        off.step(&inputs);
    }
    assert_eq!(r.lp.world().tick().0, N);

    // ---- server == offline, bit for bit; and they actually moved ----
    let mut moved = 0;
    for &p in &ids {
        let a = r.lp.world().player_state(p).unwrap();
        let b = off.player_state(p).unwrap();
        assert_eq!(
            a.to_bytes(),
            b.to_bytes(),
            "player {p} movement state differs"
        );
        assert_eq!(
            r.lp.world().weapon_state(p).unwrap().to_bytes(),
            off.weapon_state(p).unwrap().to_bytes(),
            "player {p} weapon state differs"
        );
        assert_eq!(r.lp.world().health(p), off.health(p));
        assert!(!a.blocked, "player {p} is stuck outside the terrain");
        let (spawn, _) = r.map.join_spawn(p, joined[p as usize].1);
        let d = ((a.pos.x - spawn.x).powi(2) + (a.pos.z - spawn.z).powi(2)).sqrt();
        eprintln!(
            "player {p}: moved {d:.1} m from the join point, pos {:?}",
            a.pos
        );
        if d > 3.0 {
            moved += 1;
        }
    }
    assert_eq!(moved, 4, "terrain must be loaded: every player walks");
    // Player 0 (even) fired in ticks 20..40 of the pattern: ammo went down identically.
    assert!(
        r.lp.world().weapon_state(0).unwrap().mag < mag0,
        "player 0 fired"
    );
    // Redundancy produced only benign duplicates; nothing was clamped or refused otherwise.
    assert!(rejected(&m, "duplicate") > 0);
    for reason in [
        "reserved_bits",
        "weapon_slot",
        "move_clamped",
        "yaw_rate",
        "pitch_rate",
        "too_far",
        "too_old",
        "tick_regress",
        "ring_overwrite",
        "no_session",
    ] {
        assert_eq!(rejected(&m, reason), 0, "{reason}");
    }
    assert_eq!(m.input_missing_ticks_total.get(), 0);
    assert_eq!(m.input_queue_full_total.get(), 0);

    // ---- disconnect: entities vanish, gauges fall ----
    drop(clients);
    let t0 = Instant::now();
    loop {
        tick(&mut r.lp);
        if r.lp.world().view().players().is_empty() {
            break;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(8),
            "players were not removed"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(m.players.with_label_values(&["us"]).get(), 0);
    assert_eq!(m.players.with_label_values(&["ger"]).get(), 0);
    assert_eq!(r.srv.sessions.team_counts(), [0, 0]);
    for p in 0..4 {
        assert!(r.lp.world().player_state(p).is_none());
    }
    // Late ClientGone / stray input for a removed player is harmless.
    tick(&mut r.lp);
    r.srv.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn nine_invalid_input_ticks_send_disconnect_and_remove_the_player() {
    let Some(mut r) = rig(false, Duration::ZERO).await else {
        return;
    };
    let m = r.srv.metrics.clone();
    let (mut c, w) = Conn::handshake(r.srv.target(Kind::Ws)).await;
    tick(&mut r.lp); // join
    assert!(r.lp.world().player_state(w.player_id).is_some());
    let mut seq = 0u16;
    for step in 1..=9u64 {
        let now = r.lp.world().tick().0 + 1;
        let cmds = (0..3u32)
            .map(|j| {
                seq += 1;
                let mut c = gen_cmd(w.player_id, now + j);
                c.seq = seq;
                c.buttons |= 1 << 14; // reserved bit
                c
            })
            .collect();
        let b = InputBundle {
            ack_snapshot_tick: 0,
            pred_hash: 0,
            cmds,
            fire: Default::default(),
        };
        c.send(Channel::C1, &encode(&b)).await;
        wait_c1(&m, step).await;
        tick(&mut r.lp);
        assert_eq!(m.session_kicked_total.get(), u64::from(step == 9));
    }
    assert_eq!(rejected(&m, "reserved_bits"), 27);
    let msg = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match c.recv_within(Duration::from_secs(5)).await {
                Got::Frame(Channel::C0, bytes) => break decode_c0_server(&bytes).unwrap(),
                Got::Frame(..) => {}
                Got::Closed => panic!("connection closed before C0 Disconnect"),
                Got::Timeout => panic!("C0 Disconnect timeout"),
            }
        }
    })
    .await
    .expect("C0 Disconnect timeout");
    assert_eq!(
        msg,
        ServerMsgC0::Disconnect(Disconnect {
            reason: DisconnectReason::InvalidInputStreak as u8,
        })
    );
    assert_eq!(m.session_kicked_total.get(), 1);
    assert!(
        c.expect_closed(Duration::from_secs(5)).await,
        "connection must be closed by the server"
    );
    assert!(r.lp.world().player_state(w.player_id).is_none());
    assert_eq!(m.players.with_label_values(&["us"]).get(), 0);
    // The disconnect's ClientGone arrives afterwards and is a no-op.
    let t0 = Instant::now();
    while r.srv.sessions.connections() > 0 && t0.elapsed() < Duration::from_secs(5) {
        tick(&mut r.lp);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    tick(&mut r.lp);
    assert_eq!(m.session_kicked_total.get(), 1);
    r.srv.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn replays_stale_and_far_commands_are_counted_per_reason() {
    let Some(mut r) = rig(false, Duration::ZERO).await else {
        return;
    };
    let m = r.srv.metrics.clone();
    let (mut c, w) = Conn::handshake(r.srv.target(Kind::Ws)).await;
    tick(&mut r.lp);
    for _ in 0..20 {
        tick(&mut r.lp); // now = 22
    }
    let p = w.player_id;
    let send = |cmd: frontline_protocol::msg::input::InputCmd| InputBundle {
        ack_snapshot_tick: 0,
        pred_hash: 0,
        cmds: [cmd].into_iter().collect(),
        fire: Default::default(),
    };
    let mut n = 0u64;
    let mut cmd = gen_cmd(p, 22);
    cmd.seq = 100;
    for (cmd, _why) in [
        (cmd, "accepted"),
        (cmd, "same seq again -> duplicate"),
        (
            {
                let mut x = gen_cmd(p, 5);
                x.seq = 101;
                x
            },
            "too old",
        ),
        (
            {
                let mut x = gen_cmd(p, 90);
                x.seq = 102;
                x
            },
            "too far",
        ),
        (
            {
                let mut x = gen_cmd(p, 23);
                x.seq = 99;
                x
            },
            "seq went backwards -> duplicate",
        ),
        (
            {
                let mut x = gen_cmd(p, 22);
                x.seq = 103;
                x
            },
            "tick already buffered -> duplicate",
        ),
        (
            {
                let mut x = gen_cmd(p, 23);
                x.seq = 104;
                x.weapon_slot = 9;
                x
            },
            "weapon slot",
        ),
        (
            {
                let mut x = gen_cmd(p, 24);
                x.seq = 105;
                x.move_x = 127;
                x.move_z = 127;
                x
            },
            "clamped but accepted",
        ),
    ] {
        c.send(Channel::C1, &encode(&send(cmd))).await;
        n += 1;
        wait_c1(&m, n).await;
    }
    tick(&mut r.lp);
    assert_eq!(rejected(&m, "duplicate"), 3);
    assert_eq!(rejected(&m, "too_old"), 1);
    assert_eq!(rejected(&m, "too_far"), 1);
    assert_eq!(rejected(&m, "weapon_slot"), 1);
    assert_eq!(rejected(&m, "move_clamped"), 1);
    assert_eq!(m.input_accepted_total.get(), 2); // seq 100 and the clamped one
    r.srv.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ping_flood_is_rate_limited_but_the_connection_survives() {
    let Some(r) = rig(false, Duration::ZERO).await else {
        return;
    };
    let m = r.srv.metrics.clone();
    let (mut c, _) = Conn::handshake(r.srv.target(Kind::Ws)).await;
    for i in 0..30u32 {
        assert!(c.send(Channel::C0, &ping_msg(i)).await);
    }
    let mut pongs = 0;
    while let Some(ServerMsgC0::Pong(_)) = c.server_msg(Duration::from_millis(400)).await {
        pongs += 1;
    }
    assert!((5..=8).contains(&pongs), "{pongs} pongs for a burst of 30");
    assert!(rejected(&m, "c0_rate") >= 22);
    // Budget refills: a later ping is answered.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    c.send(Channel::C0, &ping_msg(99)).await;
    assert!(matches!(
        c.server_msg(Duration::from_secs(2)).await,
        Some(ServerMsgC0::Pong(_))
    ));
    drop(c);
    r.srv.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn player_id_is_not_reused_within_one_second_of_a_disconnect() {
    let Some(mut r) = rig(false, frontline_server::net::conn::ID_REUSE_DELAY).await else {
        return;
    };
    let (c, w) = Conn::handshake(r.srv.target(Kind::Ws)).await;
    assert_eq!(w.player_id, 0);
    tick(&mut r.lp);
    drop(c);
    let t_drop = Instant::now();
    assert!(
        wait_until(Duration::from_secs(5), || r.srv.sessions.team_counts()
            == [0, 0])
        .await
    );
    let (c2, w2) = Conn::handshake(r.srv.target(Kind::Ws)).await;
    assert_ne!(w2.player_id, 0, "id 0 is cooling down");
    drop(c2);
    tokio::time::sleep(Duration::from_millis(1200).saturating_sub(t_drop.elapsed())).await;
    let (_c3, w3) = Conn::handshake(r.srv.target(Kind::Ws)).await;
    assert_eq!(w3.player_id, 0, "free again after the delay");
    let _ = r.lp.world();
    r.srv.stop().await;
}

#[test]
fn welcome_team_matches_the_team_counts_of_the_sessions() {
    // Pure bookkeeping check (no terrain needed): team assignment is by fewer players.
    let m = Metrics::new();
    let s = Sessions::with_reuse_delay(m.clone(), 4, 128, Duration::ZERO);
    let h = frontline_server::net::transport::ClientHandle::new(m);
    let teams: Vec<_> = (0..8)
        .map(|_| s.alloc_player_team(&h).unwrap().1 as u8)
        .collect();
    assert_eq!(teams, vec![0, 1, 0, 1, 0, 1, 0, 1]);
}
