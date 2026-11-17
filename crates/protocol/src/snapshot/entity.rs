//! Entity state and the per-entity snapshot record codec (ARCHITECTURE §6.5.2, §8.2).
//!
//! # Record layout (bits, MSB first)
//!
//! ```text
//! id:8, then
//!   1                  delta record: field_mask:8, then the fields whose mask bit is set
//!   01                 keyframe (full record, no baseline needed)
//!   00                 remove (entity left the client's world view)
//! ```
//! `field_mask` bits: 0 `pos`, 1 `rot`, 2 `vel` (reserved, must be 0 in M0), 3 `anim`,
//! 4 `weapon`, 5 `health_q`, 6 `flags`, 7 `near` (rot width; always 0 in no-rot records).
//!
//! * keyframe body: `x:16 y:13 z:16 | yaw:10 pitch:6 | anim:4 stance:3 | weapon:6 | flags:8 |
//!   health_q:8` = 100 bits with the header (12.5 B, ARCHITECTURE target 13 B).
//! * `pos` (delta): `xz_changed:1 [class:2 dx dz] y_changed:1 [class:2 dy]` with signed deltas
//!   in 1/32 m steps. Size class by |delta|: 0 = `<32` steps (6 bit), 1 = `<256` (9 bit),
//!   2 = `<2048` (12 bit), 3 = absolute value instead of a delta (x,z 16 bit / y 13 bit).
//!   x and z share one class (the larger of the two). Refinement of §6.5.2: the thresholds
//!   are chosen so each class value fits its width (6/9/12) exactly.
//! * `rot` near: `yaw:10 pitch:6`; far: `yaw:8` (yaw code >> 2, rounded) and pitch is *not*
//!   updated (keeps the baseline value). Far yaw therefore always has its low 2 bits clear.
//! * `anim`: `anim:4 stance:3`; `weapon`: 6 bit; `health_q`: 8 bit; `flags`: 8 bit.

use crate::bitio::{BitReader, BitWriter};
use crate::codec::CodecError;
use crate::quant::{dequant_pitch, dequant_pos_xz, dequant_pos_y, dequant_yaw, quant_pos_xz, quant_pos_y};

pub const MASK_POS: u8 = 1;
pub const MASK_ROT: u8 = 2;
pub const MASK_VEL: u8 = 4;
pub const MASK_ANIM: u8 = 8;
pub const MASK_WEAPON: u8 = 16;
pub const MASK_HEALTH: u8 = 32;
pub const MASK_FLAGS: u8 = 64;
pub const MASK_NEAR: u8 = 128;

pub const YAW_BITS_NEAR: u8 = 10;
pub const YAW_BITS_FAR: u8 = 8;
pub const PITCH_BITS: u8 = 6;
const POS_Y_MAX: u16 = (1 << 13) - 1;
const CLASS_BITS: [u8; 3] = [6, 9, 12];

/// Quantised position: x/z 16 bit and y 13 bit codes, step 1/32 m.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QPos {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

impl QPos {
    /// Quantise metres (clamped to the map box; NaN/Inf -> 0).
    pub fn from_meters(x: f32, y: f32, z: f32) -> QPos {
        QPos {
            x: quant_pos_xz(x).code,
            y: quant_pos_y(y).code,
            z: quant_pos_xz(z).code,
        }
    }

    pub fn to_meters(self) -> [f32; 3] {
        [
            dequant_pos_xz(self.x),
            dequant_pos_y(self.y),
            dequant_pos_xz(self.z),
        ]
    }

