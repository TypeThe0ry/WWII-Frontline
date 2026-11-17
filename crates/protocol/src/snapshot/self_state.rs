//! `self_state` (ARCHITECTURE §6.5.3): authoritative state of the receiving player.
//!
//! ```text
//! pos_corr_tag:4 (0 = ok, 1 = correction follows) [x:16 y:13 z:16]
//! ammo_in_mag:8 | ammo_reserve:12 | health:8 | stamina:6 | suppression:4 | stance:3 |
//! deploy_state:3 | cooldown_bits:8
//! ```
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
}

impl SelfState {
    pub fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        match self.pos_corr {
            None => w.write_bits(0, 4)?,
            Some(p) => {
                w.write_bits(1, 4)?;
                p.write_abs(w)?;
            }
        }
        w.write_bits(self.ammo_in_mag as u64, 8)?;
        w.write_bits(self.ammo_reserve as u64, 12)?;
        w.write_bits(self.health as u64, 8)?;
        w.write_bits(self.stamina as u64, 6)?;
        w.write_bits(self.suppression as u64, 4)?;
        w.write_bits(self.stance as u64, 3)?;
        w.write_bits(self.deploy_state as u64, 3)?;
        w.write_bits(self.cooldown_bits as u64, 8)
    }

    pub fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        let pos_corr = match r.read_bits(4)? {
            0 => None,
            1 => Some(QPos::read_abs(r)?),
            _ => return Err(CodecError::InvalidTag),
        };
        Ok(SelfState {
            pos_corr,
            ammo_in_mag: r.read_bits(8)? as u8,
            ammo_reserve: r.read_bits(12)? as u16,
            health: r.read_bits(8)? as u8,
            stamina: r.read_bits(6)? as u8,
            suppression: r.read_bits(4)? as u8,
            stance: r.read_bits(3)? as u8,
            deploy_state: r.read_bits(3)? as u8,
            cooldown_bits: r.read_bits(8)? as u8,
        })
    }
}
