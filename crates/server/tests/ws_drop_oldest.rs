//! Back-pressure semantics: C0/C4 reliable and ordered, C2/C3 newest-wins (M0-016).

mod common;

use std::time::Duration;

use bytes::Bytes;
use common::*;
use frontline_protocol::Channel;
use frontline_server::net::transport::Rel;

fn pkt(seq: u32, len: usize) -> Bytes {
    let mut v = vec![0u8; len.max(4)];
    v[..4].copy_from_slice(&seq.to_be_bytes());
    Bytes::from(v)
}

fn seq(p: &[u8]) -> u32 {
    u32::from_be_bytes(p[..4].try_into().unwrap())
}

/// 100 C2 packets pushed at a consumer that is not reading: only the newest survive, in order;
/// the 100 interleaved C0 packets all arrive, in order.
#[tokio::test]
async fn slow_consumer_gets_latest_snapshots_and_all_reliable() {
    let h = Harness::start().await;
    let (mut c, w) = Client::handshake(&h.url).await;
    let handle = h.ctx.sessions.handle_of(w.player_id).unwrap();
    handle.set_writer_paused(true);
    for i in 0..100u32 {
        assert!(handle.send(Channel::C2, pkt(i, 300), Rel::Unreliable));
        assert!(handle.send(Channel::C0, pkt(i, 20), Rel::Reliable));
    }
    handle.set_writer_paused(false);

    let (mut c0, mut c2) = (vec![], vec![]);
    while c0.len() < 100 || c2.is_empty() || c2.last() != Some(&99) {
        match c.recv_within(Duration::from_secs(3)).await {
            Got::Frame(Channel::C0, p) => c0.push(seq(&p)),
            Got::Frame(Channel::C2, p) => c2.push(seq(&p)),
            Got::Frame(..) => panic!("unexpected channel"),
            Got::Closed | Got::Timeout => break,
        }
    }
    // Let anything else that might be in flight show up.
    while let Got::Frame(ch, p) = c.recv_within(Duration::from_millis(200)).await {
        if ch == Channel::C2 {
            c2.push(seq(&p))
        } else {
            c0.push(seq(&p))
        }
    }
    eprintln!(
        "C0 received {}/100 in order; C2 received {:?}",
        c0.len(),
        c2
    );
    assert_eq!(
        c0,
        (0..100).collect::<Vec<_>>(),
        "C0 must be complete and ordered"
    );
    assert_eq!(*c2.last().unwrap(), 99, "latest C2 must arrive");
    assert!(c2.len() <= 4, "at most last + 3 earlier: {c2:?}");
    assert!(c2.windows(2).all(|w| w[0] < w[1]), "monotonic: {c2:?}");
    assert_eq!(
        h.metrics.net_snapshots_dropped_total.get(),
        100 - c2.len() as u64
    );
    h.stop().await;
}

/// A client that never reads: the kernel buffers fill, the writer stalls, and the server must
/// (a) start dropping stale snapshots and (b) disconnect when the reliable backlog exceeds 64 KB.
#[tokio::test]
async fn non_reading_client_is_handled_without_unbounded_memory() {
    let h = Harness::start().await;
    let (_c, w) = Client::handshake(&h.url).await; // never reads again
    let handle = h.ctx.sessions.handle_of(w.player_id).unwrap();

    // Unreliable flood: the per-client queue is bounded (cap 4), stale snapshots are dropped.
    let mut sent = 0u32;
    while h.metrics.net_snapshots_dropped_total.get() == 0 {
        assert!(handle.send(Channel::C2, pkt(sent, 3000), Rel::Unreliable));
        sent += 1;
        if sent.is_multiple_of(64) {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(sent < 2_000_000, "never started dropping");
    }
    eprintln!("first snapshot dropped after {sent} x 3000 B packets");
    assert!(handle.stats().snapshots_dropped > 0);

    // Reliable flood: backlog above 64 KB disconnects the slow client.
    let mut i = 0u32;
    while handle.send(Channel::C4, pkt(i, 4000), Rel::Reliable) {
        i += 1;
        if i.is_multiple_of(16) {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(i < 2_000_000, "slow client never disconnected");
    }
    assert!(handle.is_closed());
    assert_eq!(
        h.metrics
            .net_rejects_total
            .with_label_values(&["slow_client"])
            .get(),
        1
    );
    assert!(wait_until(Duration::from_secs(3), || h.ctx.sessions.connections() == 0).await);
    h.stop().await;
}
