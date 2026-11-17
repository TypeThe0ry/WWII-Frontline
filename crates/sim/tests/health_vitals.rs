//! M1-016b: bandages, stabilising, bleeding floor, regeneration, executions, falls, crawling.
mod common;

use common::*;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::{LifeState, SimEvent, Team, World};
use glam::Vec3;

const A: u8 = 0;
const B: u8 = 1;
const HZ: u32 = 30;

fn arena() -> frontline_sim::CollisionData {
    synth_data(14, 2, &|_, _| 5.0)
}

fn idle() -> InputCmd {
    cmd(0, 0, 49152, 0, 0)
}

fn btn(b: u16) -> InputCmd {
    cmd(0, 0, 49152, b, 0)
}

fn world(p: &[(u8, Team, f32, f32, u16)]) -> World {
    duel_world(data_with(|_| {}), &arena(), 5, p)
}

/// Steps with `f(i)` as the command of each player until `done` or `max` ticks; returns the ticks.
fn run(
    w: &mut World,
    ids: &[u8],
    max: u32,
    f: &dyn Fn(u8) -> InputCmd,
    done: &dyn Fn(&World) -> bool,
) -> u32 {
    for i in 1..=max {
        let ins: Vec<_> = ids.iter().map(|&id| (id, f(id))).collect();
        w.step(&ins);
        if done(w) {
            return i;
        }
    }
    max
}

fn secs(s: f32) -> u32 {
    (s * HZ as f32).ceil() as u32
}

#[test]
fn self_bandage_takes_3_5_s_and_gives_25_hp() {
    let mut w = world(&[(A, Team::Ger, 20.0, 60.0, 16384)]);
    w.debug_set_hp(A, 30.0);
    let n = run(&mut w, &[A], 300, &|_| btn(buttons::USE), &|w| {
        w.health(A).unwrap() >= 55.0
    });
    assert!((secs(3.5) - 1..=secs(3.5) + 2).contains(&n), "{n}");
    assert_eq!(w.health(A), Some(55.0));
}

#[test]
fn self_bandage_caps_at_75() {
    let mut w = world(&[(A, Team::Ger, 20.0, 60.0, 16384)]);
    w.debug_set_hp(A, 65.0);
    run(&mut w, &[A], secs(4.0), &|_| btn(buttons::USE), &|_| false);
    assert_eq!(w.health(A), Some(75.0));
}

#[test]
fn medic_bandages_a_friend_in_2_5_s_for_40_hp() {
    let mut w = world(&[
        (A, Team::Ger, 20.0, 60.0, 16384),
        (B, Team::Ger, 20.5, 60.0, 16384),
    ]);
    w.set_medic(A, true);
    w.debug_set_hp(B, 20.0);
    let n = run(
        &mut w,
        &[A, B],
        300,
        &|id| if id == A { btn(buttons::USE) } else { idle() },
        &|w| w.health(B).unwrap() >= 60.0,
    );
    assert!((secs(2.5) - 1..=secs(2.5) + 2).contains(&n), "{n}");
    assert_eq!(w.health(B), Some(60.0));
}

#[test]
fn non_medic_stabilises_a_downed_friend_and_the_bleeding_stops() {
    let mut w = world(&[
        (A, Team::Ger, 20.0, 60.0, 16384),
        (B, Team::Ger, 20.5, 60.0, 16384),
    ]);
    w.debug_down(B);
    let hp0 = w.health(B).unwrap();
    let n = run(
        &mut w,
        &[A, B],
        300,
        &|id| if id == A { btn(buttons::USE) } else { idle() },
        &|w| {
            matches!(
                w.player_life(B),
                Some(LifeState::Downed { stable: true, .. })
            )
        },
    );
    assert!((secs(2.5) - 1..=secs(2.5) + 2).contains(&n), "{n}");
    let hp1 = w.health(B).unwrap();
    assert!(hp1 < hp0, "bled while waiting");
    run(&mut w, &[A, B], secs(10.0), &|_| idle(), &|_| false);
    assert_eq!(w.health(B), Some(hp1), "stable: no more bleeding");
}

#[test]
fn bleeding_stops_at_the_5_hp_floor_and_natural_regen_follows() {
    let mut w = world(&[(A, Team::Ger, 20.0, 60.0, 16384)]);
    w.debug_set_hp(A, 8.0);
    // Regen is delayed 10 s after the last damage (tick 0 here), up to 60 hp at 1 hp/s.
    run(&mut w, &[A], secs(9.0), &|_| idle(), &|_| false);
    assert_eq!(w.health(A), Some(8.0), "no regen inside the delay");
    run(&mut w, &[A], secs(13.0), &|_| idle(), &|_| false);
    let hp = w.health(A).unwrap();
    assert!(
        (19.0..=21.0).contains(&hp),
        "about +12 hp after 12 s of regen: {hp}"
    );
    run(&mut w, &[A], secs(90.0), &|_| idle(), &|_| false);
    assert_eq!(w.health(A), Some(60.0), "regen stops at 60");
}

