//! M1-018: capture formula, head count filters, regression, events.
mod common;

use common::*;
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::rules::{capture_rate, eff, CaptureParams};
use frontline_sim::{GameData, ObjectiveSpec, RuleChange, Team, World};
use glam::Vec3;

const HZ: f32 = 30.0;
const CENTER: (f32, f32) = (100.0, 100.0);

fn params() -> CaptureParams {
    CaptureParams::from_def(&GameData::embedded().rules.capture)
}

fn idle() -> InputCmd {
    cmd(0, 0, 49152, 0, 0)
}

/// `us` US and `ger` German players on the point (ids 0.. then 20..), one objective (id 1) at the
/// centre with the given owner.
fn world(us: u8, ger: u8, owner: Option<Team>) -> World {
    let mut p = Vec::new();
    for i in 0..us {
        p.push((
            i,
            Team::Us,
            CENTER.0 + f32::from(i) * 0.5,
            CENTER.1,
            16384u16,
        ));
    }
    for i in 0..ger {
        p.push((
            20 + i,
            Team::Ger,
            CENTER.0 + f32::from(i) * 0.5,
            CENTER.1 + 2.0,
            49152u16,
        ));
    }
    let mut w = duel_world(data_with(|_| {}), &synth_data(14, 2, &|_, _| 5.0), 5, &p);
    w.set_objectives(&[ObjectiveSpec {
        id: 1,
        pos: Vec3::new(CENTER.0, 5.0, CENTER.1),
        radius_m: 45.0,
        owner,
    }]);
    w
}

fn ids(w: &World) -> Vec<u8> {
    w.view().players().iter().map(|p| p.player).collect()
}

fn tick(w: &mut World) -> Vec<RuleChange> {
    let ins: Vec<_> = ids(w).into_iter().map(|i| (i, idle())).collect();
    w.step(&ins).rule_changes
}

#[test]
fn eff_values() {
    let v: Vec<f32> = (0..=12).map(eff).collect();
    assert_eq!(v[0], 0.0);
    assert_eq!(v[1], 1.0);
    assert_eq!(v[4], 4.0);
    assert_eq!(v[5], 4.5);
    assert_eq!(v[10], 7.0);
    assert_eq!(v[11], 7.0, "capped at 10");
    assert_eq!(v[12], 7.0);
}

#[test]
fn rates_match_the_design_examples() {
    let p = params();
    assert!((capture_rate(5, &p) - 0.9).abs() < 1e-6);
    assert!((capture_rate(-10, &p) + 1.4).abs() < 1e-6);
    assert_eq!(capture_rate(12, &p), capture_rate(10, &p));
    assert_eq!(capture_rate(0, &p), 0.0);
    // -100 -> +100: 10 net = 142.9 s, 1 net = 1000 s.
    assert!((200.0f32 / 1.4 - 142.857).abs() < 0.01);
    assert!((200.0 / capture_rate(1, &p) - 1000.0).abs() < 1e-3);
}

#[test]
fn capture_5v5_net_advantage_takes_3333_ticks() {
    let mut w = world(6, 1, None); // net 5 for the US
    let mut n = 0;
    while w.objective_list()[0].p < 100.0 && n < 5000 {
        tick(&mut w);
        n += 1;
    }
    println!("net 5: {n} ticks");
    assert!((3300..=3367).contains(&n), "{n}");
    assert_eq!(w.objective_list()[0].owner, Some(Team::Us));
}

#[test]
fn net_10_from_minus_100_takes_143_s() {
    let mut w = world(11, 1, Some(Team::Ger));
    let mut n = 0;
    while w.objective_list()[0].p < 100.0 && n < 6000 {
        tick(&mut w);
        n += 1;
    }
    let secs = n as f32 / HZ;
    println!("net 10: {secs:.1} s");
    assert!((141.5..=144.3).contains(&secs), "{secs}");
}

