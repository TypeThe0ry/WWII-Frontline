//! One controlled moving-target shot per match through the production WT C1/C2 path.
//! Default: unimpaired loopback. For the delayed arm, run this test executable in a private
//! network namespace with `tc qdisc add dev lo root netem delay 75ms` and set
//! `FRONTLINE_LAGCOMP_NETEM=1`. That arm measures RTT on C0 and also runs an identical
//! shot with the rewind request removed. This is not a hit-rate benchmark.

mod common;

use std::collections::HashSet;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::sim_harness::start_listeners;
use common::{Conn, Got, Kind, Target};
use frontline_protocol::msg::c0::{decode_c0_server, ServerMsgC0, Welcome};
use frontline_protocol::msg::input::{
    buttons, encode_input_bundle, FireCmd, InputBundle, InputCmd,
};
use frontline_protocol::snapshot::{Event, SnapshotDecoder};
use frontline_protocol::Channel;
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::{InboundQueue, Sessions};
use frontline_server::shutdown::StopFlag;
use frontline_server::sim_bridge::NetLink;
use frontline_server::sim_thread::{self, SimThreadConfig};
use frontline_server::world_loader::{load_map, LoadedMap};
use frontline_sim::{GameMode, MapId, Team, Vec3};

const TIMEOUT: Duration = Duration::from_secs(5);
const TRIALS: u64 = 4;

fn flat_map() -> Arc<LoadedMap> {
    let root = tempfile::tempdir().unwrap();
    let map = root.path().join("maps/lagcomp");
    fs::create_dir_all(map.join("chunks")).unwrap();
    let mut chunk = vec![0; 16 + 65 * 65 * 2 + 64 * 64];
    chunk[..8].copy_from_slice(b"FCOL\x01\x00\x41\x00");
    chunk[8..12].copy_from_slice(&5.0f32.to_le_bytes());
    chunk[12..16].copy_from_slice(&5.0f32.to_le_bytes());
    fs::write(map.join("chunks/c_0_0.bin"), chunk).unwrap();
    fs::write(map.join("meta.json"), "{}").unwrap();
    fs::write(
        map.join("spawns.json"),
        r#"{"hq":{"us":{"pos":[20,30]},"ger":{"pos":[39,27]}}}"#,
    )
    .unwrap();
    Arc::new(load_map(root.path(), "lagcomp").unwrap())
}

struct StopOnDrop(StopFlag);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.stop();
    }
}

async fn next_frame(conn: &mut Conn, end: Instant) -> (Channel, Vec<u8>) {
    match conn
        .recv_within(end.saturating_duration_since(Instant::now()))
        .await
    {
        Got::Frame(ch, bytes) => (ch, bytes),
        Got::Timeout => panic!("timed out waiting for a transport frame"),
        Got::Closed => panic!("connection closed waiting for a transport frame"),
    }
}

async fn connect(target: &Target) -> (Conn, Welcome) {
    let mut conn = Conn::connect(target).await.unwrap();
    assert!(conn.send(Channel::C0, &common::good_hello()).await);
    let end = Instant::now() + TIMEOUT;
    loop {
        let (ch, bytes) = next_frame(&mut conn, end).await;
        if ch == Channel::C0 {
            match decode_c0_server(&bytes).unwrap() {
                ServerMsgC0::Welcome(w) => return (conn, w),
                other => panic!("expected Welcome, got {other:?}"),
            }
        }
    }
}

async fn c0_rtt(conn: &mut Conn) -> Duration {
    let sent = Instant::now();
    assert!(conn.send(Channel::C0, &common::ping_msg(123)).await);
    loop {
        let (ch, bytes) = next_frame(conn, sent + TIMEOUT).await;
        if ch == Channel::C0 {
            match decode_c0_server(&bytes).unwrap() {
                ServerMsgC0::Pong(p) if p.client_time == 123 => return sent.elapsed(),
                other => panic!("expected matching Pong, got {other:?}"),
            }
        }
    }
}

