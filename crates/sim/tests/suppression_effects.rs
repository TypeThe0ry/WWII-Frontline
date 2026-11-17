//! M1-017b: what suppression does to movement, ADS, spread, aim shake and pinning.
mod common;

use common::*;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::suppression::shake;
use frontline_sim::{SimEvent, Team, World};
use glam::Vec3;

const A: u8 = 0;
const B: u8 = 1;
const HZ: u8 = 30;

fn arena() -> frontline_sim::CollisionData {
    synth_data(14, 2, &|_, _| 5.0)
}

fn world() -> World {
    duel_world(
        data_with(|_| {}),
        &arena(),
        5,
        &[
            (A, Team::Ger, 20.0, 60.0, 16384),
            (B, Team::Us, 90.0, 60.0, 49152),
        ],
    )
}

fn idle() -> InputCmd {
    cmd(0, 0, 49152, 0, 0)
}

fn run_dist(s: f32, b: u16) -> f32 {
    let mut w = world();
    w.debug_set_suppression(A, s, 0);
    let p0 = w.player_state(A).unwrap().pos;
    for _ in 0..90 {
        w.step(&[(A, cmd(0, 127, 16384, b, 0)), (B, idle())]);
    }
    (w.player_state(A).unwrap().pos - p0).length()
}

#[test]
fn movement_is_up_to_10_percent_slower() {
    let free = run_dist(0.0, 0);
    let full = run_dist(100.0, 0);
    let half = run_dist(50.0, 0);
    println!("run 3 s: free {free:.2} m, S=50 {half:.2} m, S=100 {full:.2} m");
    assert!((full / free - 0.90).abs() < 0.02, "{}", full / free);
    assert!((half / free - 0.95).abs() < 0.02, "{}", half / free);
    let sf = run_dist(0.0, buttons::SPRINT);
    let ss = run_dist(100.0, buttons::SPRINT);
    assert!((ss / sf - 0.90).abs() < 0.02, "sprint {}", ss / sf);
}

/// Ticks of holding AIM until the weapon reports ADS (`ads_units` reaches its threshold, which
/// is `ads_ms x hz` units scaled by the ADS-time multiplier; one tick adds 1000 units).
fn ticks_to_ads(s: f32) -> u32 {
    let mut w = world();
    w.debug_set_suppression(A, s, 0);
    let ads_ms = frontline_sim::GameData::embedded()
        .weapon("ger_kar98k")
        .unwrap()
        .gameplay
        .ads_ms;
    let target = (ads_ms * f64::from(HZ)).round() * (1.0 + 0.30 * f64::from(s) / 100.0);
    for t in 1..400 {
        w.step(&[(A, cmd(0, 0, 16384, buttons::AIM, 0)), (B, idle())]);
        if f64::from(w.weapon_state(A).unwrap().ads_units) >= target.round() {
            return t;
        }
    }
    panic!("never aimed");
}

#[test]
fn ads_takes_30_percent_longer_at_full_suppression() {
    let free = ticks_to_ads(0.0);
    let full = ticks_to_ads(100.0);
    println!("ADS ticks: free {free}, S=100 {full}");
    assert!(full > free);
    let ratio = f64::from(full) / f64::from(free);
    assert!((ratio - 1.3).abs() < 0.12, "{ratio}");
}

#[test]
fn pinned_players_cannot_aim_for_half_a_second() {
    let mut w = world();
    // S = 90 pinned for the next 15 ticks (until tick now + 15).
    let now = w.tick().0;
    w.debug_set_suppression(A, 90.0, now + 15);
    let mut first_ads = None;
    for t in 1..60 {
        w.step(&[(A, cmd(0, 0, 16384, buttons::AIM, 0)), (B, idle())]);
        if first_ads.is_none() && w.weapon_state(A).unwrap().ads_units > 0 {
            first_ads = Some(t);
        }
    }
    // Ticks 1..=15 are ignored; aiming starts on tick 16.
    assert_eq!(first_ads, Some(16));
    // Below the threshold nothing is blocked.
    let mut w = world();
    w.debug_set_suppression(A, 84.0, 0);
    w.step(&[(A, cmd(0, 0, 16384, buttons::AIM, 0)), (B, idle())]);
    assert!(w.weapon_state(A).unwrap().ads_units > 0);
}

#[test]
fn shake_is_deterministic_bounded_and_scales_with_amplitude() {
    let seed = 99u64;
    let mut max = 0.0f32;
    for t in 0..600 {
        let a = shake(seed, frontline_sim::ids::Tick(t), 3, 0.6, HZ);
        let b = shake(seed, frontline_sim::ids::Tick(t), 3, 0.6, HZ);
        assert_eq!(a, b);
        let half = shake(seed, frontline_sim::ids::Tick(t), 3, 0.3, HZ);
        assert!((half.0 * 2.0 - a.0).abs() < 1e-6);
        max = max.max(a.0.abs()).max(a.1.abs());
    }
    assert!(max <= 0.6f32.to_radians() + 1e-6);
    assert!(max > 0.4f32.to_radians(), "uses most of the range: {max}");
    assert_eq!(
        shake(seed, frontline_sim::ids::Tick(5), 3, 0.0, HZ),
        (0.0, 0.0)
    );
    // Another player gets another stream.
    assert_ne!(
        shake(seed, frontline_sim::ids::Tick(40), 3, 0.6, HZ),
        shake(seed, frontline_sim::ids::Tick(40), 4, 0.6, HZ)
    );
}

fn rms_dev(s: f32) -> f32 {
    let mut w = world();
    w.debug_set_suppression(A, s, 0);
    let fwd = Vec3::X;
    let (mut sum, mut n) = (0.0f32, 0u32);
    for t in 0..9000 {
        let b = if t % 2 == 0 { buttons::FIRE } else { 0 };
        let mag = w.weapon_state(A).unwrap().mag;
        let b = if mag == 0 { buttons::RELOAD } else { b };
        let ev = w.step(&[(A, cmd(0, 0, 16384, b, 0)), (B, idle())]).events;
        for e in ev {
            if let SimEvent::Shot { dir, .. } = e {
                let ang = dir.dot(fwd).clamp(-1.0, 1.0).acos().to_degrees();
                sum += ang * ang;
                n += 1;
            }
        }
        w.debug_set_suppression(A, s, 0);
    }
    assert!(n > 40, "shots {n}");
    (sum / n as f32).sqrt()
}

#[test]
fn spread_grows_with_suppression() {
    let free = rms_dev(0.0);
    let full = rms_dev(100.0);
    println!("RMS angle: free {free:.3} deg, S=100 {full:.3} deg");
    let ratio = full / free;
    assert!(ratio > 2.0 && ratio < 3.2, "{ratio}");
}
