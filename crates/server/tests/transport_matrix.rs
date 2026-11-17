//! M0-017 acceptance 4: the M0-016 handshake / limits cases, run against both transports
//! (`*_ws` and `*_wt`) through the same `Conn` client. Cases that exist only at the WebSocket
//! envelope level stay in `ws_handshake.rs`; the QUIC-specific ones are in `wt_loopback.rs`.

mod common;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use common::*;
use frontline_protocol::channel::c0_type;
use frontline_protocol::msg::c0::{Disconnect, RejectReason, ServerMsgC0};
use frontline_protocol::msg::input::{encode_input_bundle, InputBundle, InputCmd};
use frontline_protocol::{BitWriter, Channel, PROTOCOL_VERSION};
use frontline_server::net::limits::HANDSHAKE_TIMEOUT;
use frontline_server::sim_thread::Command;

const T: Duration = Duration::from_secs(3);

fn rejects(h: &Harness, l: &str) -> u64 {
    h.metrics.net_rejects_total.with_label_values(&[l]).get()
}

macro_rules! matrix {
    ($($f:ident => $ws:ident, $wt:ident;)*) => {$(
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn $ws() { $f(Kind::Ws).await }
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn $wt() { $f(Kind::Wt).await }
    )*};
}

matrix! {
    welcome_ping_and_client_gone => welcome_ping_and_client_gone_ws, welcome_ping_and_client_gone_wt;
    reject_mismatches => reject_mismatches_ws, reject_mismatches_wt;
    no_hello_times_out => no_hello_times_out_ws, no_hello_times_out_wt;
    ticket_257 => ticket_257_ws, ticket_257_wt;
    per_ip_limit => per_ip_limit_ws, per_ip_limit_wt;
    illegal_messages => illegal_messages_ws, illegal_messages_wt;
    c1_queued_and_disconnect => c1_queued_and_disconnect_ws, c1_queued_and_disconnect_wt;
    flooding => flooding_ws, flooding_wt;
    handshake_latency => handshake_latency_ws, handshake_latency_wt;
}

async fn welcome_ping_and_client_gone(kind: Kind) {
    let h = Harness::start_kind(kind, HANDSHAKE_TIMEOUT, 4).await;
    let t = h.target.clone();
    h.tick.store(77, Ordering::Relaxed);
    let (mut a, wa) = Conn::handshake(&t).await;
    let (b, wb) = Conn::handshake(&t).await;
    assert_eq!((wa.player_id, wb.player_id), (0, 1));
    assert_eq!(wa.server_tick, 77);
    assert_eq!(wa.tick_hz, 30);
    assert_eq!(wa.match_seed, 0xABCD);
    assert_eq!(wa.data_hash, DATA_HASH);
    assert_eq!(h.metrics.net_connections.get(), 2);
    let label = if kind == Kind::Ws { "ws" } else { "wt" };
    assert_eq!(
        h.metrics
            .net_transport_connections
            .with_label_values(&[label])
            .get(),
        2
    );

    h.tick.store(78, Ordering::Relaxed);
    a.send(Channel::C0, &ping_msg(0xDEADBEEF)).await;
    match a.server_msg(T).await {
        Some(ServerMsgC0::Pong(p)) => {
            assert_eq!(p.client_time, 0xDEADBEEF);
            assert_eq!(p.server_tick, 78);
        }
        o => panic!("{o:?}"),
    }

    drop(a);
    assert!(
        wait_until(Duration::from_secs(6), || h
            .commands
            .try_iter()
            .any(|c| matches!(c, Command::ClientGone(0))))
        .await
    );
    let (_c, wc) = Conn::handshake(&t).await;
    assert_eq!(wc.player_id, 0);
    drop(b);
    h.stop().await;
}

async fn reject_mismatches(kind: Kind) {
    let h = Harness::start_kind(kind, HANDSHAKE_TIMEOUT, 4).await;
    for (hello, want, label) in [
        (
            hello_msg(PROTOCOL_VERSION + 1, DATA_HASH, vec![]),
            RejectReason::VersionMismatch,
            "version",
        ),
        (
            hello_msg(PROTOCOL_VERSION, DATA_HASH ^ 1, vec![]),
            RejectReason::DataHashMismatch,
            "data_hash",
        ),
    ] {
        let mut c = Conn::connect(&h.target).await.unwrap();
        c.send(Channel::C0, &hello).await;
        match c.server_msg(T).await {
            Some(ServerMsgC0::Reject(r)) => assert_eq!(r.reason, want),
            o => panic!("{o:?}"),
        }
        assert!(
            c.expect_closed(T).await,
            "connection must close after Reject"
        );
        assert_eq!(rejects(&h, label), 1);
    }
    assert!(h.ctx.sessions.handle_of(0).is_none());
    h.stop().await;
}

