use super::*;
use crate::rng::{mix64, Purpose, Rng, RngCore};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Counts {
    stage: VoteStage,
    yes: u16,
    no: u16,
    required_yes: u16,
    maps: [u16; 3],
}

#[derive(Debug, Clone, Hash)]
struct ActiveVote {
    kind: VoteKind,
    team: Option<u8>,
    deadline: Tick,
    open: bool,
    invalidated: bool,
    commander_since: Option<Tick>,
    ballots: BTreeMap<PlayerId, Ballot>,
    counts: Option<Counts>,
}

impl ActiveVote {
    fn new(
        kind: VoteKind,
        team: Option<Team>,
        deadline: Tick,
        commander_since: Option<Tick>,
    ) -> Self {
        Self {
            kind,
            team: team.map(|t| t as u8),
            deadline,
            open: commander_since.is_none(),
            invalidated: false,
            commander_since,
            ballots: BTreeMap::new(),
            counts: None,
        }
    }

    fn eligible(&self, p: &PlayerFacts) -> bool {
        !p.is_bot && self.team.is_none_or(|side| side == p.team as u8)
    }

    fn kick_or_mute(&self) -> bool {
        matches!(self.kind, VoteKind::Kick(_) | VoteKind::Mute(_))
    }

    fn target(&self) -> Option<PlayerId> {
        match self.kind {
            VoteKind::Kick(id) | VoteKind::Mute(id) | VoteKind::RemoveCommander(id) => Some(id),
            VoteKind::Map(_) => None,
        }
    }

    fn valid_target(&self, ctx: &RuleContext<'_>) -> bool {
        !self.invalidated
            && self.target().is_none_or(|id| {
                ctx.player(id).is_some_and(|p| {
                    self.eligible(p)
                        && (!matches!(self.kind, VoteKind::Kick(_)) || !p.is_admin)
                        && (self.commander_since.is_none()
                            || self.commander_since == p.commander_since)
                })
            })
    }

    fn prune(&mut self, ctx: &RuleContext<'_>) {
        let team = self.team;
        let open = self.open;
        self.ballots.retain(|&id, _| {
            ctx.player(id).is_some_and(|p| {
                !p.is_bot && team.is_none_or(|side| side == p.team as u8) && (open || p.is_officer)
            })
        });
    }

    fn counts(&self, cfg: &VoteConfig, roster_counts: [u32; 2]) -> Counts {
        let team_size = self.team.map_or(0, |team| roster_counts[team as usize]);
        let percent = |percent: u8| (team_size * u32::from(percent)).div_ceil(100) as u16;
        let mut counts = Counts {
            stage: if self.open {
                VoteStage::Open
            } else {
                VoteStage::AwaitingSponsors {
                    have: self.ballots.len() as u8,
                    need: cfg.commander_min_officers.max(1),
                }
            },
            yes: 0,
            no: 0,
            required_yes: match self.kind {
                VoteKind::Kick(_) => percent(cfg.kick_yes_percent).max(u16::from(cfg.kick_min_yes)),
                VoteKind::Mute(_) => u16::from(cfg.mute_min_yes),
                VoteKind::RemoveCommander(_) => percent(cfg.commander_yes_percent),
                VoteKind::Map(_) => 0,
            },
            maps: [0; 3],
        };
        for ballot in self.ballots.values() {
            match *ballot {
                Ballot::Yes => counts.yes += 1,
                Ballot::No => counts.no += 1,
                Ballot::Map(map) => {
                    if let VoteKind::Map(maps) = self.kind {
                        if let Some(i) = maps.iter().position(|&id| id == map) {
                            counts.maps[i] += 1;
                        }
                    }
                }
            }
        }
        counts
    }

    fn snapshot(&self, id: VoteId, counts: Counts) -> VoteSnapshot {
        VoteSnapshot {
            id,
            kind: self.kind,
            team: self.team.map(|t| team_from_index(t as usize)),
            stage: counts.stage,
            closes_at: self.deadline,
            yes: counts.yes,
            no: counts.no,
            required_yes: counts.required_yes,
            map_votes: counts.maps,
        }
    }

