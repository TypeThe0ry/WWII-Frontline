//! WebTransport loopback tests (M0-017): datagram echo, ordered C0 / C4 streams, oversize
//! datagrams, hostile input. Uses the `wtransport` client with `serverCertificateHashes`.

mod common;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use bytes::Bytes;
use common::*;
use frontline_protocol::msg::c0::{encode_c0_server, Pong, ServerMsgC0};
use frontline_protocol::msg::input::{encode_input_bundle, InputBundle, InputCmd};
use frontline_protocol::Channel;
use frontline_server::net::datagram::{encode_stream_frame, packet_budget};
use frontline_server::net::echo::DevEcho;
use frontline_server::net::limits::HANDSHAKE_TIMEOUT;
use frontline_server::net::transport::Rel;

const T: Duration = Duration::from_secs(5);

async fn wt_harness(per_ip: usize) -> Harness {
    Harness::start_kind(Kind::Wt, HANDSHAKE_TIMEOUT, per_ip).await
}

async fn wt_connect(h: &Harness) -> (WtClient, frontline_protocol::msg::c0::Welcome) {
    let t = &h.target;
    let mut c = WtClient::connect(&t.url, t.hash.clone().unwrap())
        .await
        .expect("connect");
    assert!(c.send_c0(&good_hello()).await);
    loop {
        match c.recv_within(T).await {
            Got::Frame(Channel::C0, p) => match frontline_protocol::msg::c0::decode_c0_server(&p) {
                Ok(ServerMsgC0::Welcome(w)) => return (c, w),
                o => panic!("expected Welcome, got {o:?}"),
            },
            Got::Frame(..) => continue,
            _ => panic!("no Welcome"),
        }
    }
}