async fn no_hello_times_out(kind: Kind) {
    let h = Harness::start_kind(kind, HANDSHAKE_TIMEOUT, 4).await;
    let mut c = Conn::connect(&h.target).await.unwrap();
    let t0 = Instant::now();
    assert!(
        c.expect_closed(Duration::from_secs(9)).await,
        "never closed"
    );
    let dt = t0.elapsed().as_secs_f64();
    eprintln!("[{kind:?}] handshake timeout: connection closed after {dt:.3} s");
    // The deadline starts at the first packet; the client's connect() took a few ms of it.
    assert!((4.5..=5.8).contains(&dt), "closed after {dt} s");
    assert_eq!(h.metrics.net_handshake_timeout_total.get(), 1);
    h.stop().await;
}

async fn ticket_257(kind: Kind) {
    let h = Harness::start_kind(kind, HANDSHAKE_TIMEOUT, 4).await;
    let mut w = BitWriter::new();
    w.write_bits(c0_type::HELLO as u64, 8).unwrap();
    w.write_bits(PROTOCOL_VERSION as u64, 16).unwrap();
    w.write_bits(DATA_HASH, 64).unwrap();
    w.write_bits(257, 9).unwrap();
    for _ in 0..257 {
        w.write_bits(0xAB, 8).unwrap();
    }
    w.write_bits(1, 32).unwrap();
    w.write_bits(0, 8).unwrap();
    let mut c = Conn::connect(&h.target).await.unwrap();
    c.send(Channel::C0, &w.finish()).await;
    assert!(c.expect_closed(T).await);
    assert_eq!(h.metrics.net_frames_malformed_total.get(), 1);
    let mut c = Conn::connect(&h.target).await.unwrap();
    c.send(
        Channel::C0,
        &hello_msg(PROTOCOL_VERSION, DATA_HASH, vec![7; 256]),
    )
    .await;
    assert!(matches!(
        c.server_msg(T).await,
        Some(ServerMsgC0::Welcome(_))
    ));
    h.stop().await;
}

async fn per_ip_limit(kind: Kind) {
    let h = Harness::start_kind(kind, HANDSHAKE_TIMEOUT, 4).await;
    let mut keep = vec![];
    for _ in 0..4 {
        keep.push(Conn::handshake(&h.target).await);
    }
    let fifth = Conn::connect(&h.target).await;
    assert!(
        fifth.is_err(),
        "5th connection from the same IP must be refused"
    );
    assert_eq!(rejects(&h, "ip_limit"), 1);
    keep.pop();
    assert!(wait_until(Duration::from_secs(6), || h.ctx.sessions.connections() == 3).await);
    assert!(Conn::connect(&h.target).await.is_ok());
    h.stop().await;
}

async fn illegal_messages(kind: Kind) {
    let h = Harness::start_kind(kind, HANDSHAKE_TIMEOUT, 64).await;
    let big = vec![0u8; 4097];
    // (name, channel, payload, also tried before the handshake)
    let mut cases: Vec<(&str, Channel, Vec<u8>, bool)> = vec![
        ("oversize C0", Channel::C0, big.clone(), true),
        ("unknown msg_type", Channel::C0, vec![0xEE], true),
        (
            "server-only msg (Welcome)",
            Channel::C0,
            vec![c0_type::WELCOME; 40],
            true,
        ),
        ("second Hello", Channel::C0, good_hello(), false),
        ("garbage C1", Channel::C1, vec![0xFF; 9], false),
        ("client sends C2", Channel::C2, vec![1], false),
        ("client sends C4", Channel::C4, vec![1], false),
    ];
    if kind == Kind::Ws {
        // A WebSocket message can be any size; a QUIC datagram cannot (and is lost, not rejected).
        cases.push(("oversize C1", Channel::C1, big, true));
        cases.push(("client sends C2 early", Channel::C2, vec![1], true));
        cases.push(("client sends C4 early", Channel::C4, vec![1], true));
        cases.push(("garbage C1 early", Channel::C1, vec![0xFF; 9], true));
    }
    let mut expected = 0;
    for (name, ch, payload, pre) in cases {
        for established in [false, true] {
            if established && name.ends_with("early") {
                continue;
            }
            if !established && !pre {
                continue;
            }
            let mut c = if established {
                Conn::handshake(&h.target).await.0
            } else {
                Conn::connect(&h.target).await.unwrap()
            };
            c.send(ch, &payload).await;
            assert!(
                c.expect_closed(T).await,
                "{name} (established={established}) not closed"
            );
            expected += 1;
            assert_eq!(
                h.metrics.net_frames_malformed_total.get(),
                expected,
                "{name} (established={established})"
            );
        }
    }
    h.stop().await;
}

