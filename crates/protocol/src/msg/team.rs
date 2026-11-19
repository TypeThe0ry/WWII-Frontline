//! Team channel (C3) and world state (ARCHITECTURE §6.8.2, §6.9.2).
//!
//! `TeamState` carries no player ids: entry `i` belongs to the `i`-th player of the receiving
//! team in ascending `player_id` order (the roster comes from C4 `PlayerRoster`), with
//! `state == 0` meaning "absent / dead, ignore position". At most [`MAX_TEAM_MEMBERS`]
//! entries are sent; the sender truncates in `player_id` order and counts what it dropped.

use alloc::vec::Vec;

use super::c0::{decode_whole, encode_whole};
use crate::bitio::{BitReader, BitWriter};
use crate::codec::{Codec, CodecError};

pub const MAX_TEAM_MEMBERS: usize = 49;
pub const TEAM_X_BITS: u8 = 11;

/// 32 bit per member: `x:11 z:11 role:4 state:3 facing:3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamMember {
    /// Map position codes; metric meaning (map extent / 2048 per step) is up to the consumer.
    pub x: u16,
    pub z: u16,
    pub role: u8,
    /// 0 = absent/dead, other values defined by the HUD (alive, downed, in vehicle, ...).
    pub state: u8,
    /// 8-way facing.
    pub facing: u8,
}

impl Codec for TeamMember {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.x as u64, TEAM_X_BITS)?;
        w.write_bits(self.z as u64, TEAM_X_BITS)?;
        w.write_bits(self.role as u64, 4)?;
        w.write_bits(self.state as u64, 3)?;
        w.write_bits(self.facing as u64, 3)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            x: r.read_bits(TEAM_X_BITS)? as u16,
            z: r.read_bits(TEAM_X_BITS)? as u16,
            role: r.read_bits(4)? as u8,
            state: r.read_bits(3)? as u8,
            facing: r.read_bits(3)? as u8,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamState {
    pub tick: u16,
    pub members: Vec<TeamMember>,
}

impl TeamState {
    /// Build from members in ascending `player_id` order; entries beyond
    /// [`MAX_TEAM_MEMBERS`] are dropped and counted (second tuple element).
    pub fn from_members(tick: u16, members: &[TeamMember]) -> (Self, usize) {
        let keep = members.len().min(MAX_TEAM_MEMBERS);
        (
            Self {
                tick,
                members: members[..keep].to_vec(),
            },
            members.len() - keep,
        )
    }
}

impl Codec for TeamState {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        if self.members.len() > MAX_TEAM_MEMBERS {
            return Err(CodecError::TooLong);
        }
        w.write_bits(self.tick as u64, 16)?;
        w.write_bits(self.members.len() as u64, 6)?;
        for m in &self.members {
            m.encode(w)?;
        }
        Ok(())
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        let tick = r.read_bits(16)? as u16;
        let n = r.read_bits(6)? as usize;
        if n > MAX_TEAM_MEMBERS {
            return Err(CodecError::TooLong);
        }
        if n * 32 > r.remaining_bits() {
            return Err(CodecError::Eof);
        }
        let mut members = Vec::with_capacity(n);
        for _ in 0..n {
            members.push(TeamMember::decode(r)?);
        }
        Ok(Self { tick, members })
    }
}

/// Hard limits for the optional C3 tactical marker extension.
pub const MAX_TEAM_MARKERS: usize = 64;
pub const MARKER_CELL_M: u16 = 5;
const MARKER_COORD_BITS: u8 = 12;
pub const MAX_MARKER_CELL: u16 = (1 << MARKER_COORD_BITS) - 1;
const MARKER_BYTES: usize = 5;
pub const MAX_TEAM_STATE_WITH_MARKERS_BYTES: usize =
    3 + 4 * MAX_TEAM_MEMBERS + 1 + MARKER_BYTES * MAX_TEAM_MARKERS;

/// Manual marker tags shared with C0 `MarkerPlace`. 1 and 2 are not manual marker tags:
/// enemy indicators need their own server-authoritative evidence and permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TeamMarkerKind {
    Attention = 0,
    NeedMedic = 3,
    NeedAmmo = 4,
    OrderAttack = 5,
    OrderDefend = 6,
    OrderMove = 7,
}

impl TeamMarkerKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Attention,
            3 => Self::NeedMedic,
            4 => Self::NeedAmmo,
            5 => Self::OrderAttack,
            6 => Self::OrderDefend,
            7 => Self::OrderMove,
            _ => return None,
        })
    }
}

/// Display only: no player, squad, target, or persistent marker identifier.
/// Wire: `kind:8 x:12 z:12 ttl_s:8` (5 bytes, MSB first).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamMarker {
    pub kind: TeamMarkerKind,
    /// Lower edge of the 5 m cell, relative to the map origin (`origin_x + x * 5 m`).
    pub x: u16,
    pub z: u16,
    /// Positive remaining seconds at the packet tick, rounded up.
    pub ttl_s: u8,
}

