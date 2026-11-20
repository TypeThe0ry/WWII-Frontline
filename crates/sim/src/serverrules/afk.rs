use super::*;
use std::collections::{BTreeMap, VecDeque};

const MAX_REPEAT_PERIOD: usize = 8;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
struct Motion {
    buttons: u16,
    x: i8,
    z: i8,
}

#[derive(Debug, Clone, Copy, Hash)]
struct Sample {
    motion: Motion,
    at: Tick,
    prior_activity: Tick,
}

#[derive(Debug, Clone, Copy, Hash)]
struct Input {
    motion: Motion,
    yaw: u16,
    pitch: i16,
    weapon_slot: u8,
}

#[derive(Debug, Clone, Hash)]
struct Idle {
    last_activity: Tick,
    last_input: Option<Input>,
    samples: VecDeque<Sample>,
    script_period: u8,
    script_notice: bool,
    warned: bool,
    /// Keep the original deadline even when the relinquish action changes the roster facts.
    countdown: Option<Tick>,
    vacant_vehicle: Option<EntityId>,
    kicked: bool,
    // Allows step to emit cancellation even when record_activity runs before step.
    recovered: bool,
}

impl Idle {
    fn new(ctx: &RuleContext<'_>, p: &PlayerFacts) -> Self {
        let last_activity = if p.joined_tick.lt(ctx.match_started) {
            ctx.match_started
        } else {
            p.joined_tick
        };
        Self {
            last_activity,
            last_input: None,
            samples: VecDeque::new(),
            script_period: 0,
            script_notice: false,
            warned: false,
            countdown: None,
            vacant_vehicle: None,
            kicked: false,
            recovered: false,
        }
    }

    fn active(&mut self, now: Tick) {
        self.last_activity = now;
        self.warned = false;
        self.recovered |= self.countdown.take().is_some();
        self.vacant_vehicle = None;
    }

    fn record(&mut self, cfg: &AfkConfig, ctx: &RuleContext<'_>, activity: Activity) {
        let Activity::Input(cmd) = activity else {
            self.samples.clear();
            self.script_period = 0;
            self.active(ctx.now);
            return;
        };
        let motion = Motion {
            buttons: cmd.buttons & 0x3fff,
            x: cmd.move_x,
            z: cmd.move_z,
        };
        let input = Input {
            motion,
            yaw: cmd.yaw,
            pitch: cmd.pitch,
            weapon_slot: cmd.weapon_slot,
        };
        let previous = self.last_input.replace(input);
        let old = previous.map_or(Motion::default(), |p| p.motion);
        if motion != old {
            if self.script_period != 0 {
                let period = usize::from(self.script_period);
                if self.samples[self.samples.len() - period].motion == motion {
                    let oldest = self.samples.pop_front().unwrap();
                    self.samples.push_back(Sample {
                        motion,
                        at: ctx.now,
                        prior_activity: oldest.prior_activity,
                    });
                    return;
                }
                self.script_period = 0;
                self.samples.clear();
            }
            // Runs of identical samples are compressed, so a held key is not a script.
            let window = ctx.ticks(cfg.repeat_window_s);
            let repeats = usize::from(cfg.repeat_occurrences.max(2));
            while self
                .samples
                .front()
                .is_some_and(|s| elapsed(ctx.now, s.at) >= window)
                || self.samples.len() >= repeats * MAX_REPEAT_PERIOD
            {
                self.samples.pop_front();
            }
            self.samples.push_back(Sample {
                motion,
                at: ctx.now,
                prior_activity: self.last_activity,
            });
            let n = self.samples.len();
            for period in 2..=MAX_REPEAT_PERIOD.min(n / repeats) {
                let start = n - period * repeats;
                // Cheap rejection before comparing all repetitions of the motif.
                if self.samples[n - 1 - period].motion == motion
                    && (start..n - period)
                        .rev()
                        .all(|i| self.samples[i].motion == self.samples[i + period].motion)
                {
                    self.last_activity = self.samples[start].prior_activity;
                    self.script_period = period as u8;
                    self.script_notice = true;
                    return;
                }
            }
        }
        if self.script_period == 0 {
            use frontline_protocol::msg::input::buttons;
            let moving = motion.x != 0 || motion.z != 0;
            let firing = motion.buttons & buttons::FIRE != 0;
            let pressed = motion.buttons & !old.buttons != 0;
            let changed_view = previous.is_some_and(|p| {
                p.yaw != input.yaw || p.pitch != input.pitch || p.weapon_slot != input.weapon_slot
            });
            if moving || firing || pressed || changed_view {
                self.active(ctx.now);
            }
        }
    }

