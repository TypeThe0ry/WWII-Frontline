use super::*;

fn player(id: PlayerId, team: Team) -> PlayerFacts {
    PlayerFacts {
        id,
        team,
        is_bot: false,
        alive: true,
        in_vehicle: false,
        driving: None,
        joined_tick: Tick(0),
        last_switch_tick: None,
        squad_size: 0,
        is_squad_leader: false,
        commander_since: None,
        is_officer: false,
        score: 0,
        is_admin: false,
        party: None,
    }
}

fn ctx(roster: &[PlayerFacts], now: u32) -> RuleContext<'_> {
    RuleContext {
        roster,
        now: Tick(now),
        match_started: Tick(0),
        match_seed: 17,
        tick_hz: 10,
    }
}

fn hash(rules: &ServerRules) -> u64 {
    let mut w = HashWriter::new();
    rules.hash_into(&mut w);
    w.finish()
}

#[test]
fn admission_switch_and_bot_replacement() {
    let rules = ServerRules::default();
    let mut p = [
        player(0, Team::Us),
        player(1, Team::Us),
        player(2, Team::Ger),
    ];
    assert_eq!(rules.can_join(&ctx(&p, 0), Team::Us), Ok(()));
    assert_eq!(
        rules.can_join_party(&ctx(&p, 0), Team::Us, 2),
        Err(RuleError::TeamUnbalanced)
    );
    assert_eq!(
        rules.can_switch(&ctx(&p, 1799), 0),
        Err(RuleError::SwitchTooSoon)
    );
    assert_eq!(rules.can_switch(&ctx(&p, 1800), 0), Ok(()));
    assert_eq!(
        rules.can_switch(&ctx(&p, 1800), 2),
        Err(RuleError::TeamUnbalanced)
    );
    p[0].last_switch_tick = Some(Tick(100));
    assert_eq!(
        rules.can_switch(&ctx(&p, 6090), 0),
        Err(RuleError::SwitchCooldown)
    );
    assert_eq!(rules.can_switch(&ctx(&p, 6100), 0), Ok(()));
    p[0].party = Some(7);
    p[1].party = Some(7);
    assert_eq!(
        rules.can_switch(&ctx(&p, 6100), 0),
        Err(RuleError::PartyWouldSplit)
    );
    for bot in &mut p {
        bot.is_bot = true;
        bot.team = Team::Us;
    }
    p[0].in_vehicle = true;
    p[2].alive = false;
    assert_eq!(rules.bot_replacements(&ctx(&p, 0), Team::Us), vec![2, 1, 0]);
    let full: Vec<_> = (0..50).map(|id| player(id, Team::Us)).collect();
    assert_eq!(
        rules.can_join(&ctx(&full, 0), Team::Us),
        Err(RuleError::TeamFull)
    );
}

fn six() -> Vec<PlayerFacts> {
    (0..6)
        .map(|id| player(id, if id == 5 { Team::Ger } else { Team::Us }))
        .collect()
}

fn forced(actions: &[ServerAction]) -> Option<PlayerId> {
    actions.iter().find_map(|a| match a {
        ServerAction::Move {
            player,
            kind: MoveKind::Forced,
            ..
        } => Some(*player),
        _ => None,
    })
}

#[test]
fn balance_windows_and_candidate_order() {
    let run = |p: &[PlayerFacts]| {
        let mut rules = ServerRules::default();
        rules.step(&ctx(p, 0));
        assert!(!rules
            .step(&ctx(p, 890))
            .iter()
            .any(|a| matches!(a, ServerAction::InviteSwitch { .. })));
        assert!(rules.step(&ctx(p, 900)).iter().any(|a| matches!(
            a,
            ServerAction::InviteSwitch {
                from: Team::Us,
                to: Team::Ger,
                expires_at: Tick(1500),
                xp_bonus_percent: 20
            }
        )));
        assert_eq!(forced(&rules.step(&ctx(p, 1499))), None);
        let moved = forced(&rules.step(&ctx(p, 1500)));
        assert_eq!(forced(&rules.step(&ctx(p, 1500))), None);
        rules.step(&ctx(p, 2400));
        assert_ne!(forced(&rules.step(&ctx(p, 3000))), moved);
        moved
    };
    let mut p = six();
    assert_eq!(run(&p), Some(0));
    p[3].score = -30;
    assert_eq!(run(&p), Some(3));
    p[3].squad_size = 2;
    assert_eq!(run(&p), Some(0));
    p[3].joined_tick = Tick(1);
    assert_eq!(run(&p), Some(3));
    p[3].is_squad_leader = true;
    p[0].commander_since = Some(Tick(0));
    p[1].last_switch_tick = Some(Tick(601));
    assert_eq!(run(&p), Some(2));
    p[1].last_switch_tick = Some(Tick(600));
    assert_eq!(run(&p), Some(1));
}