    pub(crate) fn write_abs(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.x as u64, 16)?;
        w.write_bits(self.y as u64, 13)?;
        w.write_bits(self.z as u64, 16)
    }

    pub(crate) fn read_abs(r: &mut BitReader<'_>) -> Result<QPos, CodecError> {
        Ok(QPos {
            x: r.read_bits(16)? as u16,
            y: r.read_bits(13)? as u16,
            z: r.read_bits(16)? as u16,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntityKind {
    #[default]
    Player,
}

/// Quantised velocity. Not sent in M0 (reserved for vehicles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QVel {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

/// Vehicle-only extras. Not sent in M0 (M1-010).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VehicleExtra {
    pub quat: u32,
    pub turret_yaw: u8,
    pub gun_pitch: u8,
    pub track_flags: u8,
    pub seats: u8,
}

/// Wire-level entity state (not an ECS component). Field ranges: `yaw` 10-bit code,
/// `pitch` 6-bit code, `anim` < 16, `stance` < 8, `weapon` < 64, `pos.y` < 8192.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntityState {
    pub id: u8,
    pub kind: EntityKind,
    pub pos: QPos,
    pub yaw: u16,
    pub pitch: u8,
    pub vel: Option<QVel>,
    pub anim: u8,
    pub stance: u8,
    pub weapon: u8,
    pub flags: u8,
    pub health_q: u8,
    pub vehicle: Option<VehicleExtra>,
}

impl EntityState {
    /// Range-check for the M0 wire format.
    pub fn validate(&self) -> Result<(), CodecError> {
        if self.vel.is_some() || self.vehicle.is_some() {
            return Err(CodecError::InvalidValue);
        }
        if self.pos.y > POS_Y_MAX
            || self.yaw >= 1 << YAW_BITS_NEAR
            || self.pitch >= 1 << PITCH_BITS
            || self.anim >= 16
            || self.stance >= 8
            || self.weapon >= 64
        {
            return Err(CodecError::Overflow);
        }
        Ok(())
    }

    pub fn pos_meters(&self) -> [f32; 3] {
        self.pos.to_meters()
    }
    pub fn yaw_rad(&self) -> f32 {
        dequant_yaw(self.yaw, YAW_BITS_NEAR)
    }
    pub fn pitch_rad(&self) -> f32 {
        dequant_pitch(self.pitch as u16, PITCH_BITS)
    }
}

/// What the receiver will reconstruct for `s` when sent as a delta against `base`.
/// Far records quantise yaw to 8 bit and do not carry pitch. Keyframes (`base == None`) are
/// exact.
pub(crate) fn make_sent(base: Option<&EntityState>, s: &EntityState, near: bool) -> EntityState {
    match (base, near) {
        (Some(b), false) => {
            let mut o = *s;
            o.yaw = ((((s.yaw + 2) >> 2) & 0xFF) << 2) as u16;
            o.pitch = b.pitch;
            o
        }
        _ => *s,
    }
}

fn class_of(d: i32) -> u8 {
    match d.unsigned_abs() {
        0..=31 => 0,
        32..=255 => 1,
        256..=2047 => 2,
        _ => 3,
    }
}

pub(crate) fn write_remove(w: &mut BitWriter, id: u8) -> Result<(), CodecError> {
    w.write_bits(id as u64, 8)?;
    w.write_bits(0, 2)
}

pub(crate) fn write_keyframe(w: &mut BitWriter, s: &EntityState) -> Result<(), CodecError> {
    w.write_bits(s.id as u64, 8)?;
    w.write_bits(0b01, 2)?;
    s.pos.write_abs(w)?;
    w.write_bits(s.yaw as u64, YAW_BITS_NEAR)?;
    w.write_bits(s.pitch as u64, PITCH_BITS)?;
    w.write_bits(s.anim as u64, 4)?;
    w.write_bits(s.stance as u64, 3)?;
    w.write_bits(s.weapon as u64, 6)?;
    w.write_bits(s.flags as u64, 8)?;
    w.write_bits(s.health_q as u64, 8)
}

/// `sent` must come from [`make_sent`] (so far records never differ in pitch).
pub(crate) fn write_delta(
    w: &mut BitWriter,
    base: &EntityState,
    sent: &EntityState,
    near: bool,
) -> Result<(), CodecError> {
    let mut mask = 0u8;
    if sent.pos != base.pos {
        mask |= MASK_POS;
    }
    if sent.yaw != base.yaw || sent.pitch != base.pitch {
        mask |= MASK_ROT;
    }
    if sent.anim != base.anim || sent.stance != base.stance {
        mask |= MASK_ANIM;
    }
    if sent.weapon != base.weapon {
        mask |= MASK_WEAPON;
    }
    if sent.health_q != base.health_q {
        mask |= MASK_HEALTH;
    }
    if sent.flags != base.flags {
        mask |= MASK_FLAGS;
    }
    if near && mask & MASK_ROT != 0 {
        mask |= MASK_NEAR;
    }
    w.write_bits(sent.id as u64, 8)?;
    w.write_bits(1, 1)?;
    w.write_bits(mask as u64, 8)?;
    if mask & MASK_POS != 0 {
        write_pos_delta(w, &base.pos, &sent.pos)?;
    }
    if mask & MASK_ROT != 0 {
        if mask & MASK_NEAR != 0 {
            w.write_bits(sent.yaw as u64, YAW_BITS_NEAR)?;
            w.write_bits(sent.pitch as u64, PITCH_BITS)?;
        } else {
            w.write_bits((sent.yaw >> 2) as u64, YAW_BITS_FAR)?;
        }
    }
    if mask & MASK_ANIM != 0 {
        w.write_bits(sent.anim as u64, 4)?;
        w.write_bits(sent.stance as u64, 3)?;
    }
    if mask & MASK_WEAPON != 0 {
        w.write_bits(sent.weapon as u64, 6)?;
    }
    if mask & MASK_HEALTH != 0 {
        w.write_bits(sent.health_q as u64, 8)?;
    }
    if mask & MASK_FLAGS != 0 {
        w.write_bits(sent.flags as u64, 8)?;
    }
    Ok(())
}

fn write_pos_delta(w: &mut BitWriter, b: &QPos, s: &QPos) -> Result<(), CodecError> {
    let (dx, dz, dy) = (
        s.x as i32 - b.x as i32,
        s.z as i32 - b.z as i32,
        s.y as i32 - b.y as i32,
    );
    if dx != 0 || dz != 0 {
        w.write_bool(true)?;
        let c = class_of(dx).max(class_of(dz));
        w.write_bits(c as u64, 2)?;
        if c == 3 {
            w.write_bits(s.x as u64, 16)?;
            w.write_bits(s.z as u64, 16)?;
        } else {
            w.write_i(dx as i64, CLASS_BITS[c as usize])?;
            w.write_i(dz as i64, CLASS_BITS[c as usize])?;
        }
    } else {
        w.write_bool(false)?;
    }
    if dy != 0 {
        w.write_bool(true)?;
        let c = class_of(dy);
        w.write_bits(c as u64, 2)?;
        if c == 3 {
            w.write_bits(s.y as u64, 13)?;
        } else {
            w.write_i(dy as i64, CLASS_BITS[c as usize])?;
        }
    } else {
        w.write_bool(false)?;
    }
    Ok(())
}

fn apply(base: u16, d: i64, max: u16) -> Result<u16, CodecError> {
    let v = base as i64 + d;
    if v < 0 || v > max as i64 {
        Err(CodecError::InvalidValue)
    } else {
        Ok(v as u16)
    }
}

fn read_pos_delta(r: &mut BitReader<'_>, b: &QPos) -> Result<QPos, CodecError> {
    let mut p = *b;
    if r.read_bool()? {
        let c = r.read_bits(2)? as usize;
        if c == 3 {
            p.x = r.read_bits(16)? as u16;
            p.z = r.read_bits(16)? as u16;
        } else {
            p.x = apply(b.x, r.read_i(CLASS_BITS[c])?, u16::MAX)?;
            p.z = apply(b.z, r.read_i(CLASS_BITS[c])?, u16::MAX)?;
        }
    }
    if r.read_bool()? {
        let c = r.read_bits(2)? as usize;
        if c == 3 {
            p.y = r.read_bits(13)? as u16;
        } else {
            p.y = apply(b.y, r.read_i(CLASS_BITS[c])?, POS_Y_MAX)?;
        }
    }
    Ok(p)
}

/// One decoded entity-section record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Record {
    Keyframe(EntityState),
    Delta(EntityState),
    Remove(u8),
}

