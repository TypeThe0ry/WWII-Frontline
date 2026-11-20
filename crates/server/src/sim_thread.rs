//! The 30 Hz simulation thread (ARCHITECTURE §6.1, §6.10). Runs on a dedicated OS thread,
//! never on a tokio worker. Input/command channels are bounded `crossbeam` channels.

use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use frontline_protocol::msg::c0::{Chat, ChatChannel, ClientMsgC0, ServerMsgC0};
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::squad::{SquadKind, SquadOp};
use frontline_sim::{
    CollisionData, GameData, GameMode, MapId, PlayerId, SimConfig, SimEvent, SpawnReq, Team, World,
};

use crate::bot_host::BotHost;
use crate::gateway_link::admin::{Action, ActionResult};
use crate::gateway_link::reports::{ChatLine, ReportJob};
use crate::seat_input::SeatInput;
use crate::sim_bridge::{NetLink, SimBridge};
use crate::snapshot::team_channel::{MapFrame, TeamChannel};
use crate::snapshot::{ClientIn, LinkInfo, Outgoing, SnapshotScheduler, DEGRADE_HOLD_TICKS};
use crate::validate::ValidateCfg;
use crate::world_loader::LoadedMap;
use serde::Serialize;

use crate::clock::{Clock, SystemClock, TickSchedule};
use crate::logging::CURRENT_TICK;
use crate::metrics::Metrics;
use crate::shutdown::StopFlag;

mod server_rules;

pub const INPUT_CHANNEL_CAP: usize = 1024;
pub const COMMAND_CHANNEL_CAP: usize = 256;
/// 1/30 s in nanoseconds (PI-006: fixed step 1000/30 ms).
pub const TICK_PERIOD: Duration = Duration::from_nanos(33_333_333);
/// Per-tick samples kept for the exit report. Beyond this many, [`Samples`] keeps a uniform random subset (reservoir) so a
/// long-running server holds a constant 2 x 128 KB instead of growing 240 B/s for ever (M0-030, PI-029).
const SAMPLE_CAP: usize = 32_768;

/// Bounded sample store: all samples up to [`SAMPLE_CAP`], then Algorithm R (uniform reservoir, fixed xorshift seed so the
/// report of a deterministic run is reproducible). The exact maximum is tracked separately.
#[derive(Debug)]
struct Samples {
    v: Vec<u32>,
    seen: u64,
    max: u32,
    rng: u64,
}