fn bundle(seq: u16) -> Vec<u8> {
    encode_input_bundle(&InputBundle {
        ack_snapshot_tick: seq,
        pred_hash: 0,
        cmds: [InputCmd {
            seq,
            tick: seq as u32 + 1,
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
    .unwrap()
}

fn pong(i: u32) -> Vec<u8> {
    encode_c0_server(&ServerMsgC0::Pong(Pong {
        client_time: i,
        server_tick: 0,
        server_time_ms: 0,
    }))
    .unwrap()
}

/// Acceptance 1a: 100 C1 datagrams in, 100 C2 echoes out, 100/100.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn datagram_echo_100_of_100() {
    let h = wt_harness(4).await;
    let echo = DevEcho::spawn(h.ctx.sessions.clone(), h.inbound.clone());
    let (mut c, w) = wt_connect(&h).await;
    for seq in 0..100u16 {
        assert!(c.send_datagram(Channel::C1, &bundle(seq)));
        tokio::time::sleep(Duration::from_millis(3)).await;
    }
    let mut got = vec![];
    let end = Instant::now() + T;
    while got.len() < 100 && Instant::now() < end {
        if let Got::Frame(Channel::C2, p) = c.recv_within(Duration::from_millis(200)).await {
            assert_eq!(p.len(), 4);
            got.push(u16::from_le_bytes([p[0], p[1]]));
        }
    }
    let in_order = got.windows(2).all(|w| w[0] < w[1]);
    eprintln!(
        "datagram echo: {}/100 received (player {}), in order: {in_order}, max_datagram_size {:?}",
        got.len(),
        w.player_id,
        c.conn.max_datagram_size()
    );
    got.sort_unstable();
    assert_eq!(got, (0..100u16).collect::<Vec<_>>(), "lost or duplicated");
    assert_eq!(h.metrics.net_c1_inbound_total.get(), 100);
    assert_eq!(h.metrics.net_datagram_oversize_total.get(), 0);
    echo.stop();
    h.stop().await;
}

/// Acceptance 1b: 1000 messages each on C0 (both directions) and C4, ordered, none lost.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn c0_and_c4_one_thousand_messages_in_order() {
    let h = wt_harness(4).await;
    let (mut c, w) = wt_connect(&h).await;
    let sh = h.ctx.sessions.handle_of(w.player_id).unwrap();

    // Server -> client: 1000 on C0 and 1000 on C4, interleaved, pushed in one burst.
    for i in 0..1000u32 {
        assert!(sh.send(Channel::C0, Bytes::from(pong(i)), Rel::Reliable));
        assert!(sh.send(Channel::C4, Bytes::copy_from_slice(&i.to_le_bytes()), Rel::Reliable));
    }
    let (mut n0, mut n4) = (0u32, 0u32);
    let end = Instant::now() + Duration::from_secs(10);
    while (n0 < 1000 || n4 < 1000) && Instant::now() < end {
        match c.recv_within(Duration::from_secs(1)).await {
            Got::Frame(Channel::C0, p) => {
                match frontline_protocol::msg::c0::decode_c0_server(&p).unwrap() {
                    ServerMsgC0::Pong(x) => assert_eq!(x.client_time, n0, "C0 order"),
                    o => panic!("{o:?}"),
                }
                n0 += 1;
            }
            Got::Frame(Channel::C4, p) => {
                assert_eq!(u32::from_le_bytes(p[..4].try_into().unwrap()), n4, "C4 order");
                n4 += 1;
            }
            Got::Closed => panic!("closed early"),
            _ => {}
        }
    }
    assert_eq!((n0, n4), (1000, 1000));

    // Client -> server -> client: 1000 pings (paced below the 150 frames/s limit), 1000 pongs.
    let t0 = Instant::now();
    let mut tick = tokio::time::interval(Duration::from_millis(8));
    let mut next = 0u32;
    let mut back = 0u32;
    while back < 1000 {
        tokio::select! {
            _ = tick.tick(), if next < 1000 => {
                assert!(c.send_c0(&ping_msg(next)).await);
                next += 1;
            }
            g = c.recv_within(Duration::from_secs(5)) => match g {
                Got::Frame(Channel::C0, p) => {
                    match frontline_protocol::msg::c0::decode_c0_server(&p).unwrap() {
                        ServerMsgC0::Pong(x) => assert_eq!(x.client_time, back, "pong order"),
                        o => panic!("{o:?}"),
                    }
                    back += 1;
                }
                Got::Closed | Got::Timeout => panic!("stalled at {back}"),
                _ => {}
            }
        }
    }
    eprintln!(
        "C0/C4: 1000+1000 server->client in order; 1000 ping/pong round trips in order ({:.1} s)",
        t0.elapsed().as_secs_f64()
    );
    assert_eq!(h.metrics.net_frames_malformed_total.get(), 0);
    h.stop().await;
}

/// Acceptance 5: a packet above `max_datagram_size` is dropped and counted, later packets flow.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn oversize_datagram_is_dropped_and_counted() {
    let h = wt_harness(4).await;
    let (mut c, w) = wt_connect(&h).await;
    let sh = h.ctx.sessions.handle_of(w.player_id).unwrap();
    let m = &h.metrics;
    assert_eq!(m.net_datagram_oversize_total.get(), 0);

    // Prime the writer so the connection's limit is published, then read it.
    sh.send(Channel::C2, Bytes::from_static(&[1]), Rel::Unreliable);
    assert!(matches!(c.recv_within(T).await, Got::Frame(Channel::C2, _)));
    let max = sh.max_datagram_size().expect("datagrams negotiated");
    eprintln!(
        "max_datagram_size {max}, PACKET_BUDGET {}",
        packet_budget(Some(max))
    );
    assert!(max >= 1100, "max_datagram_size {max}");
    assert!(packet_budget(Some(max)) >= 1050);

    // Way over any possible limit.
    // (Paced: the outbox itself keeps only the newest packet when it is backed up.)
    sh.send(Channel::C2, Bytes::from(vec![7u8; 3000]), Rel::Unreliable);
    assert!(wait_until(T, || m.net_datagram_oversize_total.get() == 1).await);
    sh.send(Channel::C3, Bytes::from(vec![7u8; 70_000]), Rel::Unreliable);
    assert!(wait_until(T, || m.net_datagram_oversize_total.get() == 2).await);
    sh.send(Channel::C2, Bytes::from_static(&[42, 42]), Rel::Unreliable);
    let mut seen = vec![];
    let end = Instant::now() + Duration::from_secs(2);
    while Instant::now() < end && seen.is_empty() {
        if let Got::Frame(Channel::C2, p) = c.recv_within(Duration::from_millis(200)).await {
            seen.push(p);
        }
    }
    assert_eq!(seen, vec![vec![42u8, 42]], "only the small packet arrives");
    assert_eq!(m.net_datagram_oversize_total.get(), 2);

    // Exact boundary: `[channel][payload]` == max fits, one more does not (skipped if PMTUD
    // moved the limit while we were looking).
    let max = c.conn.max_datagram_size().unwrap_or(0);
    let max_srv = sh.max_datagram_size().unwrap();
    if max_srv == max {
        let before = m.net_datagram_oversize_total.get();
        sh.send(Channel::C2, Bytes::from(vec![1u8; max - 1]), Rel::Unreliable);
        match c.recv_within(T).await {
            Got::Frame(Channel::C2, p) => assert_eq!(p.len(), max - 1),
            _ => panic!("full-size datagram lost"),
        }
        assert_eq!(m.net_datagram_oversize_total.get(), before);
        if sh.max_datagram_size() == Some(max_srv) {
            sh.send(Channel::C2, Bytes::from(vec![1u8; max]), Rel::Unreliable);
            assert!(wait_until(T, || m.net_datagram_oversize_total.get() == before + 1).await);
        }
    }
    // The session is unharmed.
    c.send_c0(&ping_msg(5)).await;
    assert!(matches!(c.recv_within(T).await, Got::Frame(Channel::C0, _)));
    assert_eq!(m.net_frames_malformed_total.get(), 0);
    h.stop().await;
}

/// Hostile and broken traffic on a WebTransport session: each case must end the session and
/// be counted exactly once; the server stays healthy.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn hostile_input_closes_the_session() {
    let h = wt_harness(64).await;
    let t = h.target.clone();
    let mut expected = 0u64;

    // Established session: datagram and stream abuse.
    enum Abuse {
        Dgram(Vec<u8>),
        Stream(Vec<u8>),
        ExtraBidi,
        ExtraUni,
    }
    let cases: Vec<(&str, Abuse)> = vec![
        ("empty datagram", Abuse::Dgram(vec![])),
        ("unknown datagram channel", Abuse::Dgram(vec![9, 1, 2])),
        ("C0 on a datagram", Abuse::Dgram(vec![0, 1, 2])),
        ("C4 on a datagram", Abuse::Dgram(vec![4, 1, 2])),
        ("C1 garbage", Abuse::Dgram(vec![1, 0xFF, 0xFF, 0xFF])),
        ("C1 empty", Abuse::Dgram(vec![1])),
        ("oversize stream frame header", Abuse::Stream(vec![0x01, 0x10])),
        ("max length header", Abuse::Stream(vec![0xFF, 0xFF])),
        ("garbage C0 message", Abuse::Stream(encode_stream_frame(&[0xEE, 1, 2]).unwrap())),
        ("empty C0 frame", Abuse::Stream(vec![0, 0])),
        ("second Hello", Abuse::Stream(encode_stream_frame(&good_hello()).unwrap())),
        ("second bidi stream", Abuse::ExtraBidi),
        ("client uni stream", Abuse::ExtraUni),
    ];
    for (name, a) in cases {
        let (mut c, _) = wt_connect(&h).await;
        match a {
            Abuse::Dgram(d) => {
                let (ch, p) = (d.first().copied(), d.get(1..).unwrap_or(&[]).to_vec());
                // Raw datagram: send_datagram adds nothing, so build the bytes ourselves.
                let _ = (ch, p);
                assert!(c.conn.send_datagram(d).is_ok(), "{name}");
            }
            Abuse::Stream(bytes) => {
                assert!(c.c0_tx.write_all(&bytes).await.is_ok(), "{name}");
            }
            Abuse::ExtraBidi => {
                let (mut s, _r) = c.conn.open_bi().await.unwrap().await.unwrap();
                let _ = s.write_all(&[1, 2, 3]).await;
            }
            Abuse::ExtraUni => {
                let mut s = c.conn.open_uni().await.unwrap().await.unwrap();
                let _ = s.write_all(&[1]).await;
            }
        }
        assert!(c.recv_closed(T).await, "{name}: session not closed");
        expected += 1;
        assert_eq!(h.metrics.net_frames_malformed_total.get(), expected, "{name}");
    }

    // Pre-handshake: anything but a valid Hello on C0 closes it.
    for (name, bytes) in [
        ("oversize header", vec![0xFF, 0xFF]),
        ("not a Hello", encode_stream_frame(&ping_msg(1)).unwrap()),
        ("empty frame", vec![0, 0]),
    ] {
        let mut c = WtClient::connect(&t.url, t.hash.clone().unwrap()).await.unwrap();
        assert!(c.c0_tx.write_all(&bytes).await.is_ok());
        assert!(c.recv_closed(T).await, "pre-handshake {name}");
        expected += 1;
        assert_eq!(h.metrics.net_frames_malformed_total.get(), expected, "{name}");
    }

    // Random garbage: whole frames with random payloads on C0 (pre- and post-handshake) and random
    // datagrams. ~300 sessions.
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut random_cases = 0;
    while random_cases < 300 {
        let n = (rng.next() % 80) as usize;
        let body = rng.bytes(n);
        let established = random_cases % 3 == 0;
        let as_datagram = established && random_cases % 6 == 0;
        // Skip the astronomically unlikely accidentally valid message.
        if frontline_protocol::msg::c0::decode_c0_client(&body).is_ok() && !as_datagram {
            continue;
        }
        if as_datagram {
            if let Some((&first, rest)) = body.split_first() {
                if first == 1 && frontline_protocol::msg::input::decode_input_bundle(rest).is_ok() {
                    continue;
                }
            }
        }
        let mut c = if established {
            wt_connect(&h).await.0
        } else {
            WtClient::connect(&t.url, t.hash.clone().unwrap()).await.unwrap()
        };
        if as_datagram {
            let _ = c.conn.send_datagram(body);
        } else {
            let _ = c.c0_tx.write_all(&encode_stream_frame(&body).unwrap()).await;
        }
        assert!(c.recv_closed(T).await, "random case {random_cases} not closed");
        expected += 1;
        random_cases += 1;
    }
    assert_eq!(h.metrics.net_frames_malformed_total.get(), expected);
    eprintln!("hostile WT input: {expected} cases, every session closed and counted once");

    // Healthy afterwards.
    let (_c, w) = wt_connect(&h).await;
    assert!(w.player_id < 128);
    h.stop().await;
}

