//! Handshake, limits and hostile-input tests for the WebSocket transport (M0-016).

mod common;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use common::*;
use frontline_protocol::channel::c0_type;
use frontline_protocol::framing::encode_ws_frame;
use frontline_protocol::msg::c0::{Disconnect, RejectReason, ServerMsgC0};
use frontline_protocol::msg::input::{encode_input_bundle, InputBundle, InputCmd};
use frontline_protocol::{BitWriter, Channel, PROTOCOL_VERSION};
use frontline_server::sim_thread::Command;
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;

const T: Duration = Duration::from_secs(3);

fn rejects(h: &Harness, l: &str) -> u64 {
    h.metrics.net_rejects_total.with_label_values(&[l]).get()
}

#[tokio::test]
async fn welcome_fields_player_ids_and_client_gone() {
    let h = Harness::start().await;
    h.tick.store(77, Ordering::Relaxed);
    let (mut a, wa) = Client::handshake(&h.url).await;
    let (b, wb) = Client::handshake(&h.url).await;
    assert_eq!((wa.player_id, wb.player_id), (0, 1));
    assert_eq!(wa.server_tick, 77);
    assert_eq!(wa.tick_hz, 30);
    assert_eq!(wa.match_seed, 0xABCD);
    assert_eq!(wa.data_hash, DATA_HASH);
    assert_eq!(wa.client_tick_offset, 0);
    assert_eq!(h.metrics.net_connections.get(), 2);

    // Ping -> Pong echo with the published tick.
    h.tick.store(78, Ordering::Relaxed);
    a.send(Channel::C0, &ping_msg(0xDEADBEEF)).await;
    match a.server_msg(T).await {
        Some(ServerMsgC0::Pong(p)) => {
            assert_eq!(p.client_time, 0xDEADBEEF);
            assert_eq!(p.server_tick, 78);
        }
        o => panic!("{o:?}"),
    }

    // Closing player 0 publishes ClientGone and frees the id for reuse.
    drop(a);
    assert!(
        wait_until(T, || h
            .commands
            .try_iter()
            .any(|c| matches!(c, Command::ClientGone(0))))
        .await
    );
    let (_c, wc) = Client::handshake(&h.url).await;
    assert_eq!(wc.player_id, 0);
    drop(b);
    h.stop().await;
}

#[tokio::test]
async fn version_and_data_hash_mismatch_are_rejected_then_closed() {
    let h = Harness::start().await;
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
        let mut c = Client::connect(&h.url).await.unwrap();
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
    // No player was ever created.
    assert!(h.ctx.sessions.handle_of(0).is_none());
    h.stop().await;
}

#[tokio::test]
async fn no_hello_closes_after_five_seconds() {
    let h = Harness::start().await;
    let mut c = Client::connect(&h.url).await.unwrap();
    let t0 = Instant::now();
    assert!(
        c.expect_closed(Duration::from_secs(8)).await,
        "never closed"
    );
    let dt = t0.elapsed().as_secs_f64();
    eprintln!("handshake timeout: connection closed after {dt:.3} s");
    assert!((4.95..=5.5).contains(&dt), "closed after {dt} s");
    assert_eq!(h.metrics.net_handshake_timeout_total.get(), 1);
    h.stop().await;
}