/// Smallest possible record (a remove) in bits; used to bound allocations.
pub(crate) const MIN_RECORD_BITS: usize = 10;

pub(crate) fn find_state(states: &[EntityState], id: u8) -> Option<&EntityState> {
    states
        .binary_search_by_key(&id, |s| s.id)
        .ok()
        .map(|i| &states[i])
}

/// `baseline`: states sorted by id (or `None` when the packet carries no baseline).
pub(crate) fn read_record(
    r: &mut BitReader<'_>,
    baseline: Option<&[EntityState]>,
) -> Result<Record, CodecError> {
    let id = r.read_bits(8)? as u8;
    if r.read_bool()? {
        let base = baseline
            .and_then(|b| find_state(b, id))
            .ok_or(CodecError::MissingBaseline)?;
        let mask = r.read_bits(8)? as u8;
        if mask & MASK_VEL != 0 {
            return Err(CodecError::InvalidValue);
        }
        let mut s = *base;
        s.id = id;
        if mask & MASK_POS != 0 {
            s.pos = read_pos_delta(r, &base.pos)?;
        }
        if mask & MASK_ROT != 0 {
            if mask & MASK_NEAR != 0 {
                s.yaw = r.read_bits(YAW_BITS_NEAR)? as u16;
                s.pitch = r.read_bits(PITCH_BITS)? as u8;
            } else {
                s.yaw = (r.read_bits(YAW_BITS_FAR)? as u16) << 2;
            }
        } else if mask & MASK_NEAR != 0 {
            return Err(CodecError::InvalidValue);
        }
        if mask & MASK_ANIM != 0 {
            s.anim = r.read_bits(4)? as u8;
            s.stance = r.read_bits(3)? as u8;
        }
        if mask & MASK_WEAPON != 0 {
            s.weapon = r.read_bits(6)? as u8;
        }
        if mask & MASK_HEALTH != 0 {
            s.health_q = r.read_bits(8)? as u8;
        }
        if mask & MASK_FLAGS != 0 {
            s.flags = r.read_bits(8)? as u8;
        }
        Ok(Record::Delta(s))
    } else if r.read_bool()? {
        Ok(Record::Keyframe(EntityState {
            id,
            kind: EntityKind::Player,
            pos: QPos::read_abs(r)?,
            yaw: r.read_bits(YAW_BITS_NEAR)? as u16,
            pitch: r.read_bits(PITCH_BITS)? as u8,
            vel: None,
            anim: r.read_bits(4)? as u8,
            stance: r.read_bits(3)? as u8,
            weapon: r.read_bits(6)? as u8,
            flags: r.read_bits(8)? as u8,
            health_q: r.read_bits(8)? as u8,
            vehicle: None,
        }))
    } else {
        Ok(Record::Remove(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(x: u16, z: u16) -> EntityState {
        EntityState {
            id: 5,
            pos: QPos { x, y: 100, z },
            ..Default::default()
        }
    }

    fn roundtrip_delta(base: &EntityState, s: &EntityState) -> (usize, EntityState) {
        let mut w = BitWriter::new();
        write_delta(&mut w, base, s, true).unwrap();
        let bits = w.bit_len();
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        let b = [*base];
        match read_record(&mut r, Some(&b)).unwrap() {
            Record::Delta(d) => (bits, d),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn size_class_boundaries() {
        let base = st(1000, 1000);
        // (delta, expected total bits): id8 + delta1 + mask8 + xz1 + class2 + 2*w + y1
        for (d, w) in [
            (31, 6),
            (32, 9),
            (255, 9),
            (256, 12),
            (2047, 12),
            (2048, 0),
            (-31, 6),
            (-32, 9),
            (-2048, 0),
        ] {
            let s = st((1000 + d) as u16, 1000);
            let (bits, got) = roundtrip_delta(&base, &s);
            assert_eq!(got, s, "d={d}");
            let expect = if w == 0 { 8 + 1 + 8 + 1 + 2 + 32 + 1 } else { 8 + 1 + 8 + 1 + 2 + 2 * w + 1 };
            assert_eq!(bits, expect, "d={d}");
        }
    }

    #[test]
    fn y_overflow_clamped_by_quantiser() {
        assert_eq!(QPos::from_meters(1.0, 1e9, 1.0).y, POS_Y_MAX);
        assert_eq!(QPos::from_meters(1.0, -3.0, 1.0).y, 0);
        let mut s = st(1, 1);
        s.pos.y = 8192;
        assert_eq!(s.validate(), Err(CodecError::Overflow));
    }

    #[test]
    fn mask_all_zero_and_all_set() {
        let base = st(10, 10);
        let (bits, got) = roundtrip_delta(&base, &base);
        assert_eq!((bits, got), (17, base));
        let full = EntityState {
            id: 5,
            kind: EntityKind::Player,
            pos: QPos { x: 9000, y: 8000, z: 1 },
            yaw: 1023,
            pitch: 63,
            vel: None,
            anim: 15,
            stance: 7,
            weapon: 63,
            flags: 255,
            health_q: 255,
            vehicle: None,
        };
        let (_, got) = roundtrip_delta(&base, &full);
        assert_eq!(got, full);
    }

    #[test]
    fn vel_mask_bit_rejected() {
        // id=5, delta, mask = VEL
        let mut w = BitWriter::new();
        w.write_bits(5, 8).unwrap();
        w.write_bits(1, 1).unwrap();
        w.write_bits(MASK_VEL as u64, 8).unwrap();
        let bytes = w.finish();
        let b = [st(0, 0)];
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_record(&mut r, Some(&b)), Err(CodecError::InvalidValue));
    }

    #[test]
    fn yaw_wraparound_far() {
        let mut base = st(0, 0);
        base.yaw = 1020;
        let mut s = base;
        s.yaw = 1023; // rounds up to code 256 -> wraps to 0
        let sent = make_sent(Some(&base), &s, false);
        assert_eq!(sent.yaw, 0);
        let mut w = BitWriter::new();
        write_delta(&mut w, &base, &sent, false).unwrap();
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        match read_record(&mut r, Some(&[base])).unwrap() {
            Record::Delta(d) => assert_eq!(d.yaw, 0),
            _ => panic!(),
        }
    }
}