impl Samples {
    fn new() -> Self {
        Samples {
            v: Vec::with_capacity(SAMPLE_CAP),
            seen: 0,
            max: 0,
            rng: 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn push(&mut self, x: u32) {
        self.seen += 1;
        self.max = self.max.max(x);
        if self.v.len() < SAMPLE_CAP {
            self.v.push(x);
            return;
        }
        let mut r = self.rng;
        r ^= r << 13;
        r ^= r >> 7;
        r ^= r << 17;
        self.rng = r;
        let j = r % self.seen;
        if (j as usize) < SAMPLE_CAP {
            self.v[j as usize] = x;
        }
    }

    fn clear(&mut self) {
        self.v.clear();
        self.seen = 0;
        self.max = 0;
    }

    fn quantiles(&mut self) -> Quantiles {
        let mut q = quantiles(&mut self.v);
        if !self.v.is_empty() {
            q.max = f64::from(self.max) / 1e6; // exact even when the reservoir dropped the extreme
        }
        q
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InputEnvelope {
    pub player: PlayerId,
    pub cmd: InputCmd,
}

#[derive(Debug, Clone)]
pub enum Command {
    /// Handshake finished: put the player into the world and open its input session. The team
    /// was chosen by the net layer (it is already in `Welcome`); the join position comes from the
    /// map's HQ point. (`Hello` carries no nickname in M0.)
    Join { player: PlayerId, team: Team },
    /// Spawn at an explicit place (tests, benches, bots); also opens a session.
    SpawnPlayer { req: SpawnReq },
    /// Remove a player now (no session bookkeeping beyond that).
    RemovePlayer(PlayerId),
    /// The client's connection closed (published by the net layer). Idempotent.
    ClientGone(PlayerId),
    /// A rate-limited C0 gameplay message from a connected client (M1-026).
    C0 {
        player: PlayerId,
        msg: frontline_protocol::msg::c0::ClientMsgC0,
    },
    /// Trusted squad mute notification, reserved for the M2-040 integration.
    ReportSquadMute { by: PlayerId, target: PlayerId },
    /// Keep every team at `n` players by spawning / removing bots (M1-039c); `0` turns it off.
    SetBotFill(u8),
    /// Installed before the ticket gate is armed for a registered match.
    EnableReports {
        reports: tokio::sync::mpsc::Sender<ReportJob>,
        chat_updates: tokio::sync::watch::Sender<Arc<Vec<ChatLine>>>,
    },
    Admin {
        action: Action,
        reply: Arc<AdminReply>,
    },
}

// Command remains Clone for existing in-process callers. Only one clone can acknowledge.
#[derive(Debug)]
pub struct AdminReply(std::sync::Mutex<Option<tokio::sync::oneshot::Sender<ActionResult>>>);

impl AdminReply {
    pub(crate) fn new(tx: tokio::sync::oneshot::Sender<ActionResult>) -> Arc<Self> {
        Arc::new(Self(std::sync::Mutex::new(Some(tx))))
    }
}

/// Network-side sender for inputs. Drops the new input when the channel is full and counts it.
#[derive(Clone)]
pub struct InputSender {
    tx: Sender<InputEnvelope>,
    metrics: Arc<Metrics>,
}

impl InputSender {
    /// Returns `false` if the input was dropped.
    pub fn try_send(&self, e: InputEnvelope) -> bool {
        match self.tx.try_send(e) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                self.metrics.input_queue_full_total.inc();
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }
}

pub struct SimThreadHandle {
    pub inputs_tx: InputSender,
    pub commands_tx: Sender<Command>,
    join: JoinHandle<Report>,
}

impl SimThreadHandle {
    /// Waits for the thread (the caller must have raised the [`StopFlag`]).
    pub fn join(self) -> Option<Report> {
        self.join.join().ok()
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Quantiles {
    pub p50: f64,
    pub p99: f64,
    pub max: f64,
    pub mean: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub ticks: u64,
    pub tick_ms: Quantiles,
    pub tick_interval_error_ms: Quantiles,
    pub tick_overrun_total: u64,
    pub tick_overrun_consecutive_max: u64,
    pub tick_skipped_slots_total: u64,
    pub input_queue_full_total: u64,
    pub elapsed_s: f64,
    /// Gameplay events seen by the sim during the run (shots, hits, kills, spawns; M0-027 load
    /// reports).
    pub events: EventTally,
    /// Authoritative end-of-run scoring and XP, built only when exporting a report.
    pub score_summary: frontline_sim::score::stats::MatchSummary,
}

fn quantiles(samples_ns: &mut [u32]) -> Quantiles {
    if samples_ns.is_empty() {
        return Quantiles::default();
    }
    samples_ns.sort_unstable();
    let at = |q: f64| {
        let i = ((samples_ns.len() as f64 - 1.0) * q).ceil() as usize;
        samples_ns[i.min(samples_ns.len() - 1)] as f64 / 1e6
    };
    let mean = samples_ns.iter().map(|&v| v as f64).sum::<f64>() / samples_ns.len() as f64 / 1e6;
    Quantiles {
        p50: at(0.5),
        p99: at(0.99),
        max: *samples_ns.last().unwrap() as f64 / 1e6,
        mean,
    }
}

#[derive(Clone)]
pub struct SimThreadConfig {
    pub match_seed: u64,
    pub map: MapId,
    pub mode: GameMode,
    pub pin_core: Option<usize>,
    /// Loaded terrain and spawn points; `None` = empty collision world (tests only: players
    /// cannot move without terrain).
    pub map_data: Option<Arc<LoadedMap>>,
    /// M0 load-test fixture; no vehicles are added to ordinary matches.
    pub dev_spawn_vehicle: bool,
    /// The network side (inbound C1 queue, connection handles); `None` in pure-sim tests.
    pub net: Option<NetLink>,
}

impl SimThreadConfig {
    /// A config without terrain or network (tests, benches).
    pub fn bare(match_seed: u64, map: MapId, mode: GameMode) -> Self {
        SimThreadConfig {
            match_seed,
            map,
            mode,
            pin_core: None,
            map_data: None,
            dev_spawn_vehicle: false,
            net: None,
        }
    }
}

pub struct SimLoop<C: Clock> {
    /// Per-phase wall time (ms) of the last tick (see `tick_trace_enabled`):
    /// inputs, step, post-step, bots, aoi+snapshot, send.
    phase_ms: [f64; 6],
    step_trace_ticks: Vec<u64>,
    clock: C,
    schedule: TickSchedule,
    world: World,
    dev_spawn_vehicle: bool,
    metrics: Arc<Metrics>,
    inputs_rx: Receiver<InputEnvelope>,
    commands_rx: Receiver<Command>,
    stop: StopFlag,
    bridge: SimBridge,
    seat_input: SeatInput,
    c0: crate::c0::C0Handler,
    reports: Option<crate::report::Recorder>,
    admin_disconnects: VecDeque<(Instant, crate::net::transport::ClientHandle)>,
    team_channel: TeamChannel,
    comms: crate::c0::comms::Markers,
    team_outgoing: Vec<(PlayerId, Vec<u8>)>,
    c4: crate::c4::Broadcast,
    consecutive: i64,
    ticks: u64,
    tick_ns: Samples,
    err_ns: Samples,
    started: std::time::Instant,
    /// Snapshot / AOI scheduler (M0-019) and its per-tick scratch.
    snap: SnapshotScheduler,
    step_events: Vec<SimEvent>,
    clients_in: Vec<ClientIn>,
    outgoing: Vec<Outgoing>,
    /// Bots hosted in this process (`--bots N`, M0-026); `None` without bots.
    bot_host: Option<BotHost>,
    /// Per-team target for dynamic bot fill; `None` keeps the static `--bots N`.
    bot_fill: Option<u8>,
    tactical_map_ready: bool,
    /// Running count of the sim's gameplay events (shots, hits, kills, spawns).
    tally: EventTally,
}

/// Running totals of gameplay events, for the soak / balance reports (M0-026).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct EventTally {
    pub shots: u64,
    pub hits: u64,
    pub kills: u64,
    pub headshots: u64,
    pub spawns: u64,
}

impl EventTally {
    fn add(&mut self, events: &[SimEvent]) {
        for e in events {
            match e {
                SimEvent::Shot { .. } => self.shots += 1,
                SimEvent::Hit { .. } => self.hits += 1,
                SimEvent::Kill { headshot, .. } => {
                    self.kills += 1;
                    self.headshots += u64::from(*headshot);
                }
                SimEvent::Spawn { .. } => self.spawns += 1,
                _ => {}
            }
        }
    }
}

/// Creates the channel pair plus a loop that is not yet running (tests drive it directly).
pub fn build_loop<C: Clock>(
    clock: C,
    cfg: &SimThreadConfig,
    metrics: Arc<Metrics>,
    stop: StopFlag,
) -> (SimLoop<C>, InputSender, Sender<Command>) {
    build_loop_with_bots(clock, cfg, 0, metrics, stop)
}

/// [`build_loop`] with `bots` soldier bots hosted in the loop (`--bots N`): they join before the
/// first tick, split evenly between the teams.
pub fn build_loop_with_bots<C: Clock>(
    clock: C,
    cfg: &SimThreadConfig,
    bots: u16,
    metrics: Arc<Metrics>,
    stop: StopFlag,
) -> (SimLoop<C>, InputSender, Sender<Command>) {
    let (itx, irx) = bounded(INPUT_CHANNEL_CAP);
    let (ctx, crx) = bounded(COMMAND_CHANNEL_CAP);
    let sim_cfg = SimConfig {
        tick_hz: 30,
        match_seed: cfg.match_seed,
        map: cfg.map,
        mode: cfg.mode,
        data: GameData::embedded(),
    };
    let empty = CollisionData::empty();
    let mut world = World::new(
        sim_cfg,
        cfg.map_data.as_ref().map_or(&empty, |m| &m.collision),
    );
    if cfg.net.is_some() {
        let rotation = crate::config::KNOWN_MAPS.get(..3).map(|_| [0, 1, 2]);
        world.enable_server_rules(rotation);
    }
    if let Some(m) = &cfg.map_data {
        m.apply_to(&mut world);
        if m.uses_tactical_deploy() {
            m.apply_tactical(&mut world);
            if cfg.mode != GameMode::Sandbox {
                world.begin_warmup();
            }
        }
        if cfg.dev_spawn_vehicle {
            // Bots alternate US/GER before the first human joins; US wins ties.
            let team = if bots.is_multiple_of(2) {
                Team::Us
            } else {
                Team::Ger
            };
            let (mut pos, yaw) = m.join_spawn(0, team);
            // Both HQs are flat pads. Place the hull clear of the player's feet but in boarding range.
            pos.x += 5.0 * yaw.sin();
            pos.z += 5.0 * yaw.cos();
            pos.y = m.collision.height_at(pos.x, pos.z).unwrap_or(pos.y) + 0.9;
            world
                .spawn_vehicle("us_m4a1_75", team, pos, yaw)
                .expect("embedded M0 load-test vehicle fixture");
        }
    }
    let mut bridge = SimBridge::new(
        metrics.clone(),
        cfg.net.clone(),
        cfg.map_data.clone(),
        ValidateCfg::default(),
    );
    let bot_host = (bots > 0).then(|| {
        BotHost::with_soldiers(
            bots,
            cfg.match_seed,
            &mut world,
            &mut bridge,
            cfg.map_data.as_deref(),
            cfg.net.as_ref().map(|n| &n.sessions),
            metrics.clone(),
        )
    });
    let now = clock.now();
    let step_trace_ticks =
        if tick_trace_enabled() && std::env::var("FRONTLINE_STEP_TRACE").as_deref() == Ok("1") {
            world.enable_step_profiling(step_clock_ns);
            std::env::var("FRONTLINE_STEP_TRACE_TICKS")
                .unwrap_or_default()
                .split(',')
                .filter_map(|tick| tick.trim().parse().ok())
                .collect()
        } else {
            Vec::new()
        };
    let comms_hz = world.config().tick_hz;
    let comms_map = MapFrame {
        origin_x: 0.0,
        origin_z: 0.0,
        extent_m: cfg.map_data.as_ref().map_or(256.0, |m| m.size_m),
    };
    let lp = SimLoop {
        schedule: TickSchedule::new(now + TICK_PERIOD, TICK_PERIOD),
        started: now,
        clock,
        world,
        dev_spawn_vehicle: cfg.dev_spawn_vehicle,
        metrics: metrics.clone(),
        inputs_rx: irx,
        commands_rx: crx,
        stop,
        bridge,
        seat_input: SeatInput::default(),
        c0: crate::c0::C0Handler::default(),
        reports: None,
        admin_disconnects: VecDeque::new(),
        team_channel: TeamChannel::new(
            GameData::embedded().rules.aoi.team_channel_period_s as f32,
            1.0 / TICK_PERIOD.as_secs_f32(),
            comms_map,
        ),
        comms: crate::c0::comms::Markers::new(comms_hz, comms_map),
        team_outgoing: Vec::new(),
        c4: crate::c4::Broadcast::default(),
        consecutive: 0,
        phase_ms: [0.0; 6],
        step_trace_ticks,
        ticks: 0,
        tick_ns: Samples::new(),
        err_ns: Samples::new(),
        snap: SnapshotScheduler::new(Some(metrics.clone())),
        step_events: Vec::new(),
        clients_in: Vec::new(),
        outgoing: Vec::new(),
        bot_host,
        bot_fill: None,
        tactical_map_ready: false,
        tally: EventTally::default(),
    };
    (lp, InputSender { tx: itx, metrics }, ctx)
}

/// Spawns the dedicated `sim` OS thread.
pub fn spawn(
    cfg: SimThreadConfig,
    metrics: Arc<Metrics>,
    stop: StopFlag,
) -> std::io::Result<SimThreadHandle> {
    spawn_with_bots(cfg, 0, metrics, stop)
}

/// [`spawn`] with `bots` soldier bots (`--bots N`).
pub fn spawn_with_bots(
    cfg: SimThreadConfig,
    bots: u16,
    metrics: Arc<Metrics>,
    stop: StopFlag,
) -> std::io::Result<SimThreadHandle> {
    let (lp, inputs_tx, commands_tx) = build_loop_with_bots(SystemClock, &cfg, bots, metrics, stop);
    let pin = cfg.pin_core;
    let join = std::thread::Builder::new()
        .name("sim".into())
        .spawn(move || {
            if let Some(core) = pin {
                match pin_current_thread(core) {
                    Ok(()) => tracing::info!(event = "sim_cpu_pinned", core),
                    Err(error) => tracing::warn!(
                        event = "sim_cpu_pin_failed",
                        core,
                        %error,
                        "sim CPU pinning failed; retaining inherited affinity"
                    ),
                }
            }
            let mut lp = lp;
            // A spawned loop's scheduler statistics are unreachable (only `Report` leaves the thread), so keeping the
            // per-packet / per-second sample vectors was a slow leak: 6 B per packet per client (about 125 B/s per
            // client; capped at 400k samples per vector, which a 10-client match reaches after ~33 min). M0-030.
            lp.snap.collect = false;
            lp.run()
        })?;
    Ok(SimThreadHandle {
        inputs_tx,
        commands_tx,
        join,
    })
}

#[cfg(target_os = "linux")]
fn pin_current_thread(core: usize) -> std::io::Result<()> {
    use rustix::thread::{sched_setaffinity, CpuSet};

    if core >= CpuSet::MAX_CPU {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "CPU id is outside the supported affinity mask",
        ));
    }
    let mut mask = CpuSet::new();
    mask.set(core);
    // None targets the calling thread (sim), not the process's main thread.
    Ok(sched_setaffinity(None, &mask)?)
}

#[cfg(not(target_os = "linux"))]
fn pin_current_thread(_core: usize) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "sim CPU pinning requires Linux",
    ))
}

