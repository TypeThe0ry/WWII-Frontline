//! M1-021a: squad operations, rate limits, bans, succession, quotas, balance.
use frontline_sim::roles::quota::RoleError;
use frontline_sim::squad::{Action, RosterDelta, SquadError, SquadKind, SquadManager, SquadOp};
use frontline_sim::team::balance::TeamError;
use frontline_sim::{GameData, PlayerId, Team, Tick};

const HZ: u8 = 30;
const MIN: u32 = 60 * HZ as u32;

fn mgr() -> SquadManager {
    SquadManager::new(GameData::embedded(), HZ)
}

fn t(n: u32) -> Tick {
    Tick(n)
}

fn add_us(m: &mut SquadManager, p: PlayerId) {
    // Keep the teams level so the balance rule never blocks the setup.
    m.roster_add(p, Team::Us, t(0)).unwrap();
    m.roster_add(p + 100, Team::Ger, t(0)).unwrap();
}

fn infantry() -> SquadOp {
    SquadOp::Create {
        kind: SquadKind::Infantry,
        suffix: String::new(),
    }
}

fn squad_of(m: &SquadManager, p: PlayerId) -> u16 {
    m.squad_of(p).unwrap().id
}

#[test]
fn create_makes_leader_and_names() {
    let mut m = mgr();
    add_us(&mut m, 0);
    add_us(&mut m, 1);
    m.apply(0, infantry(), t(0)).unwrap();
    m.apply(
        1,
        SquadOp::Create {
            kind: SquadKind::Infantry,
            suffix: "Hawks".into(),
        },
        t(1),
    )
    .unwrap();
    assert_eq!(m.squad_of(0).unwrap().leader, 0);
    assert_eq!(m.squad_of(0).unwrap().name(), "Alpha");
    assert_eq!(m.squad_of(1).unwrap().name(), "Bravo Hawks");
    assert_eq!(
        m.apply(0, infantry(), t(2)),
        Err(SquadError::AlreadyInSquad)
    );
}

#[test]
fn squad_size_caps_r2() {
    let mut m = mgr();
    for p in 0..7 {
        add_us(&mut m, p);
    }
    m.apply(0, infantry(), t(0)).unwrap();
    let s = squad_of(&m, 0);
    for p in 1..6 {
        m.apply(p, SquadOp::Join { squad: s }, t(1)).unwrap();
    }
    assert_eq!(m.squad(s).unwrap().size(), 6);
    assert_eq!(
        m.apply(6, SquadOp::Join { squad: s }, t(2)),
        Err(SquadError::SquadFull)
    );
}

#[test]
fn armor_and_recon_squad_limits() {
    let mut m = mgr();
    for p in 0..6 {
        add_us(&mut m, p);
    }
    let armor = SquadOp::Create {
        kind: SquadKind::ArmorCrew,
        suffix: String::new(),
    };
    // One create per player here, so the rate limit stays out of the way.
    for p in 0..4 {
        m.apply(p, armor.clone(), t(0)).unwrap();
    }
    assert_eq!(m.apply(4, armor, t(0)), Err(SquadError::KindFull));
    assert_eq!(m.squad_of(3).unwrap().name(), "Armor-4");
    let recon = SquadOp::Create {
        kind: SquadKind::Recon,
        suffix: String::new(),
    };
    m.apply(4, recon.clone(), t(0)).unwrap();
    m.apply(5, recon.clone(), t(0)).unwrap();
    let mut m2 = mgr();
    for p in 0..3 {
        add_us(&mut m2, p);
    }
    m2.apply(0, recon.clone(), t(0)).unwrap();
    m2.apply(1, recon.clone(), t(0)).unwrap();
    assert_eq!(m2.apply(2, recon, t(0)), Err(SquadError::KindFull));
    // Armor crews take at most 3, recon at most 2.
    let s = squad_of(&m2, 0);
    m2.apply(2, SquadOp::Join { squad: s }, t(1)).unwrap();
    assert_eq!(m2.squad(s).unwrap().size(), 2);
}

