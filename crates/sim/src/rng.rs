//! Deterministic RNG derivation (ARCHITECTURE 8.1 rule 3).
//!
//! Every random draw is derived from `(match_seed, tick, entity, purpose)` through a fixed
//! splitmix64-style 64-bit mix, then seeds a `Pcg64Mcg`. Changing [`mix64`] or the order of
//! the mixed fields changes every golden hash and must be done deliberately.

use crate::ids::{EntityId, Tick};
pub use rand_core::Rng as RngCore;
pub use rand_pcg::Pcg64Mcg;

/// Why a random number is drawn; part of the stream key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Purpose {
    Spread = 1,
    Respawn = 2,
    Bot = 3,
    Misc = 4,
    /// Exit deflection of a penetrating shot (M1-012).
    Penetration = 5,
}

const GOLDEN: u64 = 0x9e37_79b9_7f4a_7c15;

/// splitmix64 finaliser applied to `z + GOLDEN`.
#[inline]
pub fn mix64(z: u64) -> u64 {
    let mut z = z.wrapping_add(GOLDEN);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Namespace for [`Rng::for_`].
pub struct Rng;

impl Rng {
    /// Derives the stream for one `(tick, entity, purpose)` triple.
    pub fn for_(match_seed: u64, tick: Tick, entity: EntityId, purpose: Purpose) -> Pcg64Mcg {
        let mut h = mix64(match_seed);
        h = mix64(h ^ tick.0 as u64);
        h = mix64(h ^ entity as u64);
        h = mix64(h ^ purpose as u64);
        let lo = mix64(h ^ GOLDEN);
        Pcg64Mcg::new(((h as u128) << 64) | lo as u128)
    }
}

/// Uniform `f32` in `[0, 1)` using the top 24 bits.
#[inline]
pub fn unit_f32(rng: &mut Pcg64Mcg) -> f32 {
    (rng.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(seed: u64, t: u32, e: EntityId, p: Purpose) -> [u32; 4] {
        let mut r = Rng::for_(seed, Tick(t), e, p);
        [r.next_u32(), r.next_u32(), r.next_u32(), r.next_u32()]
    }

    #[test]
    fn same_params_same_stream() {
        assert_eq!(draw(1, 2, 3, Purpose::Misc), draw(1, 2, 3, Purpose::Misc));
    }

    #[test]
    fn different_params_different_stream() {
        let base = draw(1, 2, 3, Purpose::Misc);
        assert_ne!(base, draw(2, 2, 3, Purpose::Misc));
        assert_ne!(base, draw(1, 3, 3, Purpose::Misc));
        assert_ne!(base, draw(1, 2, 4, Purpose::Misc));
        assert_ne!(base, draw(1, 2, 3, Purpose::Spread));
    }

    #[test]
    fn golden_stream() {
        // Algorithm pin: update deliberately (with reason) if mix64/field order changes.
        assert_eq!(
            draw(0x1234_5678_9abc_def0, 100, 7, Purpose::Spread),
            GOLDEN_STREAM
        );
    }
    const GOLDEN_STREAM: [u32; 4] = [1245919636, 787409520, 993946965, 2789436446];

    #[test]
    fn unit_in_range() {
        let mut r = Rng::for_(9, Tick(0), 0, Purpose::Misc);
        for _ in 0..1000 {
            let v = unit_f32(&mut r);
            assert!((0.0..1.0).contains(&v));
        }
    }
}
