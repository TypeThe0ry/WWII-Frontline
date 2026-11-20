//! One WebTransport session as a [`ClientTransport`] (ARCHITECTURE §6.6.1).
//!
//! * C0: the client's first bidi stream, `[len u16][payload]` frames in both directions.
//! * C1: client -> server datagrams `[ch u8][payload]`.
//! * C2/C3: server -> client datagrams; a packet above the live `max_datagram_size` is dropped and
//!   counted (`net_datagram_oversize_total`), never fragmented.
//! * C4: the server's first uni stream, `[len u16][payload]` frames.
//! * C5: one short-lived uni stream per message.
//!
//! Datagrams may be lost, duplicated or reordered; nothing here retransmits (M0-018 relies on
//! input redundancy).

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use frontline_protocol::Channel;
use wtransport::error::StreamReadError;
use wtransport::{Connection, RecvStream, SendStream, VarInt};

use super::datagram::{
    datagram_fits, decode_datagram, encode_datagram, encode_stream_frame, StreamFramer,
    STREAM_HEADER_LEN,
};
use super::limits::MAX_INBOUND_PAYLOAD;
use super::transport::{ClientHandle, ClientTransport, Recv};
use crate::metrics::Metrics;

/// Close code of an orderly server-side close.
const CLOSE_NORMAL: u32 = 0;
/// Close code sent when the connection is dropped for misbehaviour / backlog.
const CLOSE_DROPPED: u32 = 1;
/// How long a graceful close waits for the peer to acknowledge the finished streams.
const FLUSH_WAIT: Duration = Duration::from_secs(1);
/// After the flush, how long the peer gets to close the session itself.
const CLOSE_GRACE: Duration = Duration::from_millis(500);

pub struct WtConn {
    handle: ClientHandle,
    conn: Connection,
    c0_rx: RecvStream,
    framer: StreamFramer,
    ip: IpAddr,
    metrics: Arc<Metrics>,
    chunk: Vec<u8>,
}

impl WtConn {
    pub(crate) fn new(
        handle: ClientHandle,
        conn: Connection,
        c0_rx: RecvStream,
        ip: IpAddr,
        metrics: Arc<Metrics>,
    ) -> Self {
        WtConn {
            handle,
            conn,
            c0_rx,
            framer: StreamFramer::new(MAX_INBOUND_PAYLOAD),
            ip,
            metrics,
            chunk: vec![0u8; 2048],
        }
    }

    /// Live QUIC `max_datagram_size` of this session (largest `[channel][payload]` datagram), for
    /// the snapshot sender's `PACKET_BUDGET = min(1150, max_datagram_size - 1)` (M0-019).
    pub fn max_datagram_size(&self) -> Option<usize> {
        self.conn.max_datagram_size()
    }
}

impl ClientTransport for WtConn {
    fn handle(&self) -> ClientHandle {
        self.handle.clone()
    }

    fn peer_ip(&self) -> IpAddr {
        self.ip
    }

    fn kind(&self) -> &'static str {
        "wt"
    }

    async fn recv(&mut self) -> Recv {
        loop {
            match self.framer.next_frame() {
                Ok(Some(p)) => {
                    self.handle.count_in(p.len() + STREAM_HEADER_LEN);
                    return Recv::Frame(Channel::C0, p);
                }
                Err(_) => return Recv::Malformed("oversize message"),
                Ok(None) => {}
            }
            tokio::select! {
                _ = self.handle.closed() => return Recv::Closed,
                dg = self.conn.receive_datagram() => {
                    let Ok(dg) = dg else { return Recv::Closed };
                    let payload = dg.payload();
                    self.handle.count_in(payload.len());
                    self.metrics.net_datagrams_in_total.inc();
                    return match decode_datagram(&payload) {
                        // Datagrams carry C1 only; everything else belongs on a stream or goes
                        // server -> client.
                        Ok((Channel::C1, p)) => Recv::Frame(Channel::C1, p),
                        Ok(_) => Recv::Malformed("datagram on a non-C1 channel"),
                        Err(_) => Recv::Malformed("bad datagram envelope"),
                    };
                }
                r = self.c0_rx.read(&mut self.chunk) => match r {
                    Ok(Some(n)) => self.framer.push(&self.chunk[..n]),
                    // Peer finished or reset its C0 stream: C0 is persistent, so this is a goodbye.
                    Ok(None) => return Recv::Closed,
                    Err(StreamReadError::Reset(_) | StreamReadError::NotConnected) => {
                        return Recv::Closed
                    }
                    Err(StreamReadError::QuicProto) => return Recv::Closed,
                },
                s = self.conn.accept_bi() => {
                    return match s {
                        Ok(_) => Recv::Malformed("unexpected bidi stream"),
                        Err(_) => Recv::Closed,
                    };
                }
                s = self.conn.accept_uni() => {
                    return match s {
                        Ok(_) => Recv::Malformed("unexpected uni stream"),
                        Err(_) => Recv::Closed,
                    };
                }
            }
        }
    }
}

