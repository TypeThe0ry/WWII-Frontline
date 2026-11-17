//! WebSocket transport (`ws://`; TLS is terminated in front, see NEEDS_HUMAN H-017).
//! One binary message = one `[channel][len u16][payload]` frame (ARCHITECTURE §6.6.2).

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use bytes::Bytes;
use frontline_protocol::framing::{decode_ws_frame, encode_ws_frame, FRAME_HEADER_LEN};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{timeout_at, Instant};
use tokio_tungstenite::tungstenite::error::{CapacityError, Error as WsError};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use super::conn::{run_connection, AdmitError, NetCtx};
use super::limits::*;
use super::transport::{ClientHandle, ClientTransport, Recv};

type Ws = WebSocketStream<TcpStream>;

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
                Some(Err(WsError::Protocol(_) | WsError::Utf8)) => {
                    return Recv::Malformed("websocket protocol violation")
                }
                Some(Err(_)) => return Recv::Closed,
            }
        }
    }
}

async fn writer(mut sink: SplitSink<Ws, Message>, handle: ClientHandle) {
    while let Some((ch, bytes)) = handle.next_outbound().await {
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
            }
            _ = handle.closed() => break,
        }
    }
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), sink.close()).await;
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
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
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
            let ws = match timeout_at(
                deadline,
                tokio_tungstenite::accept_async_with_config(sock, Some(ws_config())),
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
            let handle = ClientHandle::new(ctx.metrics.clone());
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
