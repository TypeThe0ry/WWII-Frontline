//! Exercise the gameplay-to-wire C3 route on the actual WebSocket and WebTransport listeners.
mod common;

use std::time::{Duration, Instant};

use common::sim_harness::start_listeners;
use common::{Conn, Got, Kind, Target, DATA_HASH};
use frontline_protocol::msg::c0::{
    encode_c0_client, ClientMsgC0, Hello, MarkerPlace, ServerMsgC0, SquadOp, SquadOpKind, Welcome,
    CAP_TEAM_MARKERS,
};
use frontline_protocol::msg::team::{
    decode_team_state, decode_team_state_with_markers, TeamMarker, TeamMarkerKind,
    TeamStateWithMarkers,
};
use frontline_protocol::{Channel, PROTOCOL_VERSION};
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::{InboundQueue, Sessions};
use frontline_server::shutdown::StopFlag;
use frontline_server::sim_bridge::NetLink;
use frontline_server::sim_thread::{self, SimThreadConfig};
use frontline_sim::{GameMode, MapId};

const TIMEOUT: Duration = Duration::from_secs(5);

async fn c0(c: &mut Conn, msg: ClientMsgC0) {
    assert!(c.send(Channel::C0, &encode_c0_client(&msg).unwrap()).await);
}

async fn connect(t: &Target, caps: u8) -> (Conn, Welcome) {
    let mut c = Conn::connect(t).await.unwrap();
    c0(
        &mut c,
        ClientMsgC0::Hello(Hello {
            protocol_version: PROTOCOL_VERSION,
            data_hash: DATA_HASH,
            ticket: Vec::new(),
            client_build: 1,
            caps,
        }),
    )
    .await;
    match c.server_msg(TIMEOUT).await {
        Some(ServerMsgC0::Welcome(w)) => (c, w),
        other => panic!("expected Welcome, got {other:?}"),
    }
}

async fn next_c3(c: &mut Conn, ready: impl Fn(&TeamStateWithMarkers) -> bool) -> Vec<u8> {
    let end = Instant::now() + TIMEOUT;
    loop {
        match c
            .recv_within(end.saturating_duration_since(Instant::now()))
            .await
        {
            Got::Frame(Channel::C3, payload) => {
                if ready(&decode_team_state_with_markers(&payload).unwrap()) {
                    return payload;
                }
            }
            Got::Frame(..) => {}
            _ => panic!("no matching C3 packet"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn squad_marker_reaches_only_capable_same_squad_on_ws_and_wt() {
    let metrics = Metrics::new();
    let sessions = Sessions::with_limits(metrics.clone(), 8, 16);
    let inbound = InboundQueue::new();
    let stop = StopFlag::new();
    let handle = sim_thread::spawn(
        SimThreadConfig {
            net: Some(NetLink {
                inbound: inbound.rx.clone(),
                sessions: sessions.clone(),
            }),
            ..SimThreadConfig::bare(1, MapId(0), GameMode::Sandbox)
        },
        metrics.clone(),
        stop.clone(),
    )
    .unwrap();
    let net = start_listeners(
        metrics.clone(),
        sessions,
        &inbound,
        handle.commands_tx.clone(),
        true,
    )
    .await;
    let (mut author, a) = connect(net.target(Kind::Ws), CAP_TEAM_MARKERS).await;
    let (mut enemy, b) = connect(net.target(Kind::Wt), CAP_TEAM_MARKERS).await;
    let (mut ally, c) = connect(net.target(Kind::Wt), CAP_TEAM_MARKERS).await;
    let (enemy2, d) = connect(net.target(Kind::Ws), 0).await;
    let (mut legacy, e) = connect(net.target(Kind::Ws), 0).await;
    assert_eq!([a.team, b.team, c.team, d.team, e.team], [0, 1, 0, 1, 0]);

    let op = |op, squad| {
        ClientMsgC0::SquadOp(SquadOp {
            op,
            squad,
            target: 0,
            name_key: String::new(),
        })
    };
    let ok = || metrics.squad_ops_total.with_label_values(&["ok"]).get();
    c0(&mut author, op(SquadOpKind::Create, 0)).await;
    assert!(common::wait_until(TIMEOUT, || ok() == 1).await);
    c0(&mut ally, op(SquadOpKind::Join, 1)).await;
    c0(&mut legacy, op(SquadOpKind::Join, 1)).await;
    assert!(common::wait_until(TIMEOUT, || ok() == 3).await);

    let marker = ClientMsgC0::MarkerPlace(MarkerPlace {
        kind: TeamMarkerKind::OrderAttack as u8,
        pos: [32768, 16384], // (128, 64) on the 256 m map; 5 m cells (25, 12).
        ttl: 1,              // untrusted; server applies the 180 s order lifetime.
    });
    c0(&mut author, marker).await;
    let payload = next_c3(&mut author, |s| !s.markers.is_empty()).await;
    let s = decode_team_state_with_markers(&payload).unwrap();
    assert_eq!(s.team.members.len(), 3);
    assert_eq!(
        s.markers,
        [TeamMarker {
            kind: TeamMarkerKind::OrderAttack,
            x: 25,
            z: 12,
            ttl_s: 180
        }]
    );
    let bytes = next_c3(&mut ally, |n| n.team.tick == s.team.tick).await;
    let opposing = next_c3(&mut enemy, |n| n.team.tick == s.team.tick).await;
    let old = next_c3(&mut legacy, |n| n.team.tick == s.team.tick).await;
    assert_eq!(bytes, payload);
    let opposing = decode_team_state_with_markers(&opposing).unwrap();
    assert!(opposing.markers.is_empty());
    assert_eq!(opposing.team.members.len(), 2);
    assert_eq!(decode_team_state(&old).unwrap(), s.team);

    drop((author, enemy, ally, enemy2, legacy));
    stop.stop();
    tokio::task::block_in_place(|| handle.join()).unwrap();
    net.stop().await;
}
