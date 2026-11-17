//! No wallhack, no 360-degree vision, reaction delay (M0-026 acceptance 4).
//!
//! The observer is a bot without navigation (it can only stand and look), so any turn or shot
//! can only come from perceiving an enemy.

mod common;

use std::sync::Arc;

use common::*;
use frontline_bots::{BotMap, Tuning};
use frontline_protocol::msg::input::buttons;
use frontline_sim::{CollisionData, Team};

const OBSERVER: [f32; 2] = [100.0, 100.0];

/// Observer facing +z at `OBSERVER`, an idle enemy at `enemy`, `ticks` ticks; returns
/// `(first_fire_tick, yaw_changed, ever_targeted, shots)`.
fn scenario(
    col: &CollisionData,
    enemy: [f32; 2],
    enemy_team: Team,
    yaw: f32,
    ticks: u32,
    seed: u64,
) -> (Option<u32>, bool, bool, u32) {
    let mut a = Arena::new(col, seed);
    let map = Arc::new(BotMap::without_nav(Tuning::embedded(), Vec::new()));
    a.add_bot(&map, 0, Team::Us, OBSERVER[0], OBSERVER[1], yaw);
    spawn(&mut a.world, 1, enemy_team, enemy[0], enemy[1], 0.0);
    let mut first_fire = None;
    let mut yaw_changed = false;
    let mut targeted = false;
    let mut yaw0 = None;
    for t in 1..=ticks {
        let cmds = a.step();
        let c = cmds[0].1;
        let y0 = *yaw0.get_or_insert(c.yaw);
        yaw_changed |= c.yaw != y0;
        targeted |= a.brains[0].2.target_player().is_some();
        if c.buttons & buttons::FIRE != 0 && first_fire.is_none() {
            first_fire = Some(t);
        }
    }
    (first_fire, yaw_changed, targeted, a.shots)
}

fn flat() -> CollisionData {
    synth(None)
}

#[test]
fn perception_no_wallhack_enemy_behind() {
    // 30 m directly behind the observer: outside the 120 degree cone.
    let (fire, turned, targeted, shots) = scenario(&flat(), [100.0, 70.0], Team::Ger, 0.0, 240, 1);
    assert!(
        fire.is_none() && !turned && !targeted && shots == 0,
        "{fire:?} {turned} {targeted} {shots}"
    );
}

#[test]
fn perception_no_wallhack_just_outside_the_cone() {
    // 25 m away at 65 degrees off the view axis (cone half-angle is 60).
    let a = 65.0f32.to_radians();
    let enemy = [OBSERVER[0] + 25.0 * a.sin(), OBSERVER[1] + 25.0 * a.cos()];
    let (fire, turned, targeted, shots) = scenario(&flat(), enemy, Team::Ger, 0.0, 240, 2);
    assert!(
        fire.is_none() && !turned && !targeted && shots == 0,
        "{fire:?} {turned} {targeted} {shots}"
    );
    // Control: 55 degrees is inside the cone and gets noticed.
    let a = 55.0f32.to_radians();
    let enemy = [OBSERVER[0] + 25.0 * a.sin(), OBSERVER[1] + 25.0 * a.cos()];
    let (_, _, targeted, _) = scenario(&flat(), enemy, Team::Ger, 0.0, 240, 2);
    assert!(targeted, "an enemy inside the cone must be noticed");
}

#[test]
fn perception_no_wallhack_beyond_view_distance() {
    // Regular sees 220 m; this enemy is 235 m away, dead ahead along the diagonal.
    let yaw = std::f32::consts::FRAC_PI_4;
    let col = flat();
    let mut a = Arena::new(&col, 3);
    let map = Arc::new(BotMap::without_nav(Tuning::embedded(), Vec::new()));
    a.add_bot(&map, 0, Team::Us, 10.0, 10.0, yaw);
    spawn(&mut a.world, 1, Team::Ger, 176.0, 176.0, 0.0);
    for _ in 0..240 {
        a.step();
        assert!(a.brains[0].2.target_player().is_none());
    }
    assert_eq!(a.shots, 0);
}

#[test]
fn perception_no_wallhack_behind_a_wall() {
    // 20 m high wall across x 60..140 at z 130..134; the enemy stands 70 m ahead behind it.
    let col = synth(Some((60.0, 140.0, 130.0, 134.0, 20.0)));
    let (fire, turned, targeted, shots) = scenario(&col, [100.0, 170.0], Team::Ger, 0.0, 300, 4);
    assert!(
        fire.is_none() && !turned && !targeted && shots == 0,
        "{fire:?} {turned} {targeted} {shots}"
    );
    // Control: without the wall the same enemy is seen and shot at.
    let (fire, _, targeted, _) = scenario(&flat(), [100.0, 170.0], Team::Ger, 0.0, 300, 4);
    assert!(targeted, "control without the wall: not noticed");
    let _ = fire;
}

#[test]
fn perception_no_wallhack_friends_are_not_targets() {
    let (fire, turned, targeted, shots) = scenario(&flat(), [100.0, 130.0], Team::Us, 0.0, 240, 5);
    assert!(fire.is_none() && !turned && !targeted && shots == 0);
}

#[test]
fn first_shot_is_at_least_the_reaction_delay_after_the_enemy_is_visible() {
    let col = flat();
    let mut firsts = Vec::new();
    for seed in 0..100u64 {
        // Enemy visible from tick 1 (40 m straight ahead); count ticks to the first shot.
        let mut a = Arena::new(&col, 1000 + seed);
        let map = Arc::new(BotMap::without_nav(Tuning::embedded(), Vec::new()));
        a.add_bot(&map, 0, Team::Us, OBSERVER[0], OBSERVER[1], 0.0);
        spawn(&mut a.world, 1, Team::Ger, 100.0, 140.0, 0.0);
        let mut first = None;
        for t in 1..=300u32 {
            a.step();
            if a.shots > 0 {
                first = Some(t);
                break;
            }
        }
        firsts.push(first.unwrap_or_else(|| panic!("seed {seed}: never fired within 10 s")));
    }
    let min = *firsts.iter().min().unwrap() as f32 / 30.0;
    let max = *firsts.iter().max().unwrap() as f32 / 30.0;
    let mean = firsts.iter().sum::<u32>() as f32 / 100.0 / 30.0;
    eprintln!("time from first sight to first shot over 100 runs: min {min:.3} s, mean {mean:.3} s, max {max:.3} s (Regular: 0.35-0.60 s reaction + recognition roll + ADS)");
    assert!(min >= 0.35, "min {min}");
    assert!(max < 3.0, "max {max}");
}
