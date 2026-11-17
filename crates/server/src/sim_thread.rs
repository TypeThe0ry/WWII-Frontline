//! The 30 Hz simulation thread (ARCHITECTURE §6.1, §6.10). Runs on a dedicated OS thread,
//! never on a tokio worker. Input/command channels are bounded `crossbeam` channels.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::{
    CollisionData, GameData, GameMode, MapId, PlayerId, SimConfig, SimEvent, SpawnReq, Team, World,
};

use crate::bot_host::BotHost;
use crate::sim_bridge::{NetLink, SimBridge};
use crate::snapshot::{ClientIn, LinkInfo, Outgoing, SnapshotScheduler, DEGRADE_HOLD_TICKS};
use crate::validate::ValidateCfg;
use crate::world_loader::LoadedMap;
use serde::Serialize;

use crate::clock::{Clock, SystemClock, TickSchedule};
use crate::logging::CURRENT_TICK;
use crate::metrics::Metrics;
use crate::shutdown::StopFlag;

pub const INPUT_CHANNEL_CAP: usize = 1024;
pub const COMMAND_CHANNEL_CAP: usize = 256;
/// 1/30 s in nanoseconds (PI-006: fixed step 1000/30 ms).
pub const TICK_PERIOD: Duration = Duration::from_nanos(33_333_333);
/// Cap on stored per-tick samples for the exit report (≈ 37 h at 30 Hz).
const MAX_SAMPLES: usize = 4_000_000;

#[derive(Debug, Clone, Copy)]
pub struct InputEnvelope {
    pub player: PlayerId,
    pub cmd: InputCmd,
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Serialize, Default)]
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
            net: None,
        }
    }
}

