//! Deterministic server rule decisions (M2-018). The authoritative caller owns identity,
//! phase transitions, input validation, scoring and application of all returned actions.

mod afk;
mod balance;
mod config;
mod teamswitch;
mod tk;
mod vote;

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use crate::components::Team;
use crate::hash::HashWriter;
use crate::ids::{EntityId, PlayerId, Tick};
use crate::team::balance::{elapsed, TeamError};
use frontline_protocol::msg::input::InputCmd;

pub use config::{AfkConfig, BalanceConfig, RuleConfig, TkConfig, VoteConfig};
pub use teamswitch::RoundGroup;

#[derive(Debug, Clone, Copy)]
pub struct PlayerFacts {
    pub id: PlayerId,
    pub team: Team,
    pub is_bot: bool,
    /// Whether the player is in the active play state (including downed, excluding DEPLOY/KIA).
    pub alive: bool,
    pub in_vehicle: bool,
    pub driving: Option<EntityId>,
    pub joined_tick: Tick,
    pub last_switch_tick: Option<Tick>,
    /// Zero means unassigned.
    pub squad_size: u8,
    pub is_squad_leader: bool,
    pub commander_since: Option<Tick>,
    pub is_officer: bool,
    pub score: i32,
    pub is_admin: bool,
    pub party: Option<u32>,
}

/// Use the same tick rate throughout a match. The roster contains online registered slots
/// (including players without an avatar), uniquely sorted by ascending player ID.
#[derive(Debug, Clone, Copy)]
pub struct RuleContext<'a> {
    pub now: Tick,
    pub match_started: Tick,
    pub tick_hz: u8,
    pub match_seed: u64,
    pub roster: &'a [PlayerFacts],
}