    fn emit_snapshot(&mut self, id: VoteId, counts: Counts, actions: &mut Vec<ServerAction>) {
        self.counts = Some(counts);
        actions.push(ServerAction::VoteSnapshot(self.snapshot(id, counts)));
    }

    fn sponsor(
        &mut self,
        cfg: &VoteConfig,
        ctx: &RuleContext<'_>,
        id: PlayerId,
    ) -> Result<(), RuleError> {
        self.prune(ctx);
        if self.ballots.contains_key(&id) {
            return Err(RuleError::AlreadyVoted);
        }
        self.ballots.insert(id, Ballot::Yes);
        if self.ballots.len() >= usize::from(cfg.commander_min_officers.max(1)) {
            self.open = true;
            self.deadline = ctx.after(cfg.commander_duration_s);
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Hash)]
pub(super) struct Votes {
    next_id: VoteId,
    active: BTreeMap<VoteId, ActiveVote>,
    kick_next: BTreeMap<PlayerId, Tick>,
    map_started: bool,
}

impl Votes {
    pub fn reset_round(&mut self) {
        self.active.clear();
        self.kick_next.clear();
        self.map_started = false;
    }

    pub fn round_ended(&self) -> bool {
        self.map_started
    }

    pub fn remove_player(&mut self, id: PlayerId) {
        self.kick_next.remove(&id);
        for vote in self.active.values_mut() {
            vote.ballots.remove(&id);
            if vote.target() == Some(id) {
                vote.invalidated = true;
            }
        }
    }

    fn add(
        &mut self,
        cfg: &VoteConfig,
        ctx: &RuleContext<'_>,
        mut vote: ActiveVote,
        actions: &mut Vec<ServerAction>,
    ) {
        self.next_id = self.next_id.wrapping_add(1);
        vote.emit_snapshot(self.next_id, vote.counts(cfg, human_counts(ctx)), actions);
        self.active.insert(self.next_id, vote);
    }

    pub fn start(
        &mut self,
        cfg: &VoteConfig,
        ctx: &RuleContext<'_>,
        p: &PlayerFacts,
        request: VoteRequest,
        actions: &mut Vec<ServerAction>,
    ) -> Result<(), RuleError> {
        let target = match request {
            VoteRequest::Kick(target)
            | VoteRequest::Mute(target)
            | VoteRequest::RemoveCommander(target) => target,
            VoteRequest::Surrender => return Err(RuleError::Disabled),
        };
        let q = ctx
            .player(target)
            .filter(|q| q.id != p.id && !q.is_bot)
            .ok_or(RuleError::InvalidTarget)?;
        if p.is_admin && matches!(request, VoteRequest::Mute(_)) {
            actions.push(ServerAction::Mute { player: target });
            return Ok(());
        }
        if self.map_started {
            return Err(RuleError::RoundEnded);
        }
        if p.team != q.team {
            return Err(RuleError::InvalidTarget);
        }
        let (kind, duration) = match request {
            VoteRequest::Kick(_) => {
                if q.is_admin {
                    return Err(RuleError::ProtectedTarget);
                }
                if elapsed(ctx.now, ctx.match_started) < ctx.ticks(cfg.kick_min_match_age_s) {
                    return Err(RuleError::VoteTooSoon);
                }
                if self
                    .kick_next
                    .get(&p.id)
                    .is_some_and(|&next| ctx.now.lt(next))
                {
                    return Err(RuleError::VoteCooldown);
                }
                (VoteKind::Kick(target), cfg.kick_duration_s)
            }
            VoteRequest::Mute(_) => (VoteKind::Mute(target), cfg.mute_duration_s),
            VoteRequest::RemoveCommander(_) => {
                if !p.is_officer {
                    return Err(RuleError::OfficerOnly);
                }
                let since = q.commander_since.ok_or(RuleError::InvalidTarget)?;
                if elapsed(ctx.now, since) < ctx.ticks(cfg.commander_min_tenure_s) {
                    return Err(RuleError::CommanderTenure);
                }
                if let Some((&id, vote)) = self.active.iter_mut().find(|(_, v)| {
                    matches!(v.kind, VoteKind::RemoveCommander(_)) && v.team == Some(p.team as u8)
                }) {
                    if vote.kind != VoteKind::RemoveCommander(target)
                        || vote.open
                        || !ctx.now.lt(vote.deadline)
                        || !vote.valid_target(ctx)
                    {
                        return Err(RuleError::VoteBusy);
                    }
                    vote.sponsor(cfg, ctx, p.id)?;
                    vote.emit_snapshot(id, vote.counts(cfg, human_counts(ctx)), actions);
                } else {
                    let mut vote = ActiveVote::new(
                        VoteKind::RemoveCommander(target),
                        Some(p.team),
                        ctx.after(cfg.commander_duration_s),
                        Some(since),
                    );
                    vote.sponsor(cfg, ctx, p.id)?;
                    self.add(cfg, ctx, vote, actions);
                }
                return Ok(());
            }
            VoteRequest::Surrender => unreachable!(),
        };
        if self.active.values().any(ActiveVote::kick_or_mute) {
            return Err(RuleError::VoteBusy);
        }
        if matches!(kind, VoteKind::Kick(_)) {
            self.kick_next
                .insert(p.id, ctx.after(cfg.kick_start_cooldown_s));
        }
        self.add(
            cfg,
            ctx,
            ActiveVote::new(kind, Some(p.team), ctx.after(duration), None),
            actions,
        );
        Ok(())
    }

