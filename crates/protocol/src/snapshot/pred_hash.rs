//! `pred_hash` (ARCHITECTURE §6.5.3, PLAN_ISSUES PI-023): CRC-8 over the wire-quantised
//! position, so the server can tell "the client's prediction agrees with me" from one byte.
//!
//! Definition: codes `x = quant_pos_xz`, `y = quant_pos_y` (13 bit), `z = quant_pos_xz`
//! (step 1/32 m, clamped), serialised as 6 bytes `x lo, x hi, y lo, y hi, z lo, z hi` and
//! hashed with CRC-8 (polynomial 0x07, init 0, no reflection, no final xor). The client
//! currently has a TypeScript twin (`client/src/game/pred-hash.ts`); the golden values in the
//! tests below are the shared contract. Follow-up: export this function from `sim-wasm`
//! and replace the TS implementation (M0-025).

use crate::quant::{quant_pos_xz, quant_pos_y};
use crate::snapshot::entity::QPos;

/// CRC-8 (poly 0x07, init 0, no reflection, no xorout). Check value of `"123456789"`: 0xF4.
pub fn crc8(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &b in data {
        crc ^= b;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Hash of already quantised position codes.
pub fn pred_hash_q(p: QPos) -> u8 {
    let b = [
        p.x as u8,
        (p.x >> 8) as u8,
        p.y as u8,
        (p.y >> 8) as u8,
        p.z as u8,
        (p.z >> 8) as u8,
    ];
    crc8(&b)
}

/// Hash of a position in metres (quantised first; non-finite input hashes as code 0).
pub fn pred_hash(x: f32, y: f32, z: f32) -> u8 {
    pred_hash_q(QPos {
        x: quant_pos_xz(x).code,
        y: quant_pos_y(y).code,
        z: quant_pos_xz(z).code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc8_check_value() {
        assert_eq!(crc8(b"123456789"), 0xF4);
    }

    /// Same values as `client/src/game/pred-hash.test.ts`.
    #[test]
    fn golden_positions_match_the_client() {
        assert_eq!(pred_hash(0.0, 0.0, 0.0), 0);
        assert_eq!(pred_hash(128.0, 2.4, 56.0), 66);
        assert_eq!(pred_hash(100.40625, 12.0, 300.0), 175);
        assert_eq!(pred_hash(255.97, 200.0, 3.5), 151);
        assert_eq!(pred_hash(-5.0, 300.0, 1e9), 16);
        assert_eq!(
            pred_hash(f32::NAN, f32::INFINITY, f32::NEG_INFINITY),
            pred_hash(0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn one_cell_hashes_alike() {
        assert_eq!(pred_hash(10.001, 2.0, 20.001), pred_hash(10.0, 2.0, 20.0));
        assert_ne!(pred_hash(10.04, 2.0, 20.0), pred_hash(10.0, 2.0, 20.0));
    }
}
