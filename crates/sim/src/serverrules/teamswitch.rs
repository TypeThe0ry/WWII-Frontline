use super::*;
use crate::team::balance as team_balance;

pub(super) fn can_join(
    cfg: &BalanceConfig,
    ctx: &RuleContext<'_>,
    team: Team,
    size: u8,
) -> Result<(), RuleError> {
    if size == 0 || size > cfg.party_max_players {
        return Err(RuleError::PartyTooLarge);
    }
    let humans = human_counts(ctx);
    let own = humans[team as usize] + u32::from(size - 1);
    team_balance::can_join(
        own,
        humans[other(team) as usize],
        u32::from(cfg.team_max_players),
        u32::from(cfg.join_max_difference),
    )?;
    Ok(())
}

pub(super) fn can_switch(
    cfg: &BalanceConfig,
    ctx: &RuleContext<'_>,
    id: PlayerId,
) -> Result<(), RuleError> {
    let p = ctx.player(id).ok_or(RuleError::UnknownPlayer)?;
    if p.is_bot {
        return Err(RuleError::HumanOnly);
    }
    if shared_party(ctx, p) {
        return Err(RuleError::PartyWouldSplit);
    }
    let humans = human_counts(ctx);
    team_balance::can_switch(
        humans[p.team as usize],
        humans[other(p.team) as usize],
        u32::from(cfg.team_max_players),
        u32::from(cfg.join_max_difference),
        ctx.now,
        p.joined_tick,
        p.last_switch_tick,
        ctx.ticks(cfg.switch_min_after_join_s),
        ctx.ticks(cfg.switch_cooldown_s),
    )?;
    Ok(())
}

fn bot_priority(p: &PlayerFacts) -> (bool, bool, PlayerId) {
    (p.in_vehicle, p.alive, p.id)
}

pub(super) fn bot_replacements(ctx: &RuleContext<'_>, team: Team) -> Vec<PlayerId> {
    let mut bots: Vec<_> = ctx
        .roster
        .iter()
        .filter(|p| p.is_bot && p.team == team)
        .map(bot_priority)
        .collect();
    bots.sort_unstable();
    bots.into_iter().map(|(_, _, id)| id).collect()
}

pub(super) fn move_action(
    ctx: &RuleContext<'_>,
    player: PlayerId,
    to: Team,
    kind: MoveKind,
) -> ServerAction {
    let replace_bot = ctx
        .roster
        .iter()
        .filter(|p| p.is_bot && p.team == to)
        .map(bot_priority)
        .min()
        .map(|(_, _, id)| id);
    ServerAction::Move {
        player,
        to,
        kind,
        replace_bot,
    }
}

/// A group kept intact at round rollover. The caller supplies retained squads, merges any
/// overlapping parties into the same group, and supplies singleton groups for unassigned humans.
#[derive(Debug, Clone, Copy)]
pub struct RoundGroup<'a> {
    pub members: &'a [PlayerId],
    pub score: i32,
}

pub(super) fn rebalance_next_round(
    cfg: &BalanceConfig,
    groups: &[RoundGroup<'_>],
) -> Result<Vec<(PlayerId, Team)>, RuleError> {
    let cap = usize::from(cfg.team_max_players);
    let mut seen = BTreeSet::new();
    let mut ranked = Vec::new();
    for group in groups {
        let first = *group.members.iter().min().ok_or(RuleError::InvalidTarget)?;
        if group.members.iter().any(|id| !seen.insert(*id)) {
            return Err(RuleError::InvalidTarget);
        }
        if group.members.len() > cap || seen.len() > 2 * cap {
            return Err(RuleError::TeamFull);
        }
        ranked.push((std::cmp::Reverse(group.score), first, group.members));
    }
    ranked.sort_unstable();

    // Compute possible suffix population sums. This permits the snake preference without
    // getting stuck later on indivisible groups, and finds the best attainable headcount split.
    let total = seen.len();
    let mut suffix = vec![vec![false; total + 1]; ranked.len() + 1];
    suffix[ranked.len()][0] = true;
    for i in (0..ranked.len()).rev() {
        let size = ranked[i].2.len();
        for n in 0..=total {
            suffix[i][n] = suffix[i + 1][n] || (n >= size && suffix[i + 1][n - size]);
        }
    }
    let best_diff = (total.saturating_sub(cap)..=cap.min(total))
        .filter(|&n| suffix[0][n])
        .map(|n| n.abs_diff(total - n))
        .min()
        .ok_or(RuleError::TeamFull)?;
    let mut us = 0;
    let mut result = Vec::with_capacity(total);
    for (i, (_, _, players)) in ranked.into_iter().enumerate() {
        let fits = |us: usize| {
            (total.saturating_sub(cap)..=cap.min(total))
                .any(|n| n >= us && n.abs_diff(total - n) == best_diff && suffix[i + 1][n - us])
        };
        let take_us = if matches!(i % 4, 0 | 3) {
            fits(us + players.len())
        } else {
            !fits(us)
        };
        let team = if take_us {
            us += players.len();
            Team::Us
        } else {
            Team::Ger
        };
        result.extend(players.iter().map(|id| (*id, team)));
    }
    result.sort_unstable_by_key(|p| p.0);
    Ok(result)
}