#[cfg(all(test, target_os = "linux"))]
#[test]
fn affinity_changes_only_calling_thread() {
    use rustix::thread::{sched_getaffinity, CpuSet};

    let allowed = sched_getaffinity(None).unwrap();
    let core = (0..CpuSet::MAX_CPU).find(|&id| allowed.is_set(id)).unwrap();
    std::thread::spawn(move || {
        assert!(pin_current_thread(CpuSet::MAX_CPU).is_err());
        pin_current_thread(core).unwrap();
        let pinned = sched_getaffinity(None).unwrap();
        assert!((0..CpuSet::MAX_CPU).all(|id| pinned.is_set(id) == (id == core)));
    })
    .join()
    .unwrap();
    let caller = sched_getaffinity(None).unwrap();
    assert!((0..CpuSet::MAX_CPU).all(|id| caller.is_set(id) == allowed.is_set(id)));
}

/// (on-CPU ns, run-queue wait ns) of the calling thread from `/proc/thread-self/schedstat` (Linux only).
fn thread_sched() -> Option<(u64, u64)> {
    let t = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
    let mut it = t.split_whitespace();
    Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
}

/// Tick trace (M1-068b): every overrun log line says how much of the tick the thread spent on a CPU, how long it waited in the run
/// queue (a slow tick vs a descheduled one) and the wall time of each phase. Always on: it costs six `Instant::now()` and two small
/// `/proc` reads per tick (about 10 us at 30 Hz). `FRONTLINE_TICK_TRACE=0` switches it off.
fn tick_trace_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("FRONTLINE_TICK_TRACE").map_or(true, |v| v != "0"))
}

fn step_clock_ns() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_nanos() as u64
}

impl<C: Clock> SimLoop<C> {
    pub fn world(&self) -> &World {
        &self.world
    }

    /// The hosted bots, if any (`--bots`).
    pub fn bot_host(&self) -> Option<&BotHost> {
        self.bot_host.as_ref()
    }

    pub fn bot_host_mut(&mut self) -> Option<&mut BotHost> {
        self.bot_host.as_mut()
    }

    /// Gameplay event totals since the loop was built.
    pub fn tally(&self) -> EventTally {
        self.tally
    }

    /// Runs until the stop flag is raised; returns the summary.
    pub fn run(&mut self) -> Report {
        while !self.stop.is_stopped() {
            self.tick();
        }
        self.report()
    }

    /// One scheduled tick with the real per-tick body.
    pub fn tick(&mut self) {
        self.tick_with(Self::tick_body);
    }

