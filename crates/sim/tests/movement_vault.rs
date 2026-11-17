//! M1-011b: vault / climb detection and timing, lean, head offset, spread multiplier.
mod common;

use common::*;
use frontline_protocol::msg::input::buttons;
use frontline_sim::movement::{lean, step_player_ex, MoveCtx};
use frontline_sim::{step_player, CollisionData, MoveParams, PlayerAuthState, Stance, Vec3};

const YAW_EAST: u16 = 16384; // facing +x
const USE: u16 = buttons::USE;

/// Flat ground at 0 for `x <= 9`, a plateau of height `h` for `x >= 10` (1 m steep face between).
fn step_terrain(h: f32) -> CollisionData {
    synth_data(2, 2, &move |x, _| if x <= 9.0 { 0.0 } else { h })
}

fn east(buttons: u16) -> frontline_protocol::msg::input::InputCmd {
    cmd(0, 127, YAW_EAST, buttons, 0)
}

fn settle_at(col: &CollisionData, p: &MoveParams, x: f32) -> PlayerAuthState {
    let mut s = settle(col, p, x, 20.0);
    s.yaw = std::f32::consts::FRAC_PI_2;
    s
}

/// Walks east holding `btn` until a vault starts; returns the state of the first vault tick
/// and the number of ticks the vault takes (including that first tick).
fn vault_run(h: f32, btn: u16) -> Option<(PlayerAuthState, u32)> {
    let col = step_terrain(h);
    let p = move_params();
    let mut s = settle_at(&col, &p, 5.0);
    // Walk up to the face (the steep slope stops the character), then try to vault.
    for _ in 0..150 {
        s = step_player(&s, &east(0), &p, &col);
        assert!(s.vault.is_none());
    }
    for _ in 0..20 {
        s = step_player(&s, &east(btn), &p, &col);
        if s.vault.is_some() {
            let first = s;
            let mut n = 1;
            while s.vault.is_some() {
                s = step_player(&s, &east(0), &p, &col);
                n += 1;
                assert!(n < 100);
            }
            assert!(s.grounded, "landed grounded");
            assert!(s.pos.x > 10.0, "ended beyond the face: {:?}", s.pos);
            return Some((first, n));
        }
    }
    None
}

#[test]
fn vault_boundaries_and_timing() {
    let p = move_params();
    // 1.29 m: vault, 0.8 s = 24 ticks (+-1), cost 10.
    let (first, n) = vault_run(1.29, BTN_JUMP).expect("1.29 m vaults");
    let v = first.vault.unwrap();
    assert!(!v.climb);
    assert_eq!(v.total, 24);
    assert!((i64::from(n) - 24).abs() <= 1, "vault ticks {n}");
    // 1.31 m: climb, 1.4 s = 42 ticks, cost 20.
    let (first, n) = vault_run(1.31, USE).expect("1.31 m climbs");
    let v = first.vault.unwrap();
    assert!(v.climb);
    assert_eq!(v.total, 42);
    assert!((i64::from(n) - 42).abs() <= 1, "climb ticks {n}");
    // 1.81 m: refused.
    assert!(vault_run(1.81, BTN_JUMP).is_none(), "1.81 m is refused");
    // Low steps are autostep / jump business.
    assert!(vault_run(0.25, BTN_JUMP).is_none());
    // Costs.
    let (f, _) = vault_run(1.29, BTN_JUMP).unwrap();
    assert!(
        (p.stamina_max - f.stamina - p.vault_cost).abs() < 0.2,
        "vault cost, stamina {}",
        f.stamina
    );
    let (f, _) = vault_run(1.5, BTN_JUMP).unwrap();
    assert!(
        (p.stamina_max - f.stamina - p.climb_cost).abs() < 0.2,
        "climb cost, stamina {}",
        f.stamina
    );
}

#[test]
fn vault_needs_stamina_and_forward_input_and_blocks_fire() {
    let col = step_terrain(1.0);
    let p = move_params();
    // Exhausted: no vault.
    let mut s = settle_at(&col, &p, 8.5);
    s.stamina = 5.0;
    s.sprint_locked = true;
    for _ in 0..10 {
        s = step_player(&s, &east(BTN_JUMP), &p, &col);
        assert!(s.vault.is_none());
    }
    // Standing at the face without forward input: no vault.
    let mut s = settle_at(&col, &p, 8.5);
    for _ in 0..10 {
        s = step_player(&s, &cmd(0, 0, YAW_EAST, BTN_JUMP, 0), &p, &col);
        assert!(s.vault.is_none());
    }
    // Crouched: no vault.
    let mut s = settle_at(&col, &p, 8.5);
    for _ in 0..10 {
        s = step_player(&s, &east(BTN_JUMP | BTN_CROUCH), &p, &col);
        assert!(s.vault.is_none());
    }
    // A running vault leaves the position along the scripted path, deterministic twice.
    let a = vault_run(1.0, BTN_JUMP).unwrap().0;
    let b = vault_run(1.0, BTN_JUMP).unwrap().0;
    assert!(a.bit_eq(&b));
    // The vault state survives the wire format.
    assert_eq!(PlayerAuthState::from_bytes(&a.to_bytes()), Some(a));
    // The path rises above the top at its apex.
    let v = a.vault.unwrap();
    let mid = frontline_sim::movement::vault::path(&v, v.total / 2);
    assert!(mid.y >= v.top + 0.09, "apex {} top {}", mid.y, v.top);
}

