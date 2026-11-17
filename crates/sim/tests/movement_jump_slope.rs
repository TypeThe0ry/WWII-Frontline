//! Jump height / landing and the 35 degree slope limit of `step_player`.
mod common;

use common::*;
use frontline_sim::{step_player, CollisionData, PlayerAuthState, Vec3};

#[test]
fn jump_peak_and_landing() {
    let col = flat(2, 5.0);
    let p = move_params();
    let data_h = 0.6f32;
    // From standing still and while running.
    for (label, mv) in [("standing", 0i8), ("running", 127)] {
        let mut s = settle(&col, &p, 20.0, 20.0);
        let ground = s.pos.y;
        let mut peak = ground;
        let mut airborne = 0;
        s = step_player(&s, &cmd(0, mv, 0, BTN_JUMP, 0), &p, &col);
        assert!(!s.grounded, "{label}: left the ground");
        for _ in 0..60 {
            s = step_player(&s, &cmd(0, mv, 0, 0, 0), &p, &col);
            peak = peak.max(s.pos.y);
            if !s.grounded {
                airborne += 1;
            }
            assert!(!s.blocked);
        }
        let h = peak - ground;
        println!(
            "jump {label}: peak {h:.4} m (data {data_h} m, error {:.2} %), airborne {} ticks, landed grounded={} y-ground={:.4}",
            (h - data_h).abs() / data_h * 100.0,
            airborne + 1,
            s.grounded,
            s.pos.y - ground
        );
        assert!((h - data_h).abs() / data_h < 0.05, "{label}: {h}");
        assert!(s.grounded, "{label}: landed");
        assert!((s.pos.y - ground).abs() < 0.03);
        if mv != 0 {
            assert!(s.vel.z > 4.0, "ran after landing");
        }
    }
}

#[test]
fn jump_keeps_horizontal_velocity_and_limits_air_control() {
    let col = flat(2, 5.0);
    let p = move_params();
    let mut s = settle(&col, &p, 20.0, 20.0);
    for _ in 0..5 {
        s = step_player(&s, &cmd(0, 127, 0, 0, 0), &p, &col);
    }
    s = step_player(&s, &cmd(0, 127, 0, BTN_JUMP, 0), &p, &col);
    let vz = s.vel.z;
    // Let go of the stick: velocity is kept in the air.
    for _ in 0..3 {
        s = step_player(&s, &cmd(0, 0, 0, 0, 0), &p, &col);
        assert!(!s.grounded);
        assert!((s.vel.z - vz).abs() < 1e-3, "no friction in the air");
    }
    // Steering sideways closes only air_control (0.3) of the gap per tick.
    let before = s.vel.x;
    s = step_player(&s, &cmd(127, 0, 0, 0, 0), &p, &col);
    let want = 4.4 * p.air_control;
    assert!((s.vel.x - before - want).abs() < 0.05, "{}", s.vel.x);
}

#[test]
fn no_jump_while_crouching() {
    let col = flat(2, 5.0);
    let p = move_params();
    let mut s = settle(&col, &p, 20.0, 20.0);
    for _ in 0..10 {
        s = step_player(&s, &cmd(0, 0, 0, BTN_CROUCH | BTN_JUMP, 0), &p, &col);
    }
    assert!(s.grounded && s.vel.y == 0.0);
}

fn ramp(deg: f32, rise: f32) -> CollisionData {
    let k = deg.to_radians().tan();
    synth_data(2, 2, &move |_, z| 2.0 + (k * (z - 40.0)).clamp(0.0, rise))
}

/// Runs `sec` seconds along `dir` (yaw code) with the run button profile; returns final state
/// and the mean achieved horizontal speed over the last `tail` ticks.
fn run_dir(col: &CollisionData, start: (f32, f32), yaw: u16, ticks: u32) -> (PlayerAuthState, f32) {
    let p = move_params();
    let mut s = settle(col, &p, start.0, start.1);
    let mut acc = 0.0;
    let mut n = 0;
    for t in 0..ticks {
        s = step_player(&s, &cmd(0, 127, yaw, 0, 0), &p, col);
        if t >= ticks / 2 && s.grounded {
            acc += libm::sqrtf(s.vel.x * s.vel.x + s.vel.z * s.vel.z);
            n += 1;
        }
    }
    (s, if n > 0 { acc / n as f32 } else { 0.0 })
}

#[test]
fn slope_limit_35deg_step_player() {
    for deg in [36.0f32, 38.7, 45.0, 55.0, 70.0] {
        let col = ramp(deg, 3.0);
        let (s, _) = run_dir(&col, (32.0, 30.0), 0, 150);
        let gain = s.pos.y - 2.0;
        println!(
            "run into {deg} deg ramp for 5 s: height gain {gain:.3} m, z {:.2}",
            s.pos.z
        );
        assert!(gain <= 0.5, "{deg} deg climbed: {gain}");
    }
    for deg in [10.0f32, 20.0, 25.0, 30.0, 34.0] {
        let rise = (14.0 * deg.to_radians().tan()).min(4.0);
        let col = ramp(deg, rise);
        // Measure on the slope only: 5 s run over a ramp of >= 14 m run length.
        let p = move_params();
        let mut s = settle(&col, &p, 32.0, 38.0);
        let mut acc = 0.0;
        let mut n = 0;
        for _ in 0..150 {
            s = step_player(&s, &cmd(0, 127, 0, 0, 0), &p, &col);
            if s.pos.z > 41.0 && s.pos.z < 40.0 + rise / deg.to_radians().tan() - 1.0 {
                acc += libm::sqrtf(s.vel.x * s.vel.x + s.vel.z * s.vel.z);
                n += 1;
            }
        }
        let v = acc / n as f32;
        let gain = s.pos.y - 2.0;
        println!(
            "{deg} deg ramp: horizontal speed on slope {v:.3} m/s = {:.1} % of 4.4, height gain {gain:.2} of {rise:.2} m",
            v / 4.4 * 100.0
        );
        assert!(n > 20, "not enough slope samples ({n})");
        // Acceptance 5 covers <= 30 degrees; 34 is recorded (speed ~ cos^2), it must still climb.
        if deg <= 30.0 {
            assert!(v >= 0.7 * 4.4, "{deg}: speed {v}");
        }
        assert!(gain >= 0.9 * rise, "{deg}: gain {gain} of {rise}");
    }
}

#[test]
fn slope_limit_on_m0_terrain() {
    let Some(col) = real_terrain("slope_limit_on_m0_terrain") else {
        return;
    };
    // The 38.7 degree, 8 m embankment over z = 123..133.
    let (s, _) = run_dir(&col, (128.0, 108.0), 0, 150);
    let base = col.height_at(128.0, 118.0).unwrap();
    println!(
        "M0 embankment 38.7 deg, 5 s run: height above slope foot {:.3} m, z {:.2}",
        s.pos.y - base,
        s.pos.z
    );
    assert!(s.pos.y - base <= 0.5);
    // Gentle side along +x (<= 27 degrees): climbs, foot follows terrain.
    let (s, v) = run_dir(&col, (10.0, 128.0), 16384, 150);
    let start = col.height_at(10.0, 128.0).unwrap();
    let end = col.height_at(s.pos.x, s.pos.z).unwrap();
    println!(
        "M0 gentle side: climbed {:.2} m, mean ground speed {v:.2} m/s",
        s.pos.y - start
    );
    assert!(end - start > 2.0);
    assert!(v >= 0.7 * 4.4);
    assert!((s.pos.y - end).abs() < 0.2);
    let _ = Vec3::ZERO;
}