    /// Waits for the deadline, runs `work`, then does the overrun accounting and moves the
    /// absolute schedule forward (skipping, never replaying, missed slots).
    pub fn tick_with<F: FnOnce(&mut Self)>(&mut self, work: F) {
        let deadline = self.schedule.current_deadline();
        self.clock.sleep_until(deadline);
        let t0 = self.clock.now();
        let err = t0.saturating_duration_since(deadline);
        self.metrics
            .tick_interval_error_ms
            .observe(err.as_secs_f64() * 1e3);

        let sched0 = if tick_trace_enabled() {
            thread_sched()
        } else {
            None
        };
        work(self);
        let sched1 = if sched0.is_some() {
            thread_sched()
        } else {
            None
        };

        let t1 = self.clock.now();
        let dur = t1.saturating_duration_since(t0);
        self.metrics.tick_ms.observe(dur.as_secs_f64() * 1e3);
        self.tick_ns
            .push(dur.as_nanos().min(u32::MAX as u128) as u32);
        self.err_ns
            .push(err.as_nanos().min(u32::MAX as u128) as u32);
        self.ticks += 1;
        self.metrics.ticks_run.store(self.ticks, Ordering::Relaxed);

        if dur > self.schedule.period() {
            self.metrics.tick_overrun_total.inc();
            self.consecutive += 1;
            if self.consecutive >= 3 {
                // Degradation guard (ARCHITECTURE §6.10): all clients drop to 15 Hz snapshots
                // for 3 s after the last overrun of the run.
                if self.consecutive == 3 {
                    tracing::warn!(
                        event = "tick_overrun_degrade",
                        consecutive = self.consecutive,
                        "3 consecutive tick overruns: snapshot rate reduced to 15 Hz"
                    );
                }
                self.snap.degrade(DEGRADE_HOLD_TICKS);
            }
            let (cpu_ms, runq_wait_ms) = match (sched0, sched1) {
                (Some(a), Some(b)) => (
                    b.0.saturating_sub(a.0) as f64 / 1e6,
                    b.1.saturating_sub(a.1) as f64 / 1e6,
                ),
                _ => (-1.0, -1.0),
            };
            tracing::warn!(
                event = "tick_overrun",
                tick_ms = dur.as_secs_f64() * 1e3,
                cpu_ms,
                runq_wait_ms,
                phase_inputs_ms = self.phase_ms[0],
                phase_step_ms = self.phase_ms[1],
                phase_post_step_ms = self.phase_ms[2],
                phase_bots_ms = self.phase_ms[3],
                phase_snapshot_ms = self.phase_ms[4],
                phase_send_ms = self.phase_ms[5],
                tick = self.ticks,
                consecutive = self.consecutive,
                "tick exceeded its budget"
            );
        } else {
            self.consecutive = 0;
            if dur > self.schedule.period() / 3 {
                // not an overrun, but 3x the typical tick: same diagnostics so that rare compute spikes can be attributed
                let (cpu_ms, runq_wait_ms) = match (sched0, sched1) {
                    (Some(a), Some(b)) => (
                        b.0.saturating_sub(a.0) as f64 / 1e6,
                        b.1.saturating_sub(a.1) as f64 / 1e6,
                    ),
                    _ => (-1.0, -1.0),
                };
                tracing::info!(
                    event = "tick_slow",
                    tick_ms = dur.as_secs_f64() * 1e3,
                    cpu_ms,
                    runq_wait_ms,
                    tick = self.ticks,
                    phase_inputs_ms = self.phase_ms[0],
                    phase_step_ms = self.phase_ms[1],
                    phase_post_step_ms = self.phase_ms[2],
                    phase_bots_ms = self.phase_ms[3],
                    phase_snapshot_ms = self.phase_ms[4],
                    phase_send_ms = self.phase_ms[5],
                    "tick slower than a third of its budget"
                );
            }
        }
        if let Some(t) = self.world.step_timings().filter(|_| {
            dur > self.schedule.period() / 3 || self.step_trace_ticks.contains(&self.ticks)
        }) {
            let ms = |ns: u64| ns as f64 / 1e6;
            tracing::info!(
                event = "tick_step_trace",
                tick = self.ticks,
                world_tick = self.world.tick().0,
                tick_ms = dur.as_secs_f64() * 1e3,
                phase_step_ms = self.phase_ms[1],
                movement_ms = ms(t.movement),
                movement_players_ms = ms(t.movement_players),
                movement_slowest_player_ms = ms(t.movement_slowest_player),
                movement_slowest_entity = t.movement_slowest_entity,
                weapons_ms = ms(t.weapons),
                projectiles_ms = ms(t.projectiles),
                vehicles_ms = ms(t.vehicles),
                rockets_ms = ms(t.rockets),
                shells_ms = ms(t.shells),
                throwables_ms = ms(t.throwables),
                other_ms = ms(t.other),
                prepare_ms = ms(t.prepare),
                schedule_other_ms = ms(t.schedule_other),
                lagcomp_ms = ms(t.lagcomp),
                reap_wrecks_ms = ms(t.reap_wrecks),
                sync_squads_ms = ms(t.sync_squads),
                refresh_views_ms = ms(t.refresh_views),
                collect_output_ms = ms(t.collect_output),
                "simulation step system timings"
            );
        }
        self.metrics.tick_overrun_consecutive.set(self.consecutive);
        if self.consecutive > self.metrics.tick_overrun_consecutive_max.get() {
            self.metrics
                .tick_overrun_consecutive_max
                .set(self.consecutive);
        }
        let skipped = self.schedule.advance(t1);
        if skipped > 0 {
            self.metrics.tick_skipped_slots_total.inc_by(skipped);
        }
    }

    // ---- ARCHITECTURE §6.1 steps 1-7 ----

