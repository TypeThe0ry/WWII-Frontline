//! M1-013a: SMG / carbine / pistol fire rates, reload times, reserve bookkeeping, recoil,
//! reload interruption, spread seeds.
mod common;

use common::*;
use frontline_sim::weapon::{ammo, Fsm, ReloadKind};
use frontline_sim::{GameData, Tick};

const SMGS: [(&str, f64); 3] = [
    ("us_thompson_m1a1", 700.0),
    ("us_m3_grease", 450.0),
    ("ger_mp40", 500.0),
];
const SEMIS: [(&str, f64); 3] = [
    ("us_m1_carbine", 200.0),
    ("us_m1911a1", 300.0),
    ("ger_p38", 300.0),
];
const ALL: [&str; 6] = [
    "us_thompson_m1a1",
    "us_m3_grease",
    "ger_mp40",
    "us_m1_carbine",
    "us_m1911a1",
    "ger_p38",
];

#[test]
fn auto_cyclic_rate_within_2_percent_over_30_s() {
    for (id, rpm) in SMGS {
        let mut r = Rig::new(id, 6);
        // Hold the trigger for 30 s, topping the magazine up (rate test only).
        let mut shots = 0u32;
        for _ in 0..900 {
            if r.w.mag == 0 {
                r.w.mag = r.wp.mag_capacity;
                r.w.fsm = Fsm::Ready;
            }
            if r.step(BTN_FIRE).shot.is_some() {
                shots += 1;
            }
        }
        let expect = rpm / 2.0; // 30 s
        let err = (f64::from(shots) / expect - 1.0).abs();
        println!(
            "{id}: {shots} shots in 30 s, {rpm} rpm -> {:.2} %",
            err * 100.0
        );
        assert!(err <= 0.02, "{id}: {shots} vs {expect}");
    }
}

#[test]
fn semi_auto_cap_interval_is_exact_to_one_tick() {
    for (id, rpm) in SEMIS {
        let mut r = Rig::new(id, 6);
        let shots = r.max_rate(600);
        let gap_ticks = 60.0 / rpm * 30.0;
        for pair in shots.windows(2) {
            let gap = f64::from(pair[1] - pair[0]);
            assert!((gap - gap_ticks).abs() <= 1.0, "{id}: gap {gap}");
        }
        let avg = f64::from(shots[shots.len() - 1] - shots[0]) / (shots.len() - 1) as f64;
        assert!((avg / gap_ticks - 1.0).abs() < 0.01, "{id}: avg {avg}");
        // Holding the trigger does not auto-fire a semi-auto weapon.
        let mut held = Rig::new(id, 6);
        let n = (0..200)
            .filter(|_| held.step(BTN_FIRE).shot.is_some())
            .count();
        assert_eq!(n, 1, "{id}: one shot per press");
    }
}

#[test]
fn doubled_input_rate_cannot_exceed_the_cap() {
    // A cheater clicking as fast as the input allows (press every tick, i.e. toggling) gets
    // at most cap x 1.05 shots (ARCHITECTURE 15.1).
    for (id, rpm) in SEMIS.iter().chain(SMGS.iter()) {
        let mut r = Rig::new(id, 6);
        let mut shots = 0u32;
        for i in 0..900 {
            if r.w.mag == 0 {
                r.w.mag = r.wp.mag_capacity;
                r.w.fsm = Fsm::Ready;
            }
            if r.step(if i % 2 == 0 { BTN_FIRE } else { 0 }).shot.is_some() {
                shots += 1;
            }
        }
        assert!(f64::from(shots) <= rpm / 2.0 * 1.05, "{id}: {shots}");
    }
}

#[test]
fn reload_times_of_all_six_weapons_match_the_table() {
    let data = GameData::embedded();
    let want = |ms: f64| (ms * 30.0 / 1000.0).round() as i64;
    for id in ALL {
        let g = &data.weapon(id).unwrap().gameplay.reload;
        for (shots, kind, ms) in [
            (1u32, ReloadKind::Tactical, g.tactical_ms),
            (u32::MAX, ReloadKind::Empty, g.empty_ms),
        ] {
            let mut r = Rig::new(id, 6);
            if shots == u32::MAX {
                r.w.mag = 0;
                r.w.fsm = Fsm::Empty;
            } else {
                r.w.mag -= 1;
            }
            let out = r.step(BTN_RELOAD);
            assert_eq!(out.reload_started, Some(kind), "{id}");
            let mut ticks = 1;
            while !r.step(0).reload_finished {
                ticks += 1;
                assert!(ticks < 500);
            }
            assert!(
                (i64::from(ticks) - want(ms)).abs() <= 1,
                "{id} {kind:?}: {ticks} ticks vs {ms} ms"
            );
        }
    }
}

