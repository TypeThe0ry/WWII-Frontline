//! Routes authoritative sim output onto the existing reliable connection queues (M1-026b).

use std::collections::BTreeMap;

use bytes::Bytes;
use frontline_protocol::msg::c4::{
    encode_c4, roster_flags, Death, Kill, MatchPhase, ObjectiveChange, Phase, PlayerRoster, Revive,
    ScoreDelta, ServerEventC4, SpawnKind, SpawnPointState, KILLER_ENV, KILL_DIST_BITS,
};
use frontline_protocol::{quant::quant_pos_xz, Channel};
use frontline_sim::medical::CreditKind;
use frontline_sim::rules::{CaptureReason, MatchPhaseKind};
use frontline_sim::spawn::garrison::GarrisonState;
use frontline_sim::{LifeState, PlayerId, RuleChange, SimEvent, Team, World, WorldView};

use crate::bot_host::BotHost;
use crate::net::transport::{ClientHandle, Rel};
use crate::sim_bridge::SimBridge;

struct Client {
    handle: ClientHandle,
    team: Team,
    previous_team: Option<Team>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Row {
    team: Team,
    squad: u8,
    role: u8,
    flags: u8,
}

#[derive(Default)]
pub(crate) struct Broadcast {
    clients: BTreeMap<PlayerId, Client>,
    roster: BTreeMap<PlayerId, Row>,
}

impl Broadcast {
    pub(crate) fn join(&mut self, bridge: &SimBridge, player: PlayerId) {
        let Some(handle) = bridge.net().and_then(|net| net.sessions.handle_of(player)) else {
            return;
        };
        let Some(session) = bridge.session(player) else {
            return;
        };
        // Keep this connection's handle, not a lookup by a potentially reused id at send time.
        self.clients.insert(
            player,
            Client {
                handle,
                team: session.team,
                previous_team: None,
            },
        );
    }

    pub(crate) fn leave(&mut self, player: PlayerId) {
        self.clients.remove(&player);
    }

    pub(crate) fn rule_private(&self, player: PlayerId, event: ServerEventC4) {
        send_to(self.clients.get(&player).into_iter(), || event);
    }

    pub(crate) fn rule_group(&self, world: &World, team: Option<Team>, event: ServerEventC4) {
        send_to(
            self.clients.iter().filter_map(|(&player, client)| {
                let roster = world.squads().entry(player)?;
                team.is_none_or(|t| t == roster.team).then_some(client)
            }),
            || event,
        );
    }

    pub(crate) fn tick(
        &mut self,
        world: &World,
        bridge: &SimBridge,
        bots: Option<&BotHost>,
        events: &[SimEvent],
        rules: &[RuleChange],
    ) {
        let team_of = |p| {
            world
                .squads()
                .entry(p)
                .map(|e| e.team)
                .or_else(|| bridge.session(p).map(|s| s.team))
        };
        self.clients.retain(|&p, client| {
            if let Some(team) = team_of(p).filter(|_| !client.handle.is_closed()) {
                client.team = team;
                true
            } else {
                false
            }
        });
        if self.clients.is_empty() {
            self.roster.clear();
            return;
        }

        // Roster rows are replacements, not operation records. Comparing the final projection
        // covers squad/role ops, leader changes, removals and downed/revived status without
        // applying queued pre-join events to a newcomer after its baseline.
        self.roster(world, bots);

        for (i, event) in events.iter().enumerate() {
            match *event {
                SimEvent::Downed { victim, .. } => self.death(victim, 0, true),
                SimEvent::Kill {
                    victim,
                    killer,
                    weapon,
                    headshot,
                    dist_m,
                } => {
                    if let Some(LifeState::Dead { respawn_tick, .. }) = world.player_life(victim) {
                        let remaining = respawn_tick.0.saturating_sub(world.tick().0);
                        let seconds = remaining.div_ceil(u32::from(world.config().tick_hz));
                        self.death(victim, u8::try_from(seconds).unwrap_or(u8::MAX), false);
                    }
                    let Some(team) = team_of(killer.unwrap_or(victim)) else {
                        continue;
                    };
                    // The sim emits credited KIA immediately after the associated Kill.
                    let mut slots = [KILLER_ENV; 2];
                    if let Some(SimEvent::KillCredit {
                        victim: v,
                        killer: k,
                        assists,
                        kind: CreditKind::Direct | CreditKind::Finish,
                        ..
                    }) = events.get(i + 1)
                    {
                        if (*v, *k) == (victim, killer) {
                            for (slot, &assist) in slots.iter_mut().zip(assists) {
                                *slot = assist;
                            }
                        }
                    }
                    self.send(
                        Some(team),
                        ServerEventC4::Kill(Kill {
                            victim,
                            killer: killer.unwrap_or(KILLER_ENV),
                            weapon,
                            headshot,
                            dist_q: distance_code(dist_m),
                            assist1: slots[0],
                            assist2: slots[1],
                        }),
                    );
                }
                SimEvent::SpawnPointState {
                    id,
                    team,
                    pos,
                    state,
                } => {
                    let [x, z] = [quant_pos_xz(pos.x), quant_pos_xz(pos.z)];
                    if !x.valid || !z.valid {
                        tracing::warn!(event = "c4_invalid_spawn_position", id);
                        continue;
                    }
                    self.send(
                        Some(team),
                        ServerEventC4::SpawnPointState(SpawnPointState {
                            id,
                            kind: SpawnKind::Garrison,
                            team: team as u8,
                            pos: [x.code, z.code],
                            state: match state {
                                GarrisonState::Building => 0,
                                GarrisonState::Active => 1,
                                GarrisonState::Destroyed => 2,
                            },
                        }),
                    );
                }
                SimEvent::Revive {
                    medic,
                    target_player,
                    ..
                } => {
                    // The event carries an EntityId for the medic, not a wire PlayerId.
                    if let Some(body) = world.players().iter().find(|p| p.entity == medic) {
                        if team_of(body.player) == Some(body.team)
                            && team_of(target_player) == Some(body.team)
                        {
                            self.send(
                                Some(body.team),
                                ServerEventC4::Revive(Revive {
                                    medic: body.player,
                                    target: target_player,
                                }),
                            );
                        }
                    }
                }
                SimEvent::ScoreDelta(ref score) => {
                    // Score reasons can reveal team activity (e.g. revives and spawns).
                    if let Some(team) = team_of(score.player) {
                        self.send(
                            Some(team),
                            ServerEventC4::ScoreDelta(ScoreDelta {
                                player: score.player,
                                kind: score.kind as u8,
                                value: score.value,
                            }),
                        );
                    }
                }
                _ => {}
            }
        }
        for rule in rules {
            if let Some(event) = rule_event(rule) {
                self.send(None, event);
            }
        }
    }

