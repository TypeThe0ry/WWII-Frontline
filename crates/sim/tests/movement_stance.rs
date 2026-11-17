//! M1-011a: prone, stance transition table, stamina, speed modifiers, validation interface.
mod common;

use common::*;
use frontline_protocol::msg::input::buttons;
use frontline_sim::movement::{check_displacement, modifiers, step_player_ex, MoveCtx, AUX_WALK};
use frontline_sim::{step_player, GameData, MoveParams, PlayerAuthState, Stance};

const BTN_PRONE: u16 = buttons::PRONE;
const BTN_AIM_: u16 = buttons::AIM;

fn role() -> &'static frontline_sim::data::RoleDef {
    GameData::embedded().role("rifleman").unwrap()
}

fn run(
    col: &frontline_sim::CollisionData,
    p: &MoveParams,
    mut s: PlayerAuthState,
    ticks: u32,
    mk: &dyn Fn() -> frontline_protocol::msg::input::InputCmd,
    ctx: &MoveCtx,
) -> PlayerAuthState {
    for _ in 0..ticks {
        s = step_player_ex(&s, &mk(), p, col, ctx);
    }
    s
}

fn avg_speed(
    col: &frontline_sim::CollisionData,
    mk: &dyn Fn() -> frontline_protocol::msg::input::InputCmd,
    ctx: &MoveCtx,
) -> f32 {
    let p = move_params();
    let s = settle(col, &p, 20.0, 20.0);
    let s = run(
        col,
        &p,
        s,
        60,
        &|| cmd(0, 0, 0, mk().buttons & !buttons::SPRINT, 0),
        ctx,
    );
    let s = run(col, &p, s, 20, mk, ctx);
    let start = s.pos;
    let s = run(col, &p, s, 90, mk, ctx);
    let d = s.pos - start;
    libm::sqrtf(d.x * d.x + d.z * d.z) / 3.0
}

#[test]
fn movement_speeds() {
    let col = flat(2, 5.0);
    let m = &role().movement;
    let none = MoveCtx::default();
    let reload = MoveCtx {
        reloading: true,
        load_kg: None,
    };
    // (name, expected, buttons, aux, ctx)
    let cases: Vec<(&str, f64, u16, u8, MoveCtx)> = vec![
        ("walk", m.walk_mps, 0, AUX_WALK, none),
        ("run", m.run_mps, 0, 0, none),
        ("sprint", m.sprint_mps, BTN_SPRINT, 0, none),
        ("crouch_run", m.crouch_mps, BTN_CROUCH, 0, none),
        (
            "crouch_walk",
            m.crouch_walk_mps.unwrap(),
            BTN_CROUCH,
            AUX_WALK,
            none,
        ),
        ("prone_slow", m.prone_mps, BTN_PRONE, 0, none),
        (
            "prone_fast",
            m.prone_fast_mps.unwrap(),
            BTN_PRONE | BTN_SPRINT,
            0,
            none,
        ),
        (
            "ads_stand",
            m.run_mps * m.ads_stand_mult.unwrap(),
            BTN_AIM_,
            0,
            none,
        ),
        (
            "ads_crouch",
            m.crouch_mps * m.ads_crouch_mult.unwrap(),
            BTN_CROUCH | BTN_AIM_,
            0,
            none,
        ),
        (
            "ads_prone",
            m.prone_mps * m.ads_prone_mult.unwrap(),
            BTN_PRONE | BTN_AIM_,
            0,
            none,
        ),
        (
            "reload_run",
            m.run_mps * m.reload_move_mult.unwrap(),
            0,
            0,
            reload,
        ),
        (
            "reload_crouch",
            m.crouch_mps * m.reload_move_mult.unwrap(),
            BTN_CROUCH,
            0,
            reload,
        ),
    ];
    assert!(cases.len() >= 12);
    for (name, want, btn, aux, ctx) in cases {
        let got = f64::from(avg_speed(&col, &|| fwd(btn, aux), &ctx));
        let err = (got - want).abs() / want;
        println!("speed {name}: {got:.4} vs {want:.4} ({:.3} %)", err * 100.0);
        assert!(err < 0.02, "{name}: {got} vs {want}");
    }
}

