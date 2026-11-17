//! Shared harness for the WebSocket integration tests: an in-process `WsListener` on an
//! ephemeral port plus a small client that speaks the frame format directly.
#![allow(dead_code)]

use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use frontline_protocol::framing::{decode_ws_frame, encode_ws_frame};
use frontline_protocol::msg::c0::{
    decode_c0_server, encode_c0_client, ClientMsgC0, Hello, Ping, ServerMsgC0, Welcome,
};
use frontline_protocol::{Channel, PROTOCOL_VERSION};
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::{Inbound, InboundQueue, NetCtx, Sessions};
use frontline_server::net::limits::HANDSHAKE_TIMEOUT;
use frontline_server::net::ws::WsListener;
use frontline_server::sim_thread::Command;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub const DATA_HASH: u64 = 0xDA7A_0000_1234_5678;

pub struct Harness {
    pub listener: Option<WsListener>,
    pub ctx: Arc<NetCtx>,
    pub metrics: Arc<Metrics>,
    pub commands: Receiver<Command>,
    pub inbound: Receiver<Inbound>,
    pub tick: &'static AtomicU32,
    pub url: String,
}

impl Harness {
    pub async fn start() -> Harness {
        Self::start_with(HANDSHAKE_TIMEOUT).await
    }

    pub async fn start_with(handshake_timeout: Duration) -> Harness {
        Self::start_cfg(
            handshake_timeout,
            frontline_server::net::limits::MAX_CONN_PER_IP,
        )
        .await
    }

    pub async fn start_cfg(handshake_timeout: Duration, per_ip: usize) -> Harness {
        let metrics = Metrics::new();
        let (ctx_tx, commands) = crossbeam_channel::bounded(256);
        let inbound = InboundQueue::new();
        let tick: &'static AtomicU32 = Box::leak(Box::new(AtomicU32::new(0)));
        let ctx = Arc::new(NetCtx {
            metrics: metrics.clone(),
            sessions: Sessions::with_limits(
                metrics.clone(),
                per_ip,
                frontline_server::net::limits::MAX_CONNECTIONS,
            ),
            inbound: inbound.tx.clone(),
            commands: ctx_tx,
            data_hash: DATA_HASH,
            match_seed: 0xABCD,
            tick_hz: 30,
            map_id: 0,
            mode: 0,
            server_tick: tick,
            started: Instant::now(),
            handshake_timeout,
        });
        let listener = WsListener::bind(("127.0.0.1", 0), ctx.clone())
            .await
            .expect("bind");
        let url = format!("ws://{}", listener.local_addr());
        Harness {
            listener: Some(listener),
            ctx,
            metrics,
            commands,
            inbound: inbound.rx,
            tick,
            url,
        }
    }

    pub async fn stop(mut self) {
        if let Some(l) = self.listener.take() {
            l.shutdown().await;
        }
    }
}

pub enum Got {
    Frame(Channel, Vec<u8>),
    Closed,
    Timeout,
}

pub struct Client {
    pub ws: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
}

pub fn hello_msg(version: u16, hash: u64, ticket: Vec<u8>) -> Vec<u8> {
    encode_c0_client(&ClientMsgC0::Hello(Hello {
        protocol_version: version,
        data_hash: hash,
        ticket,
        client_build: 1,
        caps: 0,
    }))
    .unwrap()
}

pub fn good_hello() -> Vec<u8> {
    hello_msg(PROTOCOL_VERSION, DATA_HASH, vec![])
}

pub fn ping_msg(t: u32) -> Vec<u8> {
    encode_c0_client(&ClientMsgC0::Ping(Ping { client_time: t })).unwrap()
}

impl Client {
    pub async fn connect(url: &str) -> Result<Client, String> {
        match tokio::time::timeout(Duration::from_secs(5), connect_async(url)).await {
            Ok(Ok((ws, _))) => Ok(Client { ws }),
            Ok(Err(e)) => Err(e.to_string()),
            Err(_) => Err("connect timeout".into()),
        }
    }

    pub async fn send_raw(&mut self, bytes: Vec<u8>) -> bool {
        self.ws.send(Message::Binary(bytes.into())).await.is_ok()
    }

    pub async fn send(&mut self, ch: Channel, payload: &[u8]) -> bool {
        self.send_raw(encode_ws_frame(ch, payload).unwrap()).await
    }

    pub async fn recv_within(&mut self, d: Duration) -> Got {
        match tokio::time::timeout(d, self.ws.next()).await {
            Err(_) => Got::Timeout,
            Ok(Some(Ok(Message::Binary(b)))) => match decode_ws_frame(&b) {
                Ok((ch, p)) => Got::Frame(ch, p.to_vec()),
                Err(_) => Got::Closed,
            },
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) | Ok(Some(Err(_))) => Got::Closed,
            Ok(Some(Ok(_))) => Box::pin(self.recv_within(d)).await,
        }
    }

    /// Waits until the server closes the connection (discarding anything it still sends).
    pub async fn expect_closed(&mut self, d: Duration) -> bool {
        let end = Instant::now() + d;
        loop {
            let left = end.saturating_duration_since(Instant::now());
            match self.recv_within(left).await {
                Got::Closed => return true,
                Got::Timeout => return false,
                Got::Frame(..) => {}
            }
        }
    }

    pub async fn server_msg(&mut self, d: Duration) -> Option<ServerMsgC0> {
        match self.recv_within(d).await {
            Got::Frame(Channel::C0, p) => decode_c0_server(&p).ok(),
            _ => None,
        }
    }

    /// Connect + Hello, returns the `Welcome`.
    pub async fn handshake(url: &str) -> (Client, Welcome) {
        let mut c = Client::connect(url).await.expect("connect");
        c.send(Channel::C0, &good_hello()).await;
        match c.server_msg(Duration::from_secs(3)).await {
            Some(ServerMsgC0::Welcome(w)) => (c, w),
            other => panic!("expected Welcome, got {other:?}"),
        }
    }
}

/// Deterministic xorshift64* for fuzz-style tests.
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
}

/// Polls until `f` is true or `d` elapses.
pub async fn wait_until(d: Duration, mut f: impl FnMut() -> bool) -> bool {
    let end = Instant::now() + d;
    while Instant::now() < end {
        if f() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    f()
}
