//! In-process server for the session tests: real WebSocket + WebTransport listeners wired to a
//! real `SimLoop` (the same code `main` uses), plus synthetic input generators.
#![allow(dead_code)]

use std::cell::Cell;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use frontline_protocol::msg::input::{buttons, encode_input_bundle, InputBundle, InputCmd};
use frontline_protocol::Channel;
use frontline_server::clock::Clock;
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::{InboundQueue, NetCtx, Sessions};
use frontline_server::net::limits::HANDSHAKE_TIMEOUT;
use frontline_server::net::ws::WsListener;
use frontline_server::net::wt::{cert_hash, dev_identity, WtListener};
use frontline_server::sim_thread::Command;
use frontline_server::world_loader::{self, LoadedMap};
use wtransport::tls::Sha256Digest;

use super::Kind;
use super::Target;

/// Test clock: `sleep_until` jumps straight to the deadline, so a manually driven loop runs as
/// fast as the CPU allows while keeping the real tick bookkeeping.
pub struct StepClock {
    base: Instant,
    off_ns: Cell<u64>,
}

impl StepClock {
    pub fn new() -> Self {
        StepClock {
            base: Instant::now(),
            off_ns: Cell::new(0),
        }
    }
}

impl Default for StepClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for StepClock {
    fn now(&self) -> Instant {
        self.base + Duration::from_nanos(self.off_ns.get())
    }
    fn sleep_until(&self, deadline: Instant) {
        let t = deadline.duration_since(self.base).as_nanos() as u64;
        if t > self.off_ns.get() {
            self.off_ns.set(t);
        }
    }
}

/// The repository terrain, or `None` (with the regeneration command printed) when it has not
/// been generated. `FRONTLINE_REQUIRE_TERRAIN=1` turns that into a failure.
pub fn terrain() -> Option<Arc<LoadedMap>> {
    static MAP: OnceLock<Option<Arc<LoadedMap>>> = OnceLock::new();
    MAP.get_or_init(|| {
        let loaded = world_loader::locate_data_dir(None, "m0")
            .and_then(|d| world_loader::load_map(&d, "m0"));
        match loaded {
            Ok(m) => Some(Arc::new(m)),
            Err(e) => {
                if std::env::var_os("FRONTLINE_REQUIRE_TERRAIN").is_some() {
                    panic!("terrain required but unavailable: {e}");
                }
                eprintln!("SKIPPED (no terrain): {e}");
                None
            }
        }
    })
    .clone()
}

pub struct TestServer {
    pub metrics: Arc<Metrics>,
    pub sessions: Arc<Sessions>,
    pub ws: Option<WsListener>,
    pub wt: Option<WtListener>,
    pub ws_target: Target,
    pub wt_target: Target,
}

impl TestServer {
    pub fn target(&self, k: Kind) -> &Target {
        match k {
            Kind::Ws => &self.ws_target,
            Kind::Wt => &self.wt_target,
        }
    }

    pub async fn stop(mut self) {
        if let Some(l) = self.ws.take() {
            l.shutdown().await;
        }
        if let Some(l) = self.wt.take() {
            l.shutdown().await;
        }
    }
}

/// Binds WS + WT listeners (ephemeral ports) that feed `inbound` / `commands`.
pub async fn start_listeners(
    metrics: Arc<Metrics>,
    sessions: Arc<Sessions>,
    inbound: &InboundQueue,
    commands: Sender<Command>,
    wt: bool,
) -> TestServer {
    let ctx = Arc::new(NetCtx {
        metrics: metrics.clone(),
        sessions: sessions.clone(),
        inbound: inbound.tx.clone(),
        commands,
        data_hash: super::DATA_HASH,
        match_seed: 1,
        tick_hz: 30,
        map_id: 0,
        mode: 0,
        server_tick: &frontline_server::logging::CURRENT_TICK,
        started: Instant::now(),
        handshake_timeout: HANDSHAKE_TIMEOUT,
        tickets: None,
    });
    let ws = WsListener::bind(("127.0.0.1", 0), ctx.clone())
        .await
        .expect("bind ws");
    let ws_target = Target {
        kind: Kind::Ws,
        url: format!("ws://{}", ws.local_addr()),
        hash: None,
    };
    let (wt_l, wt_target) = if wt {
        let id = dev_identity(13).expect("identity");
        let hash: Sha256Digest = cert_hash(&id);
        let l = WtListener::bind(([127, 0, 0, 1], 0).into(), id, ctx.clone())
            .await
            .expect("bind wt");
        let t = Target {
            kind: Kind::Wt,
            url: format!("https://127.0.0.1:{}/wt", l.local_addr().port()),
            hash: Some(hash),
        };
        (Some(l), t)
    } else {
        (None, ws_target.clone())
    };
    TestServer {
        metrics,
        sessions,
        ws: Some(ws),
        wt: wt_l,
        ws_target,
        wt_target,
    }
}

// ---------------------------------------------------------------------------------------------
// Synthetic input
// ---------------------------------------------------------------------------------------------

/// Deterministic, legal input of `player` for `tick` (seq == tick so bundles stay consecutive).
pub fn gen_cmd(player: u8, tick: u32) -> InputCmd {
    let phase = ((tick / 24) + player as u32) % 5;
    let (mx, mz): (i8, i8) = match phase {
        0 => (0, 127),
        1 => (90, 90),
        2 => (127, 0),
        3 => (0, 0),
        _ => (-60, 100),
    };
    let mut b = 0u16;
    if phase == 0 && tick % 48 < 24 {
        b |= buttons::SPRINT;
    }
    if phase == 4 {
        b |= buttons::CROUCH;
    }
    if tick.is_multiple_of(45) {
        b |= buttons::JUMP;
    }
    if player.is_multiple_of(2) && (30..40).contains(&(tick % 90)) {
        b |= buttons::FIRE;
    }
    if tick % 70 > 50 {
        b |= buttons::AIM;
    }
    InputCmd {
        seq: tick as u16,
        tick,
        move_x: mx,
        move_z: mz,
        yaw: (player as u32 * 5000 + tick * 40) as u16,
        pitch: ((tick as i32 % 200) * 20 - 2000) as i16,
        buttons: b,
        weapon_slot: 0,
        aux: 0,
        view_tick: 2,
    }
}

/// The redundant bundle a client sends when its newest command is `k`: ticks `k-2..=k`.
pub fn bundle_upto(player: u8, k: u32) -> InputBundle {
    let first = k.saturating_sub(2).max(1);
    let cmds = (first..=k).map(|t| gen_cmd(player, t)).collect();
    InputBundle {
        ack_snapshot_tick: 0,
        pred_hash: 0,
        cmds,
        fire: Default::default(),
    }
}

pub fn encode(b: &InputBundle) -> Vec<u8> {
    encode_input_bundle(b).expect("encodable bundle")
}

pub const C1: Channel = Channel::C1;
