//! Transport-independent client session abstraction. The server only talks to
//! [`ClientTransport`]; WebSocket (M0-016) and WebTransport (M0-017) implement it.
//!
//! Outbound data never blocks the sim thread: [`ClientHandle::send`] only touches a small
//! mutex-protected queue and wakes the transport's writer task.

use std::collections::VecDeque;
use std::future::Future;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use bytes::Bytes;
use frontline_protocol::framing::FRAME_HEADER_LEN;
use frontline_protocol::Channel;
use tokio::sync::{watch, Notify};

use super::limits::{MAX_BUFFERED_BYTES, UNRELIABLE_QUEUE_CAP, WRITE_BACKLOG_LIMIT};
use crate::metrics::Metrics;

/// Delivery class of an outbound message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rel {
    /// C0/C4: never dropped, ordered. A backlog above 64 KB disconnects the client.
    Reliable,
    /// C2/C3: newest wins; stale packets are dropped.
    Unreliable,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransportStats {
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub snapshots_dropped: u64,
    pub queued_reliable_bytes: usize,
}

/// One result of [`ClientTransport::recv`].
#[derive(Debug)]
pub enum Recv {
    Frame(Channel, Bytes),
    /// Not a decodable frame (bad envelope, text message, oversize message...).
    Malformed(&'static str),
    /// Peer went away or the connection was closed locally.
    Closed,
}

pub trait ClientTransport: Send {
    /// Cloneable sending side (also used by the session layer to push snapshots).
    fn handle(&self) -> ClientHandle;
    fn peer_ip(&self) -> IpAddr;
    /// Transport label for metrics and logs: `"ws"` or `"wt"`.
    fn kind(&self) -> &'static str;
    /// Next inbound frame. Cancel-safe.
    fn recv(&mut self) -> impl Future<Output = Recv> + Send;