    fn step(
        &mut self,
        cfg: &AfkConfig,
        ctx: &RuleContext<'_>,
        p: &PlayerFacts,
        actions: &mut Vec<ServerAction>,
    ) {
        let id = p.id;
        if self.kicked {
            return;
        }
        if self.script_notice {
            actions.push(ServerAction::Notice {
                player: id,
                notice: Notice::RepeatedInput,
            });
            self.script_notice = false;
        }
        let active_state = p.alive || p.in_vehicle;
        if !active_state && self.countdown.take().is_some() {
            self.recovered = true;
        }
        if std::mem::take(&mut self.recovered) {
            actions.push(ServerAction::Notice {
                player: id,
                notice: Notice::AfkCountdownCancelled,
            });
        }

        let idle = elapsed(ctx.now, self.last_activity);
        if !active_state {
            self.vacant_vehicle = None;
            if idle >= ctx.ticks(cfg.deploy_s) {
                self.kick(id, actions);
            }
            return;
        }
        if !self.warned && idle >= ctx.ticks(cfg.warning_s) {
            self.warned = true;
            actions.push(ServerAction::Notice {
                player: id,
                notice: Notice::AfkWarning,
            });
        }
        if self.countdown.is_none() {
            let leader = p.is_squad_leader || p.commander_since.is_some();
            let threshold = if leader { cfg.leader_s } else { cfg.alive_s };
            if idle < ctx.ticks(threshold) {
                return;
            }
            let kick_at = ctx.after(cfg.countdown_s);
            self.countdown = Some(kick_at);
            actions.push(ServerAction::Notice {
                player: id,
                notice: Notice::AfkCountdown { kick_at },
            });
            if p.is_squad_leader {
                actions.push(ServerAction::RelinquishLeader { player: id });
            }
            if p.commander_since.is_some()
                && !actions.contains(&ServerAction::RelinquishCommander { player: id })
            {
                actions.push(ServerAction::RelinquishCommander { player: id });
            }
        }
        if p.driving != self.vacant_vehicle {
            self.vacant_vehicle = p.driving;
            if let Some(vehicle) = p.driving {
                actions.push(ServerAction::DriverVacancy {
                    player: id,
                    vehicle,
                });
            }
        }
        if self.countdown.is_some_and(|at| !ctx.now.lt(at)) {
            self.kick(id, actions);
        }
    }

    fn kick(&mut self, id: PlayerId, actions: &mut Vec<ServerAction>) {
        self.kicked = true;
        if !actions
            .iter()
            .any(|a| matches!(a, ServerAction::Kick { player, .. } if *player == id))
        {
            actions.push(ServerAction::Kick {
                player: id,
                reason: KickReason::Afk,
                cooldown_until: None,
            });
        }
    }
}

#[derive(Debug, Default, Clone, Hash)]
pub(super) struct Afk {
    players: BTreeMap<PlayerId, Idle>,
}

impl Afk {
    pub fn remove_player(&mut self, id: PlayerId) {
        self.players.remove(&id);
    }

    pub fn is_afk(&self, id: PlayerId) -> bool {
        self.players
            .get(&id)
            .is_some_and(|p| p.countdown.is_some() || p.kicked || p.script_period != 0)
    }

    pub fn repeated(&self, id: PlayerId) -> bool {
        self.players.get(&id).is_some_and(|p| p.script_period != 0)
    }

    pub fn record(
        &mut self,
        cfg: &AfkConfig,
        ctx: &RuleContext<'_>,
        p: &PlayerFacts,
        activity: Activity,
    ) {
        self.players
            .entry(p.id)
            .or_insert_with(|| Idle::new(ctx, p))
            .record(cfg, ctx, activity);
    }

    pub fn step(
        &mut self,
        cfg: &AfkConfig,
        ctx: &RuleContext<'_>,
        actions: &mut Vec<ServerAction>,
    ) {
        self.players.retain(|&id, _| ctx.human(id));
        for p in ctx.roster.iter().filter(|p| !p.is_bot) {
            self.players
                .entry(p.id)
                .or_insert_with(|| Idle::new(ctx, p))
                .step(cfg, ctx, p, actions);
        }
    }

    pub(super) fn hash_into(&self, w: &mut HashWriter) {
        w.write_u32(self.players.len() as u32);
        for (&id, idle) in &self.players {
            w.write_u8(id);
            w.write_u32(idle.last_activity.0);
            write_option(w, idle.last_input, |w, input| {
                write_motion(w, input.motion);
                w.write_u16(input.yaw);
                w.write_i16(input.pitch);
                w.write_u8(input.weapon_slot);
            });
            w.write_u32(idle.samples.len() as u32);
            for sample in &idle.samples {
                write_motion(w, sample.motion);
                w.write_u32(sample.at.0);
                w.write_u32(sample.prior_activity.0);
            }
            w.write_u8(idle.script_period);
            w.write_u8(u8::from(idle.script_notice));
            w.write_u8(u8::from(idle.warned));
            write_option(w, idle.countdown, |w, t| w.write_u32(t.0));
            write_option(w, idle.vacant_vehicle, HashWriter::write_u16);
            w.write_u8(u8::from(idle.kicked));
            w.write_u8(u8::from(idle.recovered));
        }
    }
}

fn write_motion(w: &mut HashWriter, motion: Motion) {
    w.write_u16(motion.buttons);
    w.write_i8(motion.x);
    w.write_i8(motion.z);
}
