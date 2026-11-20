use super::*;
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Copy, Hash)]
struct Prompt {
    offender: PlayerId,
    victim: PlayerId,
    expires_at: Tick,
}

#[derive(Debug, Clone, Default, Hash)]
pub(super) struct History {
    incidents: VecDeque<Tick>,
    strikes: VecDeque<Tick>,
    match_strikes: u16,
    high_risk_until: Option<Tick>,
    rifleman_until: Option<Tick>,
}

impl History {
    pub(super) fn hash_into(&self, w: &mut HashWriter) {
        for queue in [&self.incidents, &self.strikes] {
            w.write_u32(queue.len() as u32);
            for tick in queue {
                w.write_u32(tick.0);
            }
        }
        w.write_u16(self.match_strikes);
        write_option(w, self.high_risk_until, |w, t| w.write_u32(t.0));
        write_option(w, self.rifleman_until, |w, t| w.write_u32(t.0));
    }
}

#[derive(Debug, Clone, Default, Hash)]
pub(super) struct TeamKills {
    pending: BTreeMap<TkEventId, Prompt>,
    players: BTreeMap<PlayerId, History>,
}

impl TeamKills {
    pub(super) fn take_player(&mut self, id: PlayerId) -> Option<History> {
        self.pending
            .retain(|_, p| p.offender != id && p.victim != id);
        self.players.remove(&id)
    }

    pub(super) fn restore_player(&mut self, id: PlayerId, history: History) {
        self.players.insert(id, history);
    }

    pub fn count(&self, cfg: &TkConfig, ctx: &RuleContext<'_>, id: PlayerId) -> usize {
        self.players.get(&id).map_or(0, |history| {
            history
                .incidents
                .iter()
                .rev()
                .take_while(|&&tick| elapsed(ctx.now, tick) < ctx.ticks(cfg.history_s))
                .count()
        })
    }

    pub fn report(
        &mut self,
        cfg: &TkConfig,
        ctx: &RuleContext<'_>,
        event: TkEvent,
        actions: &mut Vec<ServerAction>,
    ) {
        if event.offender == event.victim
            || !ctx.human(event.offender)
            || ctx.player(event.victim).is_none()
        {
            return;
        }
        if event.kind == TkKind::CommanderFire {
            actions.push(ServerAction::Notice {
                player: event.offender,
                notice: Notice::CommanderFriendlyFire,
            });
            return;
        }
        if event.kind != TkKind::DeathAfterDown {
            let history = self.players.entry(event.offender).or_default();
            trim(ctx.now, &mut history.incidents, ctx.ticks(cfg.history_s));
            history.incidents.push_back(ctx.now);
        }
        if event.kind != TkKind::Downed && ctx.human(event.victim) {
            let expires_at = ctx.after(cfg.prompt_s);
            self.pending.insert(
                event.event_id,
                Prompt {
                    offender: event.offender,
                    victim: event.victim,
                    expires_at,
                },
            );
            actions.push(ServerAction::TkPrompt { event, expires_at });
        }
    }

    pub fn respond(
        &mut self,
        cfg: &TkConfig,
        ctx: &RuleContext<'_>,
        victim: PlayerId,
        event_id: TkEventId,
        decision: TkDecision,
        actions: &mut Vec<ServerAction>,
    ) -> Result<(), RuleError> {
        let p = self
            .pending
            .get(&event_id)
            .copied()
            .filter(|p| p.victim == victim && ctx.human(p.offender))
            .ok_or(RuleError::TkPromptNotFound)?;
        if !ctx.now.lt(p.expires_at) {
            return Err(RuleError::TkPromptExpired);
        }
        self.pending.remove(&event_id);
        if decision == TkDecision::Forgive {
            return Ok(());
        }
        let history = self.players.entry(p.offender).or_default();
        trim(
            ctx.now,
            &mut history.strikes,
            ctx.ticks(cfg.strike_window_s),
        );
        history.strikes.push_back(ctx.now);
        history.match_strikes = history.match_strikes.saturating_add(1);
        let player = p.offender;
        let until = ctx.after(cfg.lock_s);
        let strikes = history.strikes.len();
        if history.match_strikes == u16::from(cfg.kick_match_strikes) {
            actions.push(ServerAction::Kick {
                player,
                reason: KickReason::TeamKill,
                cooldown_until: Some(ctx.after(cfg.kick_cooldown_s)),
            });
        } else if strikes == usize::from(cfg.rifleman_strikes) {
            history.rifleman_until = Some(until);
            actions.push(ServerAction::LockRole {
                player,
                lock: RoleLock::RiflemanOnly,
                until,
            });
            actions.push(ServerAction::LockVehicle { player, until });
        } else if strikes == usize::from(cfg.warning_strikes) {
            history.high_risk_until = Some(until);
            actions.push(ServerAction::Notice {
                player,
                notice: Notice::TkWarning,
            });
            if !history.rifleman_until.is_some_and(|end| ctx.now.lt(end)) {
                actions.push(ServerAction::LockRole {
                    player,
                    lock: RoleLock::HighRisk,
                    until,
                });
            }
        }
        Ok(())
    }

    pub fn role_lock(&self, now: Tick, id: PlayerId) -> Option<RoleLock> {
        let h = self.players.get(&id)?;
        if h.rifleman_until.is_some_and(|until| now.lt(until)) {
            Some(RoleLock::RiflemanOnly)
        } else if h.high_risk_until.is_some_and(|until| now.lt(until)) {
            Some(RoleLock::HighRisk)
        } else {
            None
        }
    }

    pub fn step(&mut self, cfg: &TkConfig, ctx: &RuleContext<'_>) {
        self.pending.retain(|_, p| {
            ctx.now.lt(p.expires_at) && ctx.human(p.offender) && ctx.human(p.victim)
        });
        self.players.retain(|&id, h| {
            trim(ctx.now, &mut h.incidents, ctx.ticks(cfg.history_s));
            trim(ctx.now, &mut h.strikes, ctx.ticks(cfg.strike_window_s));
            if h.high_risk_until.is_some_and(|t| !ctx.now.lt(t)) {
                h.high_risk_until = None;
            }
            if h.rifleman_until.is_some_and(|t| !ctx.now.lt(t)) {
                h.rifleman_until = None;
            }
            ctx.human(id) && (!h.incidents.is_empty() || h.match_strikes != 0)
        });
    }

    pub(super) fn hash_into(&self, w: &mut HashWriter) {
        w.write_u32(self.pending.len() as u32);
        for (&id, p) in &self.pending {
            w.write_u32(id);
            w.write_u8(p.offender);
            w.write_u8(p.victim);
            w.write_u32(p.expires_at.0);
        }
        w.write_u32(self.players.len() as u32);
        for (&id, history) in &self.players {
            w.write_u8(id);
            history.hash_into(w);
        }
    }
}

fn trim(now: Tick, queue: &mut VecDeque<Tick>, window: u32) {
    while queue
        .front()
        .is_some_and(|&tick| elapsed(now, tick) >= window)
    {
        queue.pop_front();
    }
}