impl Codec for TeamMarker {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        if self.ttl_s == 0 {
            return Err(CodecError::InvalidValue);
        }
        w.write_bits(self.kind as u64, 8)?;
        w.write_bits(self.x as u64, MARKER_COORD_BITS)?;
        w.write_bits(self.z as u64, MARKER_COORD_BITS)?;
        w.write_bits(self.ttl_s as u64, 8)
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        let kind = TeamMarkerKind::from_u8(r.read_bits(8)? as u8).ok_or(CodecError::InvalidTag)?;
        let x = r.read_bits(MARKER_COORD_BITS)? as u16;
        let z = r.read_bits(MARKER_COORD_BITS)? as u16;
        let ttl_s = r.read_bits(8)? as u8;
        if ttl_s == 0 {
            return Err(CodecError::InvalidValue);
        }
        Ok(Self { kind, x, z, ttl_s })
    }
}

/// Full replacement of the recipient's visible markers. No extension means an empty list.
/// The original TeamState (including its two zero pad bits) is the byte-exact prefix; if
/// nonempty, a `count:u8` and `count` marker records follow. Extended payloads require opt-in
/// via [`super::c0::CAP_TEAM_MARKERS`]; the legacy standalone decoder rejects trailing bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamStateWithMarkers {
    pub team: TeamState,
    pub markers: Vec<TeamMarker>,
}

impl Codec for TeamStateWithMarkers {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        if self.markers.len() > MAX_TEAM_MARKERS {
            return Err(CodecError::TooLong);
        }
        self.team.encode(w)?;
        w.write_bits(0, 2)?; // Keep the legacy prefix, including its pad bits.
        if !self.markers.is_empty() {
            w.write_bits(self.markers.len() as u64, 8)?;
            for marker in &self.markers {
                marker.encode(w)?;
            }
        }
        Ok(())
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        let team = TeamState::decode(r)?;
        if r.read_bits(2)? != 0 {
            return Err(CodecError::Trailing);
        }
        let mut markers = Vec::new();
        if r.remaining_bits() != 0 {
            let n = r.read_bits(8)? as usize;
            if n == 0 {
                return Err(CodecError::InvalidValue);
            }
            if n > MAX_TEAM_MARKERS {
                return Err(CodecError::TooLong);
            }
            let expected_bits = n * MARKER_BYTES * 8;
            match r.remaining_bits().cmp(&expected_bits) {
                core::cmp::Ordering::Less => return Err(CodecError::Eof),
                core::cmp::Ordering::Greater => return Err(CodecError::Trailing),
                core::cmp::Ordering::Equal => {}
            }
            markers.reserve_exact(n);
            for _ in 0..n {
                markers.push(TeamMarker::decode(r)?);
            }
        }
        Ok(Self { team, markers })
    }
}

pub fn encode_team_state_with_markers(m: &TeamStateWithMarkers) -> Result<Vec<u8>, CodecError> {
    encode_whole(m)
}

pub fn decode_team_state_with_markers(b: &[u8]) -> Result<TeamStateWithMarkers, CodecError> {
    if b.len() > MAX_TEAM_STATE_WITH_MARKERS_BYTES {
        return Err(CodecError::TooLong);
    }
    decode_whole(b)
}

/// One objective: `owner:2` (0 neutral, 1/2 teams, 3 contested) and `capture:7` (0..=100 %).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectiveState {
    pub owner: u8,
    pub capture: u8,
}

pub const WORLD_OBJECTIVES: usize = 5;

/// 1 Hz world state (M1 subset: timer, 5 objectives, resources, front line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldState {
    pub timer: u16,
    pub objectives: [ObjectiveState; WORLD_OBJECTIVES],
    pub manpower: u16,
    pub munitions: u16,
    pub fuel: u16,
    /// Front line position code (consumer-defined scale).
    pub front_line_pos: u16,
}

impl Codec for WorldState {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.timer as u64, 16)?;
        for o in &self.objectives {
            w.write_bits(o.owner as u64, 2)?;
            if o.capture > 127 {
                return Err(CodecError::Overflow);
            }
            w.write_bits(o.capture as u64, 7)?;
        }
        w.write_bits(self.manpower as u64, 16)?;
        w.write_bits(self.munitions as u64, 16)?;
        w.write_bits(self.fuel as u64, 16)?;
        w.write_bits(self.front_line_pos as u64, 16)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        let timer = r.read_bits(16)? as u16;
        let mut objectives = [ObjectiveState {
            owner: 0,
            capture: 0,
        }; WORLD_OBJECTIVES];
        for o in objectives.iter_mut() {
            o.owner = r.read_bits(2)? as u8;
            o.capture = r.read_bits(7)? as u8;
        }
        Ok(Self {
            timer,
            objectives,
            manpower: r.read_bits(16)? as u16,
            munitions: r.read_bits(16)? as u16,
            fuel: r.read_bits(16)? as u16,
            front_line_pos: r.read_bits(16)? as u16,
        })
    }
}

