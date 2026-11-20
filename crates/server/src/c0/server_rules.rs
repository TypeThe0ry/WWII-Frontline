//! M2-018 wire translation. Policy is evaluated exclusively in sim.

use frontline_protocol::msg::c0::ClientMsgC0;
use frontline_protocol::msg::c4::ServerEventC4;
use frontline_protocol::msg::vote::{
    team_move_flags, MapVoteResult, MapVoteState, TeamBalanceCode as Code, TeamBalanceNotice,
    TkPrompt, VoteKind as WireVote, VoteResult, VoteState,
};
use frontline_sim::serverrules::{
    Ballot, MoveKind, Notice, RoleLock, RuleCommand, RuleError, ServerAction, TkDecision, VoteKind,
    VoteOutcome, VoteRequest,
};
use frontline_sim::{PlayerId, Team, Tick, World};

pub(crate) fn is_request(msg: &ClientMsgC0) -> bool {
    matches!(
        msg,
        ClientMsgC0::VoteStart(_)
            | ClientMsgC0::VoteCast(_)
            | ClientMsgC0::TeamSwitchRequest(_)
            | ClientMsgC0::ForgivePunish(_)
            | ClientMsgC0::MapVoteCast(_)
    )
}

pub(crate) fn request(
    world: &mut World,
    p: PlayerId,
    msg: ClientMsgC0,
) -> Result<Vec<ServerAction>, ServerEventC4> {
    let kind = if matches!(
        msg,
        ClientMsgC0::VoteStart(_) | ClientMsgC0::VoteCast(_) | ClientMsgC0::MapVoteCast(_)
    ) {
        Code::VoteDenied
    } else {
        Code::Denied
    };
    let denied = |e: RuleError| notice(kind, [i32::from(e.code()), 0]);
    let command = match msg {
        ClientMsgC0::TeamSwitchRequest(m) => {
            let entry = world
                .squads()
                .entry(p)
                .ok_or_else(|| denied(RuleError::UnknownPlayer))?;
            if m.team > 1 || m.team == entry.team as u8 {
                return Err(denied(RuleError::InvalidTarget));
            }
            RuleCommand::SwitchTeam {
                accept_offer: m.accept_offer,
            }
        }
        ClientMsgC0::VoteStart(m) => RuleCommand::StartVote(match m.kind {
            WireVote::Kick => VoteRequest::Kick(m.target),
            WireVote::RemoveCommander => VoteRequest::RemoveCommander(m.target),
            WireVote::Mute => VoteRequest::Mute(m.target),
        }),
        ClientMsgC0::VoteCast(m) => RuleCommand::CastVote {
            vote: m.vote_id,
            ballot: if m.yes { Ballot::Yes } else { Ballot::No },
        },
        ClientMsgC0::MapVoteCast(m) => RuleCommand::CastVote {
            vote: m.vote_id,
            ballot: Ballot::Map(
                world
                    .rule_map_choice(m.choice)
                    .ok_or_else(|| denied(RuleError::InvalidMaps))?,
            ),
        },
        ClientMsgC0::ForgivePunish(m) => RuleCommand::RespondTk {
            event_id: m.victim_event.into(),
            decision: if m.punish {
                TkDecision::Punish
            } else {
                TkDecision::Forgive
            },
        },
        _ => unreachable!("non-rule command"),
    };
    world.server_rule_command(p, command).map_err(denied)
}

pub(crate) fn notice(code: Code, args: [i32; 2]) -> ServerEventC4 {
    ServerEventC4::TeamBalanceNotice(TeamBalanceNotice { code, args })
}

pub(crate) fn moved(team: Team, kind: MoveKind, tick: Tick) -> ServerEventC4 {
    let flags = match kind {
        MoveKind::Voluntary => 0,
        MoveKind::Volunteer { .. } => team_move_flags::VOLUNTEER_BONUS,
        MoveKind::Forced => team_move_flags::FORCED,
    };
    notice(Code::Moved, [team as i32 | (flags << 8), tick.0 as i32])
}

pub(crate) enum Audience {
    Player(PlayerId),
    Group(Option<Team>),
}