    fn tick_body(&mut self) {
        if !self.admin_disconnects.is_empty() {
            let now = self.clock.now();
            while self
                .admin_disconnects
                .front()
                .is_some_and(|(deadline, _)| *deadline <= now)
            {
                if let Some((_, handle)) = self.admin_disconnects.pop_front() {
                    handle.close_now();
                }
            }
        }
        let trace = tick_trace_enabled();
        let mut t = trace.then(std::time::Instant::now);
        let mut lap = |slot: usize, phase: &mut [f64; 6]| {
            if let Some(prev) = t {
                let n = std::time::Instant::now();
                phase[slot] = n.duration_since(prev).as_secs_f64() * 1e3;
                t = Some(n);
            }
        };
        self.handle_commands();
        // `now` is the tick this step will simulate; input windows are relative to it.
        let now = self.world.tick().0 + 1;
        self.drain_inputs(now);
        let kicked = self.bridge.process_kicks(&mut self.world);
        if !kicked.is_empty() {
            for player in kicked {
                self.c0.forget(player);
                if let Some(recorder) = &mut self.reports {
                    recorder.leave(player);
                }
                self.seat_input.forget(player);
                self.c4.leave(player);
                if let Some(host) = self.bot_host.as_mut() {
                    host.forget(player);
                }
            }
            self.update_player_gauges();
        }
        self.bridge.resolve(now);
        self.bridge.anticheat_before_step(&self.world);
        self.seat_input
            .apply(&mut self.world, self.bridge.resolved());
        lap(0, &mut self.phase_ms);
        let lagcomp_before = self.world.lagcomp_stats();
        let out = self
            .world
            .step_with_rule_inputs(self.bridge.resolved(), self.bridge.real_inputs());
        lap(1, &mut self.phase_ms);
        self.metrics
            .observe_resource_overflow(out.resource_overflow);
        self.step_events = out.events;
        if let Some(recorder) = &mut self.reports {
            recorder.record(self.clock.now(), self.bridge.resolved(), &self.step_events);
        }
        self.c4.tick(
            &self.world,
            &self.bridge,
            self.bot_host.as_ref(),
            &self.step_events,
            &out.rule_changes,
        );
        self.apply_server_actions(out.server_actions);
        self.bridge.anticheat_after_step(&self.step_events);
        self.bridge.report_if_due(std::time::Instant::now());
        CURRENT_TICK.store(self.world.tick().0, Ordering::Relaxed);
        let lagcomp_after = self.world.lagcomp_stats();
        self.metrics
            .lagcomp_history_clamped_total
            .inc_by(lagcomp_after.history_clamped - lagcomp_before.history_clamped);
        self.metrics
            .lagcomp_wall_reject_total
            .inc_by(lagcomp_after.wall_rejected - lagcomp_before.wall_rejected);
        lap(2, &mut self.phase_ms);
        self.bots_think();
        lap(3, &mut self.phase_ms);
        self.aoi_and_snapshot();
        lap(4, &mut self.phase_ms);
        self.send();
        lap(5, &mut self.phase_ms);
    }

    fn handle_commands(&mut self) {
        while let Ok(c) = self.commands_rx.try_recv() {
            match c {
                Command::Join { player, team } => {
                    self.make_room_for_human(team);
                    let identity = self
                        .bridge
                        .net()
                        .and_then(|net| net.sessions.ticket_identity_of(player));
                    let admin = identity.as_ref().is_some_and(|ticket| ticket.is_admin);
                    let guest = identity
                        .as_ref()
                        .map(crate::gateway_link::Admitted::match_guest);
                    if let Err(error) = self.world.admit_human(player, team, admin, guest) {
                        tracing::warn!(?error, player, "roster rejected join");
                        if let Some(handle) = self
                            .bridge
                            .net()
                            .and_then(|net| net.sessions.handle_of(player))
                        {
                            handle.close_graceful();
                        }
                        continue;
                    }
                    if let (Some(recorder), Some(identity)) = (&mut self.reports, identity) {
                        recorder.join(player, identity);
                    }
                    if !self.dev_spawn_vehicle {
                        if let Some(map) = self.bridge.map().filter(|m| m.uses_tactical_deploy()) {
                            // Tactical maps use C0 SquadOp/DeployRequest for role and first body.
                            // The compact M0 sandbox and vehicle fixture keep immediate spawn.
                            let (_, yaw) = map.join_spawn(player, team);
                            self.bridge.open_session(player, team, yaw);
                            self.c4.join(&self.bridge, player);
                            self.update_player_gauges();
                            continue;
                        }
                    }
                    match self.bridge.join(&mut self.world, player, team) {
                        Ok(()) => {
                            if self.dev_spawn_vehicle && player == 0 {
                                self.setup_fixture_driver(player);
                            }
                            self.c4.join(&self.bridge, player);
                            self.update_player_gauges();
                        }
                        Err(e) => {
                            self.world.roster_remove(player);
                            if let Some(handle) = self
                                .bridge
                                .net()
                                .and_then(|net| net.sessions.handle_of(player))
                            {
                                handle.close_graceful();
                            }
                            tracing::warn!(player, ?e, "spawn_player rejected a join");
                        }
                    }
                }
                Command::SpawnPlayer { req } => match self.bridge.spawn(&mut self.world, req) {
                    Ok(()) => {
                        self.c4.join(&self.bridge, req.player);
                        self.update_player_gauges();
                    }
                    Err(e) => tracing::warn!(
                        event = "spawn_failed",
                        player_id = req.player as u64,
                        error = ?e,
                        "spawn_player rejected"
                    ),
                },
                Command::C0 {
                    player,
                    msg: ClientMsgC0::MarkerPlace(marker),
                } => match self.comms.place_manual(&self.world, player, marker) {
                    Ok(()) => self
                        .world
                        .record_rule_activity(player, frontline_sim::serverrules::Activity::Map),
                    Err(key) => self.send_c0(vec![(player, crate::c0::comms::notice(key))]),
                },
                Command::C0 {
                    player,
                    msg: ClientMsgC0::PingPlace(ping),
                } => match self.comms.place_ping(&self.world, player, ping) {
                    Ok(()) => self
                        .world
                        .record_rule_activity(player, frontline_sim::serverrules::Activity::Map),
                    Err(key) => self.send_c0(vec![(player, crate::c0::comms::notice(key))]),
                },
                Command::C0 { player, msg } if crate::c0::server_rules::is_request(&msg) => {
                    self.handle_server_rule(player, msg);
                }
                Command::ReportSquadMute { by, target } => {
                    self.comms.report_squad_mute(&self.world, by, target);
                }
                Command::C0 {
                    player,
                    msg: ClientMsgC0::Report(report),
                } if self.reports.is_some() => {
                    self.submit_report(player, report);
                }
                Command::C0 { player, msg } => {
                    let mut out = Vec::new();
                    self.c0
                        .handle(&mut self.world, &self.metrics, player, msg, &mut out);
                    if let Some(recorder) = &mut self.reports {
                        recorder.chat(self.clock.now(), &out);
                    }
                    self.send_c0(out);
                }
                Command::SetBotFill(n) => self.set_bot_fill(n),
                Command::RemovePlayer(p) | Command::ClientGone(p) => self.leave_player(p),
                Command::EnableReports {
                    reports,
                    chat_updates,
                } => {
                    if self.reports.is_none() {
                        self.reports = Some(crate::report::Recorder::new(
                            reports,
                            chat_updates,
                            self.clock.now(),
                        ));
                    }
                }
                Command::Admin { action, reply } => {
                    if let Some(tx) = reply.0.lock().expect("admin reply poisoned").take() {
                        let _ = tx.send(self.admin_action(action));
                    }
                }
            }
        }
    }

    fn leave_player(&mut self, p: PlayerId) {
        if let Some(host) = self.bot_host.as_mut() {
            if let Some(s) = self.bridge.session(p) {
                host.human_left(p, s.team, self.world.tick().0);
            }
            host.forget(p);
        }
        if let Some(recorder) = &mut self.reports {
            recorder.leave(p);
        }
        self.c0.forget(p);
        self.comms.forget(p, &mut self.snap, self.world.tick());
        self.seat_input.forget(p);
        self.snap.forget(p);
        self.c4.leave(p);
        self.bridge.leave(&mut self.world, p);
        self.update_player_gauges();
    }

