//! Glue between the network layer and the sim (M0-018): owns every player's [`PlayerSession`],
//! consumes C1 bundles from the [`InboundQueue`](crate::net::conn::InboundQueue), counts what the
//! validator did, and produces the per-tick `inputs` list for `World::step` (ascending by
//! `PlayerId`, one command for every joined player).
//!
//! No game rules live here (ARCHITECTURE §4.3.4): only input-layer tolerance and anti-abuse.

use std::collections::BTreeMap;
use std::sync::Arc;

use crossbeam_channel::Receiver;
use frontline_protocol::msg::input::{InputBundle, InputCmd};
use frontline_sim::{PlayerId, SpawnError, SpawnReq, Team, World};

use crate::anticheat::DisconnectReason;
use crate::input_buffer::PushResult;
use crate::metrics::Metrics;
use crate::net::conn::{Inbound, Sessions};
use crate::session::{yaw_code, PlayerSession, Resolution};
use crate::validate::{Reject, ValidateCfg};
use crate::world_loader::LoadedMap;

/// Bundles drained from the network per tick (bounded work; the queue itself holds 1024).
pub const MAX_BUNDLES_PER_TICK: usize = 2048;

/// What the sim thread needs from the net layer.
#[derive(Clone)]
pub struct NetLink {
    pub inbound: Receiver<Inbound>,
    pub sessions: Arc<Sessions>,
}

pub struct SimBridge {
    table: BTreeMap<PlayerId, PlayerSession>,
    metrics: Arc<Metrics>,
    link: Option<NetLink>,
    map: Option<Arc<LoadedMap>>,
    cfg: ValidateCfg,
    resolved: Vec<(PlayerId, InputCmd)>,
    kicks: BTreeMap<PlayerId, DisconnectReason>,
    /// Per-player violation counts (M1-027).
    pub anticheat: crate::anticheat::AntiCheat,
    /// Bundles of players the sim has not joined yet (their `Join` command raced the first C1
    /// by less than a tick): retried once on the next tick, then counted as `no_session`.
    parked: Vec<Inbound>,
}

impl SimBridge {
    pub fn new(
        metrics: Arc<Metrics>,
        link: Option<NetLink>,
        map: Option<Arc<LoadedMap>>,
        cfg: ValidateCfg,
    ) -> Self {
        SimBridge {
            table: BTreeMap::new(),
            metrics,
            link,
            map,
            cfg,
            resolved: Vec::new(),
            kicks: BTreeMap::new(),
            anticheat: crate::anticheat::AntiCheat::new(std::time::Instant::now()),
            parked: Vec::new(),
        }
    }

    pub fn net(&self) -> Option<&NetLink> {
        self.link.as_ref()
    }

    pub fn map(&self) -> Option<&Arc<LoadedMap>> {
        self.map.as_ref()
    }

    pub fn session(&self, p: PlayerId) -> Option<&PlayerSession> {
        self.table.get(&p)
    }