#[test]
fn tactical_reload_keeps_the_old_rounds_as_a_partial_magazine() {
    let id = "us_thompson_m1a1";
    for fired in [0u16, 7, 29, 30] {
        let mut r = Rig::new(id, 3);
        let cap = r.wp.mag_capacity;
        r.w.mag = cap - fired;
        if r.w.mag == 0 {
            r.w.fsm = Fsm::Empty;
        }
        let total = r.w.mag + r.w.reserve;
        let started = r.step(BTN_RELOAD).reload_started.is_some();
        if fired == 0 {
            assert!(!started, "a full magazine does not reload");
            continue;
        }
        while !r.step(0).reload_finished {}
        assert_eq!(r.w.mag, cap);
        assert_eq!(r.w.mag + r.w.reserve, total, "rounds are conserved");
        let (full, partial) = ammo::mag_stack(r.w.reserve, cap);
        assert_eq!(full * cap + partial, r.w.reserve);
        // Spare magazines: the 3 spare clips minus what was taken, the old one's k rounds
        // sitting in the partial one.
        // The old magazine went back with its remaining rounds: k = cap - fired.
        assert_eq!(partial, (cap - fired) % cap);
    }
}

#[test]
fn reload_is_cancelled_by_sprinting_and_costs_nothing() {
    let mut r = Rig::new("us_m3_grease", 3);
    r.w.mag -= 10;
    let before = (r.w.mag, r.w.reserve);
    assert!(r.step(BTN_RELOAD).reload_started.is_some());
    r.idle(5);
    assert!(r.w.fsm.is_reloading());
    let out = r.step_cmd(cmd(0, 127, 0, BTN_SPRINT, 0));
    assert!(out.reload_cancelled);
    assert_eq!(r.w.fsm, Fsm::Ready);
    assert_eq!((r.w.mag, r.w.reserve), before, "nothing consumed");
    // Vaulting cancels too.
    let mut v = Rig::new("us_m3_grease", 3);
    v.w.mag -= 10;
    v.step(BTN_RELOAD);
    v.auth.vault = Some(frontline_sim::movement::Vault {
        ticks_left: 5,
        total: 24,
        climb: false,
        start: frontline_sim::Vec3::ZERO,
        target: frontline_sim::Vec3::X,
        top: 1.0,
    });
    assert!(v.step(0).reload_cancelled);
}

#[test]
fn recoil_of_ten_shots_and_its_recovery() {
    let id = "us_thompson_m1a1";
    let mut r = Rig::new(id, 6);
    let mut sum = 0.0f32;
    let mut shots = 0;
    while shots < 10 {
        let before = r.w.recoil_pitch_deg;
        if r.step(BTN_FIRE).shot.is_some() {
            sum += r.w.recoil_pitch_deg - before;
            shots += 1;
        }
    }
    let expect = 10.0 * r.wp.vkick_deg;
    assert!((sum / expect - 1.0).abs() < 0.2, "sum {sum} vs {expect}");
    // Same seed, same recoil (deterministic replay).
    let mut r2 = Rig::new(id, 6);
    let mut shots2 = 0;
    while shots2 < 10 {
        if r2.step(BTN_FIRE).shot.is_some() {
            shots2 += 1;
        }
    }
    assert_eq!(
        r.w.recoil_pitch_deg.to_bits(),
        r2.w.recoil_pitch_deg.to_bits()
    );
    // After the 0.12 s pause the recovery runs at recover_deg_s.
    r.idle(4); // pause
    let a = r.w.recoil_pitch_deg;
    r.step(0);
    let rate = (a - r.w.recoil_pitch_deg) * 30.0;
    assert!(
        (rate / r.wp.recover_deg_s - 1.0).abs() <= 0.05,
        "rate {rate}"
    );
}

#[test]
fn spread_seed_differs_per_shot_and_follows_the_formula() {
    let mut r = Rig::new("us_thompson_m1a1", 6);
    let mut seeds = Vec::new();
    while seeds.len() < 30 {
        let out = r.step(BTN_FIRE);
        if let Some(s) = out.shot {
            assert_eq!(
                s.spread_seed,
                frontline_sim::rng::spread_seed(r.seed, Tick(r.tick), r.entity)
            );
            seeds.push(s.spread_seed);
        }
    }
    let mut uniq = seeds.clone();
    uniq.sort_unstable();
    uniq.dedup();
    assert!(uniq.len() >= 28, "seeds look random: {seeds:?}");
}