    fn send(&self, ch: Channel, bytes: Bytes, rel: Rel) -> bool {
        self.handle().send(ch, bytes, rel)
    }
    /// Flush queued reliable data, then close.
    fn close(&self) {
        self.handle().close_graceful()
    }
    fn stats(&self) -> TransportStats {
        self.handle().stats()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CloseState {
    Open,
    /// Drain the reliable queue, then close.
    Graceful,
    /// Close now, discard queues.
    Hard,
}

struct OutState {
    reliable: VecDeque<(Channel, Bytes)>,
    reliable_bytes: usize,
    latest: VecDeque<(Channel, Bytes)>,
    latest_bytes: usize,
    /// Bytes handed to the socket writer and not yet flushed.
    inflight: usize,
    close: CloseState,
    paused: bool,
}

/// Shared outbound queue of one connection.
pub struct Outbox {
    state: Mutex<OutState>,
    notify: Notify,
    closed_tx: watch::Sender<bool>,
    metrics: Arc<Metrics>,
    pub(crate) bytes_in: AtomicU64,
    pub(crate) bytes_out: AtomicU64,
    snapshots_dropped: AtomicU64,
    /// Current datagram size limit of the connection (0 = no datagram path, e.g. WebSocket).
    max_datagram: AtomicUsize,
    team_markers: std::sync::atomic::AtomicBool,
}

/// Cloneable handle to a connection's outbound side.
#[derive(Clone)]
pub struct ClientHandle(pub(crate) Arc<Outbox>);

impl ClientHandle {
    pub fn new(metrics: Arc<Metrics>) -> ClientHandle {
        let (closed_tx, _) = watch::channel(false);
        ClientHandle(Arc::new(Outbox {
            state: Mutex::new(OutState {
                reliable: VecDeque::new(),
                reliable_bytes: 0,
                latest: VecDeque::new(),
                latest_bytes: 0,
                inflight: 0,
                close: CloseState::Open,
                paused: false,
            }),
            notify: Notify::new(),
            closed_tx,
            metrics,
            bytes_in: AtomicU64::new(0),
            bytes_out: AtomicU64::new(0),
            snapshots_dropped: AtomicU64::new(0),
            max_datagram: AtomicUsize::new(0),
            team_markers: std::sync::atomic::AtomicBool::new(false),
        }))
    }

    pub(crate) fn enable_team_markers(&self) {
        self.0.team_markers.store(true, Ordering::Relaxed);
    }

    pub(crate) fn accepts_team_markers(&self) -> bool {
        self.0.team_markers.load(Ordering::Relaxed)
    }

    fn lock(&self) -> MutexGuard<'_, OutState> {
        self.0.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Queues a message. Never blocks. Returns `false` when it was refused (connection closing,
    /// or the reliable backlog exceeded 64 KB which disconnects the slow client).
    pub fn send(&self, ch: Channel, bytes: Bytes, rel: Rel) -> bool {
        let len = bytes.len() + FRAME_HEADER_LEN;
        let mut s = self.lock();
        if s.close != CloseState::Open {
            return false;
        }
        match rel {
            Rel::Reliable => {
                if s.reliable_bytes + len > MAX_BUFFERED_BYTES {
                    drop(s);
                    self.0
                        .metrics
                        .net_rejects_total
                        .with_label_values(&["slow_client"])
                        .inc();
                    self.close_now();
                    return false;
                }
                s.reliable_bytes += len;
                s.reliable.push_back((ch, bytes));
            }
            Rel::Unreliable => {
                let mut dropped = 0u64;
                if s.inflight + s.latest_bytes > WRITE_BACKLOG_LIMIT {
                    // Socket is backed up: keep only the newest packet.
                    dropped += s.latest.len() as u64;
                    s.latest.clear();
                    s.latest_bytes = 0;
                } else if s.latest.len() >= UNRELIABLE_QUEUE_CAP {
                    if let Some((_, old)) = s.latest.pop_front() {
                        s.latest_bytes -= old.len() + FRAME_HEADER_LEN;
                        dropped += 1;
                    }
                }
                s.latest_bytes += len;
                s.latest.push_back((ch, bytes));
                if dropped > 0 {
                    self.0
                        .snapshots_dropped
                        .fetch_add(dropped, Ordering::Relaxed);
                    self.0.metrics.net_snapshots_dropped_total.inc_by(dropped);
                }
            }
        }
        drop(s);
        self.0.notify.notify_one();
        true
    }

    /// Flush the reliable queue, then close the connection.
    pub fn close_graceful(&self) {
        let mut s = self.lock();
        if s.close == CloseState::Open {
            s.close = CloseState::Graceful;
        }
        drop(s);
        self.0.notify.notify_one();
    }

    /// Close immediately and discard anything queued.
    pub fn close_now(&self) {
        let mut s = self.lock();
        s.close = CloseState::Hard;
        s.reliable.clear();
        s.latest.clear();
        s.reliable_bytes = 0;
        s.latest_bytes = 0;
        drop(s);
        self.0.closed_tx.send_replace(true);
        self.0.notify.notify_one();
    }

    /// Largest `[channel][payload]` datagram the connection can carry right now (QUIC
    /// `max_datagram_size`, updated by the WebTransport writer). `None` on transports without a
    /// datagram path (WebSocket) or before the first update.
    pub fn max_datagram_size(&self) -> Option<usize> {
        match self.0.max_datagram.load(Ordering::Relaxed) {
            0 => None,
            n => Some(n),
        }
    }

    pub(crate) fn set_max_datagram_size(&self, n: Option<usize>) {
        self.0.max_datagram.store(n.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn is_closed(&self) -> bool {
        self.lock().close == CloseState::Hard
    }

    pub(crate) fn is_open(&self) -> bool {
        self.lock().close == CloseState::Open
    }

    /// Resolves once the connection was hard-closed.
    pub async fn closed(&self) {
        let mut rx = self.0.closed_tx.subscribe();
        let _ = rx.wait_for(|c| *c).await;
    }

    pub fn stats(&self) -> TransportStats {
        TransportStats {
            bytes_in: self.0.bytes_in.load(Ordering::Relaxed),
            bytes_out: self.0.bytes_out.load(Ordering::Relaxed),
            snapshots_dropped: self.0.snapshots_dropped.load(Ordering::Relaxed),
            queued_reliable_bytes: self.lock().reliable_bytes,
        }
    }

    /// Test hook: while paused the writer task takes nothing from the queues (simulates a
    /// consumer that has stopped reading).
    #[doc(hidden)]
    pub fn set_writer_paused(&self, paused: bool) {
        self.lock().paused = paused;
        self.0.notify.notify_one();
    }

    // ---- writer side (used by transports) ----

    /// Next message to write: reliable first, then the oldest queued unreliable packet.
    /// `None` = the writer should close the connection now.
    pub async fn next_outbound(&self) -> Option<(Channel, Bytes)> {
        loop {
            {
                let mut s = self.lock();
                if s.close == CloseState::Hard {
                    return None;
                }
                if !s.paused {
                    if let Some(x) = s.reliable.pop_front() {
                        let n = x.1.len() + FRAME_HEADER_LEN;
                        s.reliable_bytes -= n;
                        s.inflight = n;
                        return Some(x);
                    }
                    if let Some(x) = s.latest.pop_front() {
                        let n = x.1.len() + FRAME_HEADER_LEN;
                        s.latest_bytes -= n;
                        s.inflight = n;
                        return Some(x);
                    }
                    if s.close == CloseState::Graceful {
                        return None;
                    }
                }
            }
            self.0.notify.notified().await;
        }
    }

    /// The writer finished flushing the message returned by [`Self::next_outbound`].
    pub fn write_done(&self, sent: usize) {
        self.lock().inflight = 0;
        self.0.bytes_out.fetch_add(sent as u64, Ordering::Relaxed);
        self.0.metrics.net_bytes_out_total.inc_by(sent as u64);
    }

    /// Test hook: pretend `n` bytes sit unflushed in the socket buffer.
    #[doc(hidden)]
    pub fn set_inflight_for_test(&self, n: usize) {
        self.lock().inflight = n;
    }

    pub(crate) fn count_in(&self, n: usize) {
        self.0.bytes_in.fetch_add(n as u64, Ordering::Relaxed);
        self.0.metrics.net_bytes_in_total.inc_by(n as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h() -> (ClientHandle, Arc<Metrics>) {
        let m = Metrics::new();
        (ClientHandle::new(m.clone()), m)
    }
    fn b(n: usize, tag: u8) -> Bytes {
        Bytes::from(vec![tag; n])
    }

    #[tokio::test]
    async fn unreliable_queue_keeps_newest_four() {
        let (h, m) = h();
        for i in 0..10u8 {
            assert!(h.send(Channel::C2, b(100, i), Rel::Unreliable));
        }
        let mut got = vec![];
        h.close_graceful();
        while let Some((_, x)) = h.next_outbound().await {
            got.push(x[0]);
            h.write_done(x.len());
        }
        assert_eq!(got, vec![6, 7, 8, 9]);
        assert_eq!(m.net_snapshots_dropped_total.get(), 6);
        assert_eq!(h.stats().snapshots_dropped, 6);
    }

    #[tokio::test]
    async fn backed_up_socket_keeps_only_latest() {
        let (h, m) = h();
        h.set_inflight_for_test(WRITE_BACKLOG_LIMIT + 1);
        for i in 0..5u8 {
            h.send(Channel::C2, b(50, i), Rel::Unreliable);
        }
        h.close_graceful();
        let mut got = vec![];
        while let Some((_, x)) = h.next_outbound().await {
            got.push(x[0]);
        }
        assert_eq!(got, vec![4]);
        assert_eq!(m.net_snapshots_dropped_total.get(), 4);
    }

    #[tokio::test]
    async fn reliable_is_ordered_and_never_dropped_until_64k() {
        let (h, m) = h();
        for i in 0..100u8 {
            assert!(h.send(Channel::C0, b(10, i), Rel::Reliable));
        }
        h.close_graceful();
        let mut n = 0u8;
        while let Some((_, x)) = h.next_outbound().await {
            assert_eq!(x[0], n);
            n += 1;
        }
        assert_eq!(n, 100);
        assert_eq!(m.net_snapshots_dropped_total.get(), 0);
    }

    #[tokio::test]
    async fn reliable_backlog_over_64k_disconnects() {
        let (h, m) = h();
        let mut refused = false;
        for _ in 0..100 {
            if !h.send(Channel::C4, b(1000, 0), Rel::Reliable) {
                refused = true;
                break;
            }
        }
        assert!(refused);
        assert!(h.is_closed());
        assert!(h.next_outbound().await.is_none());
        assert_eq!(
            m.net_rejects_total
                .with_label_values(&["slow_client"])
                .get(),
            1
        );
    }
}