pub fn encode_team_state(m: &TeamState) -> Result<Vec<u8>, CodecError> {
    encode_whole(m)
}
pub fn decode_team_state(b: &[u8]) -> Result<TeamState, CodecError> {
    decode_whole(b)
}
pub fn encode_world_state(m: &WorldState) -> Result<Vec<u8>, CodecError> {
    encode_whole(m)
}
pub fn decode_world_state(b: &[u8]) -> Result<WorldState, CodecError> {
    decode_whole(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::c0::{decode_c0_client, encode_c0_client, ClientMsgC0, MarkerPlace};

    fn state() -> TeamStateWithMarkers {
        TeamStateWithMarkers {
            team: TeamState {
                tick: 0x1234,
                members: Vec::new(),
            },
            markers: alloc::vec![TeamMarker {
                kind: TeamMarkerKind::NeedAmmo,
                x: 0xabc,
                z: 0xdef,
                ttl_s: 20,
            }],
        }
    }

    #[test]
    fn marker_wire_vector_preserves_legacy_prefix_and_request_layout() {
        let mut msg = state();
        let legacy = [0x12, 0x34, 0];
        let extended = [0x12, 0x34, 0, 1, 4, 0xab, 0xcd, 0xef, 20];
        assert_eq!(
            &extended[..legacy.len()],
            encode_team_state(&msg.team).unwrap()
        );
        assert_eq!(encode_team_state_with_markers(&msg).unwrap(), extended);
        assert_eq!(decode_team_state_with_markers(&extended), Ok(msg.clone()));
        assert_eq!(decode_team_state(&extended), Err(CodecError::Trailing));
        msg.markers.clear();
        assert_eq!(decode_team_state_with_markers(&legacy), Ok(msg.clone()));
        assert_eq!(encode_team_state_with_markers(&msg).unwrap(), legacy);

        let request = ClientMsgC0::MarkerPlace(MarkerPlace {
            kind: TeamMarkerKind::NeedAmmo as u8,
            pos: [0x1abc, 0x9def],
            ttl: 0xff,
        });
        let request_bytes = [0x14, 4, 0x1a, 0xbc, 0x9d, 0xef, 0xff];
        assert_eq!(encode_c0_client(&request).unwrap(), request_bytes);
        assert_eq!(decode_c0_client(&request_bytes), Ok(request));
    }

    #[test]
    fn marker_payload_at_cap_fits_wire_budget() {
        let mut msg = state();
        msg.team.members.resize(
            MAX_TEAM_MEMBERS,
            TeamMember {
                x: 2047,
                z: 2047,
                role: 15,
                state: 7,
                facing: 7,
            },
        );
        msg.markers[0].x = MAX_MARKER_CELL;
        msg.markers.resize(MAX_TEAM_MARKERS, msg.markers[0]);
        let bytes = encode_team_state_with_markers(&msg).unwrap();
        assert_eq!(bytes.len(), 520);
        assert_eq!(bytes.len(), MAX_TEAM_STATE_WITH_MARKERS_BYTES);
        assert_eq!(decode_team_state_with_markers(&bytes), Ok(msg.clone()));
        msg.markers.push(msg.markers[0]);
        assert_eq!(
            encode_team_state_with_markers(&msg),
            Err(CodecError::TooLong)
        );
        msg.markers.pop();
        msg.markers[0].x += 1;
        assert_eq!(
            encode_team_state_with_markers(&msg),
            Err(CodecError::Overflow)
        );
        assert_eq!(
            decode_team_state_with_markers(&[0; MAX_TEAM_STATE_WITH_MARKERS_BYTES + 1]),
            Err(CodecError::TooLong)
        );
    }

    #[test]
    fn marker_codec_rejects_malformed_extensions() {
        let mut msg = state();
        let mut bytes = encode_team_state_with_markers(&msg).unwrap();
        for len in 4..bytes.len() {
            assert!(decode_team_state_with_markers(&bytes[..len]).is_err());
        }
        bytes.push(0);
        assert_eq!(
            decode_team_state_with_markers(&bytes),
            Err(CodecError::Trailing)
        );
        bytes.pop();
        bytes[2] |= 1;
        assert_eq!(
            decode_team_state_with_markers(&bytes),
            Err(CodecError::Trailing)
        );
        bytes[2] &= !1;
        bytes[3] = 65;
        assert_eq!(
            decode_team_state_with_markers(&bytes),
            Err(CodecError::TooLong)
        );
        bytes[3] = 0;
        assert_eq!(
            decode_team_state_with_markers(&bytes[..4]),
            Err(CodecError::InvalidValue)
        );
        bytes[3] = 1;
        for unsupported in [1, 2, 255] {
            bytes[4] = unsupported;
            assert_eq!(
                decode_team_state_with_markers(&bytes),
                Err(CodecError::InvalidTag)
            );
        }
        bytes[4] = TeamMarkerKind::NeedAmmo as u8;
        bytes[8] = 0;
        assert_eq!(
            decode_team_state_with_markers(&bytes),
            Err(CodecError::InvalidValue)
        );
        msg.markers[0].ttl_s = 0;
        assert_eq!(
            encode_team_state_with_markers(&msg),
            Err(CodecError::InvalidValue)
        );
    }
}
