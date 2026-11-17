//! M1-017: suppression detection, accumulation, decay, effects, anti-farm assists.
mod common;

use common::*;
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::suppression::anti_farm::{assisting, record, Source};
use frontline_sim::suppression::detect::{amount, seg_seg_distance};
use frontline_sim::suppression::state::{decay, quantise4, SuppressionState};
use frontline_sim::suppression::{effects, D_MAX_M};
use frontline_sim::weapon::ballistics::BallisticsParams;
use frontline_sim::{GameData, SimEvent, Team, World};
use glam::Vec3;

const A: u8 = 0;
const B: u8 = 1;
const C: u8 = 2;
const D: u8 = 3;
const EYE: f32 = 1.65;
const HZ: u8 = 30;

fn exact() -> &'static GameData {
    data_with(|w| {
        w.gameplay.spread.hip_deg = 0.0;
        w.gameplay.spread.ads_deg = 0.0;
        w.gameplay.recoil.vkick_deg = 0.0;
        w.gameplay.recoil.hsigma_deg = 0.0;
        w.gameplay.recoil.bloom_per_shot_deg = 0.0;
    })
}

fn arena() -> frontline_sim::CollisionData {
    synth_data(14, 2, &|_, _| 5.0)
}

fn idle() -> InputCmd {
    cmd(0, 0, 49152, 0, 0)
}

#[test]
fn pass_distance_threshold_and_amount() {
    // 7.92x57 (bolt rifle) at 740 m/s straight through the axis: weight x 1 x 1 = 2.2.
    assert!((amount("rifle_bolt", 0.0, 740.0) - 2.2).abs() < 1e-6);
    assert_eq!(
        amount("rifle_bolt", D_MAX_M, 740.0),
        0.0,
        "d = 5.0 m gives s = 0"
    );
    assert_eq!(
        amount("rifle_bolt", 5.01, 740.0),
        0.0,
        "beyond 5 m is not counted"
    );
    // Machine gun at 2 m: 2.8 x 0.6^1.2 = 1.52 (+-2 %).
    let mg = amount("mg", 2.0, 800.0);
    assert!((mg - 1.52).abs() / 1.52 < 0.02, "{mg}");
    // Speed factor: 250 m/s halves it.
    assert!((amount("rifle_bolt", 0.0, 250.0) - 1.1).abs() < 1e-6);
    // Classes.
    assert_eq!(amount("pistol", 0.0, 500.0), 0.8);
    assert_eq!(amount("carbine", 0.0, 500.0), 1.5);
}

#[test]
fn segment_to_axis_distance() {
    let (a, b) = (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0));
    let d = seg_seg_distance(Vec3::new(-10.0, 1.0, 3.0), Vec3::new(10.0, 1.0, 3.0), a, b);
    assert!((d - 3.0).abs() < 1e-5, "{d}");
    // A segment ending before the axis: distance to its end point.
    let d = seg_seg_distance(Vec3::new(-10.0, 1.0, 0.0), Vec3::new(-4.0, 1.0, 0.0), a, b);
    assert!((d - 4.0).abs() < 1e-5, "{d}");
}

#[test]
fn decay_rates_and_window() {
    let mk = || SuppressionState {
        s: 50.0,
        window_until: 0,
        ..Default::default()
    };
    // Stand: 1 s after the window lost 6 + 0.12 x 50 = 12 (the rate falls as S falls; first tick).
    let mut st = mk();
    decay(&mut st, 1, HZ, 0.0, false);
    assert!((50.0 - st.s - 12.0 / 30.0).abs() < 1e-4, "{}", 50.0 - st.s);
    let mut st = mk();
    decay(&mut st, 1, HZ, 2.0, false);
    assert!((50.0 - st.s - 15.6 / 30.0).abs() < 1e-4, "prone x1.3");
    let mut st = mk();
    decay(&mut st, 1, HZ, 1.0, false);
    assert!(
        (50.0 - st.s - 12.0 * 1.15 / 30.0).abs() < 1e-4,
        "crouch x1.15"
    );
    let mut st = mk();
    decay(&mut st, 1, HZ, 0.0, true);
    assert!((50.0 - st.s - 12.0 * 1.4 / 30.0).abs() < 1e-4, "cover x1.4");
    // Window: a hit at tick 100 holds S for 15 ticks, decay from the 16th.
    let mut st = SuppressionState::default();
    assert!(st.add(40.0, 100, 15));
    for t in 100..=115 {
        decay(&mut st, t, HZ, 0.0, false);
        assert_eq!(st.s, 40.0, "tick {t}");
    }
    decay(&mut st, 116, HZ, 0.0, false);
    assert!(st.s < 40.0);
}

#[test]
fn bullet_cap_and_clamp() {
    let mut st = SuppressionState::default();
    let n = (0..20).filter(|_| st.add(1.0, 7, 15)).count();
    assert_eq!(n, 12, "at most 12 bullets per tick");
    assert!(st.add(1.0, 8, 15), "next tick counts again");
    let mut st = SuppressionState::default();
    st.add(500.0, 1, 15);
    assert_eq!(st.s, 100.0);
}