#[test]
fn lean_timing_head_offset_and_spread() {
    let col = flat(2, 5.0);
    let p = move_params();
    let s0 = settle(&col, &p, 20.0, 20.0);
    let lean_r = cmd(0, 0, 0, buttons::LEAN_R, 0);
    let lean_l = cmd(0, 0, 0, buttons::LEAN_L, 0);
    // 0.15 s = 4.5 ticks: full lean after 5 ticks (+-1), partial after 3.
    let mut s = s0;
    let mut reach = 0;
    for i in 1..=10 {
        s = step_player(&s, &lean_r, &p, &col);
        if i == 3 {
            assert!(s.lean > 0.0 && s.lean < 1.0);
        }
        if reach == 0 && s.lean >= 1.0 {
            reach = i;
        }
    }
    assert!((4..=6).contains(&reach), "lean reached at tick {reach}");
    assert_eq!(s.lean, 1.0);
    assert_eq!(lean::angle_deg(&s, &p), 20.0);
    // Head 0.35 m to the right (yaw 0 faces +z, right is +x).
    let h = lean::head_offset(&s, &p);
    assert!(
        (h.x - 0.35).abs() < 1e-4 && h.y == 0.0 && h.z.abs() < 1e-4,
        "{h:?}"
    );
    // Left lean mirrors; releasing returns to upright.
    let mut l = s0;
    for _ in 0..10 {
        l = step_player(&l, &lean_l, &p, &col);
    }
    assert_eq!(l.lean, -1.0);
    assert!((lean::head_offset(&l, &p).x + 0.35).abs() < 1e-4);
    for _ in 0..10 {
        s = step_player(&s, &cmd(0, 0, 0, 0, 0), &p, &col);
    }
    assert_eq!(s.lean, 0.0);
    assert_eq!(lean::head_offset(&s, &p), Vec3::ZERO);
    // Spread multiplier x1.10 while leaning.
    assert_eq!(lean::spread_mult(&s, &p), 1.0);
    assert!((lean::spread_mult(&l, &p) - 1.10).abs() < 1e-6);
    // Both buttons cancel.
    let mut b = s0;
    for _ in 0..10 {
        b = step_player(
            &b,
            &cmd(0, 0, 0, buttons::LEAN_L | buttons::LEAN_R, 0),
            &p,
            &col,
        );
    }
    assert_eq!(b.lean, 0.0);
}

#[test]
fn lean_excludes_sprint_and_prone_only_peeks() {
    let col = flat(2, 5.0);
    let p = move_params();
    let s0 = settle(&col, &p, 20.0, 20.0);
    // Sprint + lean: runs (4.4), not sprints (6.4).
    let mut s = s0;
    let mut sprint = s0;
    for _ in 0..90 {
        s = step_player(&s, &fwd(BTN_SPRINT | buttons::LEAN_R, 0), &p, &col);
        sprint = step_player(&sprint, &fwd(BTN_SPRINT, 0), &p, &col);
    }
    let hspeed = |s: &PlayerAuthState| libm::sqrtf(s.vel.x * s.vel.x + s.vel.z * s.vel.z);
    assert!((hspeed(&s) - 4.4).abs() < 0.1, "lean+sprint {}", hspeed(&s));
    assert!(hspeed(&sprint) > 6.0);
    // Prone: peek raises the head 0.15 m, no sideways displacement.
    let mut pr = s0;
    for _ in 0..40 {
        pr = step_player(&pr, &cmd(0, 0, 0, buttons::PRONE, 0), &p, &col);
    }
    assert_eq!(pr.stance, Stance::Prone);
    for _ in 0..10 {
        pr = step_player(
            &pr,
            &cmd(0, 0, 0, buttons::PRONE | buttons::LEAN_R, 0),
            &p,
            &col,
        );
    }
    let h = lean::head_offset(&pr, &p);
    assert!(
        (h.y - 0.15).abs() < 1e-4 && h.x == 0.0 && h.z == 0.0,
        "{h:?}"
    );
    let _ = (step_player_ex, MoveCtx::default());
}

#[test]
fn vaulting_blocks_fire_and_leaning_widens_spread() {
    let mut r = Rig::new("us_m1_garand", 2);
    let fire = cmd(0, 0, 0, BTN_FIRE, 0);
    // Ready and upright: shoots.
    assert!(r.step_cmd(fire).shot.is_some());
    // Vaulting: no shot, even with the trigger released and pulled again.
    let mut r = Rig::new("us_m1_garand", 2);
    r.auth.vault = Some(frontline_sim::movement::Vault {
        ticks_left: 5,
        total: 24,
        climb: false,
        start: Vec3::ZERO,
        target: Vec3::X,
        top: 1.0,
    });
    assert!(r.step_cmd(fire).shot.is_none());
    r.step_cmd(cmd(0, 0, 0, 0, 0));
    assert!(r.step_cmd(fire).shot.is_none());
    r.auth.vault = None;
    r.step_cmd(cmd(0, 0, 0, 0, 0));
    assert!(r.step_cmd(fire).shot.is_some());
}
