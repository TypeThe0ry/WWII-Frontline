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
