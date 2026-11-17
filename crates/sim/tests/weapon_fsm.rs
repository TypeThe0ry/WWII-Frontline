//! M0-012 acceptance 2-5: fire state machine, rate cap, reload timing, ammo bookkeeping, spread.
mod common;

use common::*;
use frontline_sim::weapon::{Fsm, ReloadKind, TICK_UNITS};
use frontline_sim::Stance;

const GARAND: &str = "us_m1_garand";
const KAR: &str = "ger_kar98k";

fn intervals(shots: &[u32]) -> Vec<u32> {
    shots.windows(2).map(|p| p[1] - p[0]).collect()
}

#[test]
fn magazine_sizes_come_from_data() {
    assert_eq!(Rig::new(GARAND, 6).w.mag, 8);
    assert_eq!(Rig::new(KAR, 6).w.mag, 5);
    assert_eq!(Rig::new(GARAND, 6).w.reserve, 48);
}

#[test]
fn semi_auto_needs_release_between_shots() {
    let mut r = Rig::new(GARAND, 6);
    let mut n = 0;
    for _ in 0..100 {
        if r.step(BTN_FIRE).shot.is_some() {
            n += 1;
        }
    }
    assert_eq!(n, 1, "holding the trigger fires once");
    assert_eq!(r.w.mag, 7);
    // Release, then a press after the cooldown fires again; a press inside it does not.
    r.step(0);
    assert!(r.step(BTN_FIRE).shot.is_some());
    r.step(0);
    assert!(
        r.step(BTN_FIRE).shot.is_none(),
        "inside the 15-tick cooldown"
    );
}

#[test]
fn rate_cap_semi_10_seconds() {
    // Fastest possible clicker (edge handling included), instant magazine top-ups.
    let mut r = Rig::new(GARAND, 6);
    let shots = r.max_rate(300);
    let gaps = intervals(&shots);
    println!(
        "Garand max-rate clicker 10 s @30 Hz: {} shots, min gap {} ticks ({:.3} s)",
        shots.len(),
        gaps.iter().min().unwrap(),
        *gaps.iter().min().unwrap() as f32 / 30.0
    );
    assert!(shots.len() <= 21, "{} shots", shots.len());
    assert!(
        shots.len() >= 19,
        "the cap is reached, too: {}",
        shots.len()
    );
    assert!(
        gaps.iter().all(|g| *g >= 14),
        "gaps >= data - 1 tick: {gaps:?}"
    );
    assert!(
        gaps.iter().all(|g| *g == 15),
        "120 rpm = 15 ticks exactly: {gaps:?}"
    );
    // Holding the button down continuously (no release) fires once.
    let mut h = Rig::new(GARAND, 6);
    let held = (0..300).filter(|_| h.step(BTN_FIRE).shot.is_some()).count();
    assert_eq!(held, 1);
    // Plain trigger mashing every other tick is slower (15-tick cooldown, parity): 8 rounds in the clip.
    let mut m = Rig::new(GARAND, 6);
    let mashed = m.mash(300);
    println!(
        "Garand button mashing (press/release alternately): {} shots from one clip, gaps {:?}",
        mashed.len(),
        intervals(&mashed).first()
    );
    assert_eq!(mashed.len(), 8);
    assert!(intervals(&mashed).iter().all(|g| *g >= 15));
}

#[test]
fn rate_cap_bolt_10_seconds() {
    let mut r = Rig::new(KAR, 6);
    let shots = r.max_rate(300);
    let gaps = intervals(&shots);
    println!(
        "Kar98k max-rate clicker 10 s: {} shots, min gap {:?} ticks",
        shots.len(),
        gaps.iter().min()
    );
    assert!(shots.len() <= 7, "{}", shots.len());
    assert!(shots.len() >= 6);
    assert!(gaps.iter().all(|g| *g >= 44), "{gaps:?}");
    assert!(
        gaps.iter().all(|g| *g == 45),
        "1500 ms = 45 ticks: {gaps:?}"
    );
}

