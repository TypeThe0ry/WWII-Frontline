//! Golden bot-input hash: 10 bots fight for 900 ticks (30 s) on the M0 terrain; the FNV-1a hash
//! of every command they emit is pinned. A change means bot behaviour changed - if that is
//! intended (tuning in `data/rules/bots.json`, new behaviour, sim rule change), update `GOLDEN`
//! in the same commit and say why.

mod common;

use common::*;
use frontline_sim::Team;

// M1-014: re-pinned (0x2021_804c_0223_6403 before): friendly fire and the helmet rule change bullet outcomes.
const GOLDEN: u64 = 0xfbcd_31e9_0ff9_45c7;

fn fold(mut h: u64, v: u64) -> u64 {
    h ^= v;
    h.wrapping_mul(0x0000_0100_0000_01b3)
}

pub fn run_hash(seed: u64, ticks: u32) -> Option<u64> {
    let col = terrain()?;
    let mut a = Arena::new(&col, seed);
    let map = a.bot_map();
    for i in 0..10u8 {
        let (team, x, z) = if i % 2 == 0 {
            (Team::Us, 110.0 + i as f32 * 2.0, 60.0)
        } else {
            (Team::Ger, 110.0 + i as f32 * 2.0, 196.0)
        };
        a.add_bot(
            &map,
            i,
            team,
            x,
            z,
            if team == Team::Us {
                0.0
            } else {
                std::f32::consts::PI
            },
        );
    }
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for _ in 0..ticks {
        for (p, c) in a.step() {
            for v in [
                u64::from(p),
                u64::from(c.seq),
                u64::from(c.tick),
                (c.move_x as u8 as u64) | (c.move_z as u8 as u64) << 8,
                u64::from(c.yaw) | (c.pitch as u16 as u64) << 16,
                u64::from(c.buttons) | u64::from(c.aux) << 16,
            ] {
                h = fold(h, v);
            }
        }
    }
    Some(h)
}

#[test]
fn bot_input_hash_is_pinned() {
    let Some(h) = run_hash(77, 900) else { return };
    let again = run_hash(77, 900).unwrap();
    assert_eq!(h, again, "two identical runs disagree");
    assert_ne!(
        h,
        run_hash(78, 900).unwrap(),
        "the hash must depend on the match seed"
    );
    assert_eq!(h, GOLDEN, "bot behaviour changed: new golden = {h:#018x}");
}