#[test]
fn melee_execution_takes_1_2_s_and_the_attacker_stays_put() {
    let mut w = world(&[
        (A, Team::Ger, 20.0, 60.0, 16384),
        (B, Team::Us, 21.0, 60.0, 49152),
    ]);
    w.debug_down(B);
    let p0 = w.player_state(A).unwrap().pos;
    let mut ticks = 0;
    let mut killed = false;
    for i in 0..secs(3.0) {
        let b = if i == 0 { buttons::MELEE } else { 0 };
        let mz = if i < 12 { 0 } else { 127 };
        let ev = w.step(&[(A, cmd(0, mz, 16384, b, 0)), (B, idle())]).events;
        if ev
            .iter()
            .any(|e| matches!(e, SimEvent::Kill { victim: B, .. }))
        {
            killed = true;
            ticks = i + 1;
            break;
        }
    }
    assert!(killed, "executed");
    // The bayonet's 0.35 s windup (11 ticks, +1 for input latency) comes first.
    let exec = ticks - 12;
    assert!(
        (secs(1.2) - 1..=secs(1.2) + 1).contains(&exec),
        "{ticks} total, {exec} executing"
    );
    let p1 = w.player_state(A).unwrap().pos;
    assert!(
        (p1 - p0).length() < 0.01,
        "attacker rooted: {p0:?} -> {p1:?}"
    );
}

#[test]
fn execution_is_cancelled_when_the_attacker_is_hurt() {
    let mut w = world(&[
        (A, Team::Ger, 20.0, 60.0, 16384),
        (B, Team::Us, 21.0, 60.0, 49152),
    ]);
    w.debug_down(B);
    w.step(&[(A, cmd(0, 0, 16384, buttons::MELEE, 0)), (B, idle())]);
    for _ in 0..24 {
        w.step(&[(A, cmd(0, 0, 16384, 0, 0)), (B, idle())]);
    }
    w.debug_set_hp(A, 50.0);
    let mut killed = false;
    for _ in 0..secs(2.0) {
        let ev = w.step(&[(A, cmd(0, 0, 16384, 0, 0)), (B, idle())]).events;
        killed |= ev
            .iter()
            .any(|e| matches!(e, SimEvent::Kill { victim: B, .. }));
    }
    assert!(!killed);
    assert!(matches!(w.player_life(B), Some(LifeState::Downed { .. })));
}

fn fall_from(h: f32) -> (World, f32) {
    let mut w = world(&[(A, Team::Ger, 20.0, 60.0, 16384)]);
    w.debug_teleport(A, Vec3::new(20.0, 5.02 + h, 60.0));
    run(&mut w, &[A], secs(6.0), &|_| idle(), &|w| {
        w.health(A).unwrap() < 100.0 || w.player_life(A).map(|l| l.is_dead()).unwrap_or(false)
    });
    let hp = w.health(A).unwrap();
    (w, hp)
}

#[test]
fn fall_damage_is_12_hp_per_metre_beyond_3_m() {
    let (_, hp) = fall_from(2.5);
    assert_eq!(hp, 100.0, "3 m or less is free");
    let (_, hp) = fall_from(6.0);
    assert!((hp - 64.0).abs() < 1.0, "6 m: 36 damage, got {hp}");
}

#[test]
fn a_fall_of_12_m_is_lethal() {
    let (w, _) = fall_from(12.5);
    assert!(
        w.player_life(A).map(|l| l.is_dead()).unwrap_or(false),
        "{:?}",
        w.player_life(A)
    );
}

#[test]
fn downed_players_crawl_at_0_6_mps() {
    let mut w = world(&[(B, Team::Us, 30.0, 60.0, 49152)]);
    w.debug_down(B);
    for _ in 0..30 {
        w.step(&[(B, cmd(0, 127, 49152, buttons::SPRINT, 0))]);
    }
    let p0 = w.player_state(B).unwrap().pos;
    for _ in 0..60 {
        w.step(&[(B, cmd(0, 127, 49152, buttons::SPRINT, 0))]);
    }
    let p1 = w.player_state(B).unwrap().pos;
    let v = (p1 - p0).length() / 2.0;
    assert!((0.5..=0.65).contains(&v), "crawl speed {v}");
}