/// A client that sends half a frame and finishes its stream is a disconnect, not an attack.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn half_frame_then_finish_is_a_plain_disconnect() {
    let h = wt_harness(4).await;
    let (mut c, _) = wt_connect(&h).await;
    c.c0_tx.write_all(&[10, 0, 1, 2]).await.unwrap(); // announces 10 bytes, sends 2
    c.c0_tx.finish().await.unwrap();
    assert!(c.recv_closed(T).await);
    assert_eq!(h.metrics.net_frames_malformed_total.get(), 0);
    h.stop().await;
}

/// Server shutdown closes every session.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shutdown_closes_sessions() {
    let h = wt_harness(4).await;
    let (mut c, _) = wt_connect(&h).await;
    h.stop().await;
    assert!(c.recv_closed(T).await);
}

/// A burst of C1 datagrams beyond the rate limit ends the session (same limiter as WebSocket).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn c1_flood_is_rate_limited() {
    let h = wt_harness(4).await;
    let (mut c, _) = wt_connect(&h).await;
    for seq in 0..1500u16 {
        if c.conn.send_datagram(frontline_server::net::datagram::encode_datagram(Channel::C1, &bundle(seq))).is_err() {
            break;
        }
        if seq % 100 == 99 {
            tokio::task::yield_now().await;
        }
    }
    assert!(c.recv_closed(T).await);
    assert!(
        h.metrics
            .net_rejects_total
            .with_label_values(&["rate_limit"])
            .get()
            >= 1
    );
    h.tick.store(1, Ordering::Relaxed);
    h.stop().await;
}
