//! `movement_replay_10k`: 10 000 steps with random jump / crouch / sprint inputs are
//! bit-identical across two runs (and across threads); golden trajectory hash.
mod common;

use common::*;
use frontline_sim::{step_player, CollisionData, MoveParams, PlayerAuthState, Vec3};

/// Triangle wave in `[0, 1]` with period `period` (exact float arithmetic only).
fn tri(v: f32, period: f32) -> f32 {
    let t = v.rem_euclid(period) / period;
    1.0 - (2.0 * t - 1.0).abs()
}

/// Hilly synthetic terrain, gentle enough to walk everywhere, with a few steep ridges.
fn hills() -> CollisionData {
    synth_data(3, 3, &|x, z| {
        4.0 + 3.0 * tri(x, 37.0) + 2.0 * tri(z, 29.0) + 1.5 * tri(x + z, 11.0)
    })
}

fn script(n: usize, seed: u64) -> Vec<frontline_protocol::msg::input::InputCmd> {
    let mut r = Rng(seed);
    let mut cur = cmd(0, 0, 0, 0, 0);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        // Hold each random command for 1 s (> the 0.25 s ramp), like a player would.
        if i % 30 == 0 {
            let b = r.next_u64();
            let mut buttons = 0;
            if b & 3 == 0 {
                buttons |= BTN_JUMP;
            }
            if b >> 2 & 7 == 0 {
                buttons |= BTN_CROUCH;
            }
            if b >> 5 & 3 == 0 {
                buttons |= BTN_SPRINT;
            }
            cur = cmd(
                ((r.next_u64() % 255) as i32 - 127) as i8,
                ((r.next_u64() % 255) as i32 - 127) as i8,
                (r.next_u64() & 0xffff) as u16,
                buttons,
                (b >> 8 & 1) as u8,
            );
        }
        out.push(cur);
    }
    out
}

fn run(
    col: &CollisionData,
    p: &MoveParams,
    cmds: &[frontline_protocol::msg::input::InputCmd],
) -> Vec<PlayerAuthState> {
    let mut s = PlayerAuthState::spawn(
        Vec3::new(96.0, col.height_at(96.0, 96.0).unwrap() + 0.5, 96.0),
        0.0,
    );
    cmds.iter()
        .map(|c| {
            s = step_player(&s, c, p, col);
            s
        })
        .collect()
}

fn fold(states: &[PlayerAuthState]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for s in states {
        for b in s.to_bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

/// Regression pin; update only when movement rules or data change on purpose (and say why).
/// M0-012: re-pinned (0x0427_5f5f_f85f_ebd8 before) because ground movement now ramps linearly
/// (accel 0.25 s / decel 0.20 s from data/roles/rifleman.json), and the script holds each command
/// 30 ticks instead of 7 so that the player still travels.
const GOLDEN: u64 = 0x1fe0_0bf7_4d6b_9efa;

#[test]
fn movement_replay_10k() {
    let col = hills();
    let p = move_params();
    let cmds = script(10_000, 0x5eed);
    let a = run(&col, &p, &cmds);
    // Second run on a rebuilt collision world, on another thread.
    let cmds2 = cmds.clone();
    let b = std::thread::spawn(move || run(&hills(), &move_params(), &cmds2))
        .join()
        .unwrap();
    let same = a.iter().zip(&b).filter(|(x, y)| x.bit_eq(y)).count();
    assert_eq!(same, 10_000, "{same}/10000 steps bit-identical");
    let blocked = a.iter().filter(|s| s.blocked).count();
    let airborne = a.iter().filter(|s| !s.grounded).count();
    let crouched = a.iter().filter(|s| s.stance_blend > 0.0).count();
    let (minx, maxx) = a.iter().fold((f32::MAX, f32::MIN), |m, s| {
        (m.0.min(s.pos.x), m.1.max(s.pos.x))
    });
    println!(
        "10k replay: hash {:#018x}, blocked {blocked}, airborne {airborne}, crouching {crouched}, x range {minx:.1}..{maxx:.1}",
        fold(&a)
    );
    assert!(
        airborne > 500 && crouched > 500,
        "script must exercise jumps and crouching"
    );
    assert!(maxx - minx > 20.0, "player must travel");
    assert!(a.iter().all(|s| s.pos.is_finite()));
    assert_eq!(fold(&a), GOLDEN, "golden movement trajectory changed");
}

#[test]
fn movement_replay_10k_real_terrain() {
    let Some(col) = real_terrain("movement_replay_10k_real_terrain") else {
        return;
    };
    let p = move_params();
    let cmds = script(10_000, 0xbeef);
    let a = run(&col, &p, &cmds);
    let b = run(&col.clone(), &p, &cmds);
    assert!(a.iter().zip(&b).all(|(x, y)| x.bit_eq(y)));
    assert!(a.iter().all(|s| s.pos.is_finite()));
    // Never below the surface, never above the playable box.
    for s in &a {
        if let Some(h) = col.height_at(s.pos.x, s.pos.z) {
            assert!(s.pos.y >= h - 0.05, "below terrain: {s:?} vs {h}");
        }
    }
}