#[test]
fn bolt_cycle_blocks_fire_for_1500_ms() {
    let mut r = Rig::new(KAR, 6);
    assert!(r.step(BTN_FIRE).shot.is_some()); // tick 1
                                              // Ticks 2..=45: a fresh press every other tick never fires.
    for i in 2..=45u32 {
        let out = r.step(if i % 2 == 0 { BTN_FIRE } else { 0 });
        assert!(out.shot.is_none(), "tick {i}");
        assert_eq!(r.w.fsm, Fsm::BoltCycle, "tick {i}");
    }
    // Tick 45 was a release; the press on tick 46 (1500 ms = 45 ticks after the shot) fires.
    assert!(r.step(BTN_FIRE).shot.is_some());
}

#[test]
fn fire_is_blocked_while_reloading_and_resumes_after() {
    let mut r = Rig::new(GARAND, 6);
    r.step(BTN_FIRE);
    r.idle(20);
    let out = r.step(BTN_RELOAD);
    assert_eq!(out.reload_started, Some(ReloadKind::Tactical));
    let mut shots_during = 0;
    for i in 0..60 {
        if r.step(if i % 2 == 0 { BTN_FIRE } else { 0 }).shot.is_some() {
            shots_during += 1;
        }
    }
    assert_eq!(shots_during, 0);
    assert!(matches!(r.w.fsm, Fsm::Reloading(_)));
    r.idle(60); // 3400 ms = 102 ticks in total
    assert_eq!(r.w.fsm, Fsm::Ready);
    assert_eq!((r.w.mag, r.w.reserve), (8, 48 - 1));
    assert!(r.step(BTN_FIRE).shot.is_some());
}

#[test]
fn sprinting_cannot_shoot_but_backpedalling_with_sprint_held_can() {
    let mut r = Rig::new(GARAND, 6);
    let sprint = cmd(0, 127, 0, BTN_SPRINT | BTN_FIRE, 0);
    assert!(r.step_cmd(sprint).shot.is_none());
    assert_eq!(r.w.mag, 8);
    r.step(0);
    // Sprint needs a forward component (movement rule); backwards it is just running.
    let back = cmd(0, -127, 0, BTN_SPRINT | BTN_FIRE, 0);
    assert!(r.step_cmd(back).shot.is_some());
    // Sprint button without moving does not count as sprinting either.
    r.idle(20);
    let standing = cmd(0, 0, 0, BTN_SPRINT | BTN_FIRE, 0);
    r.step(0);
    assert!(r.step_cmd(standing).shot.is_some());
}

#[test]
fn stance_transition_blocks_fire() {
    let mut r = Rig::new(GARAND, 6);
    r.auth.stance_timer = 3;
    assert!(r.step(BTN_FIRE).shot.is_none());
    r.step(0);
    r.auth.stance_timer = 0;
    assert!(r.step(BTN_FIRE).shot.is_some());
}

#[test]
fn dry_fire_has_no_projectile_and_costs_nothing() {
    let mut r = Rig::new(GARAND, 0);
    r.w.mag = 1;
    assert!(r.step(BTN_FIRE).shot.is_some());
    r.idle(30);
    assert_eq!(r.w.fsm, Fsm::Empty);
    let out = r.step(BTN_FIRE);
    assert!(out.shot.is_none() && out.dry_fire);
    assert_eq!((r.w.mag, r.w.reserve), (0, 0));
    // Held trigger: one click only.
    assert!(!r.step(BTN_FIRE).dry_fire);
    // No spare rounds: reload does nothing.
    let out = r.step(BTN_RELOAD);
    assert!(out.reload_started.is_none());
    assert_eq!(r.w.fsm, Fsm::Empty);
}