#[test]
fn balance_offer_is_single_use_and_break_in_continuity_restarts_timer() {
    let mut p = six();
    let mut rules = ServerRules::default();
    rules.step(&ctx(&p, 1800));
    p[0].team = Team::Ger;
    rules.step(&ctx(&p, 2699));
    p[0].team = Team::Us;
    assert!(!rules
        .step(&ctx(&p, 2700))
        .iter()
        .any(|a| matches!(a, ServerAction::InviteSwitch { .. })));
    rules.step(&ctx(&p, 3600));
    assert!(rules.has_balance_offer(Team::Us, Tick(3600)));
    assert!(!rules.has_balance_offer(Team::Ger, Tick(3600)));
    let cmd = RuleCommand::SwitchTeam { accept_offer: true };
    assert_eq!(
        rules.command(&ctx(&p, 3600), 0, cmd),
        Ok(vec![ServerAction::Move {
            player: 0,
            to: Team::Ger,
            kind: MoveKind::Volunteer {
                xp_bonus_percent: 20
            },
            replace_bot: None
        }])
    );
    assert_eq!(
        rules.command(&ctx(&p, 3600), 1, cmd),
        Err(RuleError::NoBalanceOffer)
    );
    assert_eq!(forced(&rules.step(&ctx(&p, 4200))), None);
}

fn tk(rules: &mut ServerRules, c: &RuleContext<'_>, id: u32) -> Vec<ServerAction> {
    let event = TkEvent {
        offender: 0,
        victim: 1,
        event_id: id,
        kind: TkKind::Killed,
    };
    assert_eq!(
        rules.report_tk(c, event),
        vec![ServerAction::TkPrompt {
            event,
            expires_at: c.after(15)
        }]
    );
    rules
        .command(
            c,
            1,
            RuleCommand::RespondTk {
                event_id: id,
                decision: TkDecision::Punish,
            },
        )
        .unwrap()
}

#[test]
fn tk_escalates_and_uses_separate_match_and_rolling_totals() {
    let p = [player(0, Team::Us), player(1, Team::Us)];
    let mut rules = ServerRules::default();
    assert!(tk(&mut rules, &ctx(&p, 0), 1).is_empty());
    assert!(tk(&mut rules, &ctx(&p, 6600), 2).is_empty());
    assert_eq!(
        tk(&mut rules, &ctx(&p, 6601), 3),
        vec![
            ServerAction::Notice {
                player: 0,
                notice: Notice::TkWarning
            },
            ServerAction::LockRole {
                player: 0,
                lock: RoleLock::HighRisk,
                until: Tick(9601)
            }
        ]
    );
    let lock = rules.role_lock(Tick(6601), 0).unwrap();
    for role in ["machine_gunner", "anti_tank", "engineer", "crewman"] {
        assert!(!lock.allows(role));
    }
    assert!(lock.allows("rifleman"));
    assert_eq!(rules.role_lock(Tick(9601), 0), None);
    assert_eq!(
        tk(&mut rules, &ctx(&p, 6602), 4),
        vec![
            ServerAction::LockRole {
                player: 0,
                lock: RoleLock::RiflemanOnly,
                until: Tick(9602)
            },
            ServerAction::LockVehicle {
                player: 0,
                until: Tick(9602)
            }
        ]
    );
    assert!(rules.vehicle_locked(Tick(9601), 0));
    assert!(!rules.vehicle_locked(Tick(9602), 0));
    assert_eq!(
        tk(&mut rules, &ctx(&p, 13000), 5),
        vec![ServerAction::Kick {
            player: 0,
            reason: KickReason::TeamKill,
            cooldown_until: Some(Tick(31000))
        }]
    );
    assert_eq!(rules.tk_count(&ctx(&p, 18000), 0), 4);
    rules.remove_player(0);
    assert_eq!(rules.tk_count(&ctx(&p, 13000), 0), 0);
}

