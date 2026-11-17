//! Quantisation helpers (ARCHITECTURE §6.3, §6.5.2).
//!
//! All quantisers clamp, then round to nearest. Non-finite input (NaN/Inf) maps to code 0 and
//! is flagged by [`Quant::valid`] being `false`. Angles are in radians.
//!
//! * position x/z: 16 bit, step 1/32 m, range `[0, 2047.96875]` m
//! * position y: 13 bit, step 1/32 m, range `[0, 255.96875]` m
//! * yaw: circular, `bits` bits over `[0, 2π)` (wraps instead of clamping)
//! * pitch: `bits` bits over `[-π/2, π/2]` (both ends representable)
//! * unit: `bits` bits over `[0, 1]`

use core::f32::consts::{FRAC_PI_2, PI, TAU};

/// Position step in metres (1/32 m).
pub const POS_STEP: f32 = 1.0 / 32.0;
pub const POS_XZ_BITS: u8 = 16;
pub const POS_Y_BITS: u8 = 13;
/// Wire yaw widths.
pub const YAW_BITS_SNAPSHOT: u8 = 10;
pub const YAW_BITS_COMPACT: u8 = 8;
/// Entity pitch in snapshots (ARCHITECTURE §6.5.2; remote-player visual only).
pub const PITCH_BITS_SNAPSHOT: u8 = 6;
/// Pitch in aim-critical wire data (event `Shot.dir`): 9 bit, max error 0.176 deg. PI-001
/// ruling: 8 bit (0.353 deg) exceeds the 0.35 deg acceptance threshold.
pub const PITCH_BITS_EVENT: u8 = 9;

/// Result of a quantiser: the code and whether the input was a finite number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct Quant<T> {
    pub code: T,
    /// `false` if the input was NaN/Inf (code is then 0).
    pub valid: bool,
}

fn q<T>(code: T, valid: bool) -> Quant<T> {
    Quant { code, valid }
}

fn max_code(bits: u8) -> u32 {
    debug_assert!((1..=16).contains(&bits));
    (1u32 << bits.clamp(1, 16)) - 1
}

fn quant_linear(v: f32, lo: f32, hi: f32, bits: u8) -> Quant<u16> {
    if !v.is_finite() {
        return q(0, false);
    }
    let m = max_code(bits) as f32;
    let t = (v.clamp(lo, hi) - lo) / (hi - lo);
    q((t * m + 0.5) as u16, true)
}

fn dequant_linear(code: u16, lo: f32, hi: f32, bits: u8) -> f32 {
    let m = max_code(bits);
    let c = (code as u32).min(m) as f32;
    lo + c / m as f32 * (hi - lo)
}

/// Fractional part in `[0, 1)` (no libm in `no_std`).
fn frac(t: f32) -> f32 {
    let i = t as i64 as f32;
    let fl = if i > t { i - 1.0 } else { i };
    let f = t - fl;
    if (0.0..1.0).contains(&f) {
        f
    } else {
        0.0
    }
}

fn quant_pos(v: f32, bits: u8) -> Quant<u16> {
    if !v.is_finite() {
        return q(0, false);
    }
    let max = max_code(bits);
    let c = (v / POS_STEP + 0.5).clamp(0.0, max as f32);
    q(c as u16, true)
}

/// x/z position -> 16-bit code (1/32 m).
pub fn quant_pos_xz(v: f32) -> Quant<u16> {
    quant_pos(v, POS_XZ_BITS)
}

pub fn dequant_pos_xz(code: u16) -> f32 {
    code as f32 * POS_STEP
}

/// y position -> 13-bit code (1/32 m, 0..256 m).
pub fn quant_pos_y(v: f32) -> Quant<u16> {
    quant_pos(v, POS_Y_BITS)
}

pub fn dequant_pos_y(code: u16) -> f32 {
    (code & (max_code(POS_Y_BITS) as u16)) as f32 * POS_STEP
}

/// Yaw (radians, any finite value; wrapped into `[0, 2π)`) -> `bits`-bit code (1..=16).
pub fn quant_yaw(v: f32, bits: u8) -> Quant<u16> {
    if !v.is_finite() {
        return q(0, false);
    }
    let n = 1u32 << bits.clamp(1, 16);
    let t = frac(v / TAU);
    let c = (t * n as f32 + 0.5) as u32 % n;
    q(c as u16, true)
}

