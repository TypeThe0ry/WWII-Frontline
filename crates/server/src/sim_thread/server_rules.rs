use super::{Clock, SimLoop};
use crate::c0::server_rules::{self as wire, Audience};
use frontline_protocol::msg::c0::ClientMsgC0;
use frontline_sim::serverrules::{RuleConfig, RuleError, ServerAction};
use frontline_sim::{PlayerId, Team};

impl<C: Clock> SimLoop<C> {
    pub(super) fn handle_server_rule(&mut self, player: PlayerId, msg: ClientMsgC0) {
        match wire::request(&mut self.world, player, msg) {
            Ok(actions) => self.apply_server_actions(actions),
            Err(event) => self.c4.rule_private(player, event),
        }
    }

    pub(super) fn make_room_for_human(&mut self, team: Team) {
        let full = self.world.squads().team_count(team)
            >= u32::from(RuleConfig::default().balance.team_max_players);
        if full || self.bot_fill.is_some() {
            if let Some(bot) = self.world.rules_bot_replacement(team) {
                self.leave_player(bot);
            }
        }
    }

    pub(super) fn apply_server_actions(&mut self, actions: Vec<ServerAction>) {
        for action in actions {
            if let Some((audience, event)) = wire::event(&self.world, action) {
                match audience {
                    Audience::Player(player) => self.c4.rule_private(player, event),
                    Audience::Group(team) => self.c4.rule_group(&self.world, team, event),
                }
            }
            match action {
                ServerAction::Move {
                    player,
                    to,
                    kind,
                    replace_bot,
                } => {
                    if self.bridge.session(player).is_none() {
                        continue;
                    }
                    if let Some(bot) = replace_bot {
                        if self.bot_fill.is_some()
                            || self.world.squads().team_count(to)
                                >= u32::from(RuleConfig::default().balance.team_max_players)
                        {
                            self.leave_player(bot);
                        }
                    }
                    match self.world.apply_rule_move(player, to, kind) {
                        Ok(()) => {
                            if let (Some(host), Some(s)) =
                                (self.bot_host.as_mut(), self.bridge.session(player))
                            {
                                host.human_left(player, s.team, self.world.tick().0);
                            }
                            self.bridge.set_team(player, to);
                            self.team_channel.forget(player);
                            self.seat_input.forget(player);
                            self.snap.forget(player);
                            self.c4
                                .rule_private(player, wire::moved(to, kind, self.world.tick()));
                            self.update_player_gauges();
                        }
                        Err(error) => self.c4.rule_private(
                            player,
                            wire::notice(
                                frontline_protocol::msg::vote::TeamBalanceCode::Denied,
                                [i32::from(RuleError::from(error).code()), 0],
                            ),
                        ),
                    }
                }
                ServerAction::Kick {
                    player,
                    reason,
                    cooldown_until,
                } => {
                    tracing::info!(event = "server_rule_kick", player, ?reason);
                    if let Some(net) = self.bridge.net() {
                        if let Some(until) = cooldown_until {
                            net.sessions.set_tk_cooldown(player, until);
                        }
                        if let Some(handle) = net.sessions.handle_of(player) {
                            crate::net::conn::disconnect(
                                &handle,
                                frontline_protocol::msg::c0::disconnect_reason::SERVER_RULE,
                            );
                        }
                    }
                    self.metrics.session_kicked_total.inc();
                    self.leave_player(player);
                }
                ServerAction::Mute { player } => self.world.apply_rule_mute(player),
                ServerAction::RelinquishLeader { player } => {
                    self.world.apply_afk_leader_transfer(player)
                }
                ServerAction::DriverVacancy { player, .. } => self.world.vacate_rule_seat(player),
                _ => {}
            }
        }
    }
}