#[test]
fn load_modifier_table() {
    let p = move_params();
    assert_eq!(modifiers::load_mult(10.0, &p), 1.0);
    assert_eq!(modifiers::load_mult(0.0, &p), 1.0);
    assert!((modifiers::load_mult(20.0, &p) - 0.88).abs() < 1e-6);
    assert_eq!(modifiers::load_mult(100.0, &p), p.load_min_mult);
    let col = flat(2, 5.0);
    let heavy = MoveCtx {
        reloading: false,
        load_kg: Some(20.0),
    };
    let got = f64::from(avg_speed(&col, &|| fwd(0, 0), &heavy));
    let want = f64::from(p.run_mps) * 0.88;
    assert!((got - want).abs() / want < 0.02, "{got} vs {want}");
}

/// Ticks until the stance settles after commanding `to` from a settled `from`.
fn transition_ticks(from: Stance, to: Stance) -> u32 {
    let col = flat(2, 5.0);
    let p = move_params();
    let btn = |s: Stance| match s {
        Stance::Stand => 0,
        Stance::Crouch => BTN_CROUCH,
        Stance::Prone => BTN_PRONE,
    };
    let mut s = settle(&col, &p, 20.0, 20.0);
    s = run(
        &col,
        &p,
        s,
        100,
        &|| cmd(0, 0, 0, btn(from), 0),
        &MoveCtx::default(),
    );
    assert_eq!((s.stance, s.stance_timer), (from, 0));
    let mut n = 0;
    loop {
        s = step_player(&s, &cmd(0, 0, 0, btn(to), 0), &p, &col);
        n += 1;
        if s.stance_timer == 0 {
            break;
        }
        assert!(n < 100);
    }
    assert!((s.stance_blend - to.pose()).abs() < 1e-5);
    n
}

#[test]
fn stance_transition_times() {
    use Stance::*;
    let p = move_params();
    let table = [
        (Stand, Crouch, 0.25),
        (Crouch, Stand, 0.25),
        (Crouch, Prone, 0.50),
        (Prone, Crouch, 0.60),
        (Stand, Prone, 0.70),
        (Prone, Stand, 0.90),
    ];
    for (a, b, secs) in table {
        let got = transition_ticks(a, b);
        let want = (secs * f64::from(p.hz)).ceil() as u32;
        println!("{a:?} -> {b:?}: {got} ticks (GD {secs} s = {want})");
        assert!(got.abs_diff(want) <= 1, "{a:?}->{b:?}: {got} vs {want}");
    }
}

#[test]
fn stance_reversal_never_jumps_capsule() {
    let col = flat(2, 5.0);
    let p = move_params();
    let mut s = settle(&col, &p, 20.0, 20.0);
    let mut last = s.stance_blend;
    // stand -> prone for 10 ticks, then back up, then down again.
    for (btn, n) in [
        (BTN_PRONE, 10),
        (0, 12),
        (BTN_CROUCH, 15),
        (BTN_PRONE, 20),
        (0, 40),
    ] {
        for _ in 0..n {
            s = step_player(&s, &cmd(0, 0, 0, btn, 0), &p, &col);
            assert!((s.stance_blend - last).abs() <= 2.0 / p.ticks_s2p.min(p.ticks_c2p) + 1e-4);
            assert!((0.0..=2.0).contains(&s.stance_blend));
            last = s.stance_blend;
        }
    }
    assert_eq!(s.stance, Stance::Stand);
    assert_eq!(s.stance_blend, 0.0);
}

