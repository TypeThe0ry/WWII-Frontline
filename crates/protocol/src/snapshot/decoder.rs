//! Client-side snapshot decoder.

use alloc::vec::Vec;

use crate::bitio::BitReader;
use crate::codec::CodecError;
use crate::quant::{dequant_pitch, dequant_pos_xz, dequant_pos_y, dequant_yaw};
use crate::snapshot::baseline::DecoderRing;
use crate::snapshot::crc::crc16_ccitt;
use crate::snapshot::entity::{
    read_record, EntityKind, EntityState, QPos, Record, MIN_RECORD_BITS, PITCH_BITS, YAW_BITS_NEAR,
};
use crate::snapshot::events::EventEntry;
use crate::snapshot::header::{Header, FLAG_CRC, FLAG_HAS_SELF};
use crate::snapshot::self_state::SelfState;
use crate::snapshot::{MAX_ENTITIES, MAX_PACKET};

/// Entities of one snapshot in struct-of-arrays form (for zero-copy hand-over to JS).
/// `pos_q`/`pos` hold 3 values per entity (x, y, z).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntitiesSoA {
    pub ids: Vec<u8>,
    /// `true` where the record was a keyframe.
    pub keyframe: Vec<bool>,
    pub pos_q: Vec<u16>,
    /// Metres.
    pub pos: Vec<f32>,
    pub yaw_q: Vec<u16>,
    pub pitch_q: Vec<u8>,
    /// Radians.
    pub yaw: Vec<f32>,
    /// Radians.
    pub pitch: Vec<f32>,
    pub anim: Vec<u8>,
    pub stance: Vec<u8>,
    pub weapon: Vec<u8>,
    pub flags: Vec<u8>,
    pub health_q: Vec<u8>,
}

impl EntitiesSoA {
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    fn push(&mut self, s: &EntityState, key: bool) {
        self.ids.push(s.id);
        self.keyframe.push(key);
        self.pos_q.extend_from_slice(&[s.pos.x, s.pos.y, s.pos.z]);
        self.pos.extend_from_slice(&[
            dequant_pos_xz(s.pos.x),
            dequant_pos_y(s.pos.y),
            dequant_pos_xz(s.pos.z),
        ]);
        self.yaw_q.push(s.yaw);
        self.pitch_q.push(s.pitch);
        self.yaw.push(dequant_yaw(s.yaw, YAW_BITS_NEAR));
        self.pitch.push(dequant_pitch(s.pitch as u16, PITCH_BITS));
        self.anim.push(s.anim);
        self.stance.push(s.stance);
        self.weapon.push(s.weapon);
        self.flags.push(s.flags);
        self.health_q.push(s.health_q);
    }

    /// Re-assemble row `i` as an [`EntityState`].
    pub fn state(&self, i: usize) -> Option<EntityState> {
        Some(EntityState {
            id: *self.ids.get(i)?,
            kind: EntityKind::Player,
            pos: QPos {
                x: *self.pos_q.get(3 * i)?,
                y: *self.pos_q.get(3 * i + 1)?,
                z: *self.pos_q.get(3 * i + 2)?,
            },
            yaw: *self.yaw_q.get(i)?,
            pitch: *self.pitch_q.get(i)?,
            vel: None,
            anim: *self.anim.get(i)?,
            stance: *self.stance.get(i)?,
            weapon: *self.weapon.get(i)?,
            flags: *self.flags.get(i)?,
            health_q: *self.health_q.get(i)?,
            vehicle: None,
        })
    }

    pub fn to_states(&self) -> Vec<EntityState> {
        (0..self.len()).filter_map(|i| self.state(i)).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedSnapshot {
    pub tick: u32,
    pub ack_input_seq: u16,
    pub input_slack: u8,
    pub self_state: Option<SelfState>,
    pub entities: EntitiesSoA,
    pub events: Vec<EventEntry>,
    pub removed_ids: Vec<u8>,
}

/// Decodes snapshots of one connection. A failed decode leaves the decoder unchanged.
#[derive(Debug, Default)]
pub struct SnapshotDecoder {
    ring: DecoderRing,
}

impl SnapshotDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget all baselines (e.g. on reconnect).
    pub fn reset(&mut self) {
        self.ring = DecoderRing::default();
    }

    /// Newest tick decoded so far.
    pub fn newest_tick(&self) -> Option<u32> {
        self.ring.newest
    }

    /// `CodecError::MissingBaseline` means "do not acknowledge this one": the server will fall
    /// back to keyframes once the ack lag exceeds the ring.
    pub fn decode(&mut self, bytes: &[u8]) -> Result<DecodedSnapshot, CodecError> {
        if bytes.len() > MAX_PACKET {
            return Err(CodecError::TooLong);
        }
        let first = *bytes.first().ok_or(CodecError::Eof)?;
        let body = if first & 0x0F & FLAG_CRC != 0 {
            let n = bytes.len().checked_sub(2).ok_or(CodecError::Eof)?;
            let (b, t) = bytes.split_at(n);
            if crc16_ccitt(b) != u16::from_be_bytes([t[0], t[1]]) {
                return Err(CodecError::BadCrc);
            }
            b
        } else {
            bytes
        };
        let mut r = BitReader::new(body);
        let hdr = Header::read(&mut r, self.ring.newest)?;
        let self_state = if hdr.flags & FLAG_HAS_SELF != 0 {
            Some(SelfState::decode(&mut r)?)
        } else {
            None
        };
        let ev_n = r.read_bits(6)? as usize;
        let mut events = Vec::with_capacity(ev_n.min(r.remaining_bits() / 29));
        for _ in 0..ev_n {
            events.push(EventEntry::decode(&mut r)?);
        }
        let ent_n = r.read_bits(7)? as usize;
        debug_assert!(ent_n <= MAX_ENTITIES);
        if ent_n * MIN_RECORD_BITS > r.remaining_bits() {
            return Err(CodecError::Eof);
        }
        let baseline = match hdr.baseline_tick() {
            Some(t) => Some(
                self.ring
                    .get(t)
                    .map(|e| e.states.as_slice())
                    .ok_or(CodecError::MissingBaseline)?,
            ),
            None => None,
        };
        let mut entities = EntitiesSoA::default();
        let mut removed_ids = Vec::new();
        let mut states = Vec::with_capacity(ent_n);
        let mut seen = [0u64; 4];
        for _ in 0..ent_n {
            let rec = read_record(&mut r, baseline)?;
            let id = match &rec {
                Record::Keyframe(s) | Record::Delta(s) => s.id,
                Record::Remove(id) => *id,
            };
            let (w, b) = (id as usize / 64, id as u64 % 64);
            if seen[w] >> b & 1 != 0 {
                return Err(CodecError::InvalidValue);
            }
            seen[w] |= 1 << b;
            match rec {
                Record::Keyframe(s) => {
                    entities.push(&s, true);
                    states.push(s);
                }
                Record::Delta(s) => {
                    entities.push(&s, false);
                    states.push(s);
                }
                Record::Remove(id) => removed_ids.push(id),
            }
        }
        r.finish()?;
        self.ring.push(hdr.tick, states);
        Ok(DecodedSnapshot {
            tick: hdr.tick,
            ack_input_seq: hdr.ack_input_seq,
            input_slack: hdr.input_slack,
            self_state,
            entities,
            events,
            removed_ids,
        })
    }
}