    fn submit_report(&mut self, player: PlayerId, report: frontline_protocol::msg::c0::Report) {
        let Some(net) = self.bridge.net() else { return };
        if self.bridge.session(player).is_none() || self.bridge.session(report.target).is_none() {
            return;
        }
        let (Some(from), Some(target)) = (
            net.sessions.ticket_identity_of(player),
            net.sessions.ticket_identity_of(report.target),
        ) else {
            return;
        };
        if self.reports.as_mut().is_some_and(|recorder| {
            recorder.submit(self.clock.now(), player, from.jti, target.jti, report)
        }) {
            self.metrics.reports_total.inc();
        }
    }

    fn admin_action(&mut self, action: Action) -> ActionResult {
        match action.action.as_str() {
            "kick" | "mute" => {
                let jti = match action
                    .jti
                    .as_deref()
                    .map(crate::gateway_link::parse_lower_hex)
                {
                    Some(Some(jti)) => Some(jti),
                    Some(None) => return ActionResult::TargetMissing,
                    None => None,
                };
                let Some((player, handle)) = self
                    .bridge
                    .net()
                    .and_then(|net| net.sessions.admin_target(jti, action.player_id))
                    .filter(|(slot, _)| self.bridge.session(*slot).is_some())
                else {
                    return ActionResult::TargetMissing;
                };
                if action.action == "kick" {
                    crate::net::conn::disconnect(
                        &handle,
                        frontline_protocol::msg::c0::disconnect_reason::KICKED,
                    );
                    // Allow the transport to flush KICKED, even if the peer is no longer reading.
                    self.admin_disconnects
                        .push_back((self.clock.now() + Duration::from_secs(1), handle));
                    self.leave_player(player);
                } else {
                    self.c0.mute(player);
                }
                ActionResult::Applied
            }
            "broadcast" => {
                let Some(text) = action
                    .text
                    .as_deref()
                    .and_then(|raw| crate::text::sanitize_chat(raw).ok())
                else {
                    return ActionResult::Unsupported;
                };
                let text = format!("[Admin] {text}");
                if text.chars().count() > frontline_protocol::msg::c0::MAX_CHAT_LEN {
                    return ActionResult::Unsupported;
                }
                let message = ServerMsgC0::Chat(Chat {
                    channel: ChatChannel::All,
                    text,
                    sender: u8::MAX,
                });
                self.send_c0(
                    self.bridge
                        .sessions()
                        .map(|(&slot, _)| (slot, message.clone()))
                        .collect(),
                );
                ActionResult::Applied
            }
            // The current team switch is roster-only and the map is loaded once at startup.
            _ => ActionResult::Unsupported,
        }
    }

    fn setup_fixture_driver(&mut self, player: PlayerId) {
        // Exercise the real boarding eligibility checks instead of bypassing them.
        let result = (|| {
            self.world
                .squad_op(
                    player,
                    SquadOp::Create {
                        kind: SquadKind::ArmorCrew,
                        suffix: String::new(),
                    },
                )
                .map_err(|e| format!("armor squad: {e:?}"))?;
            self.world
                .set_role(player, Some("crewman"))
                .map_err(|e| format!("crewman: {e:?}"))
        })();
        if let Err(error) = result {
            tracing::warn!(
                event = "vehicle_fixture_driver_setup_failed",
                error,
                "could not configure the load-test driver"
            );
        }
    }

    fn set_bot_fill(&mut self, target: u8) {
        self.bot_fill = (target > 0).then_some(target);
        if target == 0 || self.tactical_map_ready {
            return;
        }
        let map = self.bridge.map().cloned();
        if let Some(map) = &map {
            map.apply_tactical(&mut self.world);
        } else {
            // Bare sim fixtures have no map loader to supply deploy points.
            for team in [Team::Us, Team::Ger] {
                self.world.set_team_spawn(
                    team,
                    frontline_sim::Vec3::new(0.0, 0.0, (team as u8 as f32) * 16.0),
                );
            }
        }
        if self.bot_host.is_none() {
            let sessions = self.bridge.net().map(|link| link.sessions.clone());
            self.bot_host = Some(BotHost::with_soldiers(
                0,
                self.world.config().match_seed,
                &mut self.world,
                &mut self.bridge,
                map.as_deref(),
                sessions.as_ref(),
                self.metrics.clone(),
            ));
        }
        self.tactical_map_ready = true;
    }

    /// Delivers C0 replies / routed chat over each recipient's reliable stream.
    fn send_c0(&self, out: crate::c0::Outbox) {
        let Some(link) = self.bridge.net() else {
            return;
        };
        for (p, m) in out {
            let Some(h) = link.sessions.handle_of(p) else {
                continue;
            };
            if let Ok(b) = frontline_protocol::msg::c0::encode_c0_server(&m) {
                h.send(
                    frontline_protocol::Channel::C0,
                    bytes::Bytes::from(b),
                    crate::net::transport::Rel::Reliable,
                );
            }
        }
    }

    fn update_player_gauges(&mut self) {
        // `server_players` counts connected humans; bots have their own gauge (`server_bots`).
        let bots = |t: Team| self.bot_host.as_ref().map_or(0, |b| b.count_team(t));
        let mut humans = 0;
        for (team, label) in [(Team::Us, "us"), (Team::Ger, "ger")] {
            let bots = bots(team);
            let count = self.bridge.count_team(team).saturating_sub(bots);
            humans += count;
            self.metrics
                .players
                .with_label_values(&[label])
                .set(count as i64);
            self.metrics
                .bots_active
                .with_label_values(&[label])
                .set(bots as i64);
        }
        if self.world.match_state().phase == frontline_sim::MatchPhaseKind::Warmup {
            self.world
                .set_human_players(humans.min(usize::from(u8::MAX)) as u8);
        }
    }

    /// Step 1: drain the network->sim channels (bounded by their capacity) into the per-player
    /// input buffers: C1 bundles from the connection layer, then single commands from the
    /// in-process channel (bots / tests). Both go through the same validation.
    fn drain_inputs(&mut self, now: u32) {
        self.bridge.drain_net(now);
        for _ in 0..INPUT_CHANNEL_CAP {
            match self.inputs_rx.try_recv() {
                Ok(e) => {
                    self.bridge.ingest_cmd(e.player, &e.cmd, now);
                }
                Err(_) => break,
            }
        }
    }

    /// Step 5 (M0-026): every bot thinks (sharded inside the brains) and its command for the
    /// next tick enters the same validation path as a human's. Also tallies this tick's events.
    fn bots_think(&mut self) {
        self.tally.add(&self.step_events);
        let mut changed = false;
        if let Some(host) = self.bot_host.as_mut() {
            let start = std::time::Instant::now();
            let now = self.world.tick().0;
            // Once a second: top up / make room (M1-039c). Spawned bots think from the next tick.
            if let (Some(target), true) = (self.bot_fill, now.is_multiple_of(30)) {
                changed = host.reconcile(now, &mut self.world, &mut self.bridge, target) != (0, 0);
            }
            host.gameplay(
                &mut self.world,
                &self.step_events,
                self.bridge.map().map(AsRef::as_ref),
            );
            host.tick_started(&self.world, &mut self.bridge, start);
        }
        if changed {
            self.update_player_gauges();
        }
    }

