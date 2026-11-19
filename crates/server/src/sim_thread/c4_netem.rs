//! Private-netns acceptance. Uses production listeners, admission, sim ticks and outbound queues;
//! only the map, game actions and numbered transport probes are supplied by this test.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Child, Command as Process, Stdio};
use std::sync::atomic::AtomicU32;
use std::time::Instant;

use frontline_protocol::msg::c4::{decode_c4, encode_c4, roster_flags, Notice, ServerEventC4};
use frontline_protocol::msg::input::buttons;
use frontline_protocol::Channel;
use frontline_sim::spawn::garrison::{GarrisonDamage, GarrisonState};
use frontline_sim::{CollisionChunk, ObjectiveSpec, Vec3};
use serde::Deserialize;

use super::*;
use crate::clock::fake::FakeClock;
use crate::net::conn::{InboundQueue, NetCtx, Sessions};
use crate::net::transport::Rel;
use crate::net::ws::WsListener;
use crate::net::wt::{cert_hash, dev_identity, hash_hex, WtListener};
use crate::world_loader::{HqSpawn, MapSpawns};

const OBJECTIVE: u8 = 7;
const TEAMMATE: u8 = 70;
const PROBE: u16 = 65534;
const END: u16 = 65535;

struct Probe(Child);

impl Drop for Probe {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[derive(Deserialize)]
struct Observation {
    player: u8,
    team: u8,
    gameplay: Vec<Vec<u8>>,
    sequence: u32,
    spawn_tick: Option<u32>,
}

fn fixture() -> LoadedMap {
    let chunks = (0..10).map(|x| CollisionChunk::new(x, 0, vec![5.0; 65 * 65], vec![0; 64 * 64]));
    LoadedMap {
        name: "c4_fixture".into(),
        collision: CollisionData::from_chunks(chunks),
        spawns: MapSpawns {
            hq: [32.0, 608.0].map(|x| HqSpawn {
                x,
                z: 32.0,
                y: 5.0,
                radius_m: 10.0,
            }),
            center: (320.0, 32.0),
        },
        chunk_count: 10,
        patrol_points: Vec::new(),
        size_m: 640.0,
        dir: PathBuf::new(),
        objectives: Vec::new(),
        no_build_zones: Default::default(),
        garrison_sites: [None; 2],
    }
}

fn notice(sessions: &Sessions, code: u16, n: i32) {
    let bytes = bytes::Bytes::from(
        encode_c4(&ServerEventC4::Notice(Notice {
            code,
            args: [n, 0, 0],
        }))
        .unwrap(),
    );
    for id in [0, 1] {
        assert!(sessions
            .handle_of(id)
            .unwrap()
            .send(Channel::C4, bytes.clone(), Rel::Reliable));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "run via scripts/loadtest-netem.sh --c4 ws|wt"]
async fn c4_over_real_listeners() {
    static TICK: AtomicU32 = AtomicU32::new(0);
    let kind = std::env::var("FRONTLINE_C4_TRANSPORT").unwrap();
    let seconds: u32 = std::env::var("FRONTLINE_C4_SOAK_SECS")
        .unwrap()
        .parse()
        .unwrap();
    let count = seconds.checked_mul(10).expect("soak length");
    let metrics = Metrics::new();
    let sessions = Sessions::new(metrics.clone());
    let inbound = InboundQueue::new();
    let mut cfg = SimThreadConfig::bare(26, MapId(0), GameMode::Sandbox);
    cfg.map_data = Some(Arc::new(fixture()));
    cfg.net = Some(NetLink {
        inbound: inbound.rx,
        sessions: sessions.clone(),
    });
    let clock = FakeClock::new();
    let (mut server, inputs, commands) = build_loop(&clock, &cfg, metrics.clone(), StopFlag::new());
    // B must receive the already-present teammate, not just the delta for its own join.
    server.world.roster_add(TEAMMATE, Team::Ger).unwrap();
    let ctx = Arc::new(NetCtx {
        metrics: metrics.clone(),
        sessions: sessions.clone(),
        inbound: inbound.tx,
        commands,
        data_hash: GameData::embedded().hash,
        match_seed: 26,
        tick_hz: 30,
        map_id: 0,
        mode: 0,
        server_tick: &TICK,
        started: Instant::now(),
        handshake_timeout: Duration::from_secs(5),
        tickets: None,
    });
    let (ws, wt, addr, hash) = match kind.as_str() {
        "ws" => {
            let listener = WsListener::bind(("127.0.0.1", 0), ctx).await.unwrap();
            let addr = listener.local_addr();
            (Some(listener), None, addr, "-".to_owned())
        }
        "wt" => {
            let identity = dev_identity(13).unwrap();
            let hash = hash_hex(&cert_hash(&identity));
            let listener = WtListener::bind(([127, 0, 0, 1], 0).into(), identity, ctx)
                .await
                .unwrap();
            let addr = listener.local_addr();
            (None, Some(listener), addr, hash)
        }
        _ => panic!("unknown transport: {kind}"),
    };
    let mut probe = Probe(
        Process::new(std::env::var_os("FRONTLINE_C4_PROBE").unwrap())
            .args([kind.as_str(), &addr.to_string(), &hash, &count.to_string()])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );

    let mut interval = tokio::time::interval(TICK_PERIOD);
    let tick = || {
        clock.advance(TICK_PERIOD);
        TICK.fetch_add(1, Ordering::Relaxed);
    };
    let deadline = Instant::now() + Duration::from_secs(20);
    while server.bridge.len() != 2 {
        assert!(
            Instant::now() < deadline,
            "both headless connections must join"
        );
        assert!(
            probe.0.try_wait().unwrap().is_none(),
            "probe exited before join"
        );
        interval.tick().await;
        tick();
        server.tick();
    }

    // Set up near-capture progress, an in-range builder and a lethal hit. Advance the actual
    // systems to emit ObjectiveOwner, Kill and the Garrison activation/destruction events.
    let point = server.world.player_state(0).unwrap().pos;
    server.world.set_objectives(&[ObjectiveSpec {
        id: OBJECTIVE.into(),
        pos: point,
        radius_m: 4.0,
        owner: None,
    }]);
    server
        .world
        .debug_set_objective(OBJECTIVE.into(), 99.999, None, None);
    server.world.debug_kill(1);
    tick();
    server.tick();
    assert_eq!(server.world.objective_list()[0].owner, Some(Team::Us));
    assert_eq!(server.tally().kills, 1);

    let deploy_ok = metrics.deploy_requests_total.with_label_values(&["ok"]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while deploy_ok.get() == 0 {
        assert!(Instant::now() < deadline, "no successful DeployResult");
        assert!(
            probe.0.try_wait().unwrap().is_none(),
            "probe exited before deploy"
        );
        interval.tick().await;
        tick();
        server.tick();
    }
    let spawn_tick = server.world.tick().0 - 1;
    assert_eq!(deploy_ok.get(), 1);
    server.world.roster_remove(TEAMMATE);

    let site = Vec3::new(320.0, 5.0, 32.0);
    server.world.debug_teleport(0, site);
    server.world.set_officer(0, true);
    server.world.add_depot(Team::Us, point);
    server.world.debug_add_crate(Team::Us, site, 100);
    let garrison = server.world.place_garrison(0, site).unwrap();
    for seq in 1..=630 {
        let cmd = InputCmd {
            tick: server.world.tick().0 + 1,
            seq,
            move_x: 0,
            move_z: 0,
            yaw: 16384,
            pitch: 0,
            buttons: buttons::USE,
            weapon_slot: 0,
            aux: 0,
            view_tick: 0,
        };
        assert!(inputs.try_send(InputEnvelope { player: 0, cmd }));
        tick();
        server.tick();
        if server.world.garrison_list()[0].state == GarrisonState::Active {
            break;
        }
    }
    assert_eq!(server.world.garrison_list()[0].state, GarrisonState::Active);
    assert!(server
        .world
        .damage_garrison(garrison, 2000.0, GarrisonDamage::Explosion));
    tick();
    server.tick();
    assert_eq!(
        server.world.garrison_list()[0].state,
        GarrisonState::Destroyed
    );

    // Real-time 10 Hz numbered notices, independent of the gameplay assertions. Numbered
    // probes use the same reliable queue and channel but never exist in a production build.
    let started = Instant::now();
    let mut pace = tokio::time::interval(TICK_PERIOD);
    for i in 0..count * 3 {
        pace.tick().await;
        tick();
        server.tick();
        if i % 3 == 2 {
            notice(&sessions, PROBE, i32::try_from(i / 3).unwrap());
        }
    }
    if count > 0 {
        pace.tick().await;
    }
    let soak_ms = started.elapsed().as_millis();
    notice(&sessions, END, 0);
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = probe.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "probe did not receive the final barrier"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert!(status.success(), "headless probe {kind} failed: {status}");
    let observations: Vec<Observation> =
        serde_json::from_reader(probe.0.stdout.take().unwrap()).unwrap();
    assert_eq!(observations.len(), 2);
    for (id, o) in (0..=1).zip(observations) {
        assert_eq!((o.player, o.team, o.sequence), (id, id, count));
        assert_eq!(o.spawn_tick, (id == 1).then_some(spawn_tick));
        verify(o, &server.world, garrison);
    }
    println!("C4 {kind}: soak_ms={soak_ms}, numbered_per_client={count}, spawn_tick={spawn_tick}");
    if let Some(listener) = ws {
        listener.shutdown().await;
    }
    if let Some(listener) = wt {
        listener.shutdown().await;
    }
}

fn verify(o: Observation, world: &World, garrison: u16) {
    let mut kills = Vec::new();
    let mut objectives = BTreeMap::new();
    let mut captures = Vec::new();
    let mut spawn_states = Vec::new();
    let mut roster = BTreeMap::new();
    let mut roster_updates = Vec::new();
    for bytes in &o.gameplay {
        match decode_c4(bytes).unwrap() {
            ServerEventC4::Kill(kill) => kills.push(kill.victim),
            ServerEventC4::ObjectiveChange(change) => {
                objectives.insert(change.objective, change.owner);
                captures.push((change.objective, change.owner));
            }
            ServerEventC4::SpawnPointState(point) => {
                assert_eq!((point.id, point.team), (garrison, 0));
                spawn_states.push(point.state);
            }
            ServerEventC4::PlayerRoster(entry) => {
                assert_eq!(entry.team, o.team, "enemy roster row leaked");
                let left = entry.flags & roster_flags::LEFT != 0;
                roster_updates.push((entry.player, entry.role, left));
                if left {
                    roster.remove(&entry.player);
                } else {
                    roster.insert(entry.player, entry);
                }
            }
            _ => {}
        }
    }
    assert_eq!(kills, if o.team == 0 { &[][..] } else { &[1] });
    assert_eq!(captures, [(OBJECTIVE, 0)]);
    for objective in world.objective_list() {
        assert_eq!(
            objectives[&(objective.id as u8)],
            objective.owner.unwrap() as u8
        );
    }
    if o.team == 0 {
        assert_eq!(spawn_states, [1, 2]);
    } else {
        assert!(spawn_states.is_empty(), "enemy spawn point leaked");
    }
    let expected = || {
        world
            .squads()
            .roster()
            .filter(|(_, entry)| entry.team as u8 == o.team)
    };
    assert_eq!(roster.len(), expected().count());
    for (id, entry) in expected() {
        let on_wire = &roster[&id];
        assert_eq!(
            (on_wire.team, on_wire.squad),
            (entry.team as u8, entry.squad.unwrap_or(0) as u8)
        );
        if let Some(role) = entry.role {
            assert_eq!(
                world.config().data.roles[usize::from(on_wire.role) - 1].id,
                role
            );
        } else {
            assert_eq!(on_wire.role, 0);
        }
    }
    if o.team == 0 {
        assert_eq!(roster_updates, [(0, 0, false)]);
    } else {
        assert_eq!(
            roster_updates,
            [
                (1, 0, false),
                (TEAMMATE, 0, false),
                (1, roster[&1].role, false),
                (TEAMMATE, 0, true)
            ]
        );
    }
    println!("C4 client {}: gameplay={}, kills={}, captures={}, spawn_states={}, roster_updates={}, roster_final={}",
        o.player, o.gameplay.len(), kills.len(), captures.len(), spawn_states.len(), roster_updates.len(), roster.len());
}
