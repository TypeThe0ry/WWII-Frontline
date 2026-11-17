//! Per-connection logic shared by every transport: admission control, the handshake state
//! machine, the post-handshake read loop and cleanup.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bytes::Bytes;
use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use frontline_protocol::msg::c0::{
    decode_c0_client, encode_c0_server, ClientMsgC0, Pong, Reject, RejectReason, ServerMsgC0,
    Welcome,
};
use frontline_protocol::msg::input::{decode_input_bundle, InputBundle};
use frontline_protocol::Channel;
use frontline_sim::{PlayerId, Team};
use tokio::time::timeout_at;

use super::handshake::{check_hello, reject_label};
use super::limits::*;
use super::transport::{ClientHandle, ClientTransport, Recv, Rel};
use crate::metrics::Metrics;
use crate::sim_thread::Command;

/// Valid C1 bundles waiting for the session layer (M0-018 drains this).
#[derive(Debug, Clone)]
pub struct Inbound {
    pub player: PlayerId,
    pub bundle: InputBundle,
}

pub const INBOUND_QUEUE_CAP: usize = 1024;

pub struct InboundQueue {
    pub tx: Sender<Inbound>,
    pub rx: Receiver<Inbound>,
}

impl InboundQueue {
    pub fn new() -> Self {
        let (tx, rx) = bounded(INBOUND_QUEUE_CAP);
        InboundQueue { tx, rx }
    }
}

impl Default for InboundQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Everything a connection needs from the rest of the server.
pub struct NetCtx {
    pub metrics: Arc<Metrics>,
    pub sessions: Arc<Sessions>,
    pub inbound: Sender<Inbound>,
    pub commands: Sender<Command>,
    pub data_hash: u64,
    pub match_seed: u64,
    pub tick_hz: u8,
    pub map_id: u8,
    pub mode: u8,
    /// Latest tick published by the sim thread.
    pub server_tick: &'static AtomicU32,
    pub started: Instant,
    pub handshake_timeout: Duration,
}

/// A released `PlayerId` is not handed out again for this long, so a client that is still
/// processing the old player's last snapshot cannot confuse it with a new one (M0-018).
pub const ID_REUSE_DELAY: Duration = Duration::from_secs(1);

struct SessionsInner {
    per_ip: HashMap<IpAddr, usize>,
    total: usize,
    players: [Option<ClientHandle>; 128],
    /// Team of each occupied slot.
    team_of: [Team; 128],
    /// Players per team (`Team as usize`), counting occupied slots.
    team_count: [usize; 2],
    /// Slot is unavailable until this instant (released recently).
    cooling_until: [Option<Instant>; 128],
}

/// Connection admission (per IP / total) and `PlayerId` allocation.
pub struct Sessions {
    inner: Mutex<SessionsInner>,
    metrics: Arc<Metrics>,
    max_per_ip: usize,
    max_total: usize,
    reuse_delay: Duration,
}

/// RAII admission ticket: releases the IP/total slot and the connections gauge on drop.
pub struct Admission {
    sessions: Arc<Sessions>,
    ip: IpAddr,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AdmitError {
    IpLimit,
    ConnLimit,
}

impl Sessions {
    pub fn new(metrics: Arc<Metrics>) -> Arc<Sessions> {
        Self::with_limits(metrics, MAX_CONN_PER_IP, MAX_CONNECTIONS)
    }

    pub fn with_limits(
        metrics: Arc<Metrics>,
        max_per_ip: usize,
        max_total: usize,
    ) -> Arc<Sessions> {
        Self::with_reuse_delay(metrics, max_per_ip, max_total, ID_REUSE_DELAY)
    }