    fn roster(&mut self, world: &World, bots: Option<&BotHost>) {
        let mut next = BTreeMap::new();
        for (player, entry) in world.squads().roster() {
            let squad = entry.squad.unwrap_or_default();
            let Some(squad) = wire_id("squad", squad) else {
                continue;
            };
            // Match C3: zero is no assigned role; otherwise one-based GameData role index.
            let role = entry
                .role
                .and_then(|r| world.config().data.roles.iter().position(|d| d.id == r))
                .map_or(0, |i| i + 1);
            let Some(role) = wire_id("role", role) else {
                continue;
            };
            let mut flags = 0;
            match world.player_life(player) {
                Some(LifeState::Alive) => flags |= roster_flags::ALIVE,
                Some(LifeState::Downed { .. }) => flags |= roster_flags::DOWNED,
                _ => {}
            }
            if world
                .squads()
                .squad_of(player)
                .is_some_and(|s| s.leader == player)
            {
                flags |= roster_flags::SQUAD_LEADER;
            }
            if bots.is_some_and(|b| b.is_bot(player)) {
                flags |= roster_flags::BOT;
            }
            next.insert(
                player,
                Row {
                    team: entry.team,
                    squad,
                    role,
                    flags,
                },
            );
        }

        let mut entered = [false; 2];
        let mut exited = [false; 2];
        for c in self.clients.values() {
            if c.previous_team != Some(c.team) {
                entered[c.team as usize] = true;
                if let Some(team) = c.previous_team {
                    exited[team as usize] = true;
                }
            }
        }
        // Only receivers who had an old row get its tombstone. In particular, a same-tick
        // join never sees a departure it didn't know about, nor any row from the other team.
        for (&player, old) in &self.roster {
            let removed = next.get(&player).is_none_or(|r| r.team != old.team);
            if !removed && !exited[old.team as usize] {
                continue;
            }
            let recipients = self
                .clients
                .values()
                .filter(|c| c.previous_team == Some(old.team) && (removed || c.team != old.team));
            send_to(recipients, || {
                ServerEventC4::PlayerRoster(PlayerRoster {
                    player,
                    nick: String::new(),
                    team: old.team as u8,
                    squad: 0,
                    role: 0,
                    flags: roster_flags::LEFT,
                })
            });
        }
        for (&player, row) in &next {
            let changed = self.roster.get(&player) != Some(row);
            if !changed && !entered[row.team as usize] {
                continue;
            }
            let recipients = self
                .clients
                .values()
                .filter(|c| c.team == row.team && (changed || c.previous_team != Some(row.team)));
            send_to(recipients, || {
                ServerEventC4::PlayerRoster(PlayerRoster {
                    player,
                    // Hello has no nickname. Use the existing bot name or a display fallback.
                    nick: bots
                        .and_then(|b| b.nickname(player))
                        .map_or_else(|| format!("Player {player}"), str::to_owned),
                    team: row.team as u8,
                    squad: row.squad,
                    role: row.role,
                    flags: row.flags,
                })
            });
        }
        self.roster = next;
        for client in self.clients.values_mut() {
            client.previous_team = Some(client.team);
        }
    }