fn cmd(tick: u32, buttons: u16) -> InputCmd {
    InputCmd {
        tick,
        seq: tick as u16,
        buttons,
        move_x: 0,
        move_z: 0,
        yaw: 0,
        pitch: 0,
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    }
}

async fn send_input(conn: &mut Conn, input: InputCmd, ack: u16) {
    send_input_with_fire(conn, input, ack, None).await;
}

async fn send_input_with_fire(conn: &mut Conn, input: InputCmd, ack: u16, fire: Option<FireCmd>) {
    let bundle = InputBundle {
        ack_snapshot_tick: ack,
        pred_hash: 0,
        cmds: [input].into_iter().collect(),
        fire: fire.into_iter().collect(),
    };
    assert!(
        conn.send(Channel::C1, &encode_input_bundle(&bundle).unwrap())
            .await
    );
}

async fn strafe(mut conn: Conn, metrics: Arc<Metrics>) {
    let mut last_tick = 0;
    // Four ticks of send lead keeps the same movement going through 75 ms one-way delay.
    loop {
        let tick = metrics.ticks_run() as u32 + 4;
        if tick > last_tick {
            send_input(
                &mut conn,
                InputCmd {
                    move_z: 127,
                    ..cmd(tick, buttons::SPRINT)
                },
                0,
            )
            .await;
            last_tick = tick;
        }
        tokio::time::sleep(Duration::from_millis(4)).await;
    }
}