pub(crate) fn event(world: &World, action: ServerAction) -> Option<(Audience, ServerEventC4)> {
    use Audience::{Group, Player};
    let seconds = |until: Tick| {
        (until.diff(world.tick()).max(0) as u32).div_ceil(u32::from(world.config().tick_hz.max(1)))
    };
    Some(match action {
        ServerAction::InviteSwitch {
            from,
            to,
            expires_at,
            ..
        } => (
            Group(Some(from)),
            notice(Code::Invite, [to as i32, seconds(expires_at) as i32]),
        ),
        ServerAction::TkPrompt { event, expires_at } => (
            Player(event.victim),
            ServerEventC4::TkPrompt(TkPrompt {
                killer: event.offender,
                expires_s: seconds(expires_at) as u8,
                victim_event: event.event_id as u16,
            }),
        ),
        ServerAction::Notice {
            player,
            notice: why,
        } => {
            let cfg = frontline_sim::serverrules::RuleConfig::default();
            let (kind, seconds) = match why {
                Notice::TkWarning => (Code::TkWarning, i32::from(cfg.tk.lock_s)),
                Notice::CommanderFriendlyFire => (Code::TkWarning, 0),
                Notice::AfkWarning => {
                    let leader = world
                        .squads()
                        .squad_of(player)
                        .is_some_and(|s| s.leader == player);
                    let limit = if leader {
                        cfg.afk.leader_s
                    } else {
                        cfg.afk.alive_s
                    };
                    (
                        Code::AfkWarning,
                        i32::from(limit) + i32::from(cfg.afk.countdown_s)
                            - i32::from(cfg.afk.warning_s),
                    )
                }
                Notice::AfkCountdown { kick_at } => (Code::AfkCountdown, seconds(kick_at) as i32),
                Notice::AfkCountdownCancelled => (Code::AfkCancelled, 0),
                Notice::RepeatedInput => (Code::RepeatInput, 0),
            };
            (Player(player), notice(kind, [seconds, 0]))
        }
        ServerAction::LockRole {
            player,
            lock,
            until,
        } => (
            Player(player),
            notice(
                Code::LockRole,
                [
                    match lock {
                        RoleLock::HighRisk => 1,
                        RoleLock::RiflemanOnly => 2,
                    },
                    seconds(until) as i32,
                ],
            ),
        ),
        ServerAction::LockVehicle { player, until } => (
            Player(player),
            notice(Code::LockVehicle, [seconds(until) as i32, 0]),
        ),
        ServerAction::DriverVacancy { player, vehicle } => {
            let team = world.squads().entry(player)?.team;
            (
                Group(Some(team)),
                notice(Code::DriverVacant, [i32::from(vehicle), 0]),
            )
        }
        ServerAction::Mute { player } => {
            (Player(player), notice(Code::Muted, [i32::from(player), 0]))
        }
        ServerAction::VoteSnapshot(s) => {
            let (kind, target) = match s.kind {
                VoteKind::Kick(p) => (WireVote::Kick, p),
                VoteKind::Mute(p) => (WireVote::Mute, p),
                VoteKind::RemoveCommander(p) => (WireVote::RemoveCommander, p),
                VoteKind::Map(maps) => {
                    return Some((
                        Group(None),
                        ServerEventC4::MapVoteState(MapVoteState {
                            vote_id: s.id,
                            candidates: maps.map(|m| m as u8),
                            votes: s.map_votes.map(|n| n as u8),
                            ends_in_s: seconds(s.closes_at) as u8,
                        }),
                    ))
                }
            };
            (
                Group(s.team),
                ServerEventC4::VoteState(VoteState {
                    vote_id: s.id,
                    kind,
                    target,
                    yes: s.yes as u8,
                    no: s.no as u8,
                    ends_in_s: seconds(s.closes_at) as u8,
                }),
            )
        }
        ServerAction::VoteResult { snapshot, outcome } => (
            Group(snapshot.team),
            if let VoteOutcome::MapSelected(map) = outcome {
                ServerEventC4::MapVoteResult(MapVoteResult {
                    vote_id: snapshot.id,
                    winner: map as u8,
                })
            } else {
                ServerEventC4::VoteResult(VoteResult {
                    vote_id: snapshot.id,
                    passed: outcome == VoteOutcome::Passed,
                })
            },
        ),
        _ => return None,
    })
}
