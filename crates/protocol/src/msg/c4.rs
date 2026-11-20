//! C4 reliable server events (ARCHITECTURE §6.8.3).
//!
//! Wire form: `msg_type: u8` then fields in declaration order, zero padded to a byte.

use alloc::string::String;
use alloc::vec::Vec;

use super::c0::{decode_whole, encode_whole, read_str, write_str, MAX_NICK_LEN};
use super::vote::{
    MapVoteResult, MapVoteState, TeamBalanceNotice, TkPrompt, VoteResult, VoteState,
};
use crate::bitio::{BitReader, BitWriter};
use crate::channel::c4_type;
use crate::codec::{Codec, CodecError};

fn w8(w: &mut BitWriter, v: u8) -> Result<(), CodecError> {
    w.write_bits(v as u64, 8)
}
fn r8(r: &mut BitReader<'_>) -> Result<u8, CodecError> {
    Ok(r.read_bits(8)? as u8)
}

/// `killer` value meaning "environment".
pub const KILLER_ENV: u8 = 0xFF;
/// Width of `Kill.dist_q` (0..1024 m in 0.25 m steps).
pub const KILL_DIST_BITS: u8 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kill {
    pub victim: u8,
    pub killer: u8,
    pub weapon: u8,
    pub headshot: bool,
    /// 12-bit distance code (0.25 m per step).
    pub dist_q: u16,
    pub assist1: u8,
    pub assist2: u8,
}
impl Codec for Kill {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.victim)?;
        w8(w, self.killer)?;
        w8(w, self.weapon)?;
        w.write_bool(self.headshot)?;
        w.write_bits(self.dist_q as u64, KILL_DIST_BITS)?;
        w8(w, self.assist1)?;
        w8(w, self.assist2)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            victim: r8(r)?,
            killer: r8(r)?,
            weapon: r8(r)?,
            headshot: r.read_bool()?,
            dist_q: r.read_bits(KILL_DIST_BITS)? as u16,
            assist1: r8(r)?,
            assist2: r8(r)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Death {
    pub victim: u8,
    pub respawn_after: u8,
    pub can_be_revived: bool,
}
impl Codec for Death {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.victim)?;
        w8(w, self.respawn_after)?;
        w.write_bool(self.can_be_revived)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            victim: r8(r)?,
            respawn_after: r8(r)?,
            can_be_revived: r.read_bool()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revive {
    pub medic: u8,
    pub target: u8,
}
impl Codec for Revive {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.medic)?;
        w8(w, self.target)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            medic: r8(r)?,
            target: r8(r)?,
        })
    }
}

/// The two high bits of [`ScoreDelta::kind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ScoreCategory {
    Combat = 0,
    Offense = 1,
    Defense = 2,
    Support = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreDelta {
    pub player: u8,
    /// Packed category (bits 7-6) and stable category-local reason ID (bits 5-0).
    pub kind: u8,
    pub value: i16,
}
impl ScoreDelta {
    pub const fn category(&self) -> ScoreCategory {
        match self.kind >> 6 {
            0 => ScoreCategory::Combat,
            1 => ScoreCategory::Offense,
            2 => ScoreCategory::Defense,
            _ => ScoreCategory::Support,
        }
    }

    pub const fn reason(&self) -> u8 {
        self.kind & 0x3f
    }
}
impl Codec for ScoreDelta {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.player)?;
        w8(w, self.kind)?;
        w.write_i(self.value as i64, 16)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            player: r8(r)?,
            kind: r8(r)?,
            value: r.read_i(16)? as i16,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectiveChange {
    pub objective: u8,
    pub owner: u8,
    pub reason: u8,
}
impl Codec for ObjectiveChange {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.objective)?;
        w8(w, self.owner)?;
        w8(w, self.reason)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            objective: r8(r)?,
            owner: r8(r)?,
            reason: r8(r)?,
        })
    }
}

/// Spawn point kinds. `Boat` is reserved for M3 but already on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpawnKind {
    Garrison = 0,
    Outpost = 1,
    Airhead = 2,
    Op = 3,
    Vehicle = 4,
    Boat = 5,
}
impl SpawnKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Garrison,
            1 => Self::Outpost,
            2 => Self::Airhead,
            3 => Self::Op,
            4 => Self::Vehicle,
            5 => Self::Boat,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnPointState {
    pub id: u16,
    pub kind: SpawnKind,
    pub team: u8,
    /// 16-bit quantised x/z codes.
    pub pos: [u16; 2],
    pub state: u8,
}
impl Codec for SpawnPointState {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.id as u64, 16)?;
        w8(w, self.kind as u8)?;
        w8(w, self.team)?;
        w.write_bits(self.pos[0] as u64, 16)?;
        w.write_bits(self.pos[1] as u64, 16)?;
        w8(w, self.state)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            id: r.read_bits(16)? as u16,
            kind: SpawnKind::from_u8(r8(r)?).ok_or(CodecError::InvalidTag)?,
            team: r8(r)?,
            pos: [r.read_bits(16)? as u16, r.read_bits(16)? as u16],
            state: r8(r)?,
        })
    }
}