#[test]
fn tk_prompt_has_victim_and_expires_at_boundary() {
    let p = [
        player(0, Team::Us),
        player(1, Team::Us),
        player(2, Team::Us),
    ];
    let mut rules = ServerRules::default();
    let event = TkEvent {
        offender: 0,
        victim: 1,
        event_id: 5,
        kind: TkKind::Downed,
    };
    assert!(rules.report_tk(&ctx(&p, 1), event).is_empty());
    rules.report_tk(
        &ctx(&p, 3),
        TkEvent {
            kind: TkKind::DeathAfterDown,
            ..event
        },
    );
    assert_eq!(rules.tk_count(&ctx(&p, 3), 0), 1);
    let respond = RuleCommand::RespondTk {
        event_id: 5,
        decision: TkDecision::Forgive,
    };
    assert_eq!(
        rules.command(&ctx(&p, 152), 2, respond),
        Err(RuleError::TkPromptNotFound)
    );
    assert_eq!(
        rules.command(&ctx(&p, 153), 1, respond),
        Err(RuleError::TkPromptExpired)
    );
    assert_eq!(rules.command(&ctx(&p, 152), 1, respond), Ok(vec![]));
    assert_eq!(
        rules.command(&ctx(&p, 152), 1, respond),
        Err(RuleError::TkPromptNotFound)
    );
    rules.report_tk(
        &ctx(&p, 155),
        TkEvent {
            kind: TkKind::Killed,
            ..event
        },
    );
    rules.remove_player(1);
    assert_eq!(
        rules.command(&ctx(&p, 156), 1, respond),
        Err(RuleError::TkPromptNotFound)
    );
    assert_eq!(
        rules.report_tk(
            &ctx(&p, 157),
            TkEvent {
                kind: TkKind::CommanderFire,
                ..event
            }
        ),
        vec![ServerAction::Notice {
            player: 0,
            notice: Notice::CommanderFriendlyFire
        }]
    );
    assert_eq!(rules.tk_count(&ctx(&p, 157), 0), 2);
}

fn ids(n: u8) -> Vec<PlayerFacts> {
    (0..n).map(|id| player(id, Team::Us)).collect()
}

fn start(
    rules: &mut ServerRules,
    c: &RuleContext<'_>,
    player: PlayerId,
    request: VoteRequest,
) -> VoteSnapshot {
    let actions = rules
        .command(c, player, RuleCommand::StartVote(request))
        .unwrap();
    let ServerAction::VoteSnapshot(snapshot) = actions[0] else {
        panic!("expected vote snapshot")
    };
    snapshot
}

fn cast(vote: VoteId, ballot: Ballot) -> RuleCommand {
    RuleCommand::CastVote { vote, ballot }
}

fn outcome(actions: &[ServerAction]) -> VoteOutcome {
    actions
        .iter()
        .find_map(|a| match a {
            ServerAction::VoteResult { outcome, .. } => Some(*outcome),
            _ => None,
        })
        .unwrap()
}

