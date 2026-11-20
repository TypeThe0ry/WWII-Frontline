//! Budget transitions through the real scheduler, session and C4 writers.
mod common;

use std::time::Duration;

use common::sim_harness::{bundle_upto, encode, start_listeners};
use common::{Conn, Got, Kind};
use frontline_protocol::msg::c4::{decode_c4, notice_code, Notice, ServerEventC4};
use frontline_protocol::snapshot::SnapshotDecoder;
use frontline_protocol::Channel;
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::{InboundQueue, Sessions};
use frontline_server::shutdown::StopFlag;
use frontline_server::sim_bridge::NetLink;
use frontline_server::sim_thread::{self, SimThreadConfig};
use frontline_sim::{GameMode, MapId};
use tokio::time::{timeout, Instant};

async fn roundtrip(mut c: Conn, player: u8) {
    let mut dec = SnapshotDecoder::new();
    let mut degraded = None;
    loop {
        match c.recv_within(Duration::from_secs(5)).await {
            Got::Frame(Channel::C2, data) => {
                let snap = dec.decode(&data).expect("snapshot decodes");
                if degraded.is_some() {
                    let mut ack = bundle_upto(player, snap.tick.wrapping_add(2));
                    ack.ack_snapshot_tick = snap.tick as u16;
                    assert!(c.send(Channel::C1, &encode(&ack)).await);
                }
            }
            Got::Frame(Channel::C4, data) => match decode_c4(&data).expect("C4 decodes") {
                ServerEventC4::Notice(Notice {
                    code: notice_code::NET_DEGRADED,
                    args,
                }) => {
                    assert!(degraded.is_none(), "duplicate degradation notice");
                    assert!((40..70).contains(&args[0]));
                    assert_eq!(&args[1..], &[0, 0]);
                    degraded = Some(Instant::now());
                }
                ServerEventC4::Notice(Notice {
                    code: notice_code::NET_RECOVERED,
                    args,
                }) => {
                    assert_eq!(args, [0; 3]);
                    // The wire observes the 300-tick cooldown plus transport scheduling jitter.
                    assert!(
                        degraded.expect("degraded first").elapsed() >= Duration::from_millis(9800)
                    );
                    return;
                }
                _ => {}
            },
            Got::Frame(..) => {}
            _ => panic!("connection ended before recovery"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reliable_budget_notices_arrive_on_ws_and_wt() {
    let metrics = Metrics::new();
    let sessions = Sessions::with_limits(metrics.clone(), 4, 4);
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
        metrics,
        sessions,
        &inbound,
        handle.commands_tx.clone(),
        true,
    )
    .await;
    let (ws, w) = Conn::handshake(net.target(Kind::Ws)).await;
    let (wt, q) = Conn::handshake(net.target(Kind::Wt)).await;
    let result = timeout(Duration::from_secs(22), async {
        tokio::join!(roundtrip(ws, w.player_id), roundtrip(wt, q.player_id));
    })
    .await;
    stop.stop();
    tokio::task::block_in_place(|| handle.join()).unwrap();
    net.stop().await;
    assert!(result.is_ok(), "clients did not receive recovery notices");
}
