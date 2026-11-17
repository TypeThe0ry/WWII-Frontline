//! Speeds (`speed_matches_data`), stance transitions, blocking, input decoding.
mod common;

use common::*;
use frontline_sim::movement::{capsule_height, decode_move, normalize_move, AUX_WALK};
use frontline_sim::{step_player, GameData, Stance, Vec3};

fn horizontal_speed(
    col: &frontline_sim::CollisionData,
    mk: &dyn Fn() -> frontline_protocol::msg::input::InputCmd,
    warmup: u32,
) -> f32 {
    let p = move_params();
    let mut s = settle(col, &p, 20.0, 20.0);
    for _ in 0..warmup {
        s = step_player(&s, &mk(), &p, col);
    }
    let start = s.pos;
    for _ in 0..90 {
        s = step_player(&s, &mk(), &p, col);
        assert!(s.grounded && !s.blocked);
    }
    let d = s.pos - start;
    libm::sqrtf(d.x * d.x + d.z * d.z) / 3.0
}

#[test]
fn speed_matches_data() {
    let col = flat(2, 5.0);
    let role = GameData::embedded().role("rifleman").unwrap();
    let m = &role.movement;
    let cases: [(&str, f64, u16, u8); 5] = [
        ("walk", m.walk_mps, 0, AUX_WALK),
        ("run", m.run_mps, 0, 0),
        ("sprint", m.sprint_mps, BTN_SPRINT, 0),
        ("crouch_run", m.crouch_mps, BTN_CROUCH, 0),
        ("crouch_walk", m.crouch_walk_mps.unwrap(), BTN_CROUCH, AUX_WALK),
    ];
    for (name, want, btn, aux) in cases {
        let got = horizontal_speed(&col, &|| fwd(btn, aux), 10) as f64;
        let err = (got - want).abs() / want;
        println!("speed {name}: measured {got:.4} m/s, data {want} m/s, error {:.4} %", err * 100.0);
        assert!(err < 0.02, "{name}: {got} vs {want}");
    }
    // Strafing and diagonals are not faster than straight running.
    let diag = horizontal_speed(&col, &|| cmd(127, 127, 0, 0, 0), 10) as f64;
    assert!((diag - m.run_mps).abs() / m.run_mps < 0.02, "diagonal {diag}");
    // Sprint needs a forward component: backwards + sprint runs at run speed.
    let back = horizontal_speed(&col, &|| cmd(0, -127, 0, BTN_SPRINT, 0), 10) as f64;
    assert!((back - m.run_mps).abs() / m.run_mps < 0.02, "backwards sprint {back}");
}

#[test]
fn heading_follows_yaw() {
    let col = flat(2, 5.0);
    let p = move_params();
    // yaw 0 faces +z, a quarter turn (16384) faces +x.
    for (yaw, dx, dz) in [(0u16, 0.0f32, 1.0f32), (16384, 1.0, 0.0), (32768, 0.0, -1.0)] {
        let mut s = settle(&col, &p, 60.0, 60.0);
        let start = s.pos;
        for _ in 0..30 {
            s = step_player(&s, &cmd(0, 127, yaw, 0, 0), &p, &col);
        }
        let d = s.pos - start;
        assert!((d.x - dx * 4.4).abs() < 0.05 && (d.z - dz * 4.4).abs() < 0.05, "{yaw}: {d:?}");
    }
    // +x is to the right of a player facing +z.
    let mut s = settle(&col, &p, 60.0, 60.0);
    let start = s.pos;
    for _ in 0..30 {
        s = step_player(&s, &cmd(127, 0, 0, 0, 0), &p, &col);
    }
    assert!((s.pos.x - start.x - 4.4).abs() < 0.05);
}

#[test]
fn accel_and_stop_are_immediate() {
    let col = flat(2, 5.0);
    let p = move_params();
    let mut s = settle(&col, &p, 20.0, 20.0);
    s = step_player(&s, &fwd(0, 0), &p, &col);
    println!("first-tick speed {:.3} m/s (instant accel)", s.vel.z);
    assert!((s.vel.z - 4.4).abs() < 0.01);
    s = step_player(&s, &cmd(0, 0, 0, 0, 0), &p, &col);
    assert!(s.vel.x.abs() + s.vel.z.abs() < 1e-3, "stops in one tick");
}

