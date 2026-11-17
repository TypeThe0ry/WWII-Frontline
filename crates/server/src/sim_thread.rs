//! The 30 Hz simulation thread (ARCHITECTURE §6.1, §6.10). Runs on a dedicated OS thread,
//! never on a tokio worker. Input/command channels are bounded `crossbeam` channels.

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::{
    CollisionData, GameData, GameMode, MapId, PlayerId, SimConfig, SpawnReq, Team, World,
};
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
    SpawnPlayer {
        req: SpawnReq,
    },
    RemovePlayer(PlayerId),
    /// The client's connection closed (published by the net layer; M0-018 handles the session).
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

#[derive(Debug, Clone)]
pub struct SimThreadConfig {
    pub match_seed: u64,
    pub map: MapId,
    pub mode: GameMode,
    pub pin_core: Option<usize>,
}

pub struct SimLoop<C: Clock> {
    clock: C,
    schedule: TickSchedule,
    world: World,
    metrics: Arc<Metrics>,
    inputs_rx: Receiver<InputEnvelope>,
    commands_rx: Receiver<Command>,
    stop: StopFlag,
    /// Inputs drained this tick, keyed (and therefore ordered) by player.
    fresh: BTreeMap<PlayerId, InputCmd>,
    team_of: BTreeMap<PlayerId, Team>,
    consecutive: i64,
    ticks: u64,
    tick_ns: Vec<u32>,
    err_ns: Vec<u32>,
    started: std::time::Instant,
}

/// Creates the channel pair plus a loop that is not yet running (tests drive it directly).
pub fn build_loop<C: Clock>(
    clock: C,
    cfg: &SimThreadConfig,
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
    let world = World::new(sim_cfg, &CollisionData::empty());
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
        fresh: BTreeMap::new(),
        team_of: BTreeMap::new(),
        consecutive: 0,
        ticks: 0,
        tick_ns: Vec::new(),
        err_ns: Vec::new(),
    };
    (lp, InputSender { tx: itx, metrics }, ctx)
}

/// Spawns the dedicated `sim` OS thread.
pub fn spawn(
    cfg: SimThreadConfig,
    metrics: Arc<Metrics>,
    stop: StopFlag,
) -> std::io::Result<SimThreadHandle> {
    let (lp, inputs_tx, commands_tx) = build_loop(SystemClock, &cfg, metrics, stop);
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
            if self.consecutive == 3 {
                // Degradation hook: M0-019 halves the snapshot rate on this signal.
                tracing::warn!(
                    event = "tick_overrun_degrade",
                    consecutive = self.consecutive,
                    "3 consecutive tick overruns"
                );
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
        self.drain_inputs();
        let inputs = self.apply_inputs();
        self.world.step(&inputs);
        CURRENT_TICK.store(self.world.tick().0, Ordering::Relaxed);
        self.record_lag_comp();
        self.bots_think();
        self.aoi_and_snapshot();
        self.send();
    }

    fn handle_commands(&mut self) {
        while let Ok(c) = self.commands_rx.try_recv() {
            match c {
                Command::SpawnPlayer { req } => match self.world.spawn_player(req) {
                    Ok(_) => {
                        self.team_of.insert(req.player, req.team);
                        self.update_player_gauges();
                    }
                    Err(e) => tracing::warn!(
                        event = "spawn_failed",
                        player_id = req.player as u64,
                        error = ?e,
                        "spawn_player rejected"
                    ),
                },
                Command::RemovePlayer(p) | Command::ClientGone(p) => {
                    self.world.remove_player(p);
                    self.team_of.remove(&p);
                    self.update_player_gauges();
                }
            }
        }
    }

    fn update_player_gauges(&self) {
        let count = |t: Team| self.team_of.values().filter(|&&x| x == t).count() as i64;
        self.metrics
            .players
            .with_label_values(&["us"])
            .set(count(Team::Us));
        self.metrics
            .players
            .with_label_values(&["ger"])
            .set(count(Team::Ger));
    }

    /// Step 1: drain the network->sim channel (bounded by its capacity).
    fn drain_inputs(&mut self) {
        for _ in 0..INPUT_CHANNEL_CAP {
            match self.inputs_rx.try_recv() {
                Ok(e) => {
                    self.fresh.insert(e.player, e.cmd);
                }
                Err(_) => break,
            }
        }
    }

    /// Step 2: this tick's inputs, ascending by player.
    /// TODO(M0-018): per-player tick matching, repeat-last <= 5 ticks, validation.
    fn apply_inputs(&mut self) -> Vec<(PlayerId, InputCmd)> {
        std::mem::take(&mut self.fresh).into_iter().collect()
    }

    /// Step 4. TODO(M0-020): lag-compensation history.
    fn record_lag_comp(&mut self) {}

    /// Step 5. TODO(M0-026): bot think (sharded).
    fn bots_think(&mut self) {}

    /// Step 6. TODO(M0-019): AOI selection and snapshot encoding.
    fn aoi_and_snapshot(&mut self) {}

    /// Step 7. TODO(M0-019): hand packets to the network thread.
    fn send(&mut self) {}

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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::fake::FakeClock;

    fn cfg() -> SimThreadConfig {
        SimThreadConfig {
            match_seed: 1,
            map: MapId(0),
            mode: GameMode::Sandbox,
            pin_core: None,
        }
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