#[test]
fn team_cap_is_fifty() {
    let mut m = mgr();
    for p in 0..50u8 {
        add_us(&mut m, p);
    }
    assert_eq!(m.team_count(Team::Us), 50);
    assert_eq!(m.roster_add(77, Team::Us, t(0)), Err(TeamError::Full));
}

#[test]
fn create_rate_limit_is_tick_exact() {
    let mut m = mgr();
    add_us(&mut m, 0);
    let leave = SquadOp::Leave;
    m.apply(0, infantry(), t(0)).unwrap();
    m.apply(0, leave.clone(), t(10)).unwrap();
    m.apply(0, infantry(), t(20)).unwrap();
    m.apply(0, leave.clone(), t(30)).unwrap();
    // Third create inside the minute is refused, one tick before the first slot frees up...
    assert_eq!(
        m.apply(0, infantry(), t(MIN - 1)),
        Err(SquadError::RateLimited)
    );
    // ...and allowed once the first create is a full 60 s old.
    m.apply(0, infantry(), t(MIN)).unwrap();
}

#[test]
fn kick_rate_ban_and_rejoin() {
    let mut m = mgr();
    for p in 0..6 {
        add_us(&mut m, p);
    }
    m.apply(0, infantry(), t(0)).unwrap();
    let s = squad_of(&m, 0);
    for p in 1..6 {
        m.apply(p, SquadOp::Join { squad: s }, t(0)).unwrap();
    }
    assert_eq!(
        m.apply(1, SquadOp::Kick { target: 2 }, t(0)),
        Err(SquadError::NotLeader)
    );
    assert_eq!(
        m.apply(0, SquadOp::Kick { target: 0 }, t(0)),
        Err(SquadError::CannotTargetSelf)
    );
    for p in 1..4 {
        m.apply(0, SquadOp::Kick { target: p }, t(100)).unwrap();
    }
    assert_eq!(
        m.apply(0, SquadOp::Kick { target: 4 }, t(100)),
        Err(SquadError::RateLimited)
    );
    // Kicked at tick 100: banned for 9000 ticks (5 min).
    let until = 100 + 300 * HZ as u32;
    assert_eq!(
        m.apply(1, SquadOp::Join { squad: s }, t(until - 1)),
        Err(SquadError::Banned)
    );
    m.apply(1, SquadOp::Join { squad: s }, t(until)).unwrap();
    // A different squad is not affected by the ban.
    m.apply(2, infantry(), t(until)).unwrap();
}

#[test]
fn lock_invite_and_leader_leaving_unlocks() {
    let mut m = mgr();
    for p in 0..3 {
        add_us(&mut m, p);
    }
    m.apply(0, infantry(), t(0)).unwrap();
    let s = squad_of(&m, 0);
    m.apply(0, SquadOp::Lock, t(1)).unwrap();
    assert_eq!(
        m.apply(1, SquadOp::Join { squad: s }, t(2)),
        Err(SquadError::Locked)
    );
    m.apply(0, SquadOp::Invite { target: 1 }, t(2)).unwrap();
    m.apply(1, SquadOp::Join { squad: s }, t(3)).unwrap();
    assert!(m.squad(s).unwrap().locked);
    m.apply(0, SquadOp::Leave, t(4)).unwrap();
    let sq = m.squad(s).unwrap();
    assert_eq!(sq.leader, 1);
    assert!(!sq.locked, "a new leader inherits an unlocked squad");
    m.apply(2, SquadOp::Join { squad: s }, t(5)).unwrap();
}

