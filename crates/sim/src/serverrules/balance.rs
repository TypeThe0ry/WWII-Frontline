use super::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Hash)]
struct Offer {
    from: u8,
    expires_at: Tick,
}

#[derive(Debug, Clone, Default, Hash)]
pub(super) struct Balance {
    // Track continuity every tick; never infer that an imbalance held between two polls.
    heavy: Option<(u8, Tick)>,
    offer: Option<Offer>,
    retry_at: Option<Tick>,
    forced: BTreeSet<PlayerId>,
    recent: BTreeMap<PlayerId, Tick>,
}

impl Balance {
    pub fn remove_player(&mut self, id: PlayerId) {
        self.forced.remove(&id);
        self.recent.remove(&id);
    }

    pub fn has_offer(&self, team: Team, now: Tick) -> bool {
        self.offer
            .is_some_and(|offer| offer.from == team as u8 && now.lt(offer.expires_at))
    }

    pub fn volunteer(
        &self,
        cfg: &BalanceConfig,
        ctx: &RuleContext<'_>,
        p: &PlayerFacts,
    ) -> Result<MoveKind, RuleError> {
        let count = human_counts(ctx);
        let from = p.team as usize;
        if !self.has_offer(p.team, ctx.now)
            || count[from].saturating_sub(count[1 - from])
                < u32::from(cfg.trigger_difference.max(2))
        {
            return Err(RuleError::NoBalanceOffer);
        }
        Ok(MoveKind::Volunteer {
            xp_bonus_percent: cfg.volunteer_xp_bonus_percent,
        })
    }

    pub fn accepted(&mut self, ctx: &RuleContext<'_>, id: PlayerId, recent_move_s: u16) {
        self.offer = None;
        self.heavy = None;
        self.retry_at = None;
        self.recent.insert(id, ctx.after(recent_move_s));
    }

    pub fn step(
        &mut self,
        cfg: &BalanceConfig,
        ctx: &RuleContext<'_>,
        actions: &mut Vec<ServerAction>,
    ) {
        self.forced.retain(|&id| ctx.human(id));
        self.recent
            .retain(|&id, until| ctx.now.lt(*until) && ctx.human(id));
        let counts = human_counts(ctx);
        let from = u8::from(counts[1] > counts[0]);
        let to = 1 - from;
        if counts[from as usize] - counts[to as usize] < u32::from(cfg.trigger_difference.max(2)) {
            self.heavy = None;
            self.offer = None;
            self.retry_at = None;
            return;
        }
        if self.heavy.is_none_or(|(side, _)| side != from) {
            self.heavy = Some((from, ctx.now));
            self.offer = None;
            self.retry_at = None;
        }
        let to_team = team_from_index(to as usize);
        if let Some(offer) = self.offer {
            if ctx.now.lt(offer.expires_at) || self.retry_at.is_some_and(|at| ctx.now.lt(at)) {
                return;
            }
            let from_team = team_from_index(from as usize);
            let candidate = (counts[to as usize] < u32::from(cfg.team_max_players))
                .then(|| self.candidate(cfg, ctx, from_team, actions))
                .flatten();
            if let Some(player) = candidate {
                self.forced.insert(player);
                self.recent.insert(player, ctx.after(cfg.recent_move_s));
                self.offer = None;
                // Same tick is never allowed to emit a second move, even with zero tunables.
                self.heavy = Some((from, ctx.now));
                self.retry_at = Some(ctx.after(cfg.check_period_s.max(1)));
                actions.push(teamswitch::move_action(
                    ctx,
                    player,
                    to_team,
                    MoveKind::Forced,
                ));
            } else {
                self.retry_at = Some(ctx.after(cfg.check_period_s.max(1)));
            }
        } else if self.heavy.is_some_and(|(_, since)| {
            let period = ctx.ticks(cfg.check_period_s.max(1));
            let first_check = ctx.ticks(cfg.persist_s).div_ceil(period) * period;
            elapsed(ctx.now, since) >= first_check
        }) && self.retry_at.is_none_or(|at| !ctx.now.lt(at))
            && counts[to as usize] < u32::from(cfg.team_max_players)
        {
            let expires_at = ctx.after(cfg.offer_s.max(1));
            self.offer = Some(Offer { from, expires_at });
            actions.push(ServerAction::InviteSwitch {
                from: team_from_index(from as usize),
                to: to_team,
                expires_at,
                xp_bonus_percent: cfg.volunteer_xp_bonus_percent,
            });
        }
    }

    fn candidate(
        &self,
        cfg: &BalanceConfig,
        ctx: &RuleContext<'_>,
        from: Team,
        actions: &[ServerAction],
    ) -> Option<PlayerId> {
        ctx.roster
            .iter()
            .filter(|p| {
                p.team == from
                    && !p.is_bot
                    && !p.is_squad_leader
                    && p.commander_since.is_none()
                    && !self.forced.contains(&p.id)
                    && !self.recent.contains_key(&p.id)
                    && p.last_switch_tick
                        .is_none_or(|t| elapsed(ctx.now, t) >= ctx.ticks(cfg.recent_move_s))
                    && !shared_party(ctx, p)
                    && !actions
                        .iter()
                        .any(|a| matches!(a, ServerAction::Kick { player, .. } if *player == p.id))
            })
            .min_by_key(|p| (elapsed(ctx.now, p.joined_tick), p.squad_size, p.score, p.id))
            .map(|p| p.id)
    }

    pub(super) fn hash_into(&self, w: &mut HashWriter) {
        write_option(w, self.heavy, |w, (side, since)| {
            w.write_u8(side);
            w.write_u32(since.0);
        });
        write_option(w, self.offer, |w, offer| {
            w.write_u8(offer.from);
            w.write_u32(offer.expires_at.0);
        });
        write_option(w, self.retry_at, |w, tick| w.write_u32(tick.0));
        w.write_u32(self.forced.len() as u32);
        for &id in &self.forced {
            w.write_u8(id);
        }
        w.write_u32(self.recent.len() as u32);
        for (&id, &tick) in &self.recent {
            w.write_u8(id);
            w.write_u32(tick.0);
        }
    }
}
