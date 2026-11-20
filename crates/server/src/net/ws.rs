//! WebSocket transport (`ws://`; TLS is terminated in front, see NEEDS_HUMAN H-017).
//! One binary message = one `[channel][len u16][payload]` frame (ARCHITECTURE §6.6.2).

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use frontline_protocol::framing::{decode_ws_frame, encode_ws_frame, FRAME_HEADER_LEN};
use frontline_protocol::Channel;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{timeout_at, Instant};
use tokio_tungstenite::tungstenite::error::{CapacityError, Error as WsError, ProtocolError};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use super::conn::{run_connection, AdmitError, NetCtx};
use super::limits::*;
use super::transport::{ClientHandle, ClientTransport, Recv};

// Round up so even an immediately ready writer cannot exceed 15 Hz.
const SNAPSHOT_INTERVAL: Duration = Duration::from_nanos(1_000_000_000_u64.div_ceil(15));

type Ws = WebSocketStream<BackpressureIo<TcpStream>>;

/// Observe backpressure below tungstenite (a pending WebSocket sink can also just mean its
/// read half holds the split lock). This is not a count of bytes queued in the kernel or proxy.
struct BackpressureIo<S> {
    inner: S,
    handle: ClientHandle,
    blocked: bool,
}

impl<S> BackpressureIo<S> {
    fn new(inner: S, handle: ClientHandle) -> Self {
        Self {
            inner,
            handle,
            blocked: false,
        }
    }

    fn set_blocked(&mut self, blocked: bool) {
        if self.blocked != blocked {
            self.blocked = blocked;
            self.handle.set_ws_backlog(blocked);
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for BackpressureIo<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for BackpressureIo<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let result = Pin::new(&mut self.inner).poll_write(cx, buf);
        if result.is_pending() && !buf.is_empty() {
            self.set_blocked(true);
        }
        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let result = Pin::new(&mut self.inner).poll_flush(cx);
        // Tungstenite only flushes the underlying stream after draining its write buffer.
        if matches!(result, Poll::Ready(Ok(()))) {
            self.set_blocked(false);
        }
        result
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

pub struct WsTransport {
    handle: ClientHandle,
    rx: SplitStream<Ws>,
    ip: IpAddr,
}

impl ClientTransport for WsTransport {
    fn handle(&self) -> ClientHandle {
        self.handle.clone()
    }

    fn peer_ip(&self) -> IpAddr {
        self.ip
    }

    fn kind(&self) -> &'static str {
        "ws"
    }

    async fn recv(&mut self) -> Recv {
        loop {
            let msg = tokio::select! {
                _ = self.handle.closed() => return Recv::Closed,
                m = self.rx.next() => m,
            };
            match msg {
                Some(Ok(Message::Binary(b))) => {
                    self.handle.count_in(b.len());
                    return match decode_ws_frame(&b) {
                        Ok((ch, payload)) => Recv::Frame(ch, b.slice_ref(payload)),
                        Err(_) => Recv::Malformed("bad frame envelope"),
                    };
                }
                Some(Ok(Message::Text(_))) => return Recv::Malformed("text message"),
                // WebSocket-level ping/pong are answered by tungstenite itself.
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => continue,
                Some(Ok(Message::Close(_))) | None => return Recv::Closed,
                Some(Err(WsError::Capacity(CapacityError::MessageTooLong { .. }))) => {
                    return Recv::Malformed("oversize message")
                }
                // Peer vanished without a Close frame: an ordinary disconnect, not hostile input.
                Some(Err(WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake))) => {
                    return Recv::Closed
                }
                Some(Err(WsError::Protocol(_) | WsError::Utf8)) => {
                    return Recv::Malformed("websocket protocol violation")
                }
                Some(Err(_)) => return Recv::Closed,
            }
        }
    }
}

async fn writer<S: AsyncRead + AsyncWrite + Unpin>(
    mut sink: SplitSink<WebSocketStream<S>, Message>,
    handle: ClientHandle,
) {
    let mut snapshot_not_before = Instant::now();
    while let Some((ch, bytes)) = handle.next_ws_outbound(snapshot_not_before).await {
        let Ok(frame) = encode_ws_frame(ch, &bytes) else {
            handle.close_now();
            break;
        };
        let n = frame.len();
        tokio::select! {
            r = sink.send(Message::Binary(Bytes::from(frame))) => {
                if r.is_err() {
                    handle.close_now();
                    break;
                }
                handle.write_done(n);
                if ch == Channel::C2 {
                    // Base the next deadline on a completed flush, not the original attempt:
                    // after a stall we must not send a burst of queued snapshots.
                    snapshot_not_before = Instant::now() + SNAPSHOT_INTERVAL;
                }
            }
            _ = handle.closed() => break,
        }
    }
    let _ = tokio::time::timeout(Duration::from_secs(1), sink.close()).await;
}

fn ws_config() -> WebSocketConfig {
    let max = FRAME_HEADER_LEN + MAX_INBOUND_PAYLOAD;
    WebSocketConfig::default()
        .max_message_size(Some(max))
        .max_frame_size(Some(max))
        .write_buffer_size(0)
        .max_write_buffer_size(MAX_BUFFERED_BYTES)
}

pub struct WsListener {
    addr: SocketAddr,
    stop: watch::Sender<bool>,
    join: JoinHandle<()>,
}

impl WsListener {
    /// Binds and starts accepting. Use port 0 for an ephemeral port (see [`Self::local_addr`]).
    pub async fn bind(addr: impl tokio::net::ToSocketAddrs, ctx: Arc<NetCtx>) -> io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let addr = listener.local_addr()?;
        let (stop, stop_rx) = watch::channel(false);
        let join = tokio::spawn(accept_loop(listener, ctx, stop_rx));
        Ok(WsListener { addr, stop, join })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Stops accepting and closes every open connection.
    pub async fn shutdown(self) {
        let _ = self.stop.send(true);
        let _ = self.join.await;
    }
}

async fn accept_loop(listener: TcpListener, ctx: Arc<NetCtx>, mut stop: watch::Receiver<bool>) {
    loop {
        let (sock, peer) = tokio::select! {
            _ = stop.changed() => break,
            r = listener.accept() => match r {
                Ok(x) => x,
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }
            },
        };
        let deadline = Instant::now() + ctx.handshake_timeout;
        let ip = peer.ip().to_canonical();
        let admission = match ctx.sessions.admit(ip) {
            Ok(a) => a,
            Err(e) => {
                let label = if e == AdmitError::IpLimit {
                    "ip_limit"
                } else {
                    "conn_limit"
                };
                ctx.metrics
                    .net_rejects_total
                    .with_label_values(&[label])
                    .inc();
                drop(sock); // refused before the upgrade: costs us nothing
                continue;
            }
        };
        let _ = sock.set_nodelay(true);
        let ctx = ctx.clone();
        let mut stop = stop.clone();
        tokio::spawn(async move {
            let handle = ClientHandle::new(ctx.metrics.clone());
            handle.mark_websocket();
            let io = BackpressureIo::new(sock, handle.clone());
            let ws = match timeout_at(
                deadline,
                tokio_tungstenite::accept_async_with_config(io, Some(ws_config())),
            )
            .await
            {
                Ok(Ok(ws)) => ws,
                Ok(Err(_)) => return,
                Err(_) => {
                    ctx.metrics.net_handshake_timeout_total.inc();
                    return;
                }
            };
            let (sink, rx) = ws.split();
            let writer_task = tokio::spawn(writer(sink, handle.clone()));
            let t = WsTransport {
                handle: handle.clone(),
                rx,
                ip,
            };
            tokio::select! {
                _ = run_connection(ctx, t, deadline, admission) => {}
                _ = stop.wait_for(|s| *s) => handle.close_now(),
            }
            let _ = writer_task.await;
        });
    }
    // Dropping `stop` receivers' sender side is handled by shutdown(); connections observe it above.
}

