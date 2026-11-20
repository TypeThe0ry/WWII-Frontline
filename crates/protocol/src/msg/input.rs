//! C1 input messages (unreliable; ARCHITECTURE §6.3, §6.8.2, §8.2).
//!
//! `InputBundle` wire layout (MSB first, zero padded):
//! `ack_snapshot_tick:16, pred_hash:8, n:2 (1..=3), fire_n:2 (0..=2)`, then `n` commands,
//! then `fire_n` [`FireCmd`]s. Command 0 is written in full ([`InputCmd`] as a [`Codec`]);
//! commands 1.. are delta coded against the previous one (`tick` must be previous + 1; `seq`
//! uses a 1-bit "previous + 1" flag; the other fields use "same" flags / small deltas).

use alloc::vec::Vec;
use arrayvec::ArrayVec;

use crate::bitio::{BitReader, BitWriter};
use crate::codec::{Codec, CodecError};

/// Maximum commands per bundle.
pub const MAX_BUNDLE_CMDS: usize = 3;
/// Maximum redundant fire commands per bundle.
pub const MAX_BUNDLE_FIRE: usize = 2;

pub mod buttons {
    pub const FIRE: u16 = 1 << 0;
    pub const AIM: u16 = 1 << 1;
    pub const RELOAD: u16 = 1 << 2;
    pub const JUMP: u16 = 1 << 3;
    pub const CROUCH: u16 = 1 << 4;
    pub const PRONE: u16 = 1 << 5;
    pub const SPRINT: u16 = 1 << 6;
    pub const USE: u16 = 1 << 7;
    pub const LEAN_L: u16 = 1 << 8;
    pub const LEAN_R: u16 = 1 << 9;
    pub const MELEE: u16 = 1 << 10;
    pub const GADGET: u16 = 1 << 11;
    pub const SWAP: u16 = 1 << 12;
    pub const VEHICLE_SEAT: u16 = 1 << 13;
    // bits 14, 15 reserved
}

/// One simulation tick of player input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputCmd {
    pub seq: u16,
    pub tick: u32,
    /// -127..=127 (-128 is invalid).
    pub move_x: i8,
    pub move_z: i8,
    /// 0..65535 -> [0, 2π).
    pub yaw: u16,
    /// -32768..32767 -> [-π/2, +π/2].
    pub pitch: i16,
    pub buttons: u16,
    pub weapon_slot: u8,
    pub aux: u8,
    /// Lag-compensation view tick, as a difference relative to `tick`.
    pub view_tick: u16,
}

fn check_move(v: i8) -> Result<(), CodecError> {
    if v == i8::MIN {
        Err(CodecError::InvalidValue)
    } else {
        Ok(())
    }
}

impl Codec for InputCmd {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        check_move(self.move_x)?;
        check_move(self.move_z)?;
        w.write_bits(self.seq as u64, 16)?;
        w.write_bits(self.tick as u64, 32)?;
        w.write_i(self.move_x as i64, 8)?;
        w.write_i(self.move_z as i64, 8)?;
        w.write_bits(self.yaw as u64, 16)?;
        w.write_i(self.pitch as i64, 16)?;
        w.write_bits(self.buttons as u64, 16)?;
        w.write_bits(self.weapon_slot as u64, 8)?;
        w.write_bits(self.aux as u64, 8)?;
        w.write_bits(self.view_tick as u64, 16)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        let c = InputCmd {
            seq: r.read_bits(16)? as u16,
            tick: r.read_bits(32)? as u32,
            move_x: r.read_i(8)? as i8,
            move_z: r.read_i(8)? as i8,
            yaw: r.read_bits(16)? as u16,
            pitch: r.read_i(16)? as i16,
            buttons: r.read_bits(16)? as u16,
            weapon_slot: r.read_bits(8)? as u8,
            aux: r.read_bits(8)? as u8,
            view_tick: r.read_bits(16)? as u16,
        };
        check_move(c.move_x)?;
        check_move(c.move_z)?;
        Ok(c)
    }
}

// --- delta coding helpers -------------------------------------------------------------

/// 2-bit kind for 16-bit fields: 0 same, 1 small signed delta (8 bit), 2 full.
fn write_delta16(w: &mut BitWriter, prev: u16, cur: u16) -> Result<(), CodecError> {
    let d = cur.wrapping_sub(prev) as i16;
    if d == 0 {
        w.write_bits(0, 2)
    } else if (-128..=127).contains(&d) {
        w.write_bits(1, 2)?;
        w.write_i(d as i64, 8)
    } else {
        w.write_bits(2, 2)?;
        w.write_bits(cur as u64, 16)
    }
}

fn read_delta16(r: &mut BitReader<'_>, prev: u16) -> Result<u16, CodecError> {
    match r.read_bits(2)? {
        0 => Ok(prev),
        1 => Ok(prev.wrapping_add(r.read_i(8)? as i16 as u16)),
        2 => Ok(r.read_bits(16)? as u16),
        _ => Err(CodecError::InvalidTag),
    }
}