async fn trial(map: &Arc<LoadedMap>, seed: u64, delayed: bool, rewind: bool) -> usize {
    let metrics = Metrics::new();
    let sessions = Sessions::with_limits(metrics.clone(), 8, 16);
    let inbound = InboundQueue::new();
    let stop = StopOnDrop(StopFlag::new());
    let sim = sim_thread::spawn(
        SimThreadConfig {
            map_data: Some(map.clone()),
            net: Some(NetLink {
                inbound: inbound.rx.clone(),
                sessions: sessions.clone(),
            }),
            ..SimThreadConfig::bare(seed, MapId(0), GameMode::Sandbox)
        },
        metrics.clone(),
        stop.0.clone(),
    )
    .unwrap();
    let net = start_listeners(
        metrics.clone(),
        sessions,
        &inbound,
        sim.commands_tx.clone(),
        true,
    )
    .await;
    let (mut shooter, us) = connect(net.target(Kind::Wt)).await;
    let (enemy, ger) = connect(net.target(Kind::Wt)).await;
    assert_eq!((us.team, ger.team), (0, 1));
    let rtt = c0_rtt(&mut shooter).await;
    let ms = rtt.as_secs_f64() * 1000.0;
    if delayed {
        assert!(
            (140.0..400.0).contains(&ms),
            "expected private netem at 75 ms/way; RTT {ms:.2} ms"
        );
    } else {
        assert!(ms < 100.0, "expected unimpaired loopback; RTT {ms:.2} ms");
    }

    let (shooter_pos, _) = map.join_spawn(us.player_id, Team::Us);
    let (enemy_spawn, _) = map.join_spawn(ger.player_id, Team::Ger);
    let aim = cmd(metrics.ticks_run() as u32 + 4, buttons::AIM);
    send_input(&mut shooter, aim, 0).await;
    let mover = tokio::spawn(strafe(enemy, metrics.clone()));

    let mut decoder = SnapshotDecoder::new();
    let end = Instant::now() + TIMEOUT;
    let (seen_tick, seen) = loop {
        let (ch, bytes) = next_frame(&mut shooter, end).await;
        if ch != Channel::C2 {
            continue;
        }
        let s = decoder.decode(&bytes).expect("C2 snapshot");
        if s.ack_input_seq != aim.seq || s.tick < aim.tick + 10 {
            continue; // Full ADS before firing; eliminate hip-spread as a confound.
        }
        let Some(target) = (0..s.entities.len())
            .filter_map(|i| s.entities.state(i))
            .find(|e| e.id == ger.player_id)
        else {
            continue;
        };
        let p = Vec3::from_array(target.pos.to_meters());
        if p.z >= enemy_spawn.z + 2.0 {
            break (s.tick, p); // The target has accelerated into a sustained strafe.
        }
    };

    // Aim at the enemy *as seen in the C2 packet*, not at a pose read from server memory.
    // 30 Hz: 75 ms down + 75 ms up adds 4.5 ticks, rounded up over the same 2-tick lead.
    // At this range the projectile intersects the target within its first tick.
    let age = if delayed { 7 } else { 2 };
    let fire_tick = seen_tick + age;
    let mut fire = cmd(fire_tick, buttons::FIRE | buttons::AIM);
    let delta = seen - shooter_pos;
    fire.yaw = (delta.x.atan2(delta.z).rem_euclid(TAU) * (65536.0 / TAU)).round() as u16;
    // Shoot through standing torso, ~35 cm below the shooter's eye.
    fire.pitch = (-0.35f32.atan2(delta.x.hypot(delta.z)) * (32768.0 / FRAC_PI_2)).round() as i16;
    fire.view_tick = if rewind { age as u16 } else { 0 };
    // A deliberately distant origin on even seeds. It should add exactly one diagnostic,
    // without moving the projectile or changing the hit/rewind outcome.
    let offset_origin = seed.is_multiple_of(2);
    let hint = offset_origin.then(|| FireCmd {
        seq: fire.seq,
        view_tick: fire.view_tick,
        weapon: fire.weapon_slot,
        dir_yaw: fire.yaw,
        dir_pitch: (i32::from(fire.pitch) + 32768) as u16,
        origin_hint: FireCmd::encode_origin_hint([2048.0, -7.0, 1024.0]).unwrap(),
        spread_seed: 0,
    });
    send_input_with_fire(&mut shooter, fire, seen_tick as u16, hint).await;

    let mut hits = HashSet::new();
    let mut acked_fire = false;
    let end = Instant::now() + TIMEOUT;
    loop {
        let (ch, bytes) = next_frame(&mut shooter, end).await;
        if ch != Channel::C2 {
            continue;
        }
        let s = decoder.decode(&bytes).unwrap();
        acked_fire |= s.ack_input_seq == fire.seq;
        for e in s.events {
            if let Event::HitConfirm {
                target, damage_q, ..
            } = e.event
            {
                assert_eq!(target, ger.player_id);
                assert!(damage_q > 0);
                hits.insert(e.id);
            }
        }
        if s.tick >= fire_tick + 12 {
            break;
        }
    }

    mover.abort();
    let _ = mover.await;
    stop.0.stop();
    let report = sim.join().unwrap();
    net.stop().await;
    eprintln!("WT RTT {ms:.2} ms seed {seed} seen/fire {seen_tick}/{fire_tick} view_tick {}: shots={} server_hits={} unique_C2_HitConfirm={}", fire.view_tick, report.events.shots, report.events.hits, hits.len());
    assert!(acked_fire, "server did not ack the fire input");
    assert_eq!(
        report.events.shots, 1,
        "must fire even in the negative control"
    );
    assert_eq!(report.events.hits, hits.len() as u64);
    assert_eq!(
        metrics.origin_corrected_total.get(),
        u64::from(offset_origin)
    );
    assert_eq!(metrics.session_kicked_total.get(), 0);
    assert_eq!(report.tick_skipped_slots_total, 0);
    hits.len()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn moving_enemy_hitconfirm_over_webtransport() {
    let delayed = std::env::var_os("FRONTLINE_LAGCOMP_NETEM").is_some();
    let map = flat_map();
    let mut hits = 0;
    let mut controls = 0;
    for seed in 1..=TRIALS {
        hits += trial(&map, seed, delayed, true).await;
        if delayed {
            controls += trial(&map, seed, delayed, false).await;
        }
    }
    eprintln!(
        "WT delayed={delayed}: compensated {hits}/{TRIALS}; no-rewind {controls}/{}",
        if delayed { TRIALS } else { 0 }
    );
    assert_eq!(hits, TRIALS as usize);
    assert_eq!(controls, 0);
}
