//! World facts and application of authoritative server-rule actions.

use std::collections::BTreeMap;

use super::*;
use crate::medical::CreditKind;
use crate::rules::MatchPhaseKind;
use crate::serverrules::{
    Activity, MoveKind, PlayerCarry, PlayerFacts, RuleCommand, RuleContext, RuleError,
    ServerAction, ServerRules, TkEvent, TkKind,
};

const HUMAN: u8 = 1;
const ADMIN: u8 = 2;
const MUTED: u8 = 4;

struct ReconnectState {
    rules: Option<PlayerCarry>,
    last_switch: Option<Tick>,
    xp_bonus: u8,
    muted: bool,
}

pub(super) struct Authority {
    rules: ServerRules,
    slots: [u8; 128],
    xp_bonus: [u8; 128],
    guests: [Option<[u8; 8]>; 128],
    disconnected: BTreeMap<[u8; 8], ReconnectState>,
    downed_by: BTreeMap<PlayerId, PlayerId>,
    next_incident: u16,
    map_candidates: Option<[u16; 3]>,
    map_vote_started: bool,
}

impl Authority {
    pub(super) fn is_afk(&self, p: PlayerId) -> bool {
        self.rules.is_afk(p)
    }

    pub(super) fn forget(&mut self, p: PlayerId, last_switch: Option<Tick>) {
        let slot = usize::from(p);
        if let Some(guest) = self.guests[slot].take() {
            let state = ReconnectState {
                rules: self.rules.take_player_carry(p),
                last_switch,
                xp_bonus: self.xp_bonus[slot],
                muted: self.slots[slot] & MUTED != 0,
            };
            if state.rules.is_some()
                || state.last_switch.is_some()
                || state.xp_bonus != 0
                || state.muted
            {
                self.disconnected.insert(guest, state);
            }
        } else {
            self.rules.remove_player(p);
        }
        self.slots[slot] = 0;
        self.xp_bonus[slot] = 0;
        self.downed_by.retain(|v, k| *v != p && *k != p);
    }

    pub(super) fn hash_into(&self, out: &mut HashWriter) {
        out.write_u8(0xD9);
        out.write_bytes(&self.slots);
        out.write_bytes(&self.xp_bonus);
        for guest in self.guests {
            out.write_u8(u8::from(guest.is_some()));
            if let Some(guest) = guest {
                out.write_bytes(&guest);
            }
        }
        out.write_u32(self.disconnected.len() as u32);
        for (guest, state) in &self.disconnected {
            out.write_bytes(guest);
            out.write_u8(u8::from(state.rules.is_some()));
            if let Some(rules) = &state.rules {
                rules.hash_into(out);
            }
            out.write_u8(u8::from(state.last_switch.is_some()));
            if let Some(tick) = state.last_switch {
                out.write_u32(tick.0);
            }
            out.write_u8(state.xp_bonus);
            out.write_u8(u8::from(state.muted));
        }
        out.write_u16(self.next_incident);
        out.write_u8(self.downed_by.len() as u8);
        for (&victim, &killer) in &self.downed_by {
            out.write_u8(victim);
            out.write_u8(killer);
        }
        out.write_u8(u8::from(self.map_vote_started));
        out.write_u8(u8::from(self.map_candidates.is_some()));
        for map in self.map_candidates.into_iter().flatten() {
            out.write_u16(map);
        }
        self.rules.hash_into(out);
    }
}

impl World {
    /// Authoritative match servers opt in; prediction and physics-only worlds do not run policy.
    pub fn enable_server_rules(&mut self, map_candidates: Option<[u16; 3]>) {
        self.server_rules = Some(Authority {
            rules: ServerRules::default(),
            slots: [0; 128],
            xp_bonus: [0; 128],
            guests: [None; 128],
            disconnected: BTreeMap::new(),
            downed_by: BTreeMap::new(),
            next_incident: 0,
            map_candidates,
            map_vote_started: false,
        });
    }

    pub fn admit_human(
        &mut self,
        player: PlayerId,
        team: Team,
        admin: bool,
        guest: Option<[u8; 8]>,
    ) -> Result<(), crate::team::balance::TeamError> {
        let deltas = self.squads.admit(player, team, self.tick())?;
        if let Some(r) = &mut self.server_rules {
            let slot = usize::from(player);
            r.slots[slot] = HUMAN | if admin { ADMIN } else { 0 };
            r.guests[slot] = guest;
            if let Some(state) = guest.and_then(|guest| r.disconnected.remove(&guest)) {
                if let Some(carry) = state.rules {
                    r.rules.restore_player_carry(player, carry);
                }
                self.squads.restore_last_switch(player, state.last_switch);
                r.xp_bonus[slot] = state.xp_bonus;
                if state.muted {
                    r.slots[slot] |= MUTED;
                }
            }
        }
        self.push_roster(deltas);
        Ok(())
    }