#[test]
fn effects_match_the_design_table() {
    let e = effects(100.0);
    assert!((e.move_mult - 0.90).abs() < 1e-6);
    assert!((e.ads_mult - 1.30).abs() < 1e-6);
    assert!((e.spread_mult - 2.5).abs() < 1e-6);
    assert!((e.desat - 0.45).abs() < 1e-6);
    assert!((e.vignette - 0.6).abs() < 1e-6);
    assert!((e.shake_deg - 0.6).abs() < 1e-6);
    assert!((e.blur_px - 6.0).abs() < 1e-6);
    let z = effects(0.0);
    assert_eq!(
        (
            z.move_mult,
            z.ads_mult,
            z.spread_mult,
            z.desat,
            z.vignette,
            z.blur_px,
            z.pinned
        ),
        (1.0, 1.0, 1.0, 1.0, 0.0, 0.0, false)
    );
    assert!(!effects(84.9).pinned);
    assert!(effects(85.0).pinned);
}

#[test]
fn self_state_quantisation() {
    assert_eq!(quantise4(0.0), 0);
    assert_eq!(quantise4(100.0), 15);
    assert_eq!(quantise4(50.0), 8);
    let mut prev = 0;
    for i in 0..=100 {
        let q = quantise4(i as f32);
        assert!(q >= prev && q <= 15);
        prev = q;
    }
}

#[test]
fn anti_farm_window_and_team() {
    let mut l = Vec::new();
    record(&mut l, 7, Team::Ger, 100);
    record(&mut l, 8, Team::Ger, 130);
    record(&mut l, 9, Team::Us, 100);
    let w = 150;
    // Killer is a Ger teammate: suppressors 7 and 8 (not the killer 8, not enemy 9).
    assert_eq!(assisting(&l, 240, w, Some(8), Some(Team::Ger)), vec![7]);
    // Past the window nobody is left.
    assert!(assisting(&l, 251, w, Some(8), Some(Team::Ger)).is_empty());
    // Re-recording refreshes instead of duplicating.
    record(&mut l, 7, Team::Ger, 200);
    assert_eq!(l.iter().filter(|s: &&Source| s.who == 7).count(), 1);
}

/// A (Ger) at x=20 facing +x; B (Us) at x=26, `dz` metres to the side; C (Ger) x=30 facing -x,
/// D (Us) farther along as another enemy.
fn world(dz: f32) -> World {
    duel_world(
        exact(),
        &arena(),
        5,
        &[
            (A, Team::Ger, 20.0, 60.0, 16384),
            (B, Team::Us, 26.0, 60.0 + dz, 49152),
            (C, Team::Ger, 30.0, 60.0 + dz, 49152),
            (D, Team::Us, 90.0, 80.0, 49152),
        ],
    )
}

fn torso_pitch() -> f32 {
    let bp = BallisticsParams::from_def(exact().weapon("ger_kar98k").unwrap());
    solve_pitch(&bp, EYE, 6.0, 1.33)
}

/// A fires one shot; returns the events and the tick it was fired in.
fn fire_once(w: &mut World) -> Vec<SimEvent> {
    let mut out = Vec::new();
    for i in 0..4 {
        let b = if i == 0 { BTN_FIRE } else { 0 };
        out.extend(
            w.step(&[
                (A, fire_cmd(torso_pitch(), b)),
                (B, idle()),
                (C, idle()),
                (D, idle()),
            ])
            .events,
        );
    }
    out
}

#[test]
fn near_miss_suppresses_once_and_scales_with_distance() {
    let mut last = f32::MAX;
    for dz in [0.6f32, 1.5, 2.5, 3.5, 4.5] {
        let mut w = world(dz);
        let ev = fire_once(&mut w);
        assert!(
            !ev.iter().any(|e| matches!(e, SimEvent::Hit { .. })),
            "dz {dz}: must miss"
        );
        let sup: Vec<_> = ev
            .iter()
            .filter_map(|e| match e {
                SimEvent::Suppress { intensity, .. } => Some(*intensity),
                _ => None,
            })
            .collect();
        assert_eq!(sup.len(), 1, "dz {dz}: one event for one bullet");
        let s = w.suppression(B).unwrap();
        assert!(
            s > 0.0 && s < last,
            "dz {dz}: S {s} must fall with distance (prev {last})"
        );
        last = s;
        assert_eq!(w.suppression(A), Some(0.0), "shooter is not suppressed");
        assert_eq!(
            w.suppression(C),
            Some(0.0),
            "friendly bullets never suppress"
        );
    }
}

#[test]
fn a_bullet_beyond_5_m_does_nothing() {
    let mut w = world(6.0);
    let ev = fire_once(&mut w);
    assert_eq!(w.suppression(B), Some(0.0));
    assert!(!ev.iter().any(|e| matches!(e, SimEvent::Suppress { .. })));
}