    /// Step 6 (M0-019): AOI selection and snapshot encoding for every connected client that
    /// is due this tick. Bots have no connection and get no packets.
    fn aoi_and_snapshot(&mut self) {
        self.outgoing.clear();
        self.clients_in.clear();
        // Without a network link (pure-sim tests) there are no clients, but the scheduler still
        // runs: it tracks spawns / shots and counts down the degradation guard.
        if let Some(net) = self.bridge.net() {
            for (p, s) in self.bridge.sessions() {
                let Some(h) = net.sessions.handle_of(*p) else {
                    continue;
                };
                self.clients_in.push(ClientIn {
                    player: *p,
                    team: s.team,
                    ack_low: s.input.state.last_ack_snapshot_tick,
                    pred_hash: s.input.state.last_pred_hash,
                    ack_input_seq: s.input.last_applied_seq(),
                    input_slack: s.input.input_slack(),
                    link: LinkInfo::from_max_datagram(h.max_datagram_size())
                        .with_sample(h.link_sample()),
                });
            }
        }
        let now = self.world.tick().0;
        self.comms
            .tick(&self.world, &self.clients_in, &mut self.snap);
        self.snap.tick(
            &self.world,
            &self.step_events,
            &self.clients_in,
            now,
            &mut self.outgoing,
        );
        if self.team_channel.due(now) {
            if let Some(net) = self.bridge.net() {
                let clients: Vec<_> = self
                    .clients_in
                    .iter()
                    .filter_map(|c| {
                        let h = net.sessions.handle_of(c.player)?;
                        Some((c.player, c.team, h.accepts_team_markers()))
                    })
                    .collect();
                self.team_outgoing = self.team_channel.tick_with_projected_markers(
                    &self.world,
                    GameData::embedded(),
                    &clients,
                    now,
                    |player| self.comms.legacy(player, frontline_sim::Tick(now)),
                );
            }
        }
    }

    /// Step 7 (M0-019): hand the packets to the connection writers (never blocks).
    fn send(&mut self) {
        let Some(net) = self.bridge.net() else {
            self.outgoing.clear();
            return;
        };
        for (player, notice) in self.snap.drain_budget_notices() {
            use crate::net::regulator::BudgetNotice;
            use frontline_protocol::msg::c4::{encode_c4, notice_code, Notice, ServerEventC4};
            let (code, percent) = match notice {
                BudgetNotice::Degraded(percent) => (notice_code::NET_DEGRADED, i32::from(percent)),
                BudgetNotice::Recovered => (notice_code::NET_RECOVERED, 0),
            };
            if let Some(h) = net.sessions.handle_of(player) {
                if let Ok(payload) = encode_c4(&ServerEventC4::Notice(Notice {
                    code,
                    args: [percent, 0, 0],
                })) {
                    h.send(
                        frontline_protocol::Channel::C4,
                        bytes::Bytes::from(payload),
                        crate::net::transport::Rel::Reliable,
                    );
                }
            }
        }
        for o in self.outgoing.drain(..) {
            if let Some(h) = net.sessions.handle_of(o.player) {
                h.send(
                    frontline_protocol::Channel::C2,
                    bytes::Bytes::from(o.bytes),
                    crate::net::transport::Rel::Unreliable,
                );
            }
        }
        let tick = self.world.tick().0;
        if crate::snapshot::world_state::due(&self.world, tick) {
            use frontline_protocol::msg::team::encode_world_state_c2;
            let payloads =
                crate::snapshot::world_state::project(&self.world, tick).map(|projection| {
                    encode_world_state_c2(&projection)
                        .ok()
                        .map(bytes::Bytes::from)
                });
            for (player, session) in self.bridge.sessions() {
                let Some(handle) = net
                    .sessions
                    .handle_of(*player)
                    .filter(|h| h.accepts_resources())
                else {
                    continue;
                };
                // Roster authority changes before routing any new projection.
                let Some(entry) = self
                    .world
                    .squads()
                    .entry(*player)
                    .filter(|e| e.team == session.team)
                else {
                    continue;
                };
                if let Some(payload) = &payloads[entry.team as usize] {
                    handle.send(
                        frontline_protocol::Channel::C2,
                        payload.clone(),
                        crate::net::transport::Rel::Unreliable,
                    );
                }
            }
        }
        for (player, payload) in self.team_outgoing.drain(..) {
            if let Some(h) = net.sessions.handle_of(player) {
                h.send(
                    frontline_protocol::Channel::C3,
                    bytes::Bytes::from(payload),
                    crate::net::transport::Rel::Unreliable,
                );
            }
        }
    }

    /// Whether the snapshot rate is currently reduced to 15 Hz (3 consecutive tick overruns).
    pub fn snapshot_degraded(&self) -> bool {
        self.snap.is_degraded()
    }

    /// Snapshot statistics so far (benches, tests, the exit report).
    pub fn snapshot_stats(&self) -> &crate::snapshot::SchedStats {
        &self.snap.stats
    }

    /// Drops collected samples (used to exclude warm-up from a measurement window).
    pub fn discard_samples(&mut self) {
        self.tick_ns.clear();
        self.err_ns.clear();
    }

    pub fn report(&mut self) -> Report {
        let elapsed = self.clock.now().saturating_duration_since(self.started);
        Report {
            ticks: self.ticks,
            tick_ms: self.tick_ns.quantiles(),
            tick_interval_error_ms: self.err_ns.quantiles(),
            tick_overrun_total: self.metrics.tick_overrun_total.get(),
            tick_overrun_consecutive_max: self.metrics.tick_overrun_consecutive_max.get() as u64,
            tick_skipped_slots_total: self.metrics.tick_skipped_slots_total.get(),
            input_queue_full_total: self.metrics.input_queue_full_total.get(),
            elapsed_s: elapsed.as_secs_f64(),
            events: self.tally,
            score_summary: self.world.score_summary(),
        }
    }
}

#[cfg(test)]
mod c4_netem;

#[cfg(test)]
mod score_audit;