    pub fn rule_guest(&self, player: PlayerId) -> Option<[u8; 8]> {
        self.server_rules.as_ref()?.guests[usize::from(player)]
    }

    pub fn is_rule_muted(&self, p: PlayerId) -> bool {
        self.server_rules
            .as_ref()
            .is_some_and(|r| r.slots[usize::from(p)] & MUTED != 0)
    }

    /// Percentage points for the M2-017 XP consumer. Score and XP are not mutated by M2-018.
    pub fn rule_xp_bonus_percent(&self, p: PlayerId) -> u8 {
        self.server_rules
            .as_ref()
            .map_or(0, |r| r.xp_bonus[usize::from(p)])
    }

    pub fn rules_role_locked(&self, p: PlayerId, role: &str) -> bool {
        self.server_rules
            .as_ref()
            .and_then(|r| r.rules.role_lock(self.tick(), p))
            .is_some_and(|lock| !lock.allows(role))
    }

    pub fn rules_vehicle_locked(&self, p: PlayerId) -> bool {
        self.server_rules
            .as_ref()
            .is_some_and(|r| r.rules.vehicle_locked(self.tick(), p))
    }

    pub fn rules_bot_replacement(&self, team: Team) -> Option<PlayerId> {
        let r = self.server_rules.as_ref()?;
        let roster = self.rule_facts(r);
        r.rules
            .bot_replacements(&self.rule_context(&roster), team)
            .into_iter()
            .next()
    }

    pub fn rule_map_choice(&self, choice: u8) -> Option<u16> {
        let maps = self.server_rules.as_ref()?.map_candidates?;
        maps.get(usize::from(choice)).copied()
    }

    pub fn record_rule_activity(&mut self, p: PlayerId, activity: Activity) {
        let Some(r) = &self.server_rules else { return };
        let Some(fact) = self.rule_fact(r, p) else {
            return;
        };
        let facts = [fact];
        let ctx = self.rule_context(&facts);
        if let Some(r) = &mut self.server_rules {
            r.rules.record_activity(&ctx, p, activity);
        }
    }

    pub fn server_rule_command(
        &mut self,
        p: PlayerId,
        command: RuleCommand,
    ) -> Result<Vec<ServerAction>, RuleError> {
        let roster = self.rule_facts(self.server_rules.as_ref().ok_or(RuleError::Disabled)?);
        let ctx = self.rule_context(&roster);
        self.server_rules
            .as_mut()
            .expect("enabled")
            .rules
            .command(&ctx, p, command)
    }

    pub fn apply_rule_move(
        &mut self,
        player: PlayerId,
        team: Team,
        kind: MoveKind,
    ) -> Result<(), crate::team::balance::TeamError> {
        use crate::team::balance::TeamError;
        let entry = self.squads.entry(player).ok_or(TeamError::Unknown)?;
        if entry.team == team {
            return Err(TeamError::AlreadyOnRoster);
        }
        let deltas = self.squads.move_team(player, self.tick())?;
        self.push_roster(deltas);
        self.despawn_player_body(player);
        self.ecs
            .resource_mut::<crate::spawn::SpawnRules>()
            .choice
            .remove(&player);
        if let Some(r) = &mut self.server_rules {
            r.downed_by.remove(&player);
            if let MoveKind::Volunteer { xp_bonus_percent } = kind {
                r.xp_bonus[usize::from(player)] = xp_bonus_percent;
            }
        }
        Ok(())
    }

    pub fn apply_rule_mute(&mut self, p: PlayerId) {
        if let Some(r) = &mut self.server_rules {
            r.slots[usize::from(p)] |= MUTED;
        }
    }

    pub fn apply_afk_leader_transfer(&mut self, p: PlayerId) {
        let deltas = self.squads.relinquish_leader(p, self.tick());
        self.push_roster(deltas);
    }