#[test]
fn stalemate_and_filters() {
    let mut w = world(3, 3, None);
    for _ in 0..200 {
        tick(&mut w);
    }
    assert_eq!(w.objective_list()[0].p, 0.0, "3 v 3");
    assert!(w.objective_list()[0].contested);
    // Downed players do not count.
    let mut w = world(3, 3, None);
    for g in 20..23 {
        w.debug_down(g);
    }
    for _ in 0..90 {
        tick(&mut w);
    }
    let p = w.objective_list()[0].p;
    assert!((p - 3.0 * 0.2 * 3.0).abs() < 0.05, "3 net for 3 s: {p}");
    // Dead (hp 0 and not alive) neither: only the living US count, the rest of the world is empty.
    let mut w = world(2, 0, None);
    w.debug_set_hp(0, 0.0);
    for _ in 0..30 {
        tick(&mut w);
    }
    assert!((w.objective_list()[0].p - 0.2).abs() < 0.02);
}

#[test]
fn height_tolerance_is_6_m() {
    for (dy, counted) in [(5.9f32, true), (6.1, false)] {
        let mut w = world(1, 0, None);
        let pos = w.player_state(0).unwrap().pos;
        w.debug_move_objective(1, Vec3::new(CENTER.0, pos.y - dy, CENTER.1));
        for _ in 0..30 {
            tick(&mut w);
        }
        let p = w.objective_list()[0].p;
        assert_eq!(p > 0.0, counted, "dy {dy}: p {p}");
    }
}

#[test]
fn regress_to_the_last_owner() {
    let mut w = world(0, 0, None);
    w.debug_set_objective(1, 60.0, Some(Team::Us), Some(Team::Us));
    for _ in 0..30 {
        w.step(&[]);
    }
    assert!(
        (w.objective_list()[0].p - 60.15).abs() < 0.003,
        "{}",
        w.objective_list()[0].p
    );
    // A neutral point that was never held does not move.
    let mut w = world(0, 0, None);
    w.debug_set_objective(1, 60.0, None, None);
    for _ in 0..90 {
        w.step(&[]);
    }
    assert_eq!(w.objective_list()[0].p, 60.0);
    // German last owner pulls towards -100.
    let mut w = world(0, 0, None);
    w.debug_set_objective(1, -20.0, None, Some(Team::Ger));
    for _ in 0..30 {
        w.step(&[]);
    }
    assert!((w.objective_list()[0].p + 20.15).abs() < 0.003);
}

#[test]
fn capture_event_fires_once_on_the_tick_p_reaches_100() {
    let mut w = world(10, 0, None);
    w.debug_set_objective(1, 99.0, None, None);
    let mut hits = Vec::new();
    for t in 0..400 {
        for c in tick(&mut w) {
            if let RuleChange::ObjectiveOwner {
                objective,
                owner,
                repeat,
                ..
            } = c
            {
                hits.push((t, objective, owner, repeat, w.objective_list()[0].p));
            }
        }
    }
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!((hits[0].1, hits[0].2, hits[0].3), (1, Team::Us, false));
    assert_eq!(hits[0].4, 100.0, "p is 100 on the event tick");
}

#[test]
fn recapture_within_90_s_is_flagged() {
    let mut w = world(10, 0, Some(Team::Ger));
    w.debug_set_objective(1, 99.0, Some(Team::Ger), Some(Team::Ger));
    let mut flags = Vec::new();
    for _ in 0..200 {
        for c in tick(&mut w) {
            if let RuleChange::ObjectiveOwner { repeat, .. } = c {
                flags.push(repeat);
            }
        }
    }
    assert_eq!(flags, vec![false]);
    // The Germans hold it again (reset), the US retake it within 90 s.
    w.debug_set_objective(1, 99.0, Some(Team::Ger), Some(Team::Ger));
    for _ in 0..200 {
        for c in tick(&mut w) {
            if let RuleChange::ObjectiveOwner { repeat, .. } = c {
                flags.push(repeat);
            }
        }
    }
    assert_eq!(
        flags,
        vec![false, true],
        "second change within 90 s is a repeat"
    );
}

#[test]
fn deterministic_capture() {
    let run = || {
        let mut w = world(4, 2, None);
        for _ in 0..600 {
            tick(&mut w);
        }
        w.state_hash()
    };
    assert_eq!(run(), run());
}
