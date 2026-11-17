//! WebTransport (HTTP/3 over QUIC) front end (M0-017, ARCHITECTURE §5.5 / §6.6.1).
//!
//! Session setup: QUIC + TLS + HTTP/3 CONNECT -> wait for the client's first bidi stream (C0,
//! it carries `Hello`) -> open the server's first uni stream (C4) -> hand the session to
//! [`run_connection`], the same handshake / read loop the WebSocket path uses. The 5 s handshake
//! deadline starts when the first QUIC packet arrives and covers all of it.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{timeout_at, Instant};
use wtransport::endpoint::{endpoint_side, IncomingSession};
use wtransport::tls::Sha256Digest;
use wtransport::{Endpoint, Identity, ServerConfig};

use super::conn::{run_connection, AdmitError, NetCtx};
use super::transport::ClientHandle;
use super::wt_conn::{writer, WtConn};

/// QUIC keep-alive from the server so idle NAT bindings survive and dead peers are noticed.
const KEEP_ALIVE: Duration = Duration::from_secs(3);
/// A peer silent for this long is dropped (a healthy client sends 30 Hz input).
const IDLE_TIMEOUT: Duration = Duration::from_secs(15);
/// Request paths served; anything else gets a 404 on the CONNECT.
pub const WT_PATHS: &[&str] = &["/", "/wt"];
/// Longest validity of a development certificate: browsers accept `serverCertificateHashes`
/// certificates for at most 14 days; stay one day below.
pub const DEV_CERT_DAYS: u32 = 13;

/// Development certificate (ECDSA P-256, self-signed, `days` valid from now) for
/// `localhost`, `127.0.0.1` and `::1`. Clients connect with `serverCertificateHashes`.
pub fn dev_identity(days: u32) -> Result<Identity, wtransport::tls::error::InvalidSan> {
    Identity::self_signed_builder()
        .subject_alt_names(["localhost", "127.0.0.1", "::1"])
        .from_now_utc()
        .validity_days(days)
        .build()
}

/// SHA-256 of the leaf certificate, the value `serverCertificateHashes` expects.
pub fn cert_hash(identity: &Identity) -> Sha256Digest {
    identity.certificate_chain().as_slice()[0].hash()
}

/// Lower-case hex of a digest (64 chars).
pub fn hash_hex(d: &Sha256Digest) -> String {
    d.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Standard base64 (with padding) of a digest.
pub fn hash_base64(d: &Sha256Digest) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in d.as_ref().chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

pub struct WtListener {
    addr: SocketAddr,
    stop: watch::Sender<bool>,
    join: JoinHandle<()>,
}

impl WtListener {
    /// Binds the UDP socket and starts accepting. Port 0 picks an ephemeral port
    /// (see [`Self::local_addr`]).
    pub async fn bind(addr: SocketAddr, identity: Identity, ctx: Arc<NetCtx>) -> io::Result<Self> {
        let config = ServerConfig::builder()
            .with_bind_address(addr)
            .with_identity(identity)
            .keep_alive_interval(Some(KEEP_ALIVE))
            .max_idle_timeout(Some(IDLE_TIMEOUT))
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?
            .build();
        let endpoint = Endpoint::server(config)?;
        let addr = endpoint.local_addr()?;
        let (stop, stop_rx) = watch::channel(false);
        let join = tokio::spawn(accept_loop(endpoint, ctx, stop_rx));
        Ok(WtListener { addr, stop, join })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Stops accepting and closes every open session.
    pub async fn shutdown(self) {
        let _ = self.stop.send(true);
        let _ = self.join.await;
    }
}

async fn accept_loop(
    endpoint: Endpoint<endpoint_side::Server>,
    ctx: Arc<NetCtx>,
    mut stop: watch::Receiver<bool>,
) {
    loop {
        // `Endpoint::accept` panics once the endpoint is closed, so it is only ever dropped
        // (never closed) while this loop awaits it.
        let incoming = tokio::select! {
            _ = stop.changed() => break,
            i = endpoint.accept() => i,
        };
        let deadline = Instant::now() + ctx.handshake_timeout;
        // Admission before any TLS work. Unvalidated source addresses (possibly spoofed) first
        // prove they can receive packets (QUIC Retry), so they cannot burn the per-IP slots of
        // somebody else.
        if !incoming.remote_address_validated() {
            incoming.retry();
            continue;
        }
        let ip = incoming.remote_address().ip().to_canonical();
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
                incoming.refuse();
                continue;
            }
        };
        let ctx = ctx.clone();
        let stop = stop.clone();
        tokio::spawn(async move {
            serve_session(ctx, incoming, ip, deadline, admission, stop).await;
        });
    }
    endpoint.close(wtransport::VarInt::from_u32(0), b"server shutdown");
    let _ = tokio::time::timeout(Duration::from_secs(1), endpoint.wait_idle()).await;
}

async fn serve_session(
    ctx: Arc<NetCtx>,
    incoming: IncomingSession,
    ip: std::net::IpAddr,
    deadline: Instant,
    admission: super::conn::Admission,
    mut stop: watch::Receiver<bool>,
) {
    let timed_out = || ctx.metrics.net_handshake_timeout_total.inc();

    let request = match timeout_at(deadline, incoming).await {
        Ok(Ok(r)) => r,
        Ok(Err(_)) => return,
        Err(_) => return timed_out(),
    };
    if !WT_PATHS.contains(&request.path()) {
        request.not_found().await;
        return;
    }
    let conn = match timeout_at(deadline, request.accept()).await {
        Ok(Ok(c)) => c,
        Ok(Err(_)) => return,
        Err(_) => return timed_out(),
    };

    // The client opens C0 first; QUIC shows the stream to us with its first bytes (the Hello).
    let (c0_tx, c0_rx) = match timeout_at(deadline, conn.accept_bi()).await {
        Ok(Ok(s)) => s,
        Ok(Err(_)) => return,
        Err(_) => {
            timed_out();
            conn.close(wtransport::VarInt::from_u32(1), b"handshake timeout");
            return;
        }
    };
    let c4_tx = match timeout_at(deadline, async {
        let opening = conn.open_uni().await.map_err(|_| ())?;
        opening.await.map_err(|_| ())
    })
    .await
    {
        Ok(Ok(s)) => s,
        Ok(Err(_)) => return,
        Err(_) => {
            timed_out();
            conn.close(wtransport::VarInt::from_u32(1), b"handshake timeout");
            return;
        }
    };

    let handle = ClientHandle::new(ctx.metrics.clone());
    let writer_task = tokio::spawn(writer(
        conn.clone(),
        c0_tx,
        c4_tx,
        handle.clone(),
        ctx.metrics.clone(),
    ));
    let t = WtConn::new(handle.clone(), conn, c0_rx, ip, ctx.metrics.clone());
    tokio::select! {
        _ = run_connection(ctx, t, deadline, admission) => {}
        _ = stop.wait_for(|s| *s) => handle.close_now(),
    }
    let _ = writer_task.await;
}