    pub fn vacate_rule_seat(&mut self, p: PlayerId) {
        let Some((vehicle, seat)) = self.seat_of(p) else {
            return;
        };
        if let Some(v) = self
            .ecs
            .resource_mut::<crate::vehicle::Vehicles>()
            .list
            .get_mut(&vehicle)
        {
            let occupant = &mut v.seats[usize::from(seat)];
            if occupant.is_some_and(|o| o.player == p) {
                *occupant = None;
            }
        }
        let (_, body) = self.ecs.resource::<PlayerIndex>().map[&p];
        self.ecs
            .entity_mut(body)
            .insert(crate::vehicle::seats::InVehicle::default());
        self.pending_events.push(SimEvent::SeatChanged {
            player: p,
            vehicle,
            seat: None,
        });
    }

    fn rule_fact(&self, r: &Authority, player: PlayerId) -> Option<PlayerFacts> {
        let e = self.squads.entry(player).filter(|e| e.online)?;
        let squad = e.squad.and_then(|s| self.squads.squad(s));
        let seat = self.seat_of(player);
        let flags = r.slots[usize::from(player)];
        Some(PlayerFacts {
            id: player,
            team: e.team,
            is_bot: flags & HUMAN == 0,
            alive: matches!(
                self.player_life(player),
                Some(LifeState::Alive | LifeState::Downed { .. })
            ),
            in_vehicle: seat.is_some(),
            driving: seat
                .filter(|(_, seat)| *seat == 0)
                .map(|(vehicle, _)| vehicle),
            joined_tick: e.joined,
            last_switch_tick: e.last_switch,
            squad_size: squad.map_or(0, |s| s.members.len() as u8),
            is_squad_leader: squad.is_some_and(|s| s.leader == player),
            commander_since: None,
            is_officer: e.role == Some("officer") && !e.role_invalid,
            score: self.score.active_total(player).map_or(e.score, |score| {
                score.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
            }),
            is_admin: flags & ADMIN != 0,
            party: None,
        })
    }

    fn rule_facts(&self, r: &Authority) -> Vec<PlayerFacts> {
        self.squads
            .roster()
            .filter_map(|(p, _)| self.rule_fact(r, p))
            .collect()
    }

    fn rule_context<'a>(&self, roster: &'a [PlayerFacts]) -> RuleContext<'a> {
        let ms = self.match_state();
        RuleContext {
            now: self.tick(),
            match_started: if ms.phase == MatchPhaseKind::Idle {
                Tick(0)
            } else {
                Tick(ms.phase_start)
            },
            tick_hz: self.cfg.tick_hz,
            match_seed: self.cfg.match_seed,
            roster,
        }
    }

    pub(super) fn step_server_rules(
        &mut self,
        real_inputs: &[(PlayerId, InputCmd)],
        events: &[SimEvent],
    ) -> Vec<ServerAction> {
        let Some(r) = &self.server_rules else {
            return Vec::new();
        };
        let roster = self.rule_facts(r);
        let ctx = self.rule_context(&roster);
        let ended = self.match_state().phase == MatchPhaseKind::Ended;
        let r = self.server_rules.as_mut().expect("enabled");
        for &(player, cmd) in real_inputs {
            r.rules.record_activity(&ctx, player, Activity::Input(cmd));
        }
        let mut actions = Vec::new();
        for event in events {
            match *event {
                SimEvent::KillCredit {
                    victim,
                    killer: Some(offender),
                    kind,
                    team_kill: true,
                    ..
                } => {
                    let kind = match kind {
                        CreditKind::Down => {
                            r.downed_by.insert(victim, offender);
                            TkKind::Downed
                        }
                        CreditKind::Finish if r.downed_by.remove(&victim) == Some(offender) => {
                            TkKind::DeathAfterDown
                        }
                        _ => {
                            r.downed_by.remove(&victim);
                            TkKind::Killed
                        }
                    };
                    actions.extend(r.incident(&ctx, offender, victim, kind));
                }
                SimEvent::KillCredit {
                    victim,
                    kind: CreditKind::Direct | CreditKind::Finish,
                    ..
                }
                | SimEvent::Revive {
                    target_player: victim,
                    ..
                } => {
                    r.downed_by.remove(&victim);
                }
                SimEvent::Kill { victim, .. }
                    if r.downed_by.contains_key(&victim)
                        && !events.iter().any(|e| matches!(e, SimEvent::KillCredit { victim: v, kind: CreditKind::Direct | CreditKind::Finish, .. } if *v == victim)) =>
                {
                    // Bleed-out has no KillCredit. A finish is handled in the credit arm.
                    let offender = r.downed_by.remove(&victim).expect("incident checked");
                    actions.extend(r.incident(&ctx, offender, victim, TkKind::DeathAfterDown));
                }
                _ => {}
            }
        }
        if ended && !r.map_vote_started {
            r.map_vote_started = true;
            if let Some(maps) = r.map_candidates {
                actions.extend(r.rules.start_map_vote(&ctx, maps).unwrap_or_default());
            }
        }
        if !ended || r.map_candidates.is_some() {
            actions.extend(r.rules.step(&ctx));
        }
        actions
    }
}