/// `PlayerRoster.flags` bits.
pub mod roster_flags {
    pub const DOWNED: u8 = 1;
    pub const COMMANDER: u8 = 2;
    pub const SQUAD_LEADER: u8 = 4;
    /// Player left the match; the entry should be removed.
    pub const LEFT: u8 = 8;
    /// Bot-controlled.
    pub const BOT: u8 = 16;
    /// The player has a living, standing body. Absent for undeployed, downed and KIA players.
    pub const ALIVE: u8 = 32;
}

/// Incremental roster entry; the latest entry for a `player` replaces the previous one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerRoster {
    pub player: u8,
    pub nick: String,
    pub team: u8,
    pub squad: u8,
    pub role: u8,
    pub flags: u8,
}
impl Codec for PlayerRoster {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.player)?;
        write_str(w, &self.nick, MAX_NICK_LEN)?;
        w8(w, self.team)?;
        w8(w, self.squad)?;
        w8(w, self.role)?;
        w8(w, self.flags)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            player: r8(r)?,
            nick: read_str(r, MAX_NICK_LEN)?,
            team: r8(r)?,
            squad: r8(r)?,
            role: r8(r)?,
            flags: r8(r)?,
        })
    }
}

/// `Notice` codes for server-side snapshot budget changes. The envelope is unchanged.
pub mod notice_code {
    pub const NET_DEGRADED: u16 = 0x0200;
    pub const NET_RECOVERED: u16 = 0x0201;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Notice {
    pub code: u16,
    pub args: [i32; 3],
}
impl Codec for Notice {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.code as u64, 16)?;
        for a in self.args {
            w.write_i(a as i64, 32)?;
        }
        Ok(())
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            code: r.read_bits(16)? as u16,
            args: [
                r.read_i(32)? as i32,
                r.read_i(32)? as i32,
                r.read_i(32)? as i32,
            ],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    Warmup = 0,
    Live = 1,
    Overtime = 2,
    Ended = 3,
}
impl Phase {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Warmup,
            1 => Self::Live,
            2 => Self::Overtime,
            3 => Self::Ended,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchPhase {
    pub phase: Phase,
    /// Server tick at which the phase ends (0 = open ended).
    pub ends_at: u32,
}
impl Codec for MatchPhase {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.phase as u8)?;
        w.write_bits(self.ends_at as u64, 32)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            phase: Phase::from_u8(r8(r)?).ok_or(CodecError::InvalidTag)?,
            ends_at: r.read_bits(32)? as u32,
        })
    }
}

/// Team-scoped M2-011 facility. State is building (0), done (1), or removed (2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructureState {
    pub id: u16,
    /// Repair station (0) or AT gun (1).
    pub kind: u8,
    pub pos: [u16; 2],
    pub state: u8,
}

impl Codec for StructureState {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        if self.kind > 1 || self.state > 2 {
            return Err(CodecError::InvalidTag);
        }
        w.write_bits(u64::from(self.id), 16)?;
        w8(w, self.kind)?;
        w.write_bits(u64::from(self.pos[0]), 16)?;
        w.write_bits(u64::from(self.pos[1]), 16)?;
        w8(w, self.state)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        let s = Self {
            id: r.read_bits(16)? as u16,
            kind: r8(r)?,
            pos: [r.read_bits(16)? as u16, r.read_bits(16)? as u16],
            state: r8(r)?,
        };
        if s.kind > 1 || s.state > 2 {
            Err(CodecError::InvalidTag)
        } else {
            Ok(s)
        }
    }
}

/// Server -> client reliable events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerEventC4 {
    Kill(Kill),
    Death(Death),
    Revive(Revive),
    ScoreDelta(ScoreDelta),
    ObjectiveChange(ObjectiveChange),
    SpawnPointState(SpawnPointState),
    PlayerRoster(PlayerRoster),
    Notice(Notice),
    MatchPhase(MatchPhase),
    VoteState(VoteState),
    VoteResult(VoteResult),
    TkPrompt(TkPrompt),
    TeamBalanceNotice(TeamBalanceNotice),
    MapVoteState(MapVoteState),
    MapVoteResult(MapVoteResult),
    StructureState(StructureState),
}