#[test]
fn suppression_decays_after_the_window() {
    let mut w = world(1.0);
    fire_once(&mut w);
    let s0 = w.suppression(B).unwrap();
    assert!(s0 > 0.0);
    for _ in 0..8 {
        w.step(&[(A, idle()), (B, idle()), (C, idle()), (D, idle())]);
    }
    assert_eq!(w.suppression(B), Some(s0), "held inside the 0.5 s window");
    for _ in 0..120 {
        w.step(&[(A, idle()), (B, idle()), (C, idle()), (D, idle())]);
    }
    assert_eq!(w.suppression(B), Some(0.0), "gone after a few seconds");
}

fn head_pitch() -> f32 {
    let bp = BallisticsParams::from_def(exact().weapon("ger_kar98k").unwrap());
    solve_pitch(&bp, EYE, 4.0, 1.66)
}

/// A suppresses B at tick 0; C kills B `delay` ticks later. Returns the assist events.
fn assist_after(delay: u32) -> usize {
    let mut w = world(2.0);
    let mut assists = 0;
    let mut killed = false;
    for t in 0..delay + 40 {
        let a = if t == 0 {
            fire_cmd(torso_pitch(), BTN_FIRE)
        } else {
            fire_cmd(torso_pitch(), 0)
        };
        let c = if t == delay {
            let mut c = fire_cmd(head_pitch(), BTN_FIRE);
            c.yaw = 49152;
            c
        } else {
            let mut c = fire_cmd(head_pitch(), 0);
            c.yaw = 49152;
            c
        };
        for e in w.step(&[(A, a), (B, idle()), (C, c), (D, idle())]).events {
            match e {
                SimEvent::SuppressionAssist { .. } => assists += 1,
                SimEvent::Kill { victim: B, .. } => killed = true,
                _ => {}
            }
        }
    }
    assert!(killed, "B died");
    assists
}

#[test]
fn teammate_kill_within_5_s_credits_the_suppressor_once() {
    assert_eq!(assist_after(147), 1, "4.9 s");
    assert_eq!(assist_after(153), 0, "5.1 s");
}

#[test]
fn deterministic_with_suppression() {
    let run = || {
        let mut w = world(1.5);
        let ev = fire_once(&mut w);
        (event_hash(&ev), w.state_hash())
    };
    assert_eq!(run(), run());
}

/// Sustained MG42 fire at d = 2 m: 1200 rpm = 20 bullets/s, 1.52 each, stance standing, no cover.
fn mg_sweep(seconds: f32) -> f32 {
    let mut st = SuppressionState::default();
    let per = amount("mg", 2.0, 800.0);
    let ticks = (seconds * 30.0) as u32;
    for t in 1..=ticks {
        // 20 bullets/s = one every 1.5 ticks (2 of every 3 ticks).
        if t % 3 != 0 {
            st.add(per, t, 15);
        }
        decay(&mut st, t, HZ, 0.0, false);
    }
    st.s
}

#[test]
fn suppression_mg42() {
    let s15 = mg_sweep(1.5);
    let s40 = mg_sweep(4.0);
    println!("MG42 d=2 m: 1.5 s -> {s15:.1}, 4 s -> {s40:.1}");
    // PI-057: every bullet re-arms the 0.5 s no-decay window, so there is no decay while the
    // burst lasts and 30 bullets x 1.52 = 45 (GAMEDESIGN's "35" example ignores the window).
    assert!((40.0..=50.0).contains(&s15), "1.5 s: {s15}");
    assert!(s40 >= 80.0, "4 s: {s40}");
}

#[test]
fn explosions_suppress_enemies_and_friends_alike() {
    use frontline_protocol::msg::input::buttons;
    let mut w = duel_world(
        data_with(|_| {}),
        &arena(),
        5,
        &[
            (A, Team::Ger, 20.0, 60.0, 16384),
            (B, Team::Us, 28.0, 60.0, 49152),
            (C, Team::Ger, 20.0, 68.0, 49152),
            (D, Team::Us, 90.0, 80.0, 49152),
        ],
    );
    // A holds the grenade past its fuse: it goes off in the hand.
    let mut boom = false;
    for _ in 0..200 {
        let ev = w
            .step(&[
                (A, cmd(0, 0, 16384, buttons::GADGET, 0)),
                (B, idle()),
                (C, idle()),
                (D, idle()),
            ])
            .events;
        boom |= ev.iter().any(|e| matches!(e, SimEvent::Explosion { .. }));
        if boom {
            break;
        }
    }
    assert!(boom);
    // 40 (1 - r/15)^2 at r ~ 8.2 m (blast centre in the hand, measured to the feet): ~8.4.
    for p in [B, C] {
        let s = w.suppression(p).unwrap();
        assert!((s - 8.5).abs() < 0.6, "player {p}: {s}");
    }
    assert_eq!(w.suppression(D), Some(0.0));
}