#[test]
fn reload_cannot_be_restarted_or_swapped_midway() {
    let mut r = Rig::new(KAR, 6);
    r.step(BTN_FIRE);
    r.idle(50);
    assert!(r.step(BTN_RELOAD).reload_started.is_some());
    r.idle(10);
    let before = r.w.timer_units;
    // Pressing reload again or asking for another weapon slot changes nothing (no weapon switching in M0).
    let mut c = cmd(0, 0, 0, BTN_RELOAD, 0);
    c.weapon_slot = 2;
    let out = r.step_cmd(c);
    assert!(out.reload_started.is_none());
    assert_eq!(
        r.w.timer_units,
        before - TICK_UNITS,
        "timer just kept running"
    );
    // A full magazine does not reload at all.
    let mut f = Rig::new(KAR, 6);
    assert!(f.step(BTN_RELOAD).reload_started.is_none());
}

fn reload_ticks(weapon: &str, shots: u32) -> (u32, ReloadKind) {
    let mut r = Rig::new(weapon, 6);
    for _ in 0..shots {
        r.step(BTN_FIRE);
        r.idle(50);
    }
    let out = r.step(BTN_RELOAD);
    let kind = out.reload_started.expect("reload starts");
    let mut ticks = 0;
    loop {
        ticks += 1;
        if r.step(0).reload_finished {
            break;
        }
        assert!(ticks < 1000);
    }
    assert_eq!(r.w.mag, r.wp.mag_capacity, "filled up");
    (ticks, kind)
}

#[test]
fn reload_durations_match_data() {
    let data = frontline_sim::GameData::embedded();
    let g = &data.weapon("us_m1_garand").unwrap().gameplay.reload;
    let k = &data.weapon("ger_kar98k").unwrap().gameplay.reload;
    // Garand: empty magazine (8 shots), then tactical (1 shot).
    let (t_empty, kind) = reload_ticks(GARAND, 8);
    assert_eq!(kind, ReloadKind::Empty);
    let (t_tac, kind) = reload_ticks(GARAND, 1);
    assert_eq!(kind, ReloadKind::Tactical);
    // Kar98k clip: 3000 ms either way.
    let (k_empty, _) = reload_ticks(KAR, 5);
    let (k_tac, _) = reload_ticks(KAR, 1);
    let want = |ms: f64| (ms * 30.0 / 1000.0).round() as i64;
    println!(
        "reload ticks: Garand empty {t_empty} ({:.3} s, data {} ms), tactical {t_tac} ({:.3} s, data {} ms); Kar98k empty {k_empty} ({:.3} s), tactical {k_tac} ({:.3} s, data {} ms)",
        t_empty as f64 / 30.0, g.empty_ms, t_tac as f64 / 30.0, g.tactical_ms,
        k_empty as f64 / 30.0, k_tac as f64 / 30.0, k.empty_ms
    );
    assert!((i64::from(t_empty) - want(g.empty_ms)).abs() <= 1);
    assert!((i64::from(t_tac) - want(g.tactical_ms)).abs() <= 1);
    assert!((i64::from(k_empty) - want(3000.0)).abs() <= 1);
    assert!((i64::from(k_tac) - want(k.tactical_ms)).abs() <= 1);
}

#[test]
fn ammo_is_conserved() {
    for weapon in [GARAND, KAR] {
        let mut r = Rig::new(weapon, 3);
        let total0 = u32::from(r.w.mag) + u32::from(r.w.reserve);
        let mut fired = 0;
        for t in 0..4000u32 {
            let b = match t % 40 {
                0 | 10 | 20 | 30 => BTN_FIRE,
                35 => BTN_RELOAD,
                _ => 0,
            };
            if r.step(b).shot.is_some() {
                fired += 1;
            }
            assert_eq!(
                u32::from(r.w.mag) + u32::from(r.w.reserve) + fired,
                total0,
                "{weapon} tick {t}"
            );
        }
        assert!(fired > 10, "{weapon}: {fired}");
        assert_eq!(r.w.mag + r.w.reserve, (total0 - fired) as u16);
    }
}