#[test]
fn kick_vote_thresholds_concurrency_and_transport_reasons() {
    let mut p = ids(11);
    p[10].team = Team::Ger;
    p[6].is_admin = true;
    for (yes, no, passes) in [(5, 2, true), (5, 3, false), (2, 0, false), (4, 2, false)] {
        let mut rules = ServerRules::default();
        let c = ctx(&p, 1800);
        let request = RuleCommand::StartVote(VoteRequest::Kick(9));
        assert_eq!(
            rules.command(&ctx(&p, 1799), 0, request),
            Err(RuleError::VoteTooSoon)
        );
        assert_eq!(
            rules.command(&c, 0, RuleCommand::StartVote(VoteRequest::Kick(6))),
            Err(RuleError::ProtectedTarget)
        );
        let vote = start(&mut rules, &c, 0, VoteRequest::Kick(9));
        assert_eq!(vote.required_yes, 5);
        assert_eq!(
            rules.command(&c, 1, RuleCommand::StartVote(VoteRequest::Mute(9))),
            Err(RuleError::VoteBusy)
        );
        assert_eq!(
            rules.command(&c, 10, cast(vote.id, Ballot::Yes)),
            Err(RuleError::Ineligible)
        );
        for id in 0..yes + no {
            rules
                .command(
                    &c,
                    id,
                    cast(vote.id, if id < yes { Ballot::Yes } else { Ballot::No }),
                )
                .unwrap();
        }
        assert_eq!(
            rules.command(&c, 0, cast(vote.id, Ballot::No)),
            Err(RuleError::AlreadyVoted)
        );
        assert!(!rules
            .step(&ctx(&p, 2249))
            .iter()
            .any(|a| matches!(a, ServerAction::VoteResult { .. })));
        assert_eq!(
            rules.command(&ctx(&p, 2250), 8, cast(vote.id, Ballot::No)),
            Err(RuleError::VoteNotOpen)
        );
        let actions = rules.step(&ctx(&p, 2250));
        assert_eq!(
            outcome(&actions),
            if passes {
                VoteOutcome::Passed
            } else {
                VoteOutcome::Failed
            }
        );
        assert_eq!(
            actions.contains(&ServerAction::Kick {
                player: 9,
                reason: KickReason::Vote,
                cooldown_until: None
            }),
            passes
        );
        assert_eq!(
            rules.command(&ctx(&p, 4799), 0, request),
            Err(RuleError::VoteCooldown)
        );
        assert!(rules.command(&ctx(&p, 4800), 0, request).is_ok());
    }
    for yes in [2, 3] {
        let p = ids(4);
        let c = ctx(&p, 1800);
        let mut r = ServerRules::default();
        let v = start(&mut r, &c, 0, VoteRequest::Kick(3));
        for id in 0..yes {
            r.command(&c, id, cast(v.id, Ballot::Yes)).unwrap();
        }
        assert_eq!(
            outcome(&r.step(&ctx(&p, 2250))),
            if yes == 3 {
                VoteOutcome::Passed
            } else {
                VoteOutcome::Failed
            }
        );
    }
}

#[test]
fn votes_recompute_live_electorate_and_cancel_on_slot_reuse() {
    let mut p = ids(10);
    let c = ctx(&p, 1800);
    let mut rules = ServerRules::default();
    let v = start(&mut rules, &c, 0, VoteRequest::Kick(9));
    for id in 0..5 {
        rules.command(&c, id, cast(v.id, Ballot::Yes)).unwrap();
    }
    p[4].team = Team::Ger;
    p[5].team = Team::Ger;
    assert!(matches!(
        &rules.step(&ctx(&p, 1801))[0],
        ServerAction::VoteSnapshot(VoteSnapshot {
            yes: 4,
            required_yes: 4,
            ..
        })
    ));
    rules.remove_player(9);
    assert_eq!(outcome(&rules.step(&ctx(&p, 1801))), VoteOutcome::Cancelled);
    rules.remove_player(0);
    assert!(rules
        .command(
            &ctx(&p, 1802),
            0,
            RuleCommand::StartVote(VoteRequest::Kick(9))
        )
        .is_ok());
}

#[test]
fn commander_sponsors_mute_admin_and_surrender() {
    let mut p = ids(10);
    p[0].is_officer = true;
    p[1].is_officer = true;
    p[8].is_admin = true;
    p[9].commander_since = Some(Tick(2));
    for yes in [3, 4] {
        let mut r = ServerRules::default();
        let request = VoteRequest::RemoveCommander(9);
        let c = ctx(&p, 6002);
        assert_eq!(
            r.command(&ctx(&p, 6001), 0, RuleCommand::StartVote(request)),
            Err(RuleError::CommanderTenure)
        );
        assert_eq!(
            r.command(&c, 2, RuleCommand::StartVote(request)),
            Err(RuleError::OfficerOnly)
        );
        let v = start(&mut r, &c, 0, request);
        assert_eq!(v.stage, VoteStage::AwaitingSponsors { have: 1, need: 2 });
        assert_eq!(
            r.command(&c, 2, cast(v.id, Ballot::Yes)),
            Err(RuleError::VoteNotOpen)
        );
        assert_eq!(
            r.command(&c, 0, RuleCommand::StartVote(request)),
            Err(RuleError::AlreadyVoted)
        );
        assert_eq!(start(&mut r, &c, 1, request).yes, 2);
        for id in 2..yes {
            r.command(&c, id, cast(v.id, Ballot::Yes)).unwrap();
        }
        r.record_activity(&c, 9, Activity::Map);
        let actions = r.step(&ctx(&p, 6452));
        assert_eq!(
            actions.contains(&ServerAction::RelinquishCommander { player: 9 }),
            yes == 4
        );
    }
    for yes in [5, 6] {
        let mut r = ServerRules::default();
        let c = ctx(&p, 0);
        let v = start(&mut r, &c, 0, VoteRequest::Mute(9));
        for id in 0..yes {
            r.command(&c, id, cast(v.id, Ballot::Yes)).unwrap();
        }
        assert_eq!(
            r.command(&c, 8, RuleCommand::StartVote(VoteRequest::Mute(7))),
            Ok(vec![ServerAction::Mute { player: 7 }])
        );
        assert_eq!(
            r.command(&c, 0, RuleCommand::StartVote(VoteRequest::Surrender)),
            Err(RuleError::Disabled)
        );
        assert_eq!(
            r.step(&ctx(&p, 450))
                .contains(&ServerAction::Mute { player: 9 }),
            yes == 6
        );
    }
}