fn write_same_or<const N: u8>(w: &mut BitWriter, prev: u64, cur: u64) -> Result<(), CodecError> {
    if prev == cur {
        w.write_bool(true)
    } else {
        w.write_bool(false)?;
        w.write_bits(cur, N)
    }
}

fn read_same_or<const N: u8>(r: &mut BitReader<'_>, prev: u64) -> Result<u64, CodecError> {
    if r.read_bool()? {
        Ok(prev)
    } else {
        r.read_bits(N)
    }
}

fn encode_diff(w: &mut BitWriter, prev: &InputCmd, c: &InputCmd) -> Result<(), CodecError> {
    check_move(c.move_x)?;
    check_move(c.move_z)?;
    if c.tick != prev.tick.wrapping_add(1) {
        return Err(CodecError::InvalidValue);
    }
    if c.seq == prev.seq.wrapping_add(1) {
        w.write_bool(true)?;
    } else {
        w.write_bool(false)?;
        w.write_bits(c.seq as u64, 16)?;
    }
    for (p, v) in [(prev.move_x, c.move_x), (prev.move_z, c.move_z)] {
        if p == v {
            w.write_bool(true)?;
        } else {
            w.write_bool(false)?;
            w.write_i(v as i64, 8)?;
        }
    }
    write_delta16(w, prev.yaw, c.yaw)?;
    write_delta16(w, prev.pitch as u16, c.pitch as u16)?;
    write_same_or::<16>(w, prev.buttons as u64, c.buttons as u64)?;
    write_same_or::<8>(w, prev.weapon_slot as u64, c.weapon_slot as u64)?;
    write_same_or::<8>(w, prev.aux as u64, c.aux as u64)?;
    write_same_or::<16>(w, prev.view_tick as u64, c.view_tick as u64)
}

fn decode_diff(r: &mut BitReader<'_>, prev: &InputCmd) -> Result<InputCmd, CodecError> {
    let seq = if r.read_bool()? {
        prev.seq.wrapping_add(1)
    } else {
        r.read_bits(16)? as u16
    };
    let mut mv = [prev.move_x, prev.move_z];
    for m in &mut mv {
        if !r.read_bool()? {
            *m = r.read_i(8)? as i8;
            check_move(*m)?;
        }
    }
    let yaw = read_delta16(r, prev.yaw)?;
    let pitch = read_delta16(r, prev.pitch as u16)? as i16;
    let buttons = read_same_or::<16>(r, prev.buttons as u64)? as u16;
    let weapon_slot = read_same_or::<8>(r, prev.weapon_slot as u64)? as u8;
    let aux = read_same_or::<8>(r, prev.aux as u64)? as u8;
    let view_tick = read_same_or::<16>(r, prev.view_tick as u64)? as u16;
    Ok(InputCmd {
        seq,
        tick: prev.tick.wrapping_add(1),
        move_x: mv[0],
        move_z: mv[1],
        yaw,
        pitch,
        buttons,
        weapon_slot,
        aux,
        view_tick,
    })
}

/// Fire event metadata for lag compensation (redundantly attached to a bundle).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FireCmd {
    pub seq: u16,
    /// Difference relative to the current tick.
    pub view_tick: u16,
    /// Client input weapon slot (same domain as `InputCmd.weapon_slot`, not a definition ID).
    pub weapon: u8,
    /// Hinted direction: yaw 16 bit over [0, 2π), pitch 16 bit over [-π/2, π/2].
    pub dir_yaw: u16,
    pub dir_pitch: u16,
    /// Untrusted muzzle position: world XYZ, 0.25 m per 14-bit code, base (0, -128, 0) m.
    /// `[0; 3]` means unavailable; never use this field to set the authoritative muzzle.
    pub origin_hint: [u16; 3],
    pub spread_seed: u16,
}

impl FireCmd {
    const ORIGIN_BASE_M: [f32; 3] = [0.0, -128.0, 0.0];
    const ORIGIN_STEP_M: f32 = 0.25;
    const ORIGIN_MAX_CODE: u16 = 0x3fff;

    /// Round the predicted muzzle to the nearest grid point. Omit an absent, invalid or
    /// out-of-grid origin rather than sending a position that was clamped to the map edge.
    pub fn encode_origin_hint(world_xyz: [f32; 3]) -> Option<[u16; 3]> {
        let mut codes = [0; 3];
        for ((code, metres), base) in codes.iter_mut().zip(world_xyz).zip(Self::ORIGIN_BASE_M) {
            let grid = (metres - base) / Self::ORIGIN_STEP_M;
            if !(0.0..=f32::from(Self::ORIGIN_MAX_CODE)).contains(&grid) {
                return None;
            }
            *code = (grid + 0.5) as u16;
        }
        (codes != [0; 3]).then_some(codes)
    }