#[test]
fn time_accumulator_has_no_systematic_bias() {
    // A 140 rpm weapon: 428.571 ms = 12.857 ticks. Gaps alternate 12 / 13 and average exactly.
    let mut r = Rig::new(GARAND, 6);
    r.wp.interval_units = (60_000.0f64 / 140.0 * 30.0).round() as i32; // 12857 tick-milli-units
    let shots = r.max_rate(6000);
    let gaps = intervals(&shots);
    let mean = gaps.iter().sum::<u32>() as f64 / gaps.len() as f64;
    println!(
        "140 rpm @30 Hz (semi, fastest clicker): mean gap {mean:.4} ticks (exact 12.857), min {} max {}",
        gaps.iter().min().unwrap(),
        gaps.iter().max().unwrap()
    );
    assert!(gaps.iter().all(|g| *g == 12 || *g == 13), "{gaps:?}");
    assert!((mean - 12.857).abs() < 0.01, "{mean}");
    // Full auto at the same interval.
    let mut a = Rig::new(GARAND, 6);
    a.wp.mode = frontline_sim::data::defs::FireMode::Auto;
    a.wp.interval_units = 12857;
    let mut shots = Vec::new();
    for _ in 0..6000u32 {
        a.w.mag = 8;
        if a.w.fsm == Fsm::Empty {
            a.w.fsm = Fsm::Ready;
        }
        if a.step(BTN_FIRE).shot.is_some() {
            shots.push(a.tick);
        }
    }
    let g = intervals(&shots);
    let mean = g.iter().sum::<u32>() as f64 / g.len() as f64;
    println!("auto 12.857-tick interval: mean gap {mean:.4}");
    assert!((mean - 12.857).abs() < 0.01, "{mean}");
    assert!(g.iter().all(|x| *x == 12 || *x == 13));
}

#[test]
fn ads_takes_ads_ms_then_resets() {
    let mut r = Rig::new(GARAND, 6);
    let total = r.wp.ads_units;
    assert_eq!(total, 9000, "300 ms at 30 Hz");
    for i in 1..=12 {
        r.step(BTN_AIM);
        let p = r.w.ads_progress(total);
        if i < 9 {
            assert!(p < 1.0, "tick {i}: {p}");
        } else {
            assert_eq!(p, 1.0);
        }
    }
    r.step(0);
    assert_eq!(r.w.ads_progress(total), 0.0, "releasing aim resets");
    for _ in 0..12 {
        r.step(BTN_AIM);
    }
    r.step_cmd(cmd(0, 127, 0, BTN_AIM | BTN_SPRINT, 0));
    assert_eq!(r.w.ads_units, 0, "sprint drops ADS");
}

#[test]
fn recoil_accumulates_then_recovers_after_the_pause() {
    let mut r = Rig::new(GARAND, 6);
    r.step(BTN_FIRE);
    assert!(r.w.recoil_pitch_deg > 1.6 * 0.85 - 1e-3 && r.w.recoil_pitch_deg < 1.6 * 1.15 + 1e-3);
    assert!((r.w.bloom_deg - 0.4).abs() < 1e-6);
    let kicked = r.w.recoil_pitch_deg;
    // 0.12 s = 3.6 ticks without recovery: stays put on ticks 1-3 after the shot.
    r.step(0);
    r.step(0);
    r.step(0);
    assert_eq!(r.w.recoil_pitch_deg, kicked, "delay");
    r.step(0); // tick 4 after the shot: since_shot 4000 >= 3600, recovery starts
    assert!(r.w.recoil_pitch_deg < kicked);
    let mut ticks = 0;
    while r.w.recoil_pitch_deg > 0.0 || r.w.bloom_deg > 0.0 {
        r.step(0);
        ticks += 1;
        assert!(ticks < 100);
    }
    // 1.6 deg at 12 deg/s is ~4 ticks; 0.4 deg bloom at 3 deg/s needs 4 ticks.
    println!("recovery after the pause: {ticks} more ticks");
    assert!(ticks <= 6);
    // Crouching scales the kick by 0.75.
    let mut c = Rig::new(GARAND, 6);
    c.auth.stance = Stance::Crouch;
    c.step(BTN_FIRE);
    let mut s = Rig::new(GARAND, 6);
    s.step(BTN_FIRE);
    assert!((c.w.recoil_pitch_deg - 0.75 * s.w.recoil_pitch_deg).abs() < 1e-5);
}