#[tokio::test]
async fn ticket_of_257_bytes_disconnects() {
    let h = Harness::start().await;
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
    let mut c = Client::connect(&h.url).await.unwrap();
    c.send(Channel::C0, &w.finish()).await;
    assert!(c.expect_closed(T).await);
    assert_eq!(h.metrics.net_frames_malformed_total.get(), 1);
    // A 256 byte ticket is fine (M0 does not validate tickets).
    let mut c = Client::connect(&h.url).await.unwrap();
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

#[tokio::test]
async fn fifth_connection_from_one_ip_is_refused() {
    let h = Harness::start().await;
    let mut keep = vec![];
    for _ in 0..4 {
        keep.push(Client::handshake(&h.url).await);
    }
    let fifth = Client::connect(&h.url).await;
    assert!(
        fifth.is_err(),
        "5th connection from the same IP must be refused"
    );
    assert_eq!(rejects(&h, "ip_limit"), 1);
    // Freed slot is usable again.
    keep.pop();
    assert!(wait_until(T, || h.ctx.sessions.connections() == 3).await);
    assert!(Client::connect(&h.url).await.is_ok());
    h.stop().await;
}

#[tokio::test]
async fn illegal_frames_close_the_connection() {
    let h = Harness::start().await;
    let big = vec![0u8; 4097];
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("oversize C0", encode_ws_frame(Channel::C0, &big).unwrap()),
        ("oversize C1", encode_ws_frame(Channel::C1, &big).unwrap()),
        ("unknown channel", vec![9, 0, 0]),
        ("len lies", vec![0, 5, 0, 1]),
        ("empty message", vec![]),
        (
            "client sends C2",
            encode_ws_frame(Channel::C2, &[1]).unwrap(),
        ),
        (
            "client sends C4",
            encode_ws_frame(Channel::C4, &[1]).unwrap(),
        ),
        (
            "unknown msg_type",
            encode_ws_frame(Channel::C0, &[0xEE]).unwrap(),
        ),
        (
            "server-only msg (Welcome)",
            encode_ws_frame(Channel::C0, &[c0_type::WELCOME; 40]).unwrap(),
        ),
        (
            "second Hello",
            encode_ws_frame(Channel::C0, &good_hello()).unwrap(),
        ),
        (
            "garbage C1",
            encode_ws_frame(Channel::C1, &[0xFF; 9]).unwrap(),
        ),
    ];
    let mut expected = 0;
    for (name, frame) in cases {
        for established in [false, true] {
            // "second Hello" only makes sense after the handshake.
            if name == "second Hello" && !established {
                continue;
            }
            let mut c = if established {
                Client::handshake(&h.url).await.0
            } else {
                Client::connect(&h.url).await.unwrap()
            };
            c.send_raw(frame.clone()).await;
            assert!(
                c.expect_closed(T).await,
                "{name} (established={established}) not closed"
            );
            expected += 1;
            assert_eq!(
                h.metrics.net_frames_malformed_total.get(),
                expected,
                "{name}"
            );
        }
    }
    // Text message
    let mut c = Client::connect(&h.url).await.unwrap();
    c.ws.send(Message::Text("hello".into())).await.unwrap();
    assert!(c.expect_closed(T).await);
    // Oversize WebSocket message (way beyond the frame limit).
    let mut c = Client::handshake(&h.url).await.0;
    c.send_raw(vec![0u8; 70_000]).await;
    assert!(c.expect_closed(T).await);
    assert!(h.metrics.net_frames_malformed_total.get() > expected);
    h.stop().await;
}

#[tokio::test]
async fn valid_c1_is_queued_and_disconnect_is_clean() {
    let h = Harness::start().await;
    let (mut c, w) = Client::handshake(&h.url).await;
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
    assert!(c.expect_closed(T).await);
    assert_eq!(h.metrics.net_frames_malformed_total.get(), 0);
    h.stop().await;
}

#[tokio::test]
async fn flooding_is_rate_limited() {
    let h = Harness::start().await;
    let (mut c, _) = Client::handshake(&h.url).await;
    for i in 0..2000u32 {
        if !c.send(Channel::C0, &ping_msg(i)).await {
            break;
        }
    }
    assert!(c.expect_closed(T).await);
    assert_eq!(rejects(&h, "rate_limit"), 1);
    h.stop().await;
}