    pub fn decode_origin_hint(codes: [u16; 3]) -> Option<[f32; 3]> {
        if codes == [0; 3] || codes.iter().any(|&c| c > Self::ORIGIN_MAX_CODE) {
            return None;
        }
        Some(core::array::from_fn(|i| {
            Self::ORIGIN_BASE_M[i] + f32::from(codes[i]) * Self::ORIGIN_STEP_M
        }))
    }
}

#[cfg(test)]
mod origin_hint_tests {
    use super::FireCmd;

    #[test]
    fn world_grid_covers_carentan_and_rejects_unrepresentable_points() {
        let codes = FireCmd::encode_origin_hint([2048.0, -7.1, 1024.13]);
        assert_eq!(codes, Some([8192, 484, 4097]));
        assert_eq!(
            FireCmd::decode_origin_hint(codes.unwrap()),
            Some([2048.0, -7.0, 1024.25])
        );
        assert!(FireCmd::encode_origin_hint([0.0, 0.0, 0.0]).is_some());
        for pos in [
            [0.0, -128.0, 0.0], // reserved
            [0.0, -129.0, 0.0],
            [-0.01, 0.0, 0.0],
            [0.0, 0.0, 4096.0],
            [f32::NAN, 0.0, 0.0],
            [0.0, f32::INFINITY, 0.0],
        ] {
            assert_eq!(FireCmd::encode_origin_hint(pos), None);
        }
        assert_eq!(FireCmd::decode_origin_hint([0; 3]), None);
        assert_eq!(FireCmd::decode_origin_hint([0x4000, 512, 9]), None);
    }
}

impl Codec for FireCmd {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.seq as u64, 16)?;
        w.write_bits(self.view_tick as u64, 16)?;
        w.write_bits(self.weapon as u64, 8)?;
        w.write_bits(self.dir_yaw as u64, 16)?;
        w.write_bits(self.dir_pitch as u64, 16)?;
        for o in self.origin_hint {
            w.write_bits(o as u64, 14)?;
        }
        w.write_bits(self.spread_seed as u64, 16)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(FireCmd {
            seq: r.read_bits(16)? as u16,
            view_tick: r.read_bits(16)? as u16,
            weapon: r.read_bits(8)? as u8,
            dir_yaw: r.read_bits(16)? as u16,
            dir_pitch: r.read_bits(16)? as u16,
            origin_hint: [
                r.read_bits(14)? as u16,
                r.read_bits(14)? as u16,
                r.read_bits(14)? as u16,
            ],
            spread_seed: r.read_bits(16)? as u16,
        })
    }
}

/// Client -> server input packet: up to 3 consecutive `InputCmd`s plus up to 2 `FireCmd`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputBundle {
    /// Latest snapshot tick received, as a delta.
    pub ack_snapshot_tick: u16,
    pub pred_hash: u8,
    /// 1..=3 commands, ascending, consecutive ticks.
    pub cmds: ArrayVec<InputCmd, MAX_BUNDLE_CMDS>,
    pub fire: ArrayVec<FireCmd, MAX_BUNDLE_FIRE>,
}

impl Codec for InputBundle {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        if self.cmds.is_empty() {
            return Err(CodecError::InvalidValue);
        }
        w.write_bits(self.ack_snapshot_tick as u64, 16)?;
        w.write_bits(self.pred_hash as u64, 8)?;
        w.write_bits(self.cmds.len() as u64, 2)?;
        w.write_bits(self.fire.len() as u64, 2)?;
        self.cmds[0].encode(w)?;
        for pair in self.cmds.windows(2) {
            encode_diff(w, &pair[0], &pair[1])?;
        }
        for f in &self.fire {
            f.encode(w)?;
        }
        Ok(())
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        let ack_snapshot_tick = r.read_bits(16)? as u16;
        let pred_hash = r.read_bits(8)? as u8;
        let n = r.read_bits(2)? as usize;
        let fire_n = r.read_bits(2)? as usize;
        if n == 0 {
            return Err(CodecError::InvalidTag);
        }
        if fire_n > MAX_BUNDLE_FIRE {
            return Err(CodecError::InvalidTag);
        }
        let mut cmds = ArrayVec::new();
        cmds.push(InputCmd::decode(r)?);
        for i in 1..n {
            let next = decode_diff(r, &cmds[i - 1])?;
            cmds.push(next);
        }
        let mut fire = ArrayVec::new();
        for _ in 0..fire_n {
            fire.push(FireCmd::decode(r)?);
        }
        Ok(InputBundle {
            ack_snapshot_tick,
            pred_hash,
            cmds,
            fire,
        })
    }
}

pub fn encode_input_bundle(b: &InputBundle) -> Result<Vec<u8>, CodecError> {
    let mut w = BitWriter::with_capacity(32);
    b.encode(&mut w)?;
    Ok(w.finish())
}

pub fn decode_input_bundle(bytes: &[u8]) -> Result<InputBundle, CodecError> {
    let mut r = BitReader::new(bytes);
    let b = InputBundle::decode(&mut r)?;
    r.finish()?;
    Ok(b)
}