#[test]
fn map_vote_is_seeded_and_hashes_canonically_in_ballot_order() {
    let p = ids(3);
    let mut chosen = BTreeSet::new();
    for seed in 0..32 {
        let mut c = ctx(&p, 0);
        c.match_seed = seed;
        let mut a = ServerRules::default();
        assert_eq!(a.start_map_vote(&c, [1, 1, 2]), Err(RuleError::InvalidMaps));
        let started = a.start_map_vote(&c, [15, 18, 21]).unwrap();
        let ServerAction::VoteSnapshot(v) = started[0] else {
            panic!("expected snapshot")
        };
        assert_eq!(a.start_map_vote(&c, [1, 2, 3]), Err(RuleError::RoundEnded));
        assert_eq!(
            a.command(&c, 0, cast(v.id, Ballot::Map(7))),
            Err(RuleError::InvalidBallot)
        );
        let mut b = a.clone();
        for (id, map) in [(0, 15), (1, 18)] {
            a.command(&c, id, cast(v.id, Ballot::Map(map))).unwrap();
        }
        for (id, map) in [(1, 18), (0, 15)] {
            b.command(&c, id, cast(v.id, Ballot::Map(map))).unwrap();
        }
        assert_eq!(hash(&a), hash(&b));
        c.now = Tick(450);
        let result = a.step(&c);
        assert_eq!(result, b.step(&c));
        let VoteOutcome::MapSelected(map) = outcome(&result) else {
            panic!("expected map")
        };
        assert!([15, 18].contains(&map));
        chosen.insert(map);
    }
    assert_eq!(chosen.len(), 2);
}

fn notice(player: PlayerId, notice: Notice) -> ServerAction {
    ServerAction::Notice { player, notice }
}
fn afk_kick(player: PlayerId) -> ServerAction {
    ServerAction::Kick {
        player,
        reason: KickReason::Afk,
        cooldown_until: None,
    }
}

#[test]
fn afk_warning_countdown_leadership_driver_and_deploy_boundaries() {
    let mut p = ids(4);
    p[3].team = Team::Ger;
    p[0].in_vehicle = true;
    p[0].driving = Some(4);
    p[1].is_squad_leader = true;
    p[2].commander_since = Some(Tick(0));
    p[3].alive = false;
    let mut r = ServerRules::default();
    assert!(r.step(&ctx(&p, 899)).is_empty());
    assert_eq!(
        r.step(&ctx(&p, 900)),
        (0..3)
            .map(|id| notice(id, Notice::AfkWarning))
            .collect::<Vec<_>>()
    );
    assert!(r.step(&ctx(&p, 1190)).is_empty());
    assert_eq!(
        r.step(&ctx(&p, 1200)),
        vec![
            notice(
                0,
                Notice::AfkCountdown {
                    kick_at: Tick(1500)
                }
            ),
            ServerAction::DriverVacancy {
                player: 0,
                vehicle: 4
            }
        ]
    );
    assert!(r.step(&ctx(&p, 1499)).is_empty());
    assert_eq!(
        r.step(&ctx(&p, 1500)),
        vec![
            afk_kick(0),
            notice(
                1,
                Notice::AfkCountdown {
                    kick_at: Tick(1800)
                }
            ),
            ServerAction::RelinquishLeader { player: 1 },
            notice(
                2,
                Notice::AfkCountdown {
                    kick_at: Tick(1800)
                }
            ),
            ServerAction::RelinquishCommander { player: 2 }
        ]
    );
    p[1].is_squad_leader = false;
    p[2].commander_since = None;
    assert!(r.step(&ctx(&p, 1799)).is_empty());
    assert_eq!(r.step(&ctx(&p, 1800)), vec![afk_kick(1), afk_kick(2)]);
    assert!(r.step(&ctx(&p, 2999)).is_empty());
    assert_eq!(r.step(&ctx(&p, 3000)), vec![afk_kick(3)]);
}