/// Outbound pump of one session. Returns when the connection is finished; by then the QUIC
/// connection is closed.
pub(crate) async fn writer(
    conn: Connection,
    mut c0_tx: SendStream,
    mut c4_tx: SendStream,
    handle: ClientHandle,
    metrics: Arc<Metrics>,
) {
    handle.set_max_datagram_size(conn.max_datagram_size());
    let mut last_sample = None;
    tracing::debug!(
        event = "wt_session",
        max_datagram_size = conn.max_datagram_size().map(|n| n as u64).unwrap_or(0),
        rtt_ms = conn.rtt().as_secs_f64() * 1e3,
        "session established"
    );
    loop {
        let item = tokio::select! {
            i = handle.next_outbound() => i,
            _ = conn.closed() => {
                handle.close_now();
                break;
            }
        };
        let Some((ch, bytes)) = item else { break };
        let now = std::time::Instant::now();
        if last_sample.is_none_or(|t| now - t >= Duration::from_millis(100)) {
            handle.update_wt_sample(super::transport_stats::webtransport_sample(&conn));
            last_sample = Some(now);
        }
        let sent = match ch {
            Channel::C0 => write_frame(&mut c0_tx, &bytes, &handle).await,
            Channel::C4 => write_frame(&mut c4_tx, &bytes, &handle).await,
            Channel::C5 => send_bulk(&conn, bytes, &handle).await,
            Channel::C2 | Channel::C3 => send_dgram(&conn, ch, &bytes, &handle, &metrics),
            // The server never sends C1; nothing to do but not to wedge the queue.
            Channel::C1 => Some(0),
        };
        match sent {
            Some(n) => handle.write_done(n),
            None => {
                handle.close_now();
                break;
            }
        }
    }

    if handle.is_closed() {
        conn.close(VarInt::from_u32(CLOSE_DROPPED), b"closed");
        return;
    }
    // Graceful: everything queued has been handed to QUIC. Finish the streams and give the peer
    // a moment to acknowledge before the CONNECTION_CLOSE that would discard unacked data.
    let _ = c0_tx.finish().await;
    let _ = c4_tx.finish().await;
    let _ = tokio::time::timeout(FLUSH_WAIT, async {
        let _ = c0_tx.quic_stream_mut().stopped().await;
        let _ = c4_tx.quic_stream_mut().stopped().await;
    })
    .await;
    // Let the peer read what it was sent (a `Reject`, a last C4 event) and close first: a
    // CONNECTION_CLOSE can make the peer discard stream data it has not yet handed to the app.
    let _ = tokio::time::timeout(CLOSE_GRACE, conn.closed()).await;
    conn.close(VarInt::from_u32(CLOSE_NORMAL), b"bye");
}

/// `Some(bytes written)` on success, `None` when the connection is unusable.
async fn write_frame(tx: &mut SendStream, payload: &Bytes, handle: &ClientHandle) -> Option<usize> {
    let frame = encode_stream_frame(payload)?;
    tokio::select! {
        r = tx.write_all(&frame) => r.ok().map(|_| frame.len()),
        _ = handle.closed() => None,
    }
}

/// C5: a fresh uni stream per bulk message, written by its own task so a slow bulk transfer
/// never blocks C0/C4 or datagrams.
async fn send_bulk(conn: &Connection, payload: Bytes, handle: &ClientHandle) -> Option<usize> {
    let frame = encode_stream_frame(&payload)?;
    let n = frame.len();
    let opening = conn.open_uni().await.ok()?;
    let handle = handle.clone();
    tokio::spawn(async move {
        let Ok(mut s) = opening.await else { return };
        let r = tokio::select! {
            r = s.write_all(&frame) => r.is_ok(),
            _ = handle.closed() => false,
        };
        if r {
            let _ = s.finish().await;
        }
    });
    Some(n)
}

fn send_dgram(
    conn: &Connection,
    ch: Channel,
    payload: &Bytes,
    handle: &ClientHandle,
    metrics: &Metrics,
) -> Option<usize> {
    let max = conn.max_datagram_size();
    handle.set_max_datagram_size(max);
    if !datagram_fits(max, payload.len()) {
        metrics.net_datagram_oversize_total.inc();
        return Some(0);
    }
    match conn.send_datagram(encode_datagram(ch, payload)) {
        Ok(()) => {
            metrics.net_datagrams_out_total.inc();
            Some(payload.len() + 1)
        }
        // A datagram can overshoot when the path MTU estimate shrank in between.
        Err(wtransport::error::SendDatagramError::TooLarge) => {
            metrics.net_datagram_oversize_total.inc();
            Some(0)
        }
        Err(_) => None,
    }
}
