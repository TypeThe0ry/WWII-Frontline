//! Snapshot packet header (ARCHITECTURE §6.5.1, refined: `flags` moved directly after `type`
//! so the reader knows the width of `server_tick`).
//!
//! ```text
//! type:4 (=1) | flags:4 | server_tick: 32 (FULL_TICK) or 16 (low 16 bits) |
//! ack_input_seq:16 | input_slack:4 | baseline_tick_delta:5
//! ```
//!
//! * `baseline_tick_delta` stores `delta - 1`, so a baseline 1..=32 ticks old is expressible.
//!   It is only meaningful when `FLAG_HAS_BASELINE` is set.
//! * The 16-bit tick form is extended by the decoder to the value nearest to the newest tick
//!   it has decoded. The encoder only uses it once the client has acknowledged something.
//! * `flags`: bit0 `FULL_TICK`, bit1 `CRC` (CRC-16 trailer present), bit2 `HAS_SELF`,
//!   bit3 `HAS_BASELINE` (at least one delta record refers to the baseline).

use crate::bitio::{BitReader, BitWriter};
use crate::codec::CodecError;

pub const PKT_TYPE_SNAPSHOT: u8 = 1;
pub const FLAG_FULL_TICK: u8 = 1;
pub const FLAG_CRC: u8 = 2;
pub const FLAG_HAS_SELF: u8 = 4;
pub const FLAG_HAS_BASELINE: u8 = 8;

/// Header bits for a 32-bit / 16-bit tick.
pub const fn header_bits(full_tick: bool) -> usize {
    4 + 4 + if full_tick { 32 } else { 16 } + 16 + 4 + 5
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub flags: u8,
    pub tick: u32,
    pub ack_input_seq: u16,
    pub input_slack: u8,
    /// Raw 5-bit field: baseline age minus one.
    pub baseline_raw: u8,
}

impl Header {
    pub fn write(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(PKT_TYPE_SNAPSHOT as u64, 4)?;
        w.write_bits(self.flags as u64, 4)?;
        if self.flags & FLAG_FULL_TICK != 0 {
            w.write_bits(self.tick as u64, 32)?;
        } else {
            w.write_bits((self.tick & 0xFFFF) as u64, 16)?;
        }
        w.write_bits(self.ack_input_seq as u64, 16)?;
        w.write_bits(self.input_slack as u64, 4)?;
        w.write_bits(self.baseline_raw as u64, 5)
    }

    /// `last_tick`: newest tick the decoder has seen (needed for the 16-bit tick form).
    pub fn read(r: &mut BitReader<'_>, last_tick: Option<u32>) -> Result<Header, CodecError> {
        if r.read_bits(4)? != PKT_TYPE_SNAPSHOT as u64 {
            return Err(CodecError::InvalidTag);
        }
        let flags = r.read_bits(4)? as u8;
        let tick = if flags & FLAG_FULL_TICK != 0 {
            r.read_bits(32)? as u32
        } else {
            let low = r.read_bits(16)? as u16;
            let last = last_tick.ok_or(CodecError::MissingBaseline)?;
            let d = low.wrapping_sub(last as u16) as i16;
            last.wrapping_add(d as i32 as u32)
        };
        Ok(Header {
            flags,
            tick,
            ack_input_seq: r.read_bits(16)? as u16,
            input_slack: r.read_bits(4)? as u8,
            baseline_raw: r.read_bits(5)? as u8,
        })
    }

    /// Tick of the baseline this packet's delta records refer to.
    pub fn baseline_tick(&self) -> Option<u32> {
        (self.flags & FLAG_HAS_BASELINE != 0)
            .then(|| self.tick.wrapping_sub(self.baseline_raw as u32 + 1))
    }
}