pub fn dequant_yaw(code: u16, bits: u8) -> f32 {
    let n = 1u32 << bits.clamp(1, 16);
    (code as u32 % n) as f32 / n as f32 * TAU
}

/// Pitch (radians, clamped to ±π/2) -> `bits`-bit code (1..=16), code 0 = -π/2.
pub fn quant_pitch(v: f32, bits: u8) -> Quant<u16> {
    quant_linear(v, -FRAC_PI_2, FRAC_PI_2, bits)
}

pub fn dequant_pitch(code: u16, bits: u8) -> f32 {
    dequant_linear(code, -FRAC_PI_2, FRAC_PI_2, bits)
}

/// Value in `[0, 1]` (clamped) -> `bits`-bit code (1..=16).
pub fn quant_unit(v: f32, bits: u8) -> Quant<u16> {
    quant_linear(v, 0.0, 1.0, bits)
}

pub fn dequant_unit(code: u16, bits: u8) -> f32 {
    dequant_linear(code, 0.0, 1.0, bits)
}

/// `InputCmd.pitch` (`i16`, -32768..32767) -> radians (±π/2).
pub fn pitch_i16_to_rad(p: i16) -> f32 {
    (p as f32 + 0.5) / 32767.5 * FRAC_PI_2
}

/// Radians -> `InputCmd.pitch` (`i16`). Non-finite -> 0 with `valid == false`.
pub fn rad_to_pitch_i16(v: f32) -> Quant<i16> {
    if !v.is_finite() {
        return q(0, false);
    }
    let x = v.clamp(-FRAC_PI_2, FRAC_PI_2) / FRAC_PI_2 * 32767.5 - 0.5;
    // round-to-nearest without libm: shift into positive range, truncate, shift back
    let c = ((x + 32768.5) as i32 - 32768).clamp(-32768, 32767);
    q(c as i16, true)
}

/// Degrees helper used by tests/consumers.
pub fn rad_to_deg(r: f32) -> f32 {
    r * (180.0 / PI)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nan_inf_flagged() {
        for v in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                quant_pos_xz(v),
                Quant {
                    code: 0,
                    valid: false
                }
            );
            assert_eq!(
                quant_pos_y(v),
                Quant {
                    code: 0,
                    valid: false
                }
            );
            assert_eq!(
                quant_yaw(v, 10),
                Quant {
                    code: 0,
                    valid: false
                }
            );
            assert_eq!(
                quant_pitch(v, 8),
                Quant {
                    code: 0,
                    valid: false
                }
            );
            assert_eq!(
                quant_unit(v, 8),
                Quant {
                    code: 0,
                    valid: false
                }
            );
            assert!(!rad_to_pitch_i16(v).valid);
        }
    }

    #[test]
    fn clamping() {
        assert_eq!(quant_pos_xz(-5.0).code, 0);
        assert_eq!(quant_pos_xz(1e9).code, 65535);
        assert_eq!(quant_pos_y(1e9).code, 8191);
        assert_eq!(quant_pos_y(-1.0).code, 0);
        assert_eq!(quant_pitch(10.0, 8).code, 255);
        assert_eq!(quant_pitch(-10.0, 8).code, 0);
        assert_eq!(quant_unit(2.0, 6).code, 63);
        assert_eq!(quant_unit(-2.0, 6).code, 0);
        assert!(quant_pos_xz(1e9).valid);
    }

    #[test]
    fn yaw_wraps() {
        assert_eq!(quant_yaw(0.0, 10).code, 0);
        assert_eq!(quant_yaw(TAU, 10).code, 0);
        assert_eq!(quant_yaw(-TAU / 4.0, 10).code, 768);
        assert_eq!(quant_yaw(TAU - 1e-7, 10).code, 0);
    }

    #[test]
    fn pitch_i16_range() {
        assert_eq!(rad_to_pitch_i16(FRAC_PI_2).code, 32767);
        assert_eq!(rad_to_pitch_i16(-FRAC_PI_2).code, -32768);
        assert!(pitch_i16_to_rad(0).abs() < 1e-4);
    }
}