impl Authority {
    fn incident(
        &mut self,
        ctx: &RuleContext<'_>,
        offender: PlayerId,
        victim: PlayerId,
        kind: TkKind,
    ) -> Vec<ServerAction> {
        let event_id = self.next_incident.into();
        self.next_incident = self.next_incident.wrapping_add(1);
        self.rules.report_tk(
            ctx,
            TkEvent {
                event_id,
                offender,
                victim,
                kind,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoring_follows_afk_and_keeps_the_ledger_on_team_move() {
        let ground = CollisionData::from_chunks(vec![crate::CollisionChunk::new(
            0,
            0,
            vec![5.0; 65 * 65],
            vec![0; 64 * 64],
        )]);
        let mut world = World::new(
            SimConfig {
                tick_hz: 30,
                match_seed: 5,
                map: MapId(0),
                mode: GameMode::Assault,
                data: GameData::embedded(),
            },
            &ground,
        );
        world.enable_server_rules(None);
        world.admit_human(0, Team::Us, false, None).unwrap();
        world
            .spawn_player(SpawnReq {
                player: 0,
                team: Team::Us,
                pos: Vec3::new(10.0, 5.02, 10.0),
                yaw: 0.0,
            })
            .unwrap();
        world.configure_match(0.0, 180.0);
        world.begin_warmup();
        world.step(&[]);

        let build = |world: &mut World| {
            world.pending_events.push(SimEvent::ScoreAction {
                player: 0,
                action: crate::score::Action::Build(10),
            });
            world
                .step(&[])
                .events
                .into_iter()
                .any(|e| matches!(e, SimEvent::ScoreDelta(_)))
        };
        assert!(build(&mut world));
        assert_eq!(
            world.rule_facts(world.server_rules.as_ref().unwrap())[0].score,
            1
        );
        for _ in 0..120 * 30 {
            world.step(&[]);
        }
        assert!(world.server_rules.as_ref().unwrap().is_afk(0));
        assert!(!build(&mut world));
        world.record_rule_activity(0, Activity::Interaction);
        assert!(build(&mut world));

        world
            .apply_rule_move(0, Team::Ger, MoveKind::Forced)
            .unwrap();
        world.step(&[]);
        let summary = world.score_summary();
        assert_eq!(summary.players.len(), 1);
        assert_eq!(summary.players[0].total_score, 2);
        assert_eq!(summary.players[0].team, Team::Ger as u8);
    }

    #[test]
    fn serverrules_reconnect_retains_identity_state_but_not_slot_state() {
        let mut world = World::new(
            SimConfig {
                tick_hz: 30,
                match_seed: 5,
                map: MapId(0),
                mode: GameMode::Sandbox,
                data: GameData::embedded(),
            },
            &CollisionData::empty(),
        );
        let guest = [11; 8];
        world.enable_server_rules(None);
        world.admit_human(4, Team::Us, true, Some(guest)).unwrap();
        world.apply_rule_mute(4);
        world
            .apply_rule_move(
                4,
                Team::Ger,
                MoveKind::Volunteer {
                    xp_bonus_percent: 20,
                },
            )
            .unwrap();
        let moved_at = world.tick();
        world.roster_remove(4);
        assert_eq!(world.rule_guest(4), None);
        world
            .admit_human(4, Team::Us, false, Some([12; 8]))
            .unwrap();
        assert!(!world.is_rule_muted(4));
        assert_eq!(world.rule_xp_bonus_percent(4), 0);
        assert_eq!(world.squads.entry(4).unwrap().last_switch, None);
        world.admit_human(9, Team::Ger, false, Some(guest)).unwrap();
        assert!(world.is_rule_muted(9));
        assert_eq!(world.rule_xp_bonus_percent(9), 20);
        assert_eq!(world.squads.entry(9).unwrap().last_switch, Some(moved_at));
        assert!(
            !world
                .rule_fact(world.server_rules.as_ref().unwrap(), 9)
                .unwrap()
                .is_admin
        );
        assert!(world.server_rules.as_ref().unwrap().disconnected.is_empty());
    }
}