#[test]
fn active_handover_needs_confirmation_in_ten_seconds() {
    let mut m = mgr();
    for p in 0..2 {
        add_us(&mut m, p);
    }
    m.apply(0, infantry(), t(0)).unwrap();
    let s = squad_of(&m, 0);
    m.apply(1, SquadOp::Join { squad: s }, t(0)).unwrap();
    m.apply(0, SquadOp::Promote { target: 1 }, t(100)).unwrap();
    assert_eq!(
        m.apply(0, SquadOp::AcceptLead, t(101)),
        Err(SquadError::NoHandover)
    );
    let late = 100 + 10 * HZ as u32;
    assert_eq!(
        m.apply(1, SquadOp::AcceptLead, t(late)),
        Err(SquadError::HandoverExpired)
    );
    m.apply(0, SquadOp::Promote { target: 1 }, t(200)).unwrap();
    let out = m
        .apply(1, SquadOp::AcceptLead, t(200 + late - 101))
        .unwrap();
    assert_eq!(
        out,
        vec![RosterDelta::LeaderChanged {
            squad: s,
            leader: 1
        }]
    );
    assert_eq!(m.squad(s).unwrap().leader, 1);
}

fn four_squad(m: &mut SquadManager) -> u16 {
    for p in 0..4 {
        add_us(m, p);
    }
    m.apply(0, infantry(), t(0)).unwrap();
    let s = squad_of(m, 0);
    for p in 1..4 {
        m.apply(p, SquadOp::Join { squad: s }, t(10 + p as u32))
            .unwrap();
    }
    s
}

#[test]
fn succession_falls_back_to_highest_score() {
    let mut m = mgr();
    let s = four_squad(&mut m);
    m.set_score(1, 10);
    m.set_score(3, 900);
    m.apply(0, SquadOp::Leave, t(30)).unwrap();
    assert_eq!(m.squad(s).unwrap().leader, 3);
}

#[test]
fn succession_officer_beats_score_then_earliest_join() {
    let mut m = mgr();
    let s = four_squad(&mut m);
    m.set_score(1, 500);
    m.set_score(2, 10);
    m.set_score(3, 10);
    // Player 2 is made Officer (requires the lead for a moment).
    m.apply(0, SquadOp::Promote { target: 2 }, t(30)).unwrap();
    m.apply(2, SquadOp::AcceptLead, t(31)).unwrap();
    m.set_role(2, Some("officer"), t(32)).unwrap();
    m.apply(2, SquadOp::Promote { target: 0 }, t(33)).unwrap();
    m.apply(0, SquadOp::AcceptLead, t(34)).unwrap();
    m.apply(0, SquadOp::Leave, t(35)).unwrap();
    assert_eq!(
        m.squad(s).unwrap().leader,
        2,
        "Officer outranks the higher score"
    );
    // Equal scores: the earliest joiner.
    let mut m = mgr();
    let s = four_squad(&mut m);
    m.apply(0, SquadOp::Leave, t(40)).unwrap();
    assert_eq!(m.squad(s).unwrap().leader, 1);
}

#[test]
fn last_member_leaving_disbands() {
    let mut m = mgr();
    add_us(&mut m, 0);
    m.apply(0, infantry(), t(0)).unwrap();
    let s = squad_of(&m, 0);
    let out = m.apply(0, SquadOp::Leave, t(1)).unwrap();
    assert!(out.contains(&RosterDelta::SquadDisbanded { squad: s }));
    assert!(m.squad(s).is_none());
}

#[test]
fn offline_leader_replaced_after_120s() {
    let mut m = mgr();
    let s = four_squad(&mut m);
    m.set_online(0, false);
    assert!(m.update(t(100)).is_empty());
    let limit = 100 + 120 * HZ as u32;
    assert!(m.update(t(limit - 1)).is_empty());
    let out = m.update(t(limit));
    assert_eq!(
        out,
        vec![RosterDelta::LeaderChanged {
            squad: s,
            leader: 1
        }]
    );
    // A dead leader counts as absent too, until the player is back in action.
    m.set_dead_since(1, Some(t(limit)));
    assert!(m.update(t(limit + 10)).is_empty());
    m.set_dead_since(1, None);
    assert!(m.update(t(limit + 20)).is_empty());
    assert_eq!(m.squad(s).unwrap().leader, 1);
}