#[cfg(test)]
mod tests {
    use std::future::{poll_fn, Future};

    use tokio::io::duplex;
    use tokio::time::{sleep, timeout};
    use tokio_tungstenite::tungstenite::protocol::Role;

    use super::*;
    use crate::metrics::Metrics;
    use crate::net::transport::Rel;

    async fn read_frame<S: AsyncRead + AsyncWrite + Unpin>(
        peer: &mut WebSocketStream<S>,
    ) -> (Channel, u8) {
        let Message::Binary(frame) = peer.next().await.unwrap().unwrap() else {
            panic!("expected a binary WebSocket message");
        };
        let (ch, body) = decode_ws_frame(&frame).unwrap();
        (ch, body[0])
    }

    #[tokio::test]
    async fn stalled_ws_writer_coalesces_c2_and_paces_recovery() {
        let metrics = Metrics::new();
        let handle = ClientHandle::new(metrics.clone());
        handle.mark_websocket();
        // Exercise the real WebSocket codec and writer with a deterministic 64-byte send buffer.
        // Neither any one frame nor the aggregate outbox reaches the 8 KiB byte threshold.
        let (server, peer) = duplex(64);
        let server = WebSocketStream::from_raw_socket(
            BackpressureIo::new(server, handle.clone()),
            Role::Server,
            Some(ws_config()),
        )
        .await;
        let mut peer = WebSocketStream::from_raw_socket(peer, Role::Client, None).await;
        let (sink, _rx) = server.split();
        let mut writer = std::pin::pin!(writer(sink, handle.clone()));

        let snapshot = |tag| {
            assert!(handle.send(Channel::C2, Bytes::from(vec![tag; 256]), Rel::Unreliable));
        };
        snapshot(1);
        // Start the first frame and stop polling once the underlying write hits the buffer limit.
        poll_fn(|cx| {
            assert!(writer.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        snapshot(2);
        assert!(handle.send(Channel::C3, Bytes::from_static(&[3]), Rel::Unreliable));
        snapshot(4);
        snapshot(5);
        assert!(handle.send(Channel::C0, Bytes::from_static(&[6]), Rel::Reliable));
        assert_eq!(handle.stats().snapshots_dropped, 2);
        assert_eq!(metrics.ws_snapshot_dropped_total.get(), 2);
        assert_eq!(metrics.net_snapshots_dropped_total.get(), 2);

        sleep(SNAPSHOT_INTERVAL).await;
        let before_drain = Instant::now();
        let read = async {
            assert_eq!(read_frame(&mut peer).await, (Channel::C2, 1));
            assert_eq!(read_frame(&mut peer).await, (Channel::C0, 6));
            assert_eq!(read_frame(&mut peer).await, (Channel::C3, 3));
            assert_eq!(read_frame(&mut peer).await, (Channel::C2, 5));
            assert!(before_drain.elapsed() >= SNAPSHOT_INTERVAL);
        };
        tokio::select! {
            _ = writer.as_mut() => panic!("writer exited before draining the queue"),
            result = timeout(Duration::from_secs(3), read) => result.unwrap(),
        }
        handle.close_graceful();
        drop(peer);
        timeout(Duration::from_secs(2), writer).await.unwrap();
    }
}