impl ServerEventC4 {
    pub fn msg_type(&self) -> u8 {
        match self {
            Self::Kill(_) => c4_type::KILL,
            Self::Death(_) => c4_type::DEATH,
            Self::Revive(_) => c4_type::REVIVE,
            Self::ScoreDelta(_) => c4_type::SCORE_DELTA,
            Self::ObjectiveChange(_) => c4_type::OBJECTIVE_CHANGE,
            Self::SpawnPointState(_) => c4_type::SPAWN_POINT_STATE,
            Self::PlayerRoster(_) => c4_type::PLAYER_ROSTER,
            Self::Notice(_) => c4_type::NOTICE,
            Self::MatchPhase(_) => c4_type::MATCH_PHASE,
            Self::VoteState(_) => c4_type::VOTE_STATE,
            Self::VoteResult(_) => c4_type::VOTE_RESULT,
            Self::TkPrompt(_) => c4_type::TK_PROMPT,
            Self::TeamBalanceNotice(_) => c4_type::TEAM_BALANCE_NOTICE,
            Self::MapVoteState(_) => c4_type::MAP_VOTE_STATE,
            Self::MapVoteResult(_) => c4_type::MAP_VOTE_RESULT,
            Self::StructureState(_) => c4_type::STRUCTURE_STATE,
        }
    }
}

impl Codec for ServerEventC4 {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.msg_type())?;
        match self {
            Self::Kill(m) => m.encode(w),
            Self::Death(m) => m.encode(w),
            Self::Revive(m) => m.encode(w),
            Self::ScoreDelta(m) => m.encode(w),
            Self::ObjectiveChange(m) => m.encode(w),
            Self::SpawnPointState(m) => m.encode(w),
            Self::PlayerRoster(m) => m.encode(w),
            Self::Notice(m) => m.encode(w),
            Self::MatchPhase(m) => m.encode(w),
            Self::VoteState(m) => m.encode(w),
            Self::VoteResult(m) => m.encode(w),
            Self::TkPrompt(m) => m.encode(w),
            Self::TeamBalanceNotice(m) => m.encode(w),
            Self::MapVoteState(m) => m.encode(w),
            Self::MapVoteResult(m) => m.encode(w),
            Self::StructureState(m) => m.encode(w),
        }
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(match r8(r)? {
            c4_type::KILL => Self::Kill(Kill::decode(r)?),
            c4_type::DEATH => Self::Death(Death::decode(r)?),
            c4_type::REVIVE => Self::Revive(Revive::decode(r)?),
            c4_type::SCORE_DELTA => Self::ScoreDelta(ScoreDelta::decode(r)?),
            c4_type::OBJECTIVE_CHANGE => Self::ObjectiveChange(ObjectiveChange::decode(r)?),
            c4_type::SPAWN_POINT_STATE => Self::SpawnPointState(SpawnPointState::decode(r)?),
            c4_type::PLAYER_ROSTER => Self::PlayerRoster(PlayerRoster::decode(r)?),
            c4_type::NOTICE => Self::Notice(Notice::decode(r)?),
            c4_type::MATCH_PHASE => Self::MatchPhase(MatchPhase::decode(r)?),
            c4_type::VOTE_STATE => Self::VoteState(VoteState::decode(r)?),
            c4_type::VOTE_RESULT => Self::VoteResult(VoteResult::decode(r)?),
            c4_type::TK_PROMPT => Self::TkPrompt(TkPrompt::decode(r)?),
            c4_type::TEAM_BALANCE_NOTICE => Self::TeamBalanceNotice(TeamBalanceNotice::decode(r)?),
            c4_type::MAP_VOTE_STATE => Self::MapVoteState(MapVoteState::decode(r)?),
            c4_type::MAP_VOTE_RESULT => Self::MapVoteResult(MapVoteResult::decode(r)?),
            c4_type::STRUCTURE_STATE => Self::StructureState(StructureState::decode(r)?),
            _ => return Err(CodecError::InvalidTag),
        })
    }
}

pub fn encode_c4(m: &ServerEventC4) -> Result<Vec<u8>, CodecError> {
    encode_whole(m)
}
pub fn decode_c4(bytes: &[u8]) -> Result<ServerEventC4, CodecError> {
    decode_whole(bytes)
}