fn role_squad(n: u8) -> (SquadManager, u16) {
    let mut m = mgr();
    for p in 0..n {
        add_us(&mut m, p);
    }
    m.apply(0, infantry(), t(0)).unwrap();
    let s = squad_of(&m, 0);
    for p in 1..n {
        m.apply(p, SquadOp::Join { squad: s }, t(1)).unwrap();
    }
    (m, s)
}

#[test]
fn role_quotas_per_squad() {
    let (mut m, _) = role_squad(6);
    // Officer: leader only.
    assert_eq!(
        m.set_role(1, Some("officer"), t(2)),
        Err(RoleError::LeaderOnly)
    );
    m.set_role(0, Some("officer"), t(2)).unwrap();
    // Medic x2, third refused.
    m.set_role(1, Some("medic"), t(2)).unwrap();
    m.set_role(2, Some("medic"), t(2)).unwrap();
    assert_eq!(m.set_role(3, Some("medic"), t(2)), Err(RoleError::Full));
    // Support x1.
    m.set_role(3, Some("support"), t(2)).unwrap();
    assert_eq!(m.set_role(4, Some("support"), t(2)), Err(RoleError::Full));
    // Crewman only in armor.
    assert_eq!(
        m.set_role(4, Some("crewman"), t(2)),
        Err(RoleError::NotInArmor)
    );
    assert_eq!(
        m.set_role(4, Some("nope"), t(2)),
        Err(RoleError::UnknownRole)
    );
    m.set_role(4, Some("rifleman"), t(2)).unwrap();
}

#[test]
fn role_needs_squad_size_and_unassigned_is_rifleman_only() {
    let (mut m, _) = role_squad(2);
    assert_eq!(
        m.set_role(1, Some("support"), t(2)),
        Err(RoleError::SquadTooSmall)
    );
    m.set_role(1, Some("medic"), t(2)).unwrap();
    let mut lone = mgr();
    add_us(&mut lone, 9);
    assert_eq!(
        lone.set_role(9, Some("medic"), t(0)),
        Err(RoleError::Unassigned)
    );
    assert_eq!(
        lone.set_role(9, Some("officer"), t(0)),
        Err(RoleError::Unassigned)
    );
    lone.set_role(9, Some("rifleman"), t(0)).unwrap();
}

#[test]
fn armor_crew_takes_crewmen_only_max_two() {
    let mut m = mgr();
    for p in 0..3 {
        add_us(&mut m, p);
    }
    m.apply(
        0,
        SquadOp::Create {
            kind: SquadKind::ArmorCrew,
            suffix: String::new(),
        },
        t(0),
    )
    .unwrap();
    let s = squad_of(&m, 0);
    m.apply(1, SquadOp::Join { squad: s }, t(1)).unwrap();
    m.apply(2, SquadOp::Join { squad: s }, t(1)).unwrap();
    m.set_role(0, Some("crewman"), t(2)).unwrap();
    m.set_role(1, Some("crewman"), t(2)).unwrap();
    assert_eq!(m.set_role(2, Some("crewman"), t(2)), Err(RoleError::Full));
    assert_eq!(
        m.set_role(2, Some("rifleman"), t(2)),
        Err(RoleError::WrongSquadKind)
    );
}

#[test]
fn team_wide_cap_is_hard() {
    // Medic: 12 per team, 2 per squad -> 6 squads of 2 fill it.
    let mut m = mgr();
    let mut next = 0u8;
    for _ in 0..7 {
        let a = next;
        let b = next + 1;
        next += 2;
        for p in [a, b] {
            add_us(&mut m, p);
        }
        m.apply(a, infantry(), t(0)).unwrap();
        let s = squad_of(&m, a);
        m.apply(b, SquadOp::Join { squad: s }, t(0)).unwrap();
        if next <= 12 {
            m.set_role(a, Some("medic"), t(1)).unwrap();
            m.set_role(b, Some("medic"), t(1)).unwrap();
        } else {
            assert_eq!(m.set_role(a, Some("medic"), t(1)), Err(RoleError::TeamFull));
        }
    }
}