    pub fn cast(
        &mut self,
        cfg: &VoteConfig,
        ctx: &RuleContext<'_>,
        p: &PlayerFacts,
        id: VoteId,
        ballot: Ballot,
        actions: &mut Vec<ServerAction>,
    ) -> Result<(), RuleError> {
        let vote = self.active.get_mut(&id).ok_or(RuleError::VoteNotFound)?;
        if !vote.open || !ctx.now.lt(vote.deadline) || !vote.valid_target(ctx) {
            return Err(RuleError::VoteNotOpen);
        }
        if !vote.eligible(p) {
            return Err(RuleError::Ineligible);
        }
        match (vote.kind, ballot) {
            (VoteKind::Map(maps), Ballot::Map(map)) if maps.contains(&map) => (),
            (VoteKind::Map(_), _) | (_, Ballot::Map(_)) => return Err(RuleError::InvalidBallot),
            _ => (),
        }
        vote.prune(ctx);
        match vote.ballots.entry(p.id) {
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(ballot);
            }
            std::collections::btree_map::Entry::Occupied(_) => return Err(RuleError::AlreadyVoted),
        }
        vote.emit_snapshot(id, vote.counts(cfg, human_counts(ctx)), actions);
        Ok(())
    }

    pub fn start_map(
        &mut self,
        cfg: &VoteConfig,
        ctx: &RuleContext<'_>,
        maps: [u16; 3],
        actions: &mut Vec<ServerAction>,
    ) -> Result<(), RuleError> {
        if self.map_started {
            return Err(RuleError::RoundEnded);
        }
        if maps[0] == maps[1] || maps[0] == maps[2] || maps[1] == maps[2] {
            return Err(RuleError::InvalidMaps);
        }
        for (id, mut vote) in std::mem::take(&mut self.active) {
            vote.prune(ctx);
            actions.push(ServerAction::VoteResult {
                snapshot: vote.snapshot(id, vote.counts(cfg, human_counts(ctx))),
                outcome: VoteOutcome::Cancelled,
            });
        }
        self.map_started = true;
        self.add(
            cfg,
            ctx,
            ActiveVote::new(
                VoteKind::Map(maps),
                None,
                ctx.after(cfg.map_duration_s),
                None,
            ),
            actions,
        );
        Ok(())
    }