async fn c1_queued_and_disconnect(kind: Kind) {
    let h = Harness::start_kind(kind, HANDSHAKE_TIMEOUT, 4).await;
    let (mut c, w) = Conn::handshake(&h.target).await;
    let bundle = InputBundle {
        ack_snapshot_tick: 5,
        pred_hash: 1,
        cmds: [InputCmd {
            seq: 1,
            tick: 10,
            move_x: 1,
            move_z: -1,
            yaw: 100,
            pitch: 0,
            buttons: 0,
            weapon_slot: 0,
            aux: 0,
            view_tick: 3,
        }]
        .into_iter()
        .collect(),
        fire: Default::default(),
    };
    c.send(Channel::C1, &encode_input_bundle(&bundle).unwrap())
        .await;
    assert!(wait_until(T, || h.metrics.net_c1_inbound_total.get() == 1).await);
    let got = h.inbound.recv_timeout(T).unwrap();
    assert_eq!(got.player, w.player_id);
    assert_eq!(got.bundle, bundle);
    let d = frontline_protocol::msg::c0::encode_c0_client(
        &frontline_protocol::msg::c0::ClientMsgC0::Disconnect(Disconnect { reason: 0 }),
    )
    .unwrap();
    c.send(Channel::C0, &d).await;
    assert!(c.expect_closed(Duration::from_secs(5)).await);
    assert_eq!(h.metrics.net_frames_malformed_total.get(), 0);
    h.stop().await;
}

async fn flooding(kind: Kind) {
    let h = Harness::start_kind(kind, HANDSHAKE_TIMEOUT, 4).await;
    let (mut c, _) = Conn::handshake(&h.target).await;
    for i in 0..2000u32 {
        if !c.send(Channel::C0, &ping_msg(i)).await {
            break;
        }
    }
    assert!(c.expect_closed(T).await);
    assert_eq!(rejects(&h, "rate_limit"), 1);
    h.stop().await;
}

async fn handshake_latency(kind: Kind) {
    let h = Harness::start_kind(kind, HANDSHAKE_TIMEOUT, 4).await;
    let mut rtts = vec![];
    for _ in 0..40 {
        let mut c = Conn::connect(&h.target).await.unwrap();
        let t0 = Instant::now();
        c.send(Channel::C0, &good_hello()).await;
        assert!(matches!(
            c.server_msg(T).await,
            Some(ServerMsgC0::Welcome(_))
        ));
        rtts.push(t0.elapsed().as_secs_f64() * 1e3);
        // Keep the number of simultaneously open connections within the per-IP limit.
        drop(c);
        assert!(wait_until(Duration::from_secs(6), || h.ctx.sessions.connections() == 0).await);
    }
    rtts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = rtts[rtts.len() / 2];
    eprintln!(
        "[{kind:?}] Hello->Welcome RTT over 40 sequential handshakes: P50 {p50:.2} ms, max {:.2} ms",
        rtts[rtts.len() - 1]
    );
    assert!(p50 < 10.0, "P50 {p50} ms");
    h.stop().await;

    let h = Harness::start_kind(kind, HANDSHAKE_TIMEOUT, 64).await;
    let t0 = Instant::now();
    let tasks: Vec<_> = (0..10)
        .map(|_| {
            let t = h.target.clone();
            tokio::spawn(async move { Conn::handshake(&t).await })
        })
        .collect();
    let mut ids = vec![];
    let mut keep = vec![];
    for t in tasks {
        let (c, w) = t.await.unwrap();
        ids.push(w.player_id);
        keep.push(c);
    }
    let dt = t0.elapsed().as_secs_f64() * 1e3;
    eprintln!("[{kind:?}] 10 concurrent handshakes (connect+Hello+Welcome): {dt:.1} ms total");
    assert!(dt < 1500.0, "{dt} ms");
    ids.sort();
    assert_eq!(ids, (0..10).collect::<Vec<u8>>());
    h.stop().await;
}