    /// Like [`Self::with_limits`] with an explicit `PlayerId` reuse delay (tests use zero).
    pub fn with_reuse_delay(
        metrics: Arc<Metrics>,
        max_per_ip: usize,
        max_total: usize,
        reuse_delay: Duration,
    ) -> Arc<Sessions> {
        Arc::new(Sessions {
            inner: Mutex::new(SessionsInner {
                per_ip: HashMap::new(),
                total: 0,
                players: std::array::from_fn(|_| None),
                team_of: [Team::Us; 128],
                team_count: [0; 2],
                cooling_until: [None; 128],
            }),
            metrics,
            max_per_ip,
            max_total,
            reuse_delay,
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, SessionsInner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn admit(self: &Arc<Self>, ip: IpAddr) -> Result<Admission, AdmitError> {
        let mut g = self.lock();
        if g.total >= self.max_total {
            return Err(AdmitError::ConnLimit);
        }
        let n = g.per_ip.entry(ip).or_insert(0);
        if *n >= self.max_per_ip {
            return Err(AdmitError::IpLimit);
        }
        *n += 1;
        g.total += 1;
        self.metrics.net_connections.inc();
        Ok(Admission {
            sessions: self.clone(),
            ip,
        })
    }

    /// Smallest free `PlayerId` (0..=127) that is not cooling down, registering `handle` for it,
    /// and the team it joins: the team with fewer players, US on a tie (so joins alternate
    /// US, GER, US, ...).
    pub fn alloc_player_team(&self, handle: &ClientHandle) -> Option<(PlayerId, Team)> {
        let now = Instant::now();
        let mut g = self.lock();
        let id = (0..128usize)
            .find(|&i| g.players[i].is_none() && g.cooling_until[i].is_none_or(|t| t <= now))?;
        let team = if g.team_count[Team::Ger as usize] < g.team_count[Team::Us as usize] {
            Team::Ger
        } else {
            Team::Us
        };
        g.players[id] = Some(handle.clone());
        g.cooling_until[id] = None;
        g.team_of[id] = team;
        g.team_count[team as usize] += 1;
        Some((id as PlayerId, team))
    }

    /// Takes `id` out of circulation for good (a bot lives there, M0-026) and counts it in
    /// `team`'s seats so human joins keep balancing against the bots. Idempotent.
    pub fn reserve_bot(&self, id: PlayerId, team: Team) {
        let i = id as usize;
        let mut g = self.lock();
        if i >= 128 || g.players[i].is_some() || g.cooling_until[i].is_some() {
            return;
        }
        g.cooling_until[i] = Some(Instant::now() + Duration::from_secs(10 * 365 * 24 * 3600));
        g.team_of[i] = team;
        g.team_count[team as usize] += 1;
    }

    /// [`Self::alloc_player_team`] without the team.
    pub fn alloc_player(&self, handle: &ClientHandle) -> Option<PlayerId> {
        self.alloc_player_team(handle).map(|(id, _)| id)
    }

    /// Frees the slot (it becomes allocatable again after the reuse delay) and the team seat.
    pub fn release_player(&self, id: PlayerId) {
        let mut g = self.lock();
        let i = id as usize;
        if i < 128 && g.players[i].take().is_some() {
            let t = g.team_of[i] as usize;
            g.team_count[t] = g.team_count[t].saturating_sub(1);
            g.cooling_until[i] =
                (!self.reuse_delay.is_zero()).then(|| Instant::now() + self.reuse_delay);
        }
    }

    /// Connected players per team `[us, ger]`.
    pub fn team_counts(&self) -> [usize; 2] {
        self.lock().team_count
    }

    /// Sending handle of a connected player (used by the snapshot sender, M0-019).
    pub fn handle_of(&self, id: PlayerId) -> Option<ClientHandle> {
        self.lock().players.get(id as usize).and_then(|h| h.clone())
    }

    pub fn connections(&self) -> usize {
        self.lock().total
    }
}

impl Drop for Admission {
    fn drop(&mut self) {
        let mut g = self.sessions.lock();
        g.total = g.total.saturating_sub(1);
        if let Some(n) = g.per_ip.get_mut(&self.ip) {
            *n -= 1;
            if *n == 0 {
                g.per_ip.remove(&self.ip);
            }
        }
        self.sessions.metrics.net_connections.dec();
    }
}

fn reject_label_count(ctx: &NetCtx, label: &str) {
    ctx.metrics
        .net_rejects_total
        .with_label_values(&[label])
        .inc();
}

fn malformed(ctx: &NetCtx, handle: &ClientHandle, why: &str, ip: IpAddr) {
    ctx.metrics.net_frames_malformed_total.inc();
    tracing::debug!(event = "frame_malformed", why, ip = %ip, "closing connection");
    handle.close_now();
}

/// Drives one accepted connection to completion. `deadline` is the handshake deadline (TCP
/// accept time + [`NetCtx::handshake_timeout`]). The admission ticket lives as long as this runs.
pub async fn run_connection<T: ClientTransport>(
    ctx: Arc<NetCtx>,
    mut t: T,
    deadline: tokio::time::Instant,
    _admission: Admission,
) {
    let handle = t.handle();
    // Whatever way this function ends, the writer side must not linger.
    let _close_guard = CloseOnDrop(handle.clone());
    let ip = t.peer_ip();
    let kind = t.kind();
    let _conn_gauge = TransportGauge::new(&ctx.metrics, kind);
    let mut rate = RateLimiter::new(RATE_FRAMES_PER_SEC, RATE_BURST_FRAMES, Instant::now());
    let mut c0 = C0Limits::new(Instant::now());

    // ---- handshake ----
    #[allow(clippy::never_loop)]
    let hello = loop {
        match timeout_at(deadline, t.recv()).await {
            Err(_) => {
                ctx.metrics.net_handshake_timeout_total.inc();
                handle.close_now();
                return;
            }
            Ok(Recv::Closed) => return,
            Ok(Recv::Malformed(why)) => return malformed(&ctx, &handle, why, ip),
            Ok(Recv::Frame(ch, payload)) => {
                if !rate.allow(Instant::now()) {
                    reject_label_count(&ctx, "rate_limit");
                    return handle.close_now();
                }
                if ch != Channel::C0 || payload.len() > MAX_INBOUND_PAYLOAD {
                    return malformed(&ctx, &handle, "pre-handshake frame not C0 Hello", ip);
                }
                match decode_c0_client(&payload) {
                    Ok(ClientMsgC0::Hello(h)) => break h,
                    _ => return malformed(&ctx, &handle, "pre-handshake: not a valid Hello", ip),
                }
            }
        }
    };
    if let Err(reason) = check_hello(&hello, ctx.data_hash) {
        reject_label_count(&ctx, reject_label(reason));
        send_reject(&handle, reason);
        return;
    }
    let Some((player, team)) = ctx.sessions.alloc_player_team(&handle) else {
        reject_label_count(&ctx, "server_full");
        send_reject(&handle, RejectReason::ServerFull);
        return;
    };
    // The sim learns about the player before the client can send its first C1 (it only starts
    // after Welcome), so `Join` is always ordered ahead of that player's inputs.
    if ctx
        .commands
        .try_send(Command::Join { player, team })
        .is_err()
    {
        ctx.sessions.release_player(player);
        reject_label_count(&ctx, "server_full");
        send_reject(&handle, RejectReason::ServerFull);
        return;
    }
    let welcome = ServerMsgC0::Welcome(Welcome {
        player_id: player,
        server_tick: ctx.server_tick.load(Ordering::Relaxed),
        tick_hz: ctx.tick_hz,
        match_seed: ctx.match_seed,
        data_hash: ctx.data_hash,
        map_id: ctx.map_id,
        mode: ctx.mode,
        team: team as u8,
        client_tick_offset: 0,
    });
    if !send_c0(&handle, &welcome) {
        cleanup(&ctx, player).await;
        return;
    }
    tracing::debug!(event = "client_joined", player_id = player as u64, ip = %ip, transport = kind, "handshake ok");

    // ---- established ----
    loop {
        match t.recv().await {
            Recv::Closed => break,
            Recv::Malformed(why) => {
                malformed(&ctx, &handle, why, ip);
                break;
            }
            Recv::Frame(ch, payload) => {
                if !rate.allow(Instant::now()) {
                    reject_label_count(&ctx, "rate_limit");
                    handle.close_now();
                    break;
                }
                if !handle_frame(&ctx, &handle, &mut c0, player, ch, payload, ip) {
                    break;
                }
            }
        }
    }
    cleanup(&ctx, player).await;
}

/// Keeps `net_transport_connections{transport}` accurate for the lifetime of a connection.
struct TransportGauge(prometheus::IntGauge);
impl TransportGauge {
    fn new(m: &Metrics, kind: &str) -> Self {
        let g = m.net_transport_connections.with_label_values(&[kind]);
        g.inc();
        TransportGauge(g)
    }
}
impl Drop for TransportGauge {
    fn drop(&mut self) {
        self.0.dec();
    }
}

struct CloseOnDrop(ClientHandle);
impl Drop for CloseOnDrop {
    fn drop(&mut self) {
        self.0.close_graceful();
    }
}

fn send_c0(handle: &ClientHandle, m: &ServerMsgC0) -> bool {
    match encode_c0_server(m) {
        Ok(b) => handle.send(Channel::C0, Bytes::from(b), Rel::Reliable),
        Err(_) => false,
    }
}

fn send_reject(handle: &ClientHandle, reason: RejectReason) {
    send_c0(handle, &ServerMsgC0::Reject(Reject { reason }));
    handle.close_graceful();
}

/// C0 message budgets of one connection: pings 5/s (excess is dropped and counted, the
/// connection stays up). The other-message budget (`C0_OTHER_PER_SEC`, 10/s) is applied when
/// M1 adds deploy/chat messages; M0 has none besides `Hello`/`Disconnect`.
struct C0Limits {
    ping: RateLimiter,
}

impl C0Limits {
    fn new(now: Instant) -> Self {
        C0Limits {
            ping: RateLimiter::new(PING_PER_SEC, PING_PER_SEC, now),
        }
    }
}

/// Returns `false` when the connection must end.
fn handle_frame(
    ctx: &NetCtx,
    handle: &ClientHandle,
    c0: &mut C0Limits,
    player: PlayerId,
    ch: Channel,
    payload: Bytes,
    ip: IpAddr,
) -> bool {
    if payload.len() > MAX_INBOUND_PAYLOAD {
        malformed(ctx, handle, "oversize frame", ip);
        return false;
    }
    match ch {
        Channel::C0 => match decode_c0_client(&payload) {
            Ok(ClientMsgC0::Ping(p)) => {
                if !c0.ping.allow(Instant::now()) {
                    ctx.metrics
                        .input_rejected_total
                        .with_label_values(&["c0_rate"])
                        .inc();
                    return true;
                }
                let pong = ServerMsgC0::Pong(Pong {
                    client_time: p.client_time,
                    server_tick: ctx.server_tick.load(Ordering::Relaxed),
                    server_time_ms: ctx.started.elapsed().as_millis() as u32,
                });
                send_c0(handle, &pong)
            }
            Ok(ClientMsgC0::Disconnect(_)) => {
                handle.close_graceful();
                false
            }
            // A second Hello, or anything undecodable, is a protocol violation.
            Ok(ClientMsgC0::Hello(_)) | Err(_) => {
                malformed(ctx, handle, "bad C0 message", ip);
                false
            }
        },
        Channel::C1 => match decode_input_bundle(&payload) {
            Ok(bundle) => {
                match ctx.inbound.try_send(Inbound { player, bundle }) {
                    Ok(()) => ctx.metrics.net_c1_inbound_total.inc(),
                    Err(TrySendError::Full(_)) => ctx.metrics.input_queue_full_total.inc(),
                    Err(TrySendError::Disconnected(_)) => {}
                }
                true
            }
            Err(_) => {
                malformed(ctx, handle, "bad C1 bundle", ip);
                false
            }
        },
        // Clients never send C2/C3/C4/C5.
        _ => {
            malformed(ctx, handle, "client sent a server-only channel", ip);
            false
        }
    }
}

async fn cleanup(ctx: &NetCtx, player: PlayerId) {
    ctx.sessions.release_player(player);
    for _ in 0..200 {
        match ctx.commands.try_send(Command::ClientGone(player)) {
            Ok(()) => return,
            Err(TrySendError::Full(_)) => tokio::time::sleep(Duration::from_millis(5)).await,
            Err(TrySendError::Disconnected(_)) => return,
        }
    }
    tracing::warn!(
        event = "client_gone_dropped",
        player_id = player as u64,
        "command channel full"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(n: u8) -> IpAddr {
        IpAddr::from([10, 0, 0, n])
    }

    #[test]
    fn per_ip_and_total_limits() {
        let m = Metrics::new();
        let s = Sessions::with_limits(m.clone(), 4, 6);
        let a: Vec<_> = (0..4).map(|_| s.admit(ip(1)).unwrap()).collect();
        assert_eq!(s.admit(ip(1)).err(), Some(AdmitError::IpLimit));
        let _b1 = s.admit(ip(2)).unwrap();
        let _b2 = s.admit(ip(3)).unwrap();
        assert_eq!(s.admit(ip(4)).err(), Some(AdmitError::ConnLimit));
        assert_eq!(m.net_connections.get(), 6);
        drop(a);
        assert_eq!(m.net_connections.get(), 2);
        assert!(s.admit(ip(1)).is_ok());
    }

    #[test]
    fn player_ids_are_smallest_free_and_recycled() {
        let m = Metrics::new();
        let s = Sessions::with_reuse_delay(m.clone(), 4, 128, Duration::ZERO);
        let hs: Vec<_> = (0..128).map(|_| ClientHandle::new(m.clone())).collect();
        for (i, h) in hs.iter().enumerate() {
            assert_eq!(s.alloc_player(h), Some(i as u8));
        }
        assert_eq!(s.alloc_player(&hs[0]), None);
        s.release_player(5);
        s.release_player(2);
        assert_eq!(s.alloc_player(&hs[0]), Some(2));
        assert_eq!(s.alloc_player(&hs[0]), Some(5));
        assert!(s.handle_of(5).is_some());
        s.release_player(5);
        assert!(s.handle_of(5).is_none());
    }

    #[test]
    fn teams_alternate_and_stay_balanced_through_leaves() {
        let m = Metrics::new();
        let s = Sessions::with_reuse_delay(m.clone(), 4, 128, Duration::ZERO);
        let h = ClientHandle::new(m);
        let mut teams = vec![];
        for _ in 0..6 {
            teams.push(s.alloc_player_team(&h).unwrap().1);
        }
        use Team::*;
        assert_eq!(teams, vec![Us, Ger, Us, Ger, Us, Ger]);
        assert_eq!(s.team_counts(), [3, 3]);
        // Both US players leave: the next two joins go to US, then it alternates again.
        s.release_player(0);
        s.release_player(4);
        assert_eq!(s.team_counts(), [1, 3]);
        assert_eq!(s.alloc_player_team(&h).unwrap().1, Us);
        assert_eq!(s.alloc_player_team(&h).unwrap().1, Us);
        assert_eq!(s.alloc_player_team(&h).unwrap().1, Us);
        assert_eq!(s.team_counts(), [4, 3]);
        assert_eq!(s.alloc_player_team(&h).unwrap().1, Ger);
    }

    #[test]
    fn released_player_id_is_not_reused_for_one_second() {
        let m = Metrics::new();
        let s = Sessions::new(m.clone()); // default 1 s delay
        let h = ClientHandle::new(m);
        assert_eq!(s.alloc_player(&h), Some(0));
        assert_eq!(s.alloc_player(&h), Some(1));
        s.release_player(0);
        // 0 is cooling: the next allocation skips it.
        assert_eq!(s.alloc_player(&h), Some(2));
        assert!(s.handle_of(0).is_none(), "released at once for sending");
        std::thread::sleep(ID_REUSE_DELAY + Duration::from_millis(50));
        assert_eq!(s.alloc_player(&h), Some(0));
    }

    #[test]
    fn reserved_bot_ids_are_never_allocated_and_count_in_the_team_balance() {
        let m = Metrics::new();
        let s = Sessions::with_reuse_delay(m.clone(), 4, 128, Duration::ZERO);
        let h = ClientHandle::new(m);
        // Ten bots at 127, 126, ...: even indices US, odd GER.
        for i in 0..10u8 {
            s.reserve_bot(127 - i, if i % 2 == 0 { Team::Us } else { Team::Ger });
        }
        s.reserve_bot(127, Team::Ger); // idempotent
        assert_eq!(s.team_counts(), [5, 5]);
        let mut ids = vec![];
        while let Some(id) = s.alloc_player(&h) {
            ids.push(id);
        }
        assert_eq!(ids, (0..118).collect::<Vec<u8>>(), "humans must fill 0..=117 only");
        // A human leaving does not free a bot's id.
        s.release_player(3);
        assert_eq!(s.alloc_player(&h), Some(3));
        assert_eq!(s.alloc_player(&h), None);
    }
}