#[test]
fn stamina_sprint_exhausts_in_five_seconds_then_locks() {
    let col = flat(2, 5.0);
    let p = move_params();
    let mut s = settle(&col, &p, 20.0, 20.0);
    assert_eq!(s.stamina, p.stamina_max);
    let mut ticks: u32 = 0;
    while !s.sprint_locked {
        s = step_player(&s, &fwd(BTN_SPRINT, 0), &p, &col);
        ticks += 1;
        assert!(ticks < 400);
    }
    let want = (p.stamina_max / p.sprint_cost_per_s * p.hz) as u32;
    println!("sprint exhausted after {ticks} ticks (expected {want})");
    assert!(ticks.abs_diff(want) <= 1);
    assert_eq!(s.stamina, 0.0);
    // Locked: sprint button gives run speed; jump refused.
    let v0 = s.pos;
    let s2 = run(&col, &p, s, 60, &|| fwd(BTN_SPRINT, 0), &MoveCtx::default());
    let d = s2.pos - v0;
    let speed = libm::sqrtf(d.x * d.x + d.z * d.z) / 2.0;
    assert!(speed < p.sprint_mps * 0.9, "still sprinting at {speed}");
    // Still locked after regenerating to just below 30, unlocked once >= 30.
    let mut t = s;
    let mut locked_until = None;
    for i in 0..600 {
        t = step_player(&t, &cmd(0, 0, 0, 0, 0), &p, &col);
        if locked_until.is_none() && !t.sprint_locked {
            locked_until = Some(i);
            assert!(t.stamina >= p.stamina_unlock, "{}", t.stamina);
            break;
        }
        assert!(t.stamina < p.stamina_unlock);
    }
    assert!(locked_until.is_some());
}

#[test]
fn stamina_regen_rates() {
    let col = flat(2, 5.0);
    let p = move_params();
    for (btn, stance, mult) in [
        (0u16, Stance::Stand, 1.0f32),
        (BTN_CROUCH, Stance::Crouch, 1.2),
        (BTN_PRONE, Stance::Prone, 1.4),
    ] {
        let mut s = settle(&col, &p, 20.0, 20.0);
        s.stamina = 40.0;
        s.stamina_delay = 0;
        // settle the stance first (stamina refills meanwhile, so re-pin it)
        s = run(
            &col,
            &p,
            s,
            60,
            &|| cmd(0, 0, 0, btn, 0),
            &MoveCtx::default(),
        );
        assert_eq!(s.stance, stance);
        s.stamina = 40.0;
        s.stamina_delay = 0;
        let s = run(
            &col,
            &p,
            s,
            30,
            &|| cmd(0, 0, 0, btn, 0),
            &MoveCtx::default(),
        );
        let rate = (s.stamina - 40.0) / 1.0; // 30 ticks = 1 s
        let want = 12.0 * mult;
        println!("{stance:?}: regen {rate:.3}/s (want {want})");
        assert!((rate - want).abs() / want < 0.02);
    }
    // 1 s delay after spending.
    let mut s = settle(&col, &p, 20.0, 20.0);
    s = step_player(&s, &fwd(BTN_SPRINT, 0), &p, &col);
    let after_spend = s.stamina;
    for _ in 0..(p.stamina_regen_delay_ticks - 1) {
        s = step_player(&s, &cmd(0, 0, 0, 0, 0), &p, &col);
        assert_eq!(s.stamina, after_spend, "no regen during the delay");
    }
    let s = run(&col, &p, s, 3, &|| cmd(0, 0, 0, 0, 0), &MoveCtx::default());
    assert!(s.stamina > after_spend);
}

#[test]
fn jump_costs_stamina_and_is_locked_when_exhausted() {
    let col = flat(2, 5.0);
    let p = move_params();
    let s = settle(&col, &p, 20.0, 20.0);
    let j = step_player(&s, &cmd(0, 0, 0, BTN_JUMP, 0), &p, &col);
    assert!(!j.grounded);
    assert!((j.stamina - (p.stamina_max - p.jump_cost)).abs() < 1e-3);
    let mut e = s;
    e.stamina = 0.0;
    e.sprint_locked = true;
    let j = step_player(&e, &cmd(0, 0, 0, BTN_JUMP, 0), &p, &col);
    assert!(j.grounded, "no jump while exhausted");
}