impl RuleContext<'_> {
    fn player(&self, id: PlayerId) -> Option<&PlayerFacts> {
        self.roster
            .binary_search_by_key(&id, |p| p.id)
            .ok()
            .map(|i| &self.roster[i])
    }

    fn human(&self, id: PlayerId) -> bool {
        self.player(id).is_some_and(|p| !p.is_bot)
    }

    fn ticks(&self, seconds: u16) -> u32 {
        u32::from(seconds) * u32::from(self.tick_hz.max(1))
    }

    fn after(&self, seconds: u16) -> Tick {
        Tick(self.now.0.wrapping_add(self.ticks(seconds)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RuleError {
    UnknownPlayer = 1,
    HumanOnly = 2,
    InvalidTarget = 3,
    TeamFull = 4,
    TeamUnbalanced = 5,
    SwitchTooSoon = 6,
    SwitchCooldown = 7,
    NoBalanceOffer = 8,
    PartyWouldSplit = 9,
    VoteBusy = 10,
    VoteTooSoon = 11,
    VoteCooldown = 12,
    ProtectedTarget = 13,
    OfficerOnly = 14,
    CommanderTenure = 15,
    VoteNotFound = 16,
    VoteNotOpen = 17,
    Ineligible = 18,
    InvalidBallot = 19,
    AlreadyVoted = 20,
    InvalidMaps = 21,
    Disabled = 22,
    TkPromptNotFound = 23,
    TkPromptExpired = 24,
    PartyTooLarge = 25,
    RoundEnded = 26,
}

impl RuleError {
    pub const fn code(self) -> u8 {
        self as u8
    }
}

impl From<TeamError> for RuleError {
    fn from(error: TeamError) -> Self {
        match error {
            TeamError::Unknown => Self::UnknownPlayer,
            TeamError::AlreadyOnRoster => Self::InvalidTarget,
            TeamError::Full => Self::TeamFull,
            TeamError::Unbalanced => Self::TeamUnbalanced,
            TeamError::TooSoon => Self::SwitchTooSoon,
            TeamError::SwitchCooldown => Self::SwitchCooldown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    Input(InputCmd),
    Voice,
    Chat,
    Map,
    Interaction,
}

pub type TkEventId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TkKind {
    Downed,
    Killed,
    /// Same incident as an already counted downing; open a victim prompt without another count.
    DeathAfterDown,
    CommanderFire,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TkEvent {
    pub event_id: TkEventId,
    pub offender: PlayerId,
    pub victim: PlayerId,
    pub kind: TkKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TkDecision {
    Forgive,
    Punish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleLock {
    HighRisk,
    RiflemanOnly,
}

impl RoleLock {
    pub fn allows(self, role_id: &str) -> bool {
        match self {
            Self::HighRisk => !matches!(
                role_id,
                "machine_gunner" | "mg" | "anti_tank" | "at" | "engineer" | "crewman"
            ),
            Self::RiflemanOnly => role_id == "rifleman",
        }
    }
}

pub type VoteId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoteRequest {
    Kick(PlayerId),
    Mute(PlayerId),
    RemoveCommander(PlayerId),
    Surrender,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoteKind {
    Kick(PlayerId),
    Mute(PlayerId),
    RemoveCommander(PlayerId),
    Map([u16; 3]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ballot {
    Yes,
    No,
    Map(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoteStage {
    AwaitingSponsors { have: u8, need: u8 },
    Open,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoteSnapshot {
    pub id: VoteId,
    pub kind: VoteKind,
    /// Only this team receives a team vote; `None` is a server-wide map vote.
    pub team: Option<Team>,
    pub stage: VoteStage,
    pub closes_at: Tick,
    pub yes: u16,
    pub no: u16,
    pub required_yes: u16,
    /// Counts in candidate order for map votes, zeroes otherwise.
    pub map_votes: [u16; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteOutcome {
    Passed,
    Failed,
    Cancelled,
    MapSelected(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleCommand {
    StartVote(VoteRequest),
    CastVote {
        vote: VoteId,
        ballot: Ballot,
    },
    /// Switches to the opposite team. Offer acceptance also grants the match XP marker.
    SwitchTeam {
        accept_offer: bool,
    },
    RespondTk {
        event_id: TkEventId,
        decision: TkDecision,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice {
    TkWarning,
    CommanderFriendlyFire,
    AfkWarning,
    AfkCountdown { kick_at: Tick },
    AfkCountdownCancelled,
    RepeatedInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    Voluntary,
    Volunteer { xp_bonus_percent: u8 },
    Forced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KickReason {
    Vote,
    TeamKill,
    Afk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerAction {
    InviteSwitch {
        from: Team,
        to: Team,
        expires_at: Tick,
        xp_bonus_percent: u8,
    },
    /// Apply the move atomically: replace the selected bot, clear squad/commandership,
    /// KIA any active avatar without a death, and preserve match XP.
    Move {
        player: PlayerId,
        to: Team,
        kind: MoveKind,
        replace_bot: Option<PlayerId>,
    },
    Kick {
        player: PlayerId,
        reason: KickReason,
        cooldown_until: Option<Tick>,
    },
    Mute {
        player: PlayerId,
    },
    RelinquishLeader {
        player: PlayerId,
    },
    RelinquishCommander {
        player: PlayerId,
    },
    DriverVacancy {
        player: PlayerId,
        vehicle: EntityId,
    },
    TkPrompt {
        event: TkEvent,
        expires_at: Tick,
    },
    LockRole {
        player: PlayerId,
        lock: RoleLock,
        until: Tick,
    },
    LockVehicle {
        player: PlayerId,
        until: Tick,
    },
    VoteSnapshot(VoteSnapshot),
    VoteResult {
        snapshot: VoteSnapshot,
        outcome: VoteOutcome,
    },
    Notice {
        player: PlayerId,
        notice: Notice,
    },
}

#[derive(Debug, Clone, Default, Hash)]
pub struct ServerRules {
    config: RuleConfig,
    balance: balance::Balance,
    tk: tk::TeamKills,
    votes: vote::Votes,
    afk: afk::Afk,
}

/// Match-scoped rule history detached from a slot. The caller owns identity binding and expiry
/// at round reset; this value must only be restored for the same authenticated player.
#[derive(Debug, Clone)]
pub(crate) struct PlayerCarry {
    forced: bool,
    recent_until: Option<Tick>,
    tk: Option<tk::History>,
    kick_next: Option<Tick>,
}

impl PlayerCarry {
    pub(crate) fn hash_into(&self, w: &mut HashWriter) {
        w.write_bytes(b"rulecarry\x01");
        w.write_u8(u8::from(self.forced));
        write_option(w, self.recent_until, |w, t| w.write_u32(t.0));
        write_option(w, self.tk.as_ref(), |w, history| history.hash_into(w));
        write_option(w, self.kick_next, |w, t| w.write_u32(t.0));
    }
}

impl ServerRules {
    pub fn new(config: RuleConfig) -> Self {
        Self {
            config,
            ..Self::default()
        }
    }

    pub fn config(&self) -> &RuleConfig {
        &self.config
    }

    /// Stable field layout, with fixed-width little-endian integers and ordered collections.
    pub fn hash_into(&self, w: &mut HashWriter) {
        w.write_bytes(b"serverrules\x01");
        self.config.hash_into(w);
        self.balance.hash_into(w);
        self.tk.hash_into(w);
        self.votes.hash_into(w);
        self.afk.hash_into(w);
    }

    pub fn has_balance_offer(&self, team: Team, now: Tick) -> bool {
        self.balance.has_offer(team, now)
    }

    /// Clear match-scoped state without reusing old vote IDs for the next match.
    pub fn reset_round(&mut self) {
        self.balance = balance::Balance::default();
        self.tk = tk::TeamKills::default();
        self.votes.reset_round();
        self.afk = afk::Afk::default();
    }

    /// Required on disconnect before reusing a player slot, even within one tick.
    pub fn remove_player(&mut self, id: PlayerId) {
        self.take_player_carry(id);
    }

    /// Clear all slot state and return only the history eligible for an authenticated reconnect.
    /// Call before the next rule step (which prunes absent players).
    pub(crate) fn take_player_carry(&mut self, id: PlayerId) -> Option<PlayerCarry> {
        let (forced, recent_until) = self.balance.take_player(id);
        let tk = self.tk.take_player(id);
        let kick_next = self.votes.take_player(id);
        self.afk.remove_player(id);
        (forced || recent_until.is_some() || tk.is_some() || kick_next.is_some()).then_some(
            PlayerCarry {
                forced,
                recent_until,
                tk,
                kick_next,
            },
        )
    }

    /// Restore history into a clean slot, before the returning player's first rule command.
    /// Absolute tick deadlines are retained, not restarted.
    pub(crate) fn restore_player_carry(&mut self, id: PlayerId, carry: PlayerCarry) {
        self.balance
            .restore_player(id, carry.forced, carry.recent_until);
        if let Some(history) = carry.tk {
            self.tk.restore_player(id, history);
        }
        if let Some(until) = carry.kick_next {
            self.votes.restore_player(id, until);
        }
    }

    pub fn step(&mut self, ctx: &RuleContext<'_>) -> Vec<ServerAction> {
        debug_assert!(ctx.roster.windows(2).all(|p| p[0].id < p[1].id));
        let mut actions = Vec::new();
        self.tk.step(&self.config.tk, ctx);
        self.votes.step(&self.config.vote, ctx, &mut actions);
        if !self.votes.round_ended() {
            self.afk.step(&self.config.afk, ctx, &mut actions);
            // Don't pick a player already scheduled to disconnect or relinquish leadership.
            self.balance.step(&self.config.balance, ctx, &mut actions);
        }
        actions
    }

    pub fn command(
        &mut self,
        ctx: &RuleContext<'_>,
        id: PlayerId,
        command: RuleCommand,
    ) -> Result<Vec<ServerAction>, RuleError> {
        let p = ctx.player(id).ok_or(RuleError::UnknownPlayer)?;
        if p.is_bot {
            return Err(RuleError::HumanOnly);
        }
        let mut actions = Vec::new();
        match command {
            RuleCommand::StartVote(request) => {
                self.votes
                    .start(&self.config.vote, ctx, p, request, &mut actions)?;
            }
            RuleCommand::CastVote { vote, ballot } => {
                self.votes
                    .cast(&self.config.vote, ctx, p, vote, ballot, &mut actions)?;
            }
            RuleCommand::SwitchTeam { accept_offer } => {
                if self.votes.round_ended() {
                    return Err(RuleError::RoundEnded);
                }
                let offer = accept_offer
                    .then(|| self.balance.volunteer(&self.config.balance, ctx, p))
                    .transpose()?;
                self.can_switch(ctx, id)?;
                let to = other(p.team);
                actions.push(teamswitch::move_action(
                    ctx,
                    id,
                    to,
                    offer.unwrap_or(MoveKind::Voluntary),
                ));
                if accept_offer {
                    self.balance
                        .accepted(ctx, id, self.config.balance.recent_move_s);
                }
            }
            RuleCommand::RespondTk { event_id, decision } => {
                self.tk
                    .respond(&self.config.tk, ctx, id, event_id, decision, &mut actions)?;
            }
        }
        Ok(actions)
    }

    pub fn record_activity(&mut self, ctx: &RuleContext<'_>, id: PlayerId, activity: Activity) {
        if let Some(p) = ctx.player(id).filter(|p| !p.is_bot) {
            self.afk.record(&self.config.afk, ctx, p, activity);
        }
    }

    /// `event` is a trusted, deduplicated friendly incident from combat. Sim scoring is separate.
    pub fn report_tk(&mut self, ctx: &RuleContext<'_>, event: TkEvent) -> Vec<ServerAction> {
        let mut actions = Vec::new();
        self.tk.report(&self.config.tk, ctx, event, &mut actions);
        actions
    }

    pub fn tk_count(&self, ctx: &RuleContext<'_>, id: PlayerId) -> usize {
        self.tk.count(&self.config.tk, ctx, id)
    }

    pub fn role_lock(&self, now: Tick, id: PlayerId) -> Option<RoleLock> {
        self.tk.role_lock(now, id)
    }

    pub fn vehicle_locked(&self, now: Tick, id: PlayerId) -> bool {
        self.role_lock(now, id) == Some(RoleLock::RiflemanOnly)
    }

    pub fn is_afk(&self, id: PlayerId) -> bool {
        self.afk.is_afk(id)
    }

    pub fn repeated_input(&self, id: PlayerId) -> bool {
        self.afk.repeated(id)
    }

    /// Called once by World at round end with its three map-rotation candidates.
    pub fn start_map_vote(
        &mut self,
        ctx: &RuleContext<'_>,
        maps: [u16; 3],
    ) -> Result<Vec<ServerAction>, RuleError> {
        let mut actions = Vec::new();
        self.votes
            .start_map(&self.config.vote, ctx, maps, &mut actions)?;
        Ok(actions)
    }

    pub fn can_join(&self, ctx: &RuleContext<'_>, team: Team) -> Result<(), RuleError> {
        self.can_join_party(ctx, team, 1)
    }

    /// A party admission is all-or-nothing. Registration and bot replacement are caller-owned.
    pub fn can_join_party(
        &self,
        ctx: &RuleContext<'_>,
        team: Team,
        size: u8,
    ) -> Result<(), RuleError> {
        teamswitch::can_join(&self.config.balance, ctx, team, size)
    }

    pub fn can_switch(&self, ctx: &RuleContext<'_>, id: PlayerId) -> Result<(), RuleError> {
        teamswitch::can_switch(&self.config.balance, ctx, id)
    }

    /// Order in which bots yield to arriving humans, including when spare capacity exists.
    pub fn bot_replacements(&self, ctx: &RuleContext<'_>, team: Team) -> Vec<PlayerId> {
        teamswitch::bot_replacements(ctx, team)
    }

    /// Optional inter-round allocation for caller-supplied, disjoint squad/party groups.
    pub fn rebalance_next_round(
        &self,
        groups: &[RoundGroup<'_>],
    ) -> Result<Vec<(PlayerId, Team)>, RuleError> {
        teamswitch::rebalance_next_round(&self.config.balance, groups)
    }
}

fn other(team: Team) -> Team {
    match team {
        Team::Us => Team::Ger,
        Team::Ger => Team::Us,
    }
}

fn team_from_index(side: usize) -> Team {
    [Team::Us, Team::Ger][side]
}

fn human_counts(ctx: &RuleContext<'_>) -> [u32; 2] {
    let mut counts = [0; 2];
    for p in ctx.roster.iter().filter(|p| !p.is_bot) {
        counts[p.team as usize] += 1;
    }
    counts
}

fn shared_party(ctx: &RuleContext<'_>, p: &PlayerFacts) -> bool {
    p.party.is_some_and(|party| {
        ctx.roster
            .iter()
            .any(|q| q.id != p.id && !q.is_bot && q.party == Some(party))
    })
}

fn write_option<T>(w: &mut HashWriter, value: Option<T>, write: impl FnOnce(&mut HashWriter, T)) {
    w.write_u8(u8::from(value.is_some()));
    if let Some(value) = value {
        write(w, value);
    }
}