    fn death(&self, victim: PlayerId, respawn_after: u8, can_be_revived: bool) {
        send_to(self.clients.get(&victim).into_iter(), || {
            ServerEventC4::Death(Death {
                victim,
                respawn_after,
                can_be_revived,
            })
        });
    }

    fn send(&self, team: Option<Team>, event: ServerEventC4) {
        send_to(
            self.clients
                .values()
                .filter(|c| team.is_none_or(|t| t == c.team)),
            || event,
        );
    }
}

fn send_to<'a>(
    recipients: impl Iterator<Item = &'a Client>,
    event: impl FnOnce() -> ServerEventC4,
) {
    let mut recipients = recipients.peekable();
    if recipients.peek().is_none() {
        return;
    }
    let event = event();
    let payload = match encode_c4(&event) {
        Ok(bytes) => Bytes::from(bytes),
        Err(error) => {
            tracing::warn!(
                event = "c4_encode_failed",
                msg_type = event.msg_type(),
                ?error
            );
            return;
        }
    };
    for client in recipients {
        // The shared queue already accounts for and disconnects slow peers independently.
        client
            .handle
            .send(Channel::C4, payload.clone(), Rel::Reliable);
    }
}

fn wire_id<I: Copy + std::fmt::Debug>(field: &'static str, id: I) -> Option<u8>
where
    u8: TryFrom<I>,
{
    match u8::try_from(id) {
        Ok(code) => Some(code),
        Err(_) => {
            tracing::warn!(event = "c4_id_out_of_range", field, ?id);
            None
        }
    }
}

fn distance_code(m: f32) -> u16 {
    let max = (1u16 << KILL_DIST_BITS) - 1;
    (m * 4.0).round().clamp(0.0, f32::from(max)) as u16
}

fn rule_event(rule: &RuleChange) -> Option<ServerEventC4> {
    match *rule {
        RuleChange::ObjectiveOwner {
            objective,
            owner,
            reason,
            ..
        } => Some(ServerEventC4::ObjectiveChange(ObjectiveChange {
            objective: wire_id("objective", objective)?,
            owner: owner as u8,
            reason: match reason {
                CaptureReason::Captured => 0,
            },
        })),
        RuleChange::MatchPhase { phase, ends_at } => Some(ServerEventC4::MatchPhase(MatchPhase {
            phase: match phase {
                MatchPhaseKind::Idle => return None,
                MatchPhaseKind::Warmup => Phase::Warmup,
                MatchPhaseKind::Live => Phase::Live,
                MatchPhaseKind::Overtime => Phase::Overtime,
                MatchPhaseKind::Ended => Phase::Ended,
            },
            ends_at,
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::fake::FakeClock;
    use crate::metrics::Metrics;
    use crate::net::conn::{InboundQueue, Sessions};
    use crate::shutdown::StopFlag;
    use crate::sim_bridge::NetLink;
    use crate::sim_thread::{build_loop, Command, SimThreadConfig};
    use frontline_protocol::msg::c4::decode_c4;
    use frontline_sim::{CollisionData, GameData, GameMode, MapId, SimConfig, Vec3};

    async fn drain(handle: &ClientHandle) -> Vec<ServerEventC4> {
        let mut events = Vec::new();
        while handle.stats().queued_reliable_bytes > 0 {
            let (channel, payload) = handle.next_outbound().await.unwrap();
            assert_eq!(channel, Channel::C4);
            events.push(decode_c4(&payload).unwrap());
            handle.write_done(payload.len());
        }
        events
    }

    #[tokio::test]
    async fn join_baseline_skips_same_tick_departure_and_opponent_rows() {
        let metrics = Metrics::new();
        let sessions = Sessions::new(metrics.clone());
        let queue = InboundQueue::new();
        let mut config = SimThreadConfig::bare(0, MapId(0), GameMode::Sandbox);
        config.net = Some(NetLink {
            inbound: queue.rx,
            sessions: sessions.clone(),
        });
        let (mut sim, _, commands) =
            build_loop(FakeClock::new(), &config, metrics.clone(), StopFlag::new());
        let handles: [_; 5] = std::array::from_fn(|_| ClientHandle::new(metrics.clone()));
        let join = |i: usize| {
            let (player, team) = sessions.alloc_player_team(&handles[i]).unwrap();
            assert_eq!(usize::from(player), i);
            commands.send(Command::Join { player, team }).unwrap();
        };
        for i in 0..4 {
            join(i);
        }
        sim.tick();
        let rows = |ev: Vec<ServerEventC4>| {
            ev.into_iter()
                .filter_map(|event| match event {
                    ServerEventC4::PlayerRoster(row) => Some((row.player, row.team, row.flags)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert!(sim.world().player_life(0).is_some_and(|l| l.is_alive()));
        let alive = roster_flags::ALIVE;
        assert_eq!(
            rows(drain(&handles[2]).await),
            [(0, 0, alive), (2, 0, alive)]
        );
        assert_eq!(
            rows(drain(&handles[1]).await),
            [(1, 1, alive), (3, 1, alive)]
        );

        commands.send(Command::RemovePlayer(0)).unwrap();
        join(4);
        sim.tick();
        assert_eq!(
            rows(drain(&handles[4]).await),
            [(2, 0, alive), (4, 0, alive)]
        );
        assert_eq!(
            rows(drain(&handles[2]).await),
            [(0, 0, roster_flags::LEFT), (4, 0, alive)]
        );
        assert!(rows(drain(&handles[1]).await).is_empty());
        sim.tick();
        assert!(rows(drain(&handles[4]).await).is_empty());
    }

    #[tokio::test]
    async fn route_only_to_eligible_connections_and_check_wire_ids() {
        let metrics = Metrics::new();
        let sessions = Sessions::new(metrics.clone());
        let queue = InboundQueue::new();
        let mut bridge = SimBridge::new(
            metrics.clone(),
            Some(NetLink {
                inbound: queue.rx,
                sessions: sessions.clone(),
            }),
            None,
            Default::default(),
        );
        let us = ClientHandle::new(metrics.clone());
        let ger = ClientHandle::new(metrics.clone());
        let not_joined = ClientHandle::new(metrics);
        let mut c4 = Broadcast::default();
        for handle in [&us, &ger] {
            let (id, team) = sessions.alloc_player_team(handle).unwrap();
            bridge.open_session(id, team, 0.0);
            c4.join(&bridge, id);
        }
        sessions.alloc_player_team(&not_joined).unwrap();
        let world = World::new(
            SimConfig {
                tick_hz: 30,
                match_seed: 0,
                map: MapId(0),
                mode: GameMode::Sandbox,
                data: GameData::embedded(),
            },
            &CollisionData::empty(),
        );
        let events = [
            SimEvent::Kill {
                victim: 0,
                killer: None,
                weapon: 4,
                headshot: false,
                dist_m: 0.0,
            },
            SimEvent::Kill {
                victim: 0,
                killer: Some(1),
                weapon: 4,
                headshot: true,
                dist_m: 5.5,
            },
            SimEvent::SpawnPointState {
                id: 600,
                team: Team::Us,
                pos: Vec3::new(1.0, 0.0, 2.0),
                state: GarrisonState::Active,
            },
            SimEvent::ScoreDelta(frontline_sim::score::ScoreDelta {
                player: 0,
                kind: frontline_sim::score::ScoreKind::Revive,
                category: frontline_sim::score::Category::Support,
                value: 20,
                role_at_time: 2,
            }),
        ];
        let rules = [256, 255].map(|objective| RuleChange::ObjectiveOwner {
            objective,
            owner: Team::Ger,
            reason: CaptureReason::Captured,
            repeat: true,
        });
        c4.tick(&world, &bridge, None, &events, &rules);

        let all = ServerEventC4::ObjectiveChange(ObjectiveChange {
            objective: 255,
            owner: 1,
            reason: 0,
        });
        assert_eq!(
            drain(&us).await,
            [
                ServerEventC4::Kill(Kill {
                    victim: 0,
                    killer: KILLER_ENV,
                    weapon: 4,
                    headshot: false,
                    dist_q: 0,
                    assist1: KILLER_ENV,
                    assist2: KILLER_ENV,
                }),
                ServerEventC4::SpawnPointState(SpawnPointState {
                    id: 600,
                    kind: SpawnKind::Garrison,
                    team: 0,
                    pos: [32, 64],
                    state: 1,
                }),
                ServerEventC4::ScoreDelta(ScoreDelta {
                    player: 0,
                    kind: 0xc0,
                    value: 20,
                }),
                all.clone(),
            ]
        );
        assert_eq!(
            drain(&ger).await,
            [
                ServerEventC4::Kill(Kill {
                    victim: 0,
                    killer: 1,
                    weapon: 4,
                    headshot: true,
                    dist_q: 22,
                    assist1: KILLER_ENV,
                    assist2: KILLER_ENV,
                }),
                all,
            ]
        );
        assert!(drain(&not_joined).await.is_empty());
    }
}
