//! M1-018: match phases and the simplified win rule.
mod common;

use common::*;
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::{MatchPhaseKind, ObjectiveSpec, RuleChange, Team, VictoryReason, World};
use glam::Vec3;

fn idle() -> InputCmd {
    cmd(0, 0, 49152, 0, 0)
}

/// Three points in a row: US L1 (id 1, x=60), neutral L2 (id 2, x=140), German L3 (id 3, x=220);
/// 3 US players camp on L2.
fn world() -> World {
    let players: Vec<_> = (0..3u8)
        .map(|i| (i, Team::Us, 140.0 + f32::from(i), 100.0, 16384u16))
        .collect();
    let mut w = duel_world(
        data_with(|_| {}),
        &synth_data(14, 2, &|_, _| 5.0),
        5,
        &players,
    );
    let spec = |id, x, owner| ObjectiveSpec {
        id,
        pos: Vec3::new(x, 5.0, 100.0),
        radius_m: 45.0,
        owner,
    };
    w.set_objectives(&[
        spec(1, 60.0, Some(Team::Us)),
        spec(2, 140.0, None),
        spec(3, 220.0, Some(Team::Ger)),
    ]);
    w
}

fn step(w: &mut World) -> Vec<RuleChange> {
    let ins: Vec<_> = w
        .view()
        .players()
        .iter()
        .map(|p| (p.player, idle()))
        .collect();
    w.step(&ins).rule_changes
}

#[test]
fn warmup_locks_capture_and_ends_after_60_s() {
    let mut w = world();
    w.begin_warmup();
    assert_eq!(w.match_state().phase, MatchPhaseKind::Warmup);
    let mut live_at = None;
    for t in 1..=2000u32 {
        for c in step(&mut w) {
            if let RuleChange::MatchPhase {
                phase: MatchPhaseKind::Live,
                ..
            } = c
            {
                live_at = Some(t);
            }
        }
        if live_at.is_none() {
            assert_eq!(
                w.objective_list()[1].p,
                0.0,
                "p frozen during warm-up (tick {t})"
            );
        }
    }
    let live_at = live_at.expect("live");
    assert!((1799..=1801).contains(&live_at), "{live_at}");
    assert!(w.objective_list()[1].p > 0.0, "capture runs once live");
}

#[test]
fn two_humans_start_the_match_early() {
    let mut w = world();
    w.begin_warmup();
    for _ in 0..10 {
        step(&mut w);
    }
    w.set_human_players(1);
    step(&mut w);
    assert_eq!(w.match_state().phase, MatchPhaseKind::Warmup);
    w.set_human_players(2);
    let ch = step(&mut w);
    assert_eq!(w.match_state().phase, MatchPhaseKind::Live);
    assert!(ch.iter().any(|c| matches!(c, RuleChange::MatchPhase { phase: MatchPhaseKind::Live, ends_at } if *ends_at > 1000)));
}

#[test]
fn holding_all_three_points_wins_at_once() {
    let mut w = world();
    w.configure_match(0.0, 600.0);
    w.begin_warmup();
    step(&mut w);
    assert_eq!(w.match_state().phase, MatchPhaseKind::Live);
    // The US own 1; give them 2 and 3.
    w.debug_set_objective(2, 99.9, None, None);
    w.debug_set_objective(3, 100.0, Some(Team::Us), Some(Team::Us));
    let mut ended = None;
    for _ in 0..20 {
        for c in step(&mut w) {
            if let RuleChange::MatchEnded { reason, winner } = c {
                ended = Some((reason, winner));
            }
        }
    }
    assert_eq!(ended, Some((VictoryReason::FullControl, Some(Team::Us))));
    assert_eq!(w.match_state().phase, MatchPhaseKind::Ended);
}

fn time_up(setup: &dyn Fn(&mut World)) -> Option<(VictoryReason, Option<Team>)> {
    let mut w = world();
    w.configure_match(0.0, 3.0);
    w.begin_warmup();
    step(&mut w);
    setup(&mut w);
    let mut ended = None;
    for _ in 0..200 {
        for c in step(&mut w) {
            if let RuleChange::MatchEnded { reason, winner } = c {
                ended = Some((reason, winner));
            }
        }
    }
    ended
}

#[test]
fn time_limit_compares_points_then_progress() {
    // The camping US players are not moved: L2 stays contested only by them (they capture it).
    // Points: US 1 + (L2 not yet), GER 1; US progress on L2 breaks the tie.
    let r = time_up(&|_| {});
    assert_eq!(
        r,
        Some((VictoryReason::TimeLimitLead, Some(Team::Us))),
        "p sum tie-break"
    );
    // US lose L1 to the Germans (2 v 1 on points).
    let r = time_up(&|w| {
        w.debug_set_objective(1, -100.0, Some(Team::Ger), Some(Team::Ger));
    });
    assert_eq!(r, Some((VictoryReason::TimeLimitLead, Some(Team::Ger))));
    // Full tie: all three neutral, no players -> draw.
    let mut w = world();
    for p in 0..3u8 {
        w.remove_player(p);
    }
    for id in 1..=3 {
        w.debug_set_objective(id, 0.0, None, None);
    }
    w.configure_match(0.0, 2.0);
    w.begin_warmup();
    let mut ended = None;
    for _ in 0..200 {
        for c in w.step(&[]).rule_changes {
            if let RuleChange::MatchEnded { reason, winner } = c {
                ended = Some((reason, winner));
            }
        }
    }
    assert_eq!(ended, Some((VictoryReason::Draw, None)));
}

#[test]
fn movement_is_frozen_for_3_s_after_the_end() {
    let mut w = world();
    w.configure_match(0.0, 600.0);
    w.begin_warmup();
    step(&mut w);
    w.debug_set_objective(2, 100.0, Some(Team::Us), Some(Team::Us));
    w.debug_set_objective(3, 100.0, Some(Team::Us), Some(Team::Us));
    step(&mut w);
    assert_eq!(w.match_state().phase, MatchPhaseKind::Ended);
    let p0 = w.player_state(0).unwrap().pos;
    for _ in 0..60 {
        w.step(&[(0, cmd(0, 127, 16384, 0, 0))]);
    }
    let moved = (w.player_state(0).unwrap().pos - p0).length();
    assert!(moved < 0.01, "frozen: {moved}");
    for _ in 0..60 {
        w.step(&[(0, cmd(0, 127, 16384, 0, 0))]);
    }
    assert!(
        (w.player_state(0).unwrap().pos - p0).length() > 1.0,
        "free after 3 s"
    );
}

#[test]
fn deterministic_match() {
    let run = || {
        let mut w = world();
        w.configure_match(1.0, 20.0);
        w.begin_warmup();
        for _ in 0..1000 {
            step(&mut w);
        }
        w.state_hash()
    };
    assert_eq!(run(), run());
}