pub struct SimLoop<C: Clock> {
    clock: C,
    schedule: TickSchedule,
    world: World,
    metrics: Arc<Metrics>,
    inputs_rx: Receiver<InputEnvelope>,
    commands_rx: Receiver<Command>,
    stop: StopFlag,
    bridge: SimBridge,
    consecutive: i64,
    ticks: u64,
    tick_ns: Vec<u32>,
    err_ns: Vec<u32>,
    started: std::time::Instant,
    /// Snapshot / AOI scheduler (M0-019) and its per-tick scratch.
    snap: SnapshotScheduler,
    step_events: Vec<SimEvent>,
    clients_in: Vec<ClientIn>,
    outgoing: Vec<Outgoing>,
    /// Bots hosted in this process (`--bots N`, M0-026); `None` without bots.
    bot_host: Option<BotHost>,
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
    if let Some(m) = &cfg.map_data {
        m.apply_to(&mut world);
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
    let lp = SimLoop {
        schedule: TickSchedule::new(now + TICK_PERIOD, TICK_PERIOD),
        started: now,
        clock,
        world,
        metrics: metrics.clone(),
        inputs_rx: irx,
        commands_rx: crx,
        stop,
        bridge,
        consecutive: 0,
        ticks: 0,
        tick_ns: Vec::new(),
        err_ns: Vec::new(),
        snap: SnapshotScheduler::new(Some(metrics.clone())),
        step_events: Vec::new(),
        clients_in: Vec::new(),
        outgoing: Vec::new(),
        bot_host,
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
    let (lp, inputs_tx, commands_tx) =
        build_loop_with_bots(SystemClock, &cfg, bots, metrics, stop);
    let pin = cfg.pin_core;
    let join = std::thread::Builder::new()
        .name("sim".into())
        .spawn(move || {
            if let Some(core) = pin {
                tracing::warn!(
                    event = "pin_sim_core_ignored",
                    core,
                    "CPU pinning is a deployment item (M1-067); ignored"
                );
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

        work(self);

        let t1 = self.clock.now();
        let dur = t1.saturating_duration_since(t0);
        self.metrics.tick_ms.observe(dur.as_secs_f64() * 1e3);
        if self.tick_ns.len() < MAX_SAMPLES {
            self.tick_ns
                .push(dur.as_nanos().min(u32::MAX as u128) as u32);
            self.err_ns
                .push(err.as_nanos().min(u32::MAX as u128) as u32);
        }
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
            tracing::warn!(
                event = "tick_overrun",
                tick_ms = dur.as_secs_f64() * 1e3,
                consecutive = self.consecutive,
                "tick exceeded its budget"
            );
        } else {
            self.consecutive = 0;
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
        self.handle_commands();
        // `now` is the tick this step will simulate; input windows are relative to it.
        let now = self.world.tick().0 + 1;
        self.drain_inputs(now);
        let kicked = self.bridge.process_kicks(&mut self.world);
        if !kicked.is_empty() {
            self.update_player_gauges();
        }
        let inputs = self.bridge.resolve(now);
        let out = self.world.step(inputs);
        self.step_events = out.events;
        CURRENT_TICK.store(self.world.tick().0, Ordering::Relaxed);
        self.record_lag_comp();
        self.bots_think();
        self.aoi_and_snapshot();
        self.send();
    }

    fn handle_commands(&mut self) {
        while let Ok(c) = self.commands_rx.try_recv() {
            match c {
                Command::Join { player, team } => {
                    match self.bridge.join(&mut self.world, player, team) {
                        Ok(()) => self.update_player_gauges(),
                        Err(e) => tracing::warn!(
                            event = "join_failed",
                            player_id = player as u64,
                            error = ?e,
                            "spawn_player rejected a join"
                        ),
                    }
                }
                Command::SpawnPlayer { req } => match self.bridge.spawn(&mut self.world, req) {
                    Ok(()) => self.update_player_gauges(),
                    Err(e) => tracing::warn!(
                        event = "spawn_failed",
                        player_id = req.player as u64,
                        error = ?e,
                        "spawn_player rejected"
                    ),
                },
                Command::RemovePlayer(p) | Command::ClientGone(p) => {
                    self.bridge.leave(&mut self.world, p);
                    self.update_player_gauges();
                }
            }
        }
    }

    fn update_player_gauges(&self) {
        // `server_players` counts connected humans; bots have their own gauge (`server_bots`).
        let bots = |t: Team| self.bot_host.as_ref().map_or(0, |b| b.count_team(t));
        self.metrics
            .players
            .with_label_values(&["us"])
            .set(self.bridge.count_team(Team::Us).saturating_sub(bots(Team::Us)) as i64);
        self.metrics
            .players
            .with_label_values(&["ger"])
            .set(self.bridge.count_team(Team::Ger).saturating_sub(bots(Team::Ger)) as i64);
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

    /// Step 4. TODO(M0-020): lag-compensation history.
    fn record_lag_comp(&mut self) {}

    /// Step 5 (M0-026): every bot thinks (sharded inside the brains) and its command for the
    /// next tick enters the same validation path as a human's. Also tallies this tick's events.
    fn bots_think(&mut self) {
        self.tally.add(&self.step_events);
        if let Some(host) = self.bot_host.as_mut() {
            host.tick(&self.world, &mut self.bridge);
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
                    link: LinkInfo::from_max_datagram(h.max_datagram_size()),
                });
            }
        }
        let now = self.world.tick().0;
        self.snap.tick(
            &self.world,
            &self.step_events,
            &self.clients_in,
            now,
            &mut self.outgoing,
        );
    }

    /// Step 7 (M0-019): hand the packets to the connection writers (never blocks).
    fn send(&mut self) {
        let Some(net) = self.bridge.net() else {
            self.outgoing.clear();
            return;
        };
        for o in self.outgoing.drain(..) {
            if let Some(h) = net.sessions.handle_of(o.player) {
                h.send(
                    frontline_protocol::Channel::C2,
                    bytes::Bytes::from(o.bytes),
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
            tick_ms: quantiles(&mut self.tick_ns),
            tick_interval_error_ms: quantiles(&mut self.err_ns),
            tick_overrun_total: self.metrics.tick_overrun_total.get(),
            tick_overrun_consecutive_max: self.metrics.tick_overrun_consecutive_max.get() as u64,
            tick_skipped_slots_total: self.metrics.tick_skipped_slots_total.get(),
            input_queue_full_total: self.metrics.input_queue_full_total.get(),
            elapsed_s: elapsed.as_secs_f64(),
            events: self.tally,
        }
    }
}

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
    fn quantiles_basic() {
        let mut v: Vec<u32> = (1..=100).map(|i| i * 1_000_000).collect();
        let q = quantiles(&mut v);
        assert_eq!(q.max, 100.0);
        assert!((99.0..=100.0).contains(&q.p99));
        assert!((q.p50 - 51.0).abs() <= 1.0);
    }
}
