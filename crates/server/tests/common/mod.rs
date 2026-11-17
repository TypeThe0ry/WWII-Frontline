//! Shared harness for the WebSocket integration tests: an in-process `WsListener` on an
//! ephemeral port plus a small client that speaks the frame format directly.
#![allow(dead_code)]

pub mod sim_harness;
pub mod synth;

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
use frontline_server::net::wt::{cert_hash, dev_identity, WtListener};
use frontline_server::sim_thread::Command;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use wtransport::tls::Sha256Digest;

pub const DATA_HASH: u64 = 0xDA7A_0000_1234_5678;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Ws,
    Wt,
}

impl Kind {
    pub const ALL: [Kind; 2] = [Kind::Ws, Kind::Wt];
}

/// Where a test client connects.
#[derive(Clone)]
pub struct Target {
    pub kind: Kind,
    pub url: String,
    /// Certificate hash for `serverCertificateHashes` (WebTransport only).
    pub hash: Option<Sha256Digest>,
}

pub struct Harness {
    pub listener: Option<WsListener>,
    pub wt: Option<WtListener>,
    pub target: Target,
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
        Self::start_kind(Kind::Ws, handshake_timeout, per_ip).await
    }

    pub async fn start_kind(kind: Kind, handshake_timeout: Duration, per_ip: usize) -> Harness {
        let metrics = Metrics::new();
        let (ctx_tx, commands) = crossbeam_channel::bounded(256);
        let inbound = InboundQueue::new();
        let tick: &'static AtomicU32 = Box::leak(Box::new(AtomicU32::new(0)));
        let ctx = Arc::new(NetCtx {
            metrics: metrics.clone(),
            // Zero reuse delay: these tests expect the smallest free PlayerId straight away
            // (the 1 s delay has its own test in session_flow).
            sessions: Sessions::with_reuse_delay(
                metrics.clone(),
                per_ip,
                frontline_server::net::limits::MAX_CONNECTIONS,
                Duration::ZERO,
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
        let (listener, wt, target) = match kind {
            Kind::Ws => {
                let l = WsListener::bind(("127.0.0.1", 0), ctx.clone())
                    .await
                    .expect("bind");
                let url = format!("ws://{}", l.local_addr());
                (
                    Some(l),
                    None,
                    Target {
                        kind,
                        url,
                        hash: None,
                    },
                )
            }
            Kind::Wt => {
                let id = dev_identity(13).expect("identity");
                let hash = cert_hash(&id);
                let l = WtListener::bind(([127, 0, 0, 1], 0).into(), id, ctx.clone())
                    .await
                    .expect("bind wt");
                let url = format!("https://127.0.0.1:{}/wt", l.local_addr().port());
                (
                    None,
                    Some(l),
                    Target {
                        kind,
                        url,
                        hash: Some(hash),
                    },
                )
            }
        };
        let url = target.url.clone();
        Harness {
            listener,
            wt,
            target,
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
        if let Some(l) = self.wt.take() {
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

// ---------------------------------------------------------------------------------------------
// Transport-agnostic client (WebSocket or WebTransport) for the parameterised tests.
// ---------------------------------------------------------------------------------------------

use frontline_server::net::datagram::{
    decode_datagram, encode_datagram, encode_stream_frame, StreamFramer,
};
use tokio::sync::mpsc;
use wtransport::{ClientConfig, Connection, Endpoint, SendStream};

pub struct WtClient {
    pub conn: Connection,
    pub c0_tx: SendStream,
    rx: mpsc::UnboundedReceiver<Got>,
    _endpoint: Endpoint<wtransport::endpoint::endpoint_side::Client>,
}

impl WtClient {
    pub async fn connect(url: &str, hash: Sha256Digest) -> Result<WtClient, String> {
        Self::connect_from([127, 0, 0, 1], url, hash).await
    }

    /// Binds the client socket to `ip` (any 127.x.y.z works on Linux), so many clients can look
    /// like different peers to the per-IP limit.
    pub async fn connect_from(
        ip: [u8; 4],
        url: &str,
        hash: Sha256Digest,
    ) -> Result<WtClient, String> {
        let cfg = ClientConfig::builder()
            .with_bind_address((ip, 0).into())
            .with_server_certificate_hashes([hash])
            .keep_alive_interval(Some(Duration::from_secs(3)))
            .build();
        let ep = Endpoint::client(cfg).map_err(|e| e.to_string())?;
        let conn = match tokio::time::timeout(Duration::from_secs(5), ep.connect(url)).await {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => return Err(e.to_string()),
            Err(_) => return Err("connect timeout".into()),
        };
        let (c0_tx, mut c0_rx) = conn
            .open_bi()
            .await
            .map_err(|e| e.to_string())?
            .await
            .map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::unbounded_channel();
        // C0 reader
        let t = tx.clone();
        tokio::spawn(async move {
            let mut f = StreamFramer::new(65_535);
            let mut buf = vec![0u8; 4096];
            loop {
                match c0_rx.read(&mut buf).await {
                    Ok(Some(n)) => {
                        f.push(&buf[..n]);
                        while let Ok(Some(p)) = f.next_frame() {
                            let _ = t.send(Got::Frame(Channel::C0, p.to_vec()));
                        }
                    }
                    _ => return,
                }
            }
        });
        // C4 (server uni stream) reader
        let t = tx.clone();
        let c = conn.clone();
        tokio::spawn(async move {
            let Ok(mut s) = c.accept_uni().await else {
                return;
            };
            let mut f = StreamFramer::new(65_535);
            let mut buf = vec![0u8; 4096];
            loop {
                match s.read(&mut buf).await {
                    Ok(Some(n)) => {
                        f.push(&buf[..n]);
                        while let Ok(Some(p)) = f.next_frame() {
                            let _ = t.send(Got::Frame(Channel::C4, p.to_vec()));
                        }
                    }
                    _ => return,
                }
            }
        });
        // datagrams
        let t = tx.clone();
        let c = conn.clone();
        tokio::spawn(async move {
            while let Ok(d) = c.receive_datagram().await {
                if let Ok((ch, p)) = decode_datagram(&d.payload()) {
                    let _ = t.send(Got::Frame(ch, p.to_vec()));
                }
            }
        });
        // closure
        let c = conn.clone();
        tokio::spawn(async move {
            c.closed().await;
            let _ = tx.send(Got::Closed);
        });
        Ok(WtClient {
            conn,
            c0_tx,
            rx,
            _endpoint: ep,
        })
    }

    pub async fn recv_within(&mut self, d: Duration) -> Got {
        match tokio::time::timeout(d, self.rx.recv()).await {
            Err(_) => Got::Timeout,
            Ok(Some(g)) => g,
            Ok(None) => Got::Closed,
        }
    }

    /// Waits for the connection to close (frames still arriving are discarded).
    pub async fn recv_closed(&mut self, d: Duration) -> bool {
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

    pub async fn send_c0(&mut self, payload: &[u8]) -> bool {
        match encode_stream_frame(payload) {
            Some(f) => self.c0_tx.write_all(&f).await.is_ok(),
            None => false,
        }
    }

    pub fn send_datagram(&self, ch: Channel, payload: &[u8]) -> bool {
        self.conn
            .send_datagram(encode_datagram(ch, payload))
            .is_ok()
    }
}

pub enum Conn {
    Ws(Box<Client>),
    Wt(Box<WtClient>),
}

impl Conn {
    pub async fn connect(t: &Target) -> Result<Conn, String> {
        match t.kind {
            Kind::Ws => Client::connect(&t.url).await.map(|c| Conn::Ws(Box::new(c))),
            Kind::Wt => WtClient::connect(&t.url, t.hash.clone().expect("hash"))
                .await
                .map(|c| Conn::Wt(Box::new(c))),
        }
    }

    /// Sends one message on `ch` the way that transport carries it: WebSocket envelope, C0 stream
    /// frame, or datagram `[ch][payload]` (C1, and illegal channels for the hostile-input tests).
    pub async fn send(&mut self, ch: Channel, payload: &[u8]) -> bool {
        match self {
            Conn::Ws(c) => c.send(ch, payload).await,
            Conn::Wt(c) => match ch {
                Channel::C0 => c.send_c0(payload).await,
                _ => c.send_datagram(ch, payload),
            },
        }
    }

    pub async fn recv_within(&mut self, d: Duration) -> Got {
        match self {
            Conn::Ws(c) => c.recv_within(d).await,
            Conn::Wt(c) => c.recv_within(d).await,
        }
    }

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

    pub async fn handshake(t: &Target) -> (Conn, Welcome) {
        let mut c = Conn::connect(t).await.expect("connect");
        c.send(Channel::C0, &good_hello()).await;
        match c.server_msg(Duration::from_secs(3)).await {
            Some(ServerMsgC0::Welcome(w)) => (c, w),
            other => panic!("expected Welcome, got {other:?}"),
        }
    }
}