#[test]
fn quantised_stamina() {
    let p = move_params();
    let mut s = PlayerAuthState::spawn(frontline_sim::Vec3::ZERO, 0.0);
    assert_eq!(frontline_sim::movement::stamina_quantised(&s, &p), 63);
    s.stamina = 0.0;
    assert_eq!(frontline_sim::movement::stamina_quantised(&s, &p), 0);
    s.stamina = 50.0;
    assert_eq!(frontline_sim::movement::stamina_quantised(&s, &p), 32);
}

#[test]
fn stance_tables_match_gamedesign() {
    let p = move_params();
    use Stance::*;
    for (s, rec, spr, vis, h) in [
        (Stand, 1.0, 1.0, 1.0, 1.8),
        (Crouch, 0.75, 0.8, 0.85, 1.2),
        (Prone, 0.5, 0.6, 0.65, 0.5),
    ] {
        assert_eq!(modifiers::stance_recoil(s, &p), rec);
        assert_eq!(modifiers::stance_spread(s, &p), spr);
        assert_eq!(modifiers::visibility(s, &p), vis);
        assert_eq!(modifiers::hitbox_height(s, &p), h);
    }
    // The weapon module keeps its own copies of recoil/spread: they must agree with the data.
    assert_eq!(frontline_sim::weapon::recoil::stance_recoil(Prone), 0.5);
    assert_eq!(frontline_sim::weapon::spread::stance_spread(Prone), 0.6);
    assert_eq!(modifiers::move_spread(0.0, &p), Some(1.0));
    assert_eq!(modifiers::move_spread(p.walk_mps, &p), Some(1.5));
    assert_eq!(modifiers::move_spread(p.run_mps, &p), Some(2.5));
    assert_eq!(modifiers::move_spread(p.sprint_mps, &p), None);
}

#[test]
fn check_displacement_flags_1_3x_not_1_1x() {
    let p = move_params();
    let mut rng = Rng(42);
    for stance in [Stance::Stand, Stance::Crouch, Stance::Prone] {
        let vmax = frontline_sim::movement::max_speed_for(stance, &p);
        let mut flagged = 0;
        let mut false_pos = 0;
        for _ in 0..1000 {
            let ticks = 10;
            let dir = rng.range(0.0, core::f32::consts::TAU);
            let jitter = rng.range(-0.03, 0.03);
            let from = frontline_sim::Vec3::new(10.0, 3.0, 10.0);
            let mk = |k: f32| {
                let d = vmax * k * ticks as f32 * p.dt;
                from + frontline_sim::Vec3::new(d * dir.cos(), 0.0, d * dir.sin())
            };
            if check_displacement(from, mk(1.30 + jitter.abs()), ticks, stance, stance, &p).is_err()
            {
                flagged += 1;
            }
            if check_displacement(from, mk(1.10 + jitter), ticks, stance, stance, &p).is_err() {
                false_pos += 1;
            }
        }
        assert_eq!(flagged, 1000, "{stance:?}");
        assert_eq!(false_pos, 0, "{stance:?}");
    }
    assert!(check_displacement(
        frontline_sim::Vec3::ZERO,
        frontline_sim::Vec3::new(f32::NAN, 0.0, 0.0),
        5,
        Stance::Stand,
        Stance::Stand,
        &p
    )
    .is_err());
}

#[test]
fn prone_capsule_shrinks_and_fits_under_low_ceiling_rule() {
    let p = move_params();
    use frontline_sim::movement::{capsule_height, eye_height};
    assert_eq!(capsule_height(0.0, &p), p.height_stand_m);
    assert_eq!(capsule_height(1.0, &p), p.height_crouch_m);
    assert_eq!(capsule_height(2.0, &p), p.height_prone_m);
    assert!((eye_height(2.0, &p) - p.eye_height_prone_m).abs() < 1e-6);
    assert!(capsule_height(1.5, &p) < p.height_crouch_m);
}