/// 10^4 random byte frames (pre-handshake and after it) and every strict prefix of valid frames.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn malformed_frames_never_panic_and_close_the_connection() {
    // Per-IP limit lifted: connections of finished iterations may still be draining server-side.
    let h = std::sync::Arc::new(Harness::start_cfg(Duration::from_secs(5), 64).await);
    let bundle = encode_input_bundle(&InputBundle {
        ack_snapshot_tick: 1,
        pred_hash: 0,
        cmds: [InputCmd {
            seq: 1,
            tick: 2,
            move_x: 0,
            move_z: 0,
            yaw: 0,
            pitch: 0,
            buttons: 0,
            weapon_slot: 0,
            aux: 0,
            view_tick: 0,
        }]
        .into_iter()
        .collect(),
        fire: Default::default(),
    })
    .unwrap();
    let valid: Vec<(bool, Vec<u8>)> = vec![
        (false, encode_ws_frame(Channel::C0, &good_hello()).unwrap()),
        (true, encode_ws_frame(Channel::C0, &ping_msg(1)).unwrap()),
        (
            true,
            encode_ws_frame(
                Channel::C0,
                &frontline_protocol::msg::c0::encode_c0_client(
                    &frontline_protocol::msg::c0::ClientMsgC0::Disconnect(Disconnect { reason: 1 }),
                )
                .unwrap(),
            )
            .unwrap(),
        ),
        (true, encode_ws_frame(Channel::C1, &bundle).unwrap()),
    ];
    // Work items: (established, bytes).
    let mut work: Vec<(bool, Vec<u8>)> = vec![];
    for (est, f) in &valid {
        for n in 0..f.len() {
            work.push((*est, f[..n].to_vec()));
        }
    }
    let prefixes = work.len();
    let mut rng = Rng(0x9E3779B97F4A7C15);
    while work.len() < prefixes + 10_000 {
        let n = (rng.next() % 80) as usize;
        let b = rng.bytes(n);
        // Skip the (astronomically unlikely) accidentally valid frame.
        if let Ok((ch, p)) = frontline_protocol::framing::decode_ws_frame(&b) {
            if (ch == Channel::C0 || ch == Channel::C1)
                && frontline_protocol::msg::c0::decode_c0_client(p).is_ok()
            {
                continue;
            }
        }
        let established = work.len().is_multiple_of(8);
        work.push((established, b));
    }
    let total = work.len();
    let work = std::sync::Arc::new(std::sync::Mutex::new(work));
    let t0 = Instant::now();
    let gone = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut tasks = vec![];
    for _ in 0..4 {
        let gone = gone.clone();
        let h = h.clone();
        let work = work.clone();
        tasks.push(tokio::spawn(async move {
            let mut closed = 0usize;
            loop {
                let Some((est, bytes)) = work.lock().unwrap().pop() else {
                    break;
                };
                let mut c = if est {
                    Client::handshake(&h.url).await.0
                } else {
                    match Client::connect(&h.url).await {
                        Ok(c) => c,
                        Err(e) => panic!(
                            "connect failed: {e}; conns={} rejects ip={} conn={}",
                            h.ctx.sessions.connections(),
                            rejects(&h, "ip_limit"),
                            rejects(&h, "conn_limit")
                        ),
                    }
                };
                c.send_raw(bytes.clone()).await;
                assert!(
                    c.expect_closed(Duration::from_secs(3)).await,
                    "connection not closed for frame {bytes:?} (established={est})"
                );
                closed += 1;
                // Nobody runs a sim thread here: keep the command channel from filling up.
                gone.fetch_add(h.commands.try_iter().count(), Ordering::Relaxed);
            }
            closed
        }));
    }
    let mut closed = 0;
    for t in tasks {
        closed += t.await.expect("worker panicked");
    }
    let malformed = h.metrics.net_frames_malformed_total.get() as usize;
    eprintln!(
        "malformed test: {total} frames ({prefixes} truncated prefixes), all connections closed ({closed}), malformed counter {malformed}, {:.1} s",
        t0.elapsed().as_secs_f64()
    );
    assert_eq!(closed, total);
    // Everything except the empty-prefix and rate/graceful cases is counted; each closed
    // connection corresponds to exactly one malformed frame.
    assert_eq!(malformed, total);
    // Server still healthy afterwards.
    let (_c, w) = Client::handshake(&h.url).await;
    assert!(w.player_id < 128);
    // The sim command channel only ever sees ClientGone events (no spawn without M0-018).
    assert!(h
        .commands
        .try_iter()
        .all(|c| matches!(c, Command::ClientGone(_))));
    eprintln!("ClientGone events seen: {}", gone.load(Ordering::Relaxed));
    let h = std::sync::Arc::try_unwrap(h).ok().expect("sole owner");
    h.stop().await;
}

#[tokio::test]
async fn handshake_latency_and_parallel_handshakes() {
    let h = Harness::start().await;
    // Sequential: Hello -> Welcome round trip.
    let mut rtts = vec![];
    for _ in 0..40 {
        let mut c = Client::connect(&h.url).await.unwrap();
        let t0 = Instant::now();
        c.send(Channel::C0, &good_hello()).await;
        assert!(matches!(
            c.server_msg(T).await,
            Some(ServerMsgC0::Welcome(_))
        ));
        rtts.push(t0.elapsed().as_secs_f64() * 1e3);
    }
    rtts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = rtts[rtts.len() / 2];
    eprintln!(
        "Hello->Welcome RTT over 40 sequential handshakes: P50 {p50:.2} ms, max {:.2} ms",
        rtts[rtts.len() - 1]
    );
    assert!(p50 < 5.0, "P50 {p50} ms");
    assert!(wait_until(T, || h.ctx.sessions.connections() == 0).await);

    h.stop().await;

    // 10 concurrent clients doing connect + Hello + Welcome. The per-IP limit is lifted for this
    // measurement (all test clients share 127.0.0.1).
    let h = Harness::start_cfg(Duration::from_secs(5), 64).await;
    let t0 = Instant::now();
    let tasks: Vec<_> = (0..10)
        .map(|_| {
            let url = h.url.clone();
            tokio::spawn(async move { Client::handshake(&url).await })
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
    eprintln!("10 concurrent handshakes (connect+Hello+Welcome): {dt:.1} ms total");
    assert!(dt < 500.0, "{dt} ms");
    ids.sort();
    assert_eq!(ids, (0..10).collect::<Vec<u8>>());
    h.stop().await;
}