    pub fn step(
        &mut self,
        cfg: &VoteConfig,
        ctx: &RuleContext<'_>,
        actions: &mut Vec<ServerAction>,
    ) {
        self.kick_next
            .retain(|&id, next| ctx.now.lt(*next) && ctx.human(id));
        if self.active.is_empty() {
            return;
        }
        let roster_counts = human_counts(ctx);
        self.active.retain(|&id, vote| {
            vote.prune(ctx);
            let counts = vote.counts(cfg, roster_counts);
            let outcome = if !vote.valid_target(ctx) {
                VoteOutcome::Cancelled
            } else if ctx.now.lt(vote.deadline) {
                if vote.counts != Some(counts) {
                    vote.emit_snapshot(id, counts, actions);
                }
                return true;
            } else if let VoteKind::Map(maps) = vote.kind {
                VoteOutcome::MapSelected(select_map(ctx, vote.deadline, maps, counts.maps))
            } else if vote.open
                && counts.yes >= counts.required_yes
                && (!matches!(vote.kind, VoteKind::Kick(_))
                    || u32::from(counts.yes)
                        > u32::from(counts.no) * u32::from(cfg.kick_no_multiplier))
            {
                VoteOutcome::Passed
            } else {
                VoteOutcome::Failed
            };
            actions.push(ServerAction::VoteResult {
                snapshot: vote.snapshot(id, counts),
                outcome,
            });
            if outcome == VoteOutcome::Passed {
                actions.push(match vote.kind {
                    VoteKind::Kick(player) => ServerAction::Kick {
                        player,
                        reason: KickReason::Vote,
                        cooldown_until: None,
                    },
                    VoteKind::Mute(player) => ServerAction::Mute { player },
                    VoteKind::RemoveCommander(player) => {
                        ServerAction::RelinquishCommander { player }
                    }
                    VoteKind::Map(_) => unreachable!(),
                });
            }
            false
        });
    }

    pub(super) fn hash_into(&self, w: &mut HashWriter) {
        w.write_u32(self.next_id);
        w.write_u8(u8::from(self.map_started));
        w.write_u32(self.kick_next.len() as u32);
        for (&id, tick) in &self.kick_next {
            w.write_u8(id);
            w.write_u32(tick.0);
        }
        w.write_u32(self.active.len() as u32);
        for (&id, vote) in &self.active {
            w.write_u32(id);
            match vote.kind {
                VoteKind::Kick(target) => {
                    w.write_u8(0);
                    w.write_u8(target);
                }
                VoteKind::Mute(target) => {
                    w.write_u8(1);
                    w.write_u8(target);
                }
                VoteKind::RemoveCommander(target) => {
                    w.write_u8(2);
                    w.write_u8(target);
                }
                VoteKind::Map(maps) => {
                    w.write_u8(3);
                    for map in maps {
                        w.write_u16(map);
                    }
                }
            }
            write_option(w, vote.team, HashWriter::write_u8);
            w.write_u32(vote.deadline.0);
            w.write_u8(u8::from(vote.open));
            w.write_u8(u8::from(vote.invalidated));
            write_option(w, vote.commander_since, |w, tick| w.write_u32(tick.0));
            w.write_u32(vote.ballots.len() as u32);
            for (&player, &ballot) in &vote.ballots {
                w.write_u8(player);
                match ballot {
                    Ballot::Yes => w.write_u8(0),
                    Ballot::No => w.write_u8(1),
                    Ballot::Map(map) => {
                        w.write_u8(2);
                        w.write_u16(map);
                    }
                }
            }
            write_option(w, vote.counts, |w, c| {
                match c.stage {
                    VoteStage::Open => w.write_u8(0),
                    VoteStage::AwaitingSponsors { have, need } => {
                        w.write_u8(1);
                        w.write_u8(have);
                        w.write_u8(need);
                    }
                }
                w.write_u16(c.yes);
                w.write_u16(c.no);
                w.write_u16(c.required_yes);
                for count in c.maps {
                    w.write_u16(count);
                }
            });
        }
    }
}

fn select_map(ctx: &RuleContext<'_>, tick: Tick, maps: [u16; 3], counts: [u16; 3]) -> u16 {
    let max = counts.into_iter().max().unwrap();
    let mut tied = [0; 3];
    let mut n = 0;
    for (map, count) in maps.into_iter().zip(counts) {
        if count == max {
            tied[n] = map;
            n += 1;
        }
    }
    if n == 1 {
        return tied[0];
    }
    // Domain-separate vote_tie without adding a discriminant to the shared RNG enum.
    let mut rng = Rng::for_(
        mix64(ctx.match_seed ^ u64::from_le_bytes(*b"vote_tie")),
        tick,
        0,
        Purpose::Misc,
    );
    let n = n as u32;
    loop {
        let draw = rng.next_u32();
        if draw >= n.wrapping_neg() % n {
            return tied[(draw % n) as usize];
        }
    }
}