    /// All sessions in ascending `PlayerId` order (the snapshot scheduler iterates these).
    pub fn sessions(&self) -> impl Iterator<Item = (&PlayerId, &PlayerSession)> {
        self.table.iter()
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    pub fn count_team(&self, t: Team) -> usize {
        self.table.values().filter(|s| s.team == t).count()
    }

    /// The commands of the last [`Self::resolve`].
    pub fn resolved(&self) -> &[(PlayerId, InputCmd)] {
        &self.resolved
    }

    /// Anti-cheat look at the commands about to be stepped (call after `resolve`).
    pub fn anticheat_before_step(&mut self, world: &World) {
        self.anticheat.before_step(world, &self.resolved);
    }

    /// Anti-cheat look at the events of the step just run.
    pub fn anticheat_after_step(&mut self, events: &[frontline_sim::SimEvent]) {
        self.anticheat.after_step(events, &self.metrics);
    }

    /// Per-player and per-kind bookkeeping of a rejection or clamp (M1-027).
    fn suspect(&mut self, p: PlayerId, r: Reject, now: u32) {
        match r {
            Reject::YawRate | Reject::PitchRate => self.metrics.view_rate_clamped_total.inc(),
            Reject::MoveClamped | Reject::BadMove => self.metrics.suspicion_move_total.inc(),
            _ => {}
        }
        if let Some(v) = crate::anticheat::Violation::of(r) {
            self.anticheat.note(p, v);
        }
        if let Some(reason) = self.anticheat.input_violation(p, r, now) {
            self.kicks.entry(p).or_insert(reason);
        }
    }

    fn count(&self, r: Reject) {
        self.metrics
            .input_rejected_total
            .with_label_values(&[r.label()])
            .inc();
    }

    /// Spawns the player into `world` at the join point and opens its session.
    pub fn join(
        &mut self,
        world: &mut World,
        player: PlayerId,
        team: Team,
    ) -> Result<(), SpawnError> {
        let (pos, yaw) = crate::world_loader::join_spawn_at(self.map.as_deref(), player, team);
        self.spawn(
            world,
            SpawnReq {
                player,
                team,
                pos,
                yaw,
            },
        )
    }

    /// Spawns with an explicit request (tests, benches, bots) and opens a session.
    pub fn spawn(&mut self, world: &mut World, req: SpawnReq) -> Result<(), SpawnError> {
        world.spawn_player(req)?;
        self.open_session(req.player, req.team, req.yaw);
        Ok(())
    }

    /// Opens the input session for a deploy-managed player (who may not have a body yet).
    pub(crate) fn open_session(&mut self, player: PlayerId, team: Team, yaw: f32) {
        let mut s = PlayerSession::new(player, team, self.cfg, yaw_code(yaw));
        s.joined();
        self.table.insert(player, s);
    }

    /// Removes the player (disconnect, kick). Idempotent; returns whether a session existed.
    pub fn leave(&mut self, world: &mut World, player: PlayerId) -> bool {
        world.remove_player(player);
        world.roster_remove(player);
        self.anticheat.forget(player);
        self.kicks.remove(&player);
        match self.table.remove(&player) {
            Some(mut s) => {
                s.gone();
                true
            }
            None => false,
        }
    }

    /// One command of `player` (bots and the legacy single-command channel use this too).
    pub fn ingest_cmd(&mut self, player: PlayerId, cmd: &InputCmd, now: u32) -> PushResult {
        let Some(s) = self.table.get_mut(&player) else {
            self.count(Reject::NoSession);
            return PushResult::Invalid(Reject::NoSession);
        };
        if cmd.buttons & frontline_protocol::msg::input::buttons::FIRE != 0
            && frontline_sim::combat::lagcomp::rewind_ticks(cmd.view_tick).1
        {
            self.metrics.lagcomp_clamped_total.inc();
        }
        let out = s.input.ingest(cmd, now);
        let accepted = out.result.is_accepted();
        if accepted {
            self.metrics.input_accepted_total.inc();
        }
        let rejection = out.result.reject();
        self.anticheat.input_outcome(
            player,
            accepted,
            rejection.is_some_and(Reject::is_violation),
        );
        if let Some(r) = rejection {
            self.count(r);
            self.suspect(player, r, now);
        }
        for r in out.clamps.reasons() {
            self.count(r);
            self.suspect(player, r, now);
        }
        out.result
    }

    /// A whole C1 bundle: its commands in order, then ack / prediction hash / fire commands.
    pub fn ingest_bundle(&mut self, player: PlayerId, b: &InputBundle, now: u32) {
        if !self.table.contains_key(&player) {
            self.count(Reject::NoSession);
            return;
        }
        for c in &b.cmds {
            self.ingest_cmd(player, c, now);
        }
        let Some(s) = self.table.get_mut(&player) else {
            return;
        };
        let refused = s.input.note_bundle(b);
        for r in refused {
            self.count(r);
            self.suspect(player, r, now);
        }
    }

    /// Drains the network inbound queue (step 1 of the tick).
    pub fn drain_net(&mut self, now: u32) {
        // Bundles that arrived a tick before their player's Join was processed.
        for i in std::mem::take(&mut self.parked) {
            self.ingest_bundle(i.player, &i.bundle, now);
        }
        let Some(link) = &self.link else {
            return;
        };
        let rx = link.inbound.clone();
        for _ in 0..MAX_BUNDLES_PER_TICK {
            match rx.try_recv() {
                Ok(i) if !self.table.contains_key(&i.player) && self.parked.len() < 256 => {
                    self.parked.push(i)
                }
                Ok(i) => self.ingest_bundle(i.player, &i.bundle, now),
                Err(_) => break,
            }
        }
    }

    /// Finalizes the input tick, then removes offenders and queues an ordered C0 disconnect.
    pub fn process_kicks(&mut self, world: &mut World) -> Vec<PlayerId> {
        let now = world.tick().0.wrapping_add(1);
        self.anticheat.finish_input_tick(now, |p, reason| {
            self.kicks.entry(p).or_insert(reason);
        });
        let kicked = std::mem::take(&mut self.kicks);
        for (&p, &reason) in &kicked {
            self.metrics.session_kicked_total.inc();
            tracing::warn!(event = "session_kicked", player_id = p, tick = now, ?reason);
            if let Some(h) = self
                .link
                .as_ref()
                .and_then(|link| link.sessions.handle_of(p))
            {
                crate::net::conn::disconnect_for_input(&h, reason);
            }
            self.leave(world, p);
        }
        kicked.into_keys().collect()
    }

    /// Step 2: one command per joined player for `tick`, ascending by `PlayerId`.
    pub fn resolve(&mut self, tick: u32) -> &[(PlayerId, InputCmd)] {
        self.resolved.clear();
        for (&p, s) in self.table.iter_mut() {
            let r = s.input.resolve(tick);
            if r.kind != Resolution::Real {
                self.metrics.input_missing_ticks_total.inc();
            }
            if r.new_stall {
                self.metrics.session_stall_total.inc();
                tracing::warn!(
                    event = "session_stall",
                    player_id = p as u64,
                    gap = s.input.state.gap as u64,
                    "no input for 9 ticks; holding the player still"
                );
            }
            self.resolved.push((p, r.cmd));
        }
        &self.resolved
    }
}
