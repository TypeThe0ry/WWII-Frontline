//! `self_state` (ARCHITECTURE §6.5.3): authoritative state of the receiving player.
//!
//! ```text
//! tag:4 [x:16 y:13 z:16 if tag & 1 != 0]
//! ammo_in_mag:8 | ammo_reserve:12 | health:8 | stamina:6 | suppression:4 | stance:3 | tail:11
//! ```
//! On foot, `tag >> 1` is 0 and the tail is `deploy_state:3 | cooldown_bits:8` (unchanged
//! from v3). Seated, `tag >> 1` is `1 + seat * 2 + slot_bit_6` (1..=6) and the tail is
//! `slot_low:6 | cooldown_bits:5`. Only the five defined weapon bits are sent seated, and
//! `deploy_state` is implicitly 0 (the current server value). A seated slot has bit 7 set.
//! 56 bits (7 B) normally, 101 bits (13 B) with a position correction.

use crate::bitio::{BitReader, BitWriter};
use crate::codec::CodecError;
use crate::snapshot::entity::QPos;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelfState {
    /// Present only when the client's `pred_hash` did not match.
    pub pos_corr: Option<QPos>,
    pub ammo_in_mag: u8,
    /// 0..=4095
    pub ammo_reserve: u16,
    pub health: u8,
    /// 0..=63
    pub stamina: u8,
    /// 0..=15
    pub suppression: u8,
    /// 0..=7
    pub stance: u8,
    /// 0..=7
    pub deploy_state: u8,
    pub cooldown_bits: u8,
    /// Vehicle wire ID, 128..=255; 0 when on foot.
    pub vehicle_slot: u8,
    /// 0 driver, 1 gunner, 2 commander; 0 (ignored) on foot.
    pub seat: u8,
}

impl SelfState {
    pub fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        let (seat_tag, tail) = match self.vehicle_slot {
            0 if self.seat == 0 => (
                0,
                (u16::from(self.deploy_state) << 8) | u16::from(self.cooldown_bits),
            ),
            128..=255 if self.seat <= 2 && self.deploy_state == 0 && self.cooldown_bits < 32 => {
                let slot_bit_6 = (self.vehicle_slot >> 6) & 1;
                let slot_low = self.vehicle_slot & 63;
                (
                    1 + self.seat * 2 + slot_bit_6,
                    (u16::from(slot_low) << 5) | u16::from(self.cooldown_bits),
                )
            }
            _ => return Err(CodecError::InvalidValue),
        };
        w.write_bits(
            u64::from(seat_tag << 1) | u64::from(self.pos_corr.is_some()),
            4,
        )?;
        if let Some(p) = self.pos_corr {
            p.write_abs(w)?;
        }
        w.write_bits(self.ammo_in_mag as u64, 8)?;
        w.write_bits(self.ammo_reserve as u64, 12)?;
        w.write_bits(self.health as u64, 8)?;
        w.write_bits(self.stamina as u64, 6)?;
        w.write_bits(self.suppression as u64, 4)?;
        w.write_bits(self.stance as u64, 3)?;
        w.write_bits(u64::from(tail), 11)
    }

    pub fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        let tag = r.read_bits(4)? as u8;
        let seat_tag = tag >> 1;
        if seat_tag == 7 {
            return Err(CodecError::InvalidTag);
        }
        let pos_corr = if tag & 1 != 0 {
            Some(QPos::read_abs(r)?)
        } else {
            None
        };
        let mut state = SelfState {
            pos_corr,
            ammo_in_mag: r.read_bits(8)? as u8,
            ammo_reserve: r.read_bits(12)? as u16,
            health: r.read_bits(8)? as u8,
            stamina: r.read_bits(6)? as u8,
            suppression: r.read_bits(4)? as u8,
            stance: r.read_bits(3)? as u8,
            ..SelfState::default()
        };
        let tail = r.read_bits(11)? as u16;
        if seat_tag == 0 {
            state.deploy_state = (tail >> 8) as u8;
            state.cooldown_bits = tail as u8;
        } else {
            let code = seat_tag - 1;
            state.seat = code >> 1;
            state.vehicle_slot = 128 | ((code & 1) << 6) | (tail >> 5) as u8;
            state.cooldown_bits = (tail & 31) as u8;
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vehicle_seat_round_trips_with_and_without_correction() {
        for pos_corr in [None, Some(QPos { x: 1, y: 2, z: 3 })] {
            for vehicle_slot in 128..=255 {
                for seat in 0..=2 {
                    let state = SelfState {
                        pos_corr,
                        ammo_in_mag: 255,
                        ammo_reserve: 4095,
                        health: 200,
                        stamina: 63,
                        suppression: 15,
                        stance: 2,
                        cooldown_bits: 31,
                        vehicle_slot,
                        seat,
                        ..SelfState::default()
                    };
                    let mut writer = BitWriter::new();
                    state.encode(&mut writer).unwrap();
                    assert_eq!(
                        SelfState::decode(&mut BitReader::new(writer.as_bytes())),
                        Ok(state)
                    );
                }
            }
        }
    }

    #[test]
    fn on_foot_keeps_v3_bits() {
        let state = SelfState {
            ammo_in_mag: 0x12,
            ammo_reserve: 0x345,
            health: 0x67,
            stamina: 0x22,
            suppression: 0xA,
            stance: 0x5,
            deploy_state: 0x3,
            cooldown_bits: 0xCD,
            ..SelfState::default()
        };
        let mut writer = BitWriter::new();
        state.encode(&mut writer).unwrap();
        assert_eq!(
            writer.as_bytes(),
            [0x01, 0x23, 0x45, 0x67, 0x8A, 0xAB, 0xCD]
        );
        assert_eq!(
            SelfState::decode(&mut BitReader::new(writer.as_bytes())),
            Ok(state)
        );
    }

    #[test]
    fn rejects_unrepresentable_seated_state_and_reserved_tags() {
        for (vehicle_slot, seat, deploy_state, cooldown_bits) in [
            (127, 0, 0, 0),
            (0, 1, 0, 0),
            (128, 3, 0, 0),
            (128, 0, 1, 0),
            (128, 0, 0, 32),
        ] {
            let state = SelfState {
                vehicle_slot,
                seat,
                deploy_state,
                cooldown_bits,
                ..SelfState::default()
            };
            assert_eq!(
                state.encode(&mut BitWriter::new()),
                Err(CodecError::InvalidValue)
            );
        }
        for first_byte in [0xE0, 0xF0] {
            assert_eq!(
                SelfState::decode(&mut BitReader::new(&[first_byte])),
                Err(CodecError::InvalidTag)
            );
        }
    }
}