#[test]
fn afk_cancel_on_input_or_death_and_rearm_after_redeploy() {
    let mut p = ids(1);
    let mut r = ServerRules::default();
    r.step(&ctx(&p, 1200));
    p[0].alive = false;
    assert_eq!(
        r.step(&ctx(&p, 1300)),
        vec![notice(0, Notice::AfkCountdownCancelled)]
    );
    assert!(!r.is_afk(0));
    p[0].alive = true;
    assert_eq!(
        r.step(&ctx(&p, 1400)),
        vec![notice(
            0,
            Notice::AfkCountdown {
                kick_at: Tick(1700)
            }
        )]
    );
    r.record_activity(&ctx(&p, 1500), 0, Activity::Map);
    assert_eq!(
        r.step(&ctx(&p, 1500)),
        vec![notice(0, Notice::AfkCountdownCancelled)]
    );
    assert!(r.step(&ctx(&p, 1700)).is_empty());
}

fn input(x: i8, z: i8, buttons: u16, yaw: u16) -> Activity {
    Activity::Input(InputCmd {
        seq: 0,
        tick: 0,
        move_x: x,
        move_z: z,
        yaw,
        pitch: 0,
        buttons,
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    })
}

#[test]
fn repeat_classifier_twenty_synthetic_traces() {
    let p = ids(1);
    for trace in 0..20 {
        let mut r = ServerRules::default();
        for sample in 0..32 {
            let motif = 2 + trace % 3;
            let x = if trace < 10 {
                10 + sample % motif
            } else if trace < 15 {
                7
            } else {
                10 + sample
            };
            r.record_activity(
                &ctx(&p, sample as u32 * 2),
                0,
                input(x, 0, 0, sample as u16),
            );
        }
        assert_eq!(r.repeated_input(0), trace < 10, "trace {trace}");
        if trace < 10 {
            let actions = r.step(&ctx(&p, 1200));
            assert!(actions.contains(&notice(0, Notice::RepeatedInput)));
            assert!(actions.contains(&notice(
                0,
                Notice::AfkCountdown {
                    kick_at: Tick(1500)
                }
            )));
            r.record_activity(&ctx(&p, 1201), 0, Activity::Chat);
            assert!(!r.repeated_input(0));
        }
    }
}

#[test]
fn wrapping_ticks_and_identical_replay() {
    let mut p = ids(1);
    p[0].joined_tick = Tick(u32::MAX - 600);
    let mut c = ctx(&p, 0);
    c.match_started = p[0].joined_tick;
    let mut a = ServerRules::default();
    let mut b = a.clone();
    for offset in [0, 1, 899, 900, 1199, 1200, 1499, 1500] {
        c.now = Tick(c.match_started.0.wrapping_add(offset));
        let actions = a.step(&c);
        assert_eq!(actions, b.step(&c));
        assert_eq!(hash(&a), hash(&b));
        if offset == 1500 {
            assert_eq!(actions, vec![afk_kick(0)]);
        }
    }
    assert_ne!(hash(&a), hash(&ServerRules::default()));
}

#[test]
fn round_rebalance_keeps_groups_and_reaches_best_headcount() {
    let rules = ServerRules::default();
    let groups = [
        RoundGroup {
            members: &[0, 1, 2, 3],
            score: 400,
        },
        RoundGroup {
            members: &[4, 5, 6],
            score: 200,
        },
        RoundGroup {
            members: &[7, 8, 9],
            score: 100,
        },
        RoundGroup {
            members: &[10, 11],
            score: 50,
        },
    ];
    let sides = rules.rebalance_next_round(&groups).unwrap();
    assert_eq!(sides.iter().filter(|s| s.1 == Team::Us).count(), 6);
    assert!(groups.iter().all(|g| g
        .members
        .iter()
        .all(|&id| sides[id as usize].1 == sides[g.members[0] as usize].1)));
    assert_eq!(
        rules.rebalance_next_round(&[groups[0], groups[0]]),
        Err(RuleError::InvalidTarget)
    );
}