fn angle_deg(a: frontline_sim::Vec3, b: frontline_sim::Vec3) -> f32 {
    libm::acosf(a.dot(b).clamp(-1.0, 1.0)).to_degrees()
}

#[test]
fn spread_hip_99_9_percent_inside_half_angle_and_ads_radius_at_100m() {
    use frontline_sim::weapon::spread::view_dir;
    let hip = frontline_sim::GameData::embedded()
        .weapon("us_m1_garand")
        .unwrap()
        .gameplay
        .spread
        .clone();
    // 10 000 blind (hip) shots, standing still, fresh weapon each time.
    let mut inside = 0;
    let mut max_angle = 0.0f32;
    for i in 0..10_000u32 {
        let mut r = Rig::new(GARAND, 6);
        r.tick = i;
        r.auth.yaw = 0.4;
        r.auth.pitch = 0.1;
        let out = r.step(BTN_FIRE);
        let shot = out.shot.expect("shot");
        let a = angle_deg(shot.dir, view_dir(0.4, 0.1));
        max_angle = max_angle.max(a);
        if a <= hip.hip_deg as f32 + 1e-3 {
            inside += 1;
        }
    }
    println!(
        "hip spread: {inside}/10000 within {} deg (max {max_angle:.3} deg)",
        hip.hip_deg
    );
    assert!(inside >= 9_990, "99.9 % within the half angle: {inside}");

    // ADS, standing still: 95 % of the impact points at 100 m within 0.35 m (data ads_deg 0.15).
    let mut radii = Vec::new();
    for i in 0..10_000u32 {
        let mut r = Rig::new(GARAND, 6);
        r.tick = i;
        for _ in 0..10 {
            r.step(BTN_AIM);
        }
        assert_eq!(r.w.ads_progress(r.wp.ads_units), 1.0);
        let out = r.step(BTN_AIM | BTN_FIRE);
        let d = out.shot.expect("ADS shot").dir;
        // Lateral miss distance of a straight line at 100 m measured perpendicular to the view axis.
        let ang = angle_deg(d, view_dir(0.0, 0.0)).to_radians();
        radii.push(100.0 * libm::tanf(ang));
    }
    radii.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95 = radii[(radii.len() as f32 * 0.95) as usize];
    println!(
        "ADS (0.15 deg) 100 m: 95 % radius {p95:.3} m, max {:.3} m",
        radii.last().unwrap()
    );
    assert!(p95 <= 0.35, "{p95}");
    assert!(*radii.last().unwrap() <= 100.0 * 0.15f32.to_radians().tan() + 1e-3);
}

#[test]
fn move_and_stance_scale_the_spread() {
    use frontline_sim::weapon::spread::view_dir;
    let maxang = |setup: &dyn Fn(&mut Rig)| {
        let mut m = 0.0f32;
        for i in 0..3000u32 {
            let mut r = Rig::new(GARAND, 6);
            r.tick = i;
            setup(&mut r);
            let a = r
                .step(BTN_FIRE)
                .shot
                .map(|s| angle_deg(s.dir, view_dir(0.0, 0.0)));
            m = m.max(a.unwrap());
        }
        m
    };
    let still = maxang(&|_| {});
    let walk = maxang(&|r| r.auth.vel = frontline_sim::Vec3::new(0.0, 0.0, 2.0));
    let run = maxang(&|r| r.auth.vel = frontline_sim::Vec3::new(0.0, 0.0, 4.4));
    let crouch = maxang(&|r| {
        r.auth.stance = Stance::Crouch;
        r.auth.stance_blend = 1.0;
    });
    println!("max spread angle: still {still:.2}, walk {walk:.2}, run {run:.2}, crouch {crouch:.2} deg (data 3.5 x 1 / 1.5 / 2.5 / 0.8)");
    assert!((still - 3.5).abs() < 0.1);
    assert!((walk - 5.25).abs() < 0.15);
    assert!((run - 8.75).abs() < 0.25);
    assert!((crouch - 2.8).abs() < 0.1);
}
