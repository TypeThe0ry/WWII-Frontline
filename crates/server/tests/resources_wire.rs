//! M2-009 5v5 resource projection over real WebSocket connections across five simulated minutes.

mod common;

use std::cell::Cell;
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::{sim_harness::start_listeners, Client, Got, Kind, DATA_HASH};
use frontline_protocol::msg::c0::{
    decode_c0_server, encode_c0_client, ClientMsgC0, Hello, ServerMsgC0, CAP_RESOURCES,
};
use frontline_protocol::msg::c4::{decode_c4, ServerEventC4};
use frontline_protocol::msg::team::{decode_world_state_c2, WORLD_STATE_C2_TAG};
use frontline_protocol::{Channel, PROTOCOL_VERSION};
use frontline_server::clock::Clock;
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::{InboundQueue, Sessions};
use frontline_server::shutdown::StopFlag;
use frontline_server::sim_bridge::NetLink;
use frontline_server::sim_thread::{self, SimThreadConfig};
use frontline_server::world_loader::load_map;
use frontline_sim::rules::ObjectiveSpec;
use frontline_sim::{GameMode, MapId, Team, Vec3};

struct SimClock(Cell<Instant>);

impl Clock for SimClock {
    fn now(&self) -> Instant {
        self.0.get()
    }

    fn sleep_until(&self, deadline: Instant) {
        self.0.set(self.now().max(deadline));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn frontline_resources_reach_ten_websocket_clients() {
    let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let mut map = load_map(&data, "m0").unwrap();
    // Reuse the tracked collision chunks with a synthetic arrangement of five strongpoints.
    map.name = "resources-fixture".into();
    map.objectives = [Some(Team::Us), Some(Team::Us), None, None, Some(Team::Ger)]
        .into_iter()
        .enumerate()
        .map(|(i, owner)| ObjectiveSpec {
            id: i as u16 + 1,
            pos: Vec3::new(40.0 + 40.0 * i as f32, 0.0, 128.0),
            radius_m: 8.0,
            owner,
        })
        .collect();

    let metrics = Metrics::new();
    let sessions = Sessions::with_limits(metrics.clone(), 10, 10);
    let inbound = InboundQueue::new();
    let (mut server, _, commands) = sim_thread::build_loop(
        SimClock(Cell::new(Instant::now())),
        &SimThreadConfig {
            map_data: Some(Arc::new(map)),
            net: Some(NetLink {
                inbound: inbound.rx.clone(),
                sessions: sessions.clone(),
            }),
            ..SimThreadConfig::bare(1, MapId(0), GameMode::Frontline)
        },
        metrics.clone(),
        StopFlag::new(),
    );
    // Keep the queued clients under the 300 s deployment AFK threshold at tick 9000.
    server.tick();
    let net = start_listeners(metrics.clone(), sessions, &inbound, commands, false).await;
    let hello = encode_c0_client(&ClientMsgC0::Hello(Hello {
        protocol_version: PROTOCOL_VERSION,
        data_hash: DATA_HASH,
        ticket: Vec::new(),
        client_build: 0,
        caps: CAP_RESOURCES,
    }))
    .unwrap();
    let mut clients = Vec::new();
    for _ in 0..10 {
        let mut client = Client::connect(&net.target(Kind::Ws).url).await.unwrap();
        assert!(client.send(Channel::C0, &hello).await);
        let Got::Frame(Channel::C0, bytes) = client.recv_within(Duration::from_secs(3)).await
        else {
            panic!("expected a Welcome frame");
        };
        let ServerMsgC0::Welcome(welcome) = decode_c0_server(&bytes).unwrap() else {
            panic!("expected Welcome");
        };
        clients.push((client, usize::from(welcome.team)));
    }
    server.tick();
    assert_eq!(
        ["us", "ger"].map(|team| metrics.players.with_label_values(&[team]).get()),
        [5, 5]
    );

    let income = [[42, 18, 10], [36, 15, 8]];
    for minute in 1..=5 {
        let tick = minute * 1800;
        while server.world().tick().0 < tick {
            server.tick();
        }
        let timer = server
            .world()
            .match_state()
            .ends_at
            .saturating_sub(tick)
            .div_ceil(30);
        let pools = [Team::Us, Team::Ger].map(|team| {
            let pool = server.world().view().team_resources(team);
            let balances = [pool.manpower, pool.munitions, pool.fuel];
            let starts = [200, 80, 40];
            assert_eq!(
                balances,
                std::array::from_fn(|i| starts[i] + income[team as usize][i] * minute as i32)
            );
            balances.map(|value| u16::try_from(value).unwrap())
        });
        for (client, team) in &mut clients {
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut deltas = [None; 3];
            loop {
                match client
                    .recv_within(deadline.saturating_duration_since(Instant::now()))
                    .await
                {
                    Got::Frame(Channel::C4, bytes) => {
                        if let ServerEventC4::ResourceDelta(delta) = decode_c4(&bytes).unwrap() {
                            assert_eq!(usize::from(delta.team), *team);
                            assert_eq!(delta.reason, 0);
                            assert!(deltas[usize::from(delta.kind)]
                                .replace(delta.value)
                                .is_none());
                        }
                    }
                    Got::Frame(Channel::C2, bytes)
                        if bytes.first() == Some(&WORLD_STATE_C2_TAG) =>
                    {
                        let packet = decode_world_state_c2(&bytes).unwrap();
                        assert_eq!(usize::from(packet.team), *team);
                        assert!(packet.tick <= tick);
                        if packet.tick == tick {
                            assert_eq!(u32::from(packet.state.timer), timer);
                            assert_eq!(
                                [
                                    packet.state.manpower,
                                    packet.state.munitions,
                                    packet.state.fuel
                                ],
                                pools[*team]
                            );
                            // WS queues the reliable deltas before the world state of the same tick.
                            assert_eq!(deltas, income[*team].map(Some));
                            break;
                        }
                    }
                    Got::Frame(..) => {}
                    _ => panic!("team {team} did not receive the world state at tick {tick}"),
                }
            }
        }
    }
    drop(clients);
    net.stop().await;
}