#[test]
fn stance_transition_takes_eight_ticks() {
    let col = flat(2, 5.0);
    let p = move_params();
    assert_eq!(p.stance_ticks, 7.5);
    let mut s = settle(&col, &p, 20.0, 20.0);
    let crouch = cmd(0, 0, 0, BTN_CROUCH, 0);
    let mut done_at = None;
    for t in 1..=12 {
        s = step_player(&s, &crouch, &p, &col);
        if done_at.is_none() && s.stance_timer == 0 {
            done_at = Some(t);
        }
        // Linear blend: capsule height never jumps by more than 1/7.5 of the delta.
    }
    assert_eq!(done_at, Some(8), "250 ms at 30 Hz = 7.5 ticks -> completes on tick 8");
    assert_eq!(s.stance, Stance::Crouch);
    assert_eq!(s.stance_blend, 1.0);
    assert_eq!(capsule_height(s.stance_blend, &p), 1.2);
    // Back up: also 8 ticks; reversing mid-way continues from the current blend.
    let stand = cmd(0, 0, 0, 0, 0);
    let mut s2 = s;
    for _ in 0..3 {
        s2 = step_player(&s2, &stand, &p, &col);
    }
    assert!(s2.stance_blend > 0.5 && s2.stance_blend < 0.7, "{}", s2.stance_blend);
    let mid = s2.stance_blend;
    s2 = step_player(&s2, &crouch, &p, &col);
    assert!(s2.stance_blend > mid, "reversal is continuous");
    for t in 1..=8 {
        s = step_player(&s, &stand, &p, &col);
        if t < 8 {
            assert!(s.stance_timer > 0, "tick {t}");
        }
    }
    assert_eq!((s.stance_timer, s.stance_blend), (0, 0.0));
    // Foot stays on the ground while the capsule changes height.
    let y = col.height_at(s.pos.x, s.pos.z).unwrap();
    assert!((s.pos.y - y).abs() < 0.03, "foot {} vs ground {y}", s.pos.y);
}

#[test]
fn unloaded_terrain_blocks() {
    let col = flat(1, 5.0); // chunk (0,0) only: x, z in [0, 64]
    let p = move_params();
    let mut s = settle(&col, &p, 32.0, 50.0);
    for _ in 0..300 {
        s = step_player(&s, &fwd(0, 0), &p, &col);
    }
    assert!(s.blocked, "ran off the loaded area: {s:?}");
    assert!(s.pos.z <= 64.0 && s.pos.z > 63.0 - 0.01 - 4.4 / 30.0, "{:?}", s.pos);
    let before = s.pos;
    let again = step_player(&s, &fwd(0, 0), &p, &col);
    assert_eq!(again.pos, before, "blocked state keeps its position");
    // Turning around lets the player walk back in.
    let mut back = again;
    for _ in 0..5 {
        back = step_player(&back, &cmd(0, 127, 32768, 0, 0), &p, &col);
    }
    assert!(!back.blocked && back.pos.z < before.z);
    // No collision at all: nothing moves.
    let empty = frontline_sim::CollisionData::empty();
    let s0 = frontline_sim::PlayerAuthState::spawn(Vec3::new(1.0, 2.0, 3.0), 0.0);
    let s1 = step_player(&s0, &fwd(0, 0), &p, &empty);
    assert!(s1.blocked);
    assert_eq!(s1.pos, s0.pos);
}

#[test]
fn input_decoding() {
    assert_eq!(normalize_move(0.0, 0.0), (0.0, 0.0));
    assert_eq!(normalize_move(f32::NAN, f32::INFINITY), (0.0, 0.0));
    assert_eq!(normalize_move(f32::NAN, 0.5), (0.0, 0.5));
    assert_eq!(normalize_move(5.0, 0.0), (1.0, 0.0));
    let (x, z) = normalize_move(1.0, 1.0);
    assert!((libm::sqrtf(x * x + z * z) - 1.0).abs() < 1e-6);
    let (x, z) = normalize_move(0.3, 0.4);
    assert_eq!((x, z), (0.3, 0.4));
    // -128 is invalid on the wire; it must still be handled as -127.
    let c = cmd(-128, -128, 0, 0, 0);
    let (x, z) = decode_move(&c);
    assert!((libm::sqrtf(x * x + z * z) - 1.0).abs() < 1e-6);
    assert!(x < 0.0 && z < 0.0);
}
