//! Comms types shared by C0 placement and C2 marker events.

use crate::{BitReader, BitWriter, Codec, CodecError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PingKind {
    Attention = 0,
    Enemy = 1,
    NeedMedic = 2,
    NeedAmmo = 3,
    NeedEngineer = 4,
    NeedAt = 5,
    NeedSupplies = 6,
    Thanks = 7,
}

impl PingKind {
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Attention,
            1 => Self::Enemy,
            2 => Self::NeedMedic,
            3 => Self::NeedAmmo,
            4 => Self::NeedEngineer,
            5 => Self::NeedAt,
            6 => Self::NeedSupplies,
            7 => Self::Thanks,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PingPlace {
    pub kind: PingKind,
    /// Normalized x/z: `map_origin + code / 65535 * map_extent`, as in `MarkerPlace`.
    pub pos: [u16; 2],
    /// Advisory input only. The server resolves targets from authoritative state.
    pub target: u16,
}

impl Codec for PingPlace {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.kind as u64, 8)?;
        w.write_bits(self.pos[0] as u64, 16)?;
        w.write_bits(self.pos[1] as u64, 16)?;
        w.write_bits(self.target as u64, 16)
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            kind: PingKind::from_u8(r.read_bits(8)? as u8).ok_or(CodecError::InvalidValue)?,
            pos: [r.read_bits(16)? as u16, r.read_bits(16)? as u16],
            target: r.read_bits(16)? as u16,
        })
    }
}
