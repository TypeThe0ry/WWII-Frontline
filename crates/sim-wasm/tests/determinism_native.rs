//! M0-029 native determinism: same inputs, two independent runs, per-tick `state_hash` equal.
//! The 3 x 18 000-tick CI run is `examples/determinism.rs` (release); this is the `cargo test` form.
#[path = "support/determinism.rs"]
mod det;

use frontline_protocol::msg::input::InputCmd;

/// Final `Predictor.stateHash()`-equivalent after seed 1 / 18 000 ticks (native, rustc 1.97.0,
/// x86_64-linux). Re-pin only for a deliberate sim/data change.
const GOLDEN_SEED1_18000: u64 = 0xa9a1_b218_b112_f339;

#[test]
fn inputs_round_trip_and_are_stable() {
    let a = det::gen_inputs(1, 500);
    assert_eq!(a, det::gen_inputs(1, 500), "generator is deterministic");
    assert_ne!(a, det::gen_inputs(2, 500), "seeds differ");
    let b = det::inputs_to_bytes(&a);
    assert_eq!(b.len(), 4 + 18 * 500);
    assert_eq!(det::inputs_from_bytes(&b).unwrap(), a);
    assert!(det::inputs_from_bytes(&b[..b.len() - 1]).is_err());
    // Variety: all button kinds appear within a 10-minute sequence.
    let all = det::gen_inputs(1, det::TEN_MIN_TICKS);
    let or = all.iter().fold(0u16, |m, c| m | c.buttons);
    use frontline_protocol::msg::input::buttons::*;
    for bit in [JUMP, CROUCH, SPRINT, FIRE, RELOAD, AIM] {
        assert_eq!(or & bit, bit, "button {bit:#x} never pressed");
    }
}

#[test]
fn predict_native_x2_identical_18000_ticks_with_golden() {
    let Some(col) = det::terrain_or_skip() else {
        return;
    };
    let cmds = det::gen_inputs(1, det::TEN_MIN_TICKS);
    let r1 = det::run_predict(1, &col, &cmds, None);
    let junk: Vec<Vec<u8>> = (0..64).map(|i| vec![i as u8; 1000 + i * 37]).collect();
    let r2 = det::run_predict(1, &col, &cmds, None);
    drop(junk);
    assert_eq!(det::first_diff(&r1.hashes, &r2.hashes), None);
    assert_eq!(r1.hashes.len(), det::TEN_MIN_TICKS);
    assert!(
        r1.shots > 50,
        "run must exercise the weapon ({} shots)",
        r1.shots
    );
    assert_eq!(
        r1.hashes[det::TEN_MIN_TICKS - 1],
        GOLDEN_SEED1_18000,
        "golden changed"
    );
}

#[test]
fn world_ten_players_native_x2_identical() {
    let Some(col) = det::terrain_or_skip() else {
        return;
    };
    let (a, _) = det::run_world(2, &col, 1500);
    let (b, _) = det::run_world(2, &col, 1500);
    assert_eq!(det::first_diff(&a, &b), None);
}

/// Acceptance 5: the check must notice real non-determinism. The injected fault perturbs the
/// second run's yaw from wall-clock nanoseconds (a stand-in for `Instant::now()` leaking into state).
#[test]
fn injected_nondeterminism_is_detected() {
    let Some(col) = det::terrain_or_skip() else {
        return;
    };
    let cmds = det::gen_inputs(3, 1200);
    let clean = det::run_predict(3, &col, &cmds, None);
    let mut leak = |i: usize, c: &mut InputCmd| {
        if i == 700 {
            let ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos();
            c.yaw = c.yaw.wrapping_add(1 + (ns & 0x3ff) as u16);
        }
    };
    let faulty = det::run_predict(3, &col, &cmds, Some(&mut leak));
    assert_eq!(det::first_diff(&clean.hashes, &faulty.hashes), Some(700));
}