#[test]
fn shrinking_squad_flags_role_for_next_deploy_only() {
    let (mut m, _) = role_squad(6);
    for p in 1..4 {
        m.set_role(p, Some("rifleman"), t(2)).unwrap();
    }
    m.set_role(4, Some("support"), t(2)).unwrap();
    for p in [1, 2, 3, 5] {
        m.apply(p, SquadOp::Leave, t(3)).unwrap_or_default();
    }
    // Squad of {0, 4}: Support needs 3.
    let e = m.entry(4).unwrap();
    assert_eq!(e.role, Some("support"), "living players keep their role");
    assert!(e.role_invalid);
    assert_eq!(
        m.role_available(4, "support"),
        Err(RoleError::SquadTooSmall)
    );
}

#[test]
fn permissions() {
    let (mut m, _) = role_squad(2);
    assert!(m.can(0, Action::PlaceSquadMarker));
    assert!(!m.can(1, Action::PlaceSquadMarker));
    assert!(!m.can(0, Action::PlaceGarrison));
    m.set_role(0, Some("officer"), t(1)).unwrap();
    assert!(m.can(0, Action::PlaceGarrison));
    assert!(!m.can(1, Action::Kick));
}

#[test]
fn team_balance_and_switching() {
    let mut m = mgr();
    // 0 vs 0 -> 1 vs 0 -> 2 vs 0 ok; third on the same side is refused.
    m.roster_add(0, Team::Us, t(0)).unwrap();
    m.roster_add(1, Team::Us, t(0)).unwrap();
    assert_eq!(m.roster_add(2, Team::Us, t(0)), Err(TeamError::Unbalanced));
    m.roster_add(10, Team::Ger, t(0)).unwrap();
    m.roster_add(2, Team::Us, t(0)).unwrap();
    assert_eq!(m.roster_add(3, Team::Us, t(0)), Err(TeamError::Unbalanced));
    // Switching: 3 minutes after joining, never twice within 10 minutes.
    m.roster_add(11, Team::Ger, t(0)).unwrap();
    m.roster_add(12, Team::Ger, t(0)).unwrap();
    m.roster_add(13, Team::Ger, t(0)).unwrap();
    let three = 180 * HZ as u32;
    // 2 (US) -> Ger: counts are 3 v 4, so the move is balanced.
    assert_eq!(m.can_switch_team(13, t(three - 1)), Err(TeamError::TooSoon));
    // Ger 13 -> US leaves 3 v 4 -> 4 v 3: fine.
    m.switch_team(13, t(three)).unwrap();
    assert_eq!(m.entry(13).unwrap().team, Team::Us);
    let ten = 600 * HZ as u32;
    assert_eq!(
        m.can_switch_team(13, t(three + ten - 1)),
        Err(TeamError::SwitchCooldown)
    );
    // Counts are now US 4 v GER 3; 13 moving back would be 3 v 4: balanced, allowed at +10 min.
    assert_eq!(m.can_switch_team(13, t(three + ten)), Ok(()));
}

#[test]
fn deterministic_hash_tracks_roster() {
    use frontline_sim::hash::HashWriter;
    let run = || {
        let (mut m, _) = role_squad(4);
        m.set_role(0, Some("officer"), t(2)).unwrap();
        let mut h = HashWriter::new();
        m.hash_into(&mut h);
        h.finish()
    };
    assert_eq!(run(), run());
    let (m, _) = role_squad(4);
    let mut h = HashWriter::new();
    m.hash_into(&mut h);
    assert_ne!(h.finish(), run());
}