#[cfg(test)]
mod seat_input_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::fake::FakeClock;

    fn cfg() -> SimThreadConfig {
        SimThreadConfig::bare(1, MapId(0), GameMode::Sandbox)
    }

    fn mk(c: &FakeClock) -> (SimLoop<&FakeClock>, Arc<Metrics>) {
        let m = Metrics::new();
        let (lp, _tx, _ctx) = build_loop(c, &cfg(), m.clone(), StopFlag::new());
        (lp, m)
    }

    #[test]
    fn score_summary_is_exported_and_projects_losslessly_to_gateway_rows() {
        use crate::gateway_link::result::MatchResult;
        use frontline_sim::score::stats::PlayerStats;

        let c = FakeClock::new();
        let (mut lp, _) = mk(&c);
        lp.tick();
        let mut report = lp.report();
        assert_eq!(report.ticks, 1);
        assert_eq!(report.score_summary, lp.world.score_summary());
        report.score_summary.players = vec![PlayerStats {
            player: 7,
            kills: 70_000,
            deaths: 80_000,
            total_score: -3_000_000_000,
            ..Default::default()
        }];
        let export = serde_json::to_value(&report).unwrap();
        assert_eq!(
            export["score_summary"]["players"][0]["total_score"],
            -3_000_000_000i64
        );
        let result = MatchResult::from_report(1, 0, 0, &report);
        assert_eq!(result.score_summary.as_ref(), Some(&report.score_summary));
        assert_eq!(
            serde_json::to_value(&result.rows).unwrap(),
            serde_json::json!([{ "player": 7, "kills": 70_000, "deaths": 80_000, "score": -3_000_000_000i64 }]),
        );
    }

    #[test]
    fn fifty_ms_tick_counts_one_overrun_and_next_deadline_stays_on_grid() {
        let c = FakeClock::new();
        let (mut lp, m) = mk(&c);
        let base = lp.schedule.current_deadline(); // slot 0 deadline
        lp.tick_with(|_| c.advance(Duration::from_millis(50)));
        assert_eq!(m.tick_overrun_total.get(), 1);
        // Slot 1 (base+33.3 ms) was missed and skipped; next is slot 2 on the ideal grid.
        let d = lp.schedule.current_deadline();
        assert_eq!(d.duration_since(base), TICK_PERIOD.mul_f64(2.0));
        assert!(d > c.now());
        // A normal tick afterwards: no new overrun, interval error is ~0 (fake clock).
        lp.tick_with(|_| c.advance(Duration::from_millis(1)));
        assert_eq!(m.tick_overrun_total.get(), 1);
        assert_eq!(m.tick_overrun_consecutive.get(), 0);
        assert_eq!(
            lp.schedule.current_deadline().duration_since(base),
            TICK_PERIOD.mul_f64(3.0)
        );
        assert_eq!(m.tick_skipped_slots_total.get(), 1);
    }

    /// M1-039c: `SetBotFill` tops every team up to the target within a second and the gauge follows.
    #[test]
    fn set_bot_fill_tops_teams_up_within_a_second() {
        let c = FakeClock::new();
        let m = Metrics::new();
        let (mut lp, _tx, ctx) = build_loop_with_bots(&c, &cfg(), 2, m.clone(), StopFlag::new());
        assert_eq!(m.bots.get(), 2);
        ctx.send(Command::SetBotFill(4)).unwrap();
        for _ in 0..31 {
            lp.tick();
        }
        let host = lp.bot_host().unwrap();
        assert_eq!(
            (host.count_team(Team::Us), host.count_team(Team::Ger)),
            (4, 4)
        );
        assert_eq!(m.bots.get(), 8);
        ctx.send(Command::SetBotFill(0)).unwrap();
        for _ in 0..60 {
            lp.tick();
        }
        assert_eq!(
            lp.bot_host().unwrap().len(),
            8,
            "0 switches fill off, bots stay"
        );
    }

    #[test]
    fn three_consecutive_overruns_set_max_to_three() {
        let c = FakeClock::new();
        let (mut lp, m) = mk(&c);
        for _ in 0..3 {
            lp.tick_with(|_| c.advance(Duration::from_millis(50)));
        }
        assert_eq!(m.tick_overrun_total.get(), 3);
        assert_eq!(m.tick_overrun_consecutive.get(), 3);
        assert_eq!(m.tick_overrun_consecutive_max.get(), 3);
        lp.tick_with(|_| c.advance(Duration::from_millis(1)));
        assert_eq!(m.tick_overrun_consecutive.get(), 0);
        assert_eq!(m.tick_overrun_consecutive_max.get(), 3);
    }

    #[test]
    fn three_consecutive_overruns_reduce_the_snapshot_rate_and_recovery_restores_it() {
        let c = FakeClock::new();
        let (mut lp, m) = mk(&c);
        assert!(!lp.snapshot_degraded());
        for _ in 0..2 {
            lp.tick_with(|_| c.advance(Duration::from_millis(50)));
        }
        assert!(!lp.snapshot_degraded(), "two overruns are tolerated");
        lp.tick_with(|_| c.advance(Duration::from_millis(50)));
        assert!(lp.snapshot_degraded(), "the third one triggers the guard");
        assert_eq!(m.tick_overrun_consecutive.get(), 3);
        // the hold time is 90 ticks of the scheduler pass; here the passes run via tick()
        for _ in 0..crate::snapshot::DEGRADE_HOLD_TICKS + 2 {
            lp.tick();
        }
        assert!(!lp.snapshot_degraded());
        assert_eq!(
            lp.snapshot_stats().degraded_ticks,
            crate::snapshot::DEGRADE_HOLD_TICKS as u64
        );
    }

    #[test]
    fn full_input_channel_drops_new_inputs_and_counts() {
        let c = FakeClock::new();
        let m = Metrics::new();
        let (_lp, tx, _ctx) = build_loop(&c, &cfg(), m.clone(), StopFlag::new());
        let cmd = InputCmd {
            seq: 0,
            tick: 0,
            move_x: 0,
            move_z: 0,
            yaw: 0,
            pitch: 0,
            buttons: 0,
            weapon_slot: 0,
            aux: 0,
            view_tick: 0,
        };
        let e = InputEnvelope { player: 1, cmd };
        for _ in 0..INPUT_CHANNEL_CAP {
            assert!(tx.try_send(e));
        }
        assert!(!tx.try_send(e));
        assert!(!tx.try_send(e));
        assert_eq!(m.input_queue_full_total.get(), 2);
    }

    #[test]
    fn samples_are_bounded_and_keep_the_exact_maximum() {
        let mut s = Samples::new();
        for i in 1..=(SAMPLE_CAP as u32 * 4) {
            s.push(i);
        }
        assert_eq!(s.v.len(), SAMPLE_CAP, "memory is constant");
        assert!(s.v.capacity() <= SAMPLE_CAP * 2);
        let q = s.quantiles();
        assert_eq!(q.max, f64::from(SAMPLE_CAP as u32 * 4) / 1e6, "exact max");
        // uniform reservoir: the median / p99 of 4 x CAP uniform values stay within 2 % of the true ones
        let n = f64::from(SAMPLE_CAP as u32 * 4);
        assert!((q.p50 * 1e6 - n * 0.5).abs() < n * 0.02, "p50 {}", q.p50);
        assert!((q.p99 * 1e6 - n * 0.99).abs() < n * 0.02, "p99 {}", q.p99);
        s.clear();
        assert_eq!(s.quantiles().max, 0.0);
    }

    #[test]
    fn quantiles_basic() {
        let mut v: Vec<u32> = (1..=100).map(|i| i * 1_000_000).collect();
        let q = quantiles(&mut v);
        assert_eq!(q.max, 100.0);
        assert!((99.0..=100.0).contains(&q.p99));
        assert!((q.p50 - 51.0).abs() <= 1.0);
    }
}
