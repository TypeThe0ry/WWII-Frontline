//! M2 server-rule requests (C0) and notifications (C4).

use super::c0::{r8, u8_enum, w8};
use crate::{BitReader, BitWriter, Codec, CodecError};

u8_enum! {
    /// Player-targeted vote kinds. Map votes are server-initiated and have separate messages.
    VoteKind { Kick = 0, RemoveCommander = 1, Mute = 2 }
}

u8_enum! {
    /// Outcomes and invitations reported to the affected client.
    TeamBalanceCode {
        Invite = 1, Moved = 2, Denied = 3, AfkWarning = 4, AfkCountdown = 5,
        TkWarning = 6, LockRole = 7, LockVehicle = 8, VoteDenied = 9, DriverVacant = 10,
        Muted = 11, RepeatInput = 12, AfkCancelled = 13,
    }
}

pub mod team_move_flags {
    pub const FORCED: i32 = 1 << 0;
    pub const VOLUNTEER_BONUS: i32 = 1 << 1;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoteStart {
    pub kind: VoteKind,
    pub target: u8,
}

impl Codec for VoteStart {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.kind as u8)?;
        w8(w, self.target)
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            kind: VoteKind::from_u8(r8(r)?).ok_or(CodecError::InvalidValue)?,
            target: r8(r)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoteCast {
    pub vote_id: u32,
    pub yes: bool,
}

impl Codec for VoteCast {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.vote_id.into(), 32)?;
        w.write_bool(self.yes)
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            vote_id: r.read_bits(32)? as u32,
            yes: r.read_bool()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamSwitchRequest {
    /// As in `Welcome.team`, 0 is US and 1 is GER. The server can deny other values.
    pub team: u8,
    /// True only when accepting an outstanding balance invitation.
    pub accept_offer: bool,
}

impl Codec for TeamSwitchRequest {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.team)?;
        w.write_bool(self.accept_offer)
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            team: r8(r)?,
            accept_offer: r.read_bool()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgivePunish {
    /// Echoes `TkPrompt.victim_event` from the prompt being answered.
    pub victim_event: u16,
    pub punish: bool,
}

impl Codec for ForgivePunish {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.victim_event.into(), 16)?;
        w.write_bool(self.punish)
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            victim_event: r.read_bits(16)? as u16,
            punish: r.read_bool()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoteState {
    pub vote_id: u32,
    pub kind: VoteKind,
    pub target: u8,
    pub yes: u8,
    pub no: u8,
    pub ends_in_s: u8,
}

impl Codec for VoteState {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.vote_id.into(), 32)?;
        for v in [
            self.kind as u8,
            self.target,
            self.yes,
            self.no,
            self.ends_in_s,
        ] {
            w8(w, v)?;
        }
        Ok(())
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            vote_id: r.read_bits(32)? as u32,
            kind: VoteKind::from_u8(r8(r)?).ok_or(CodecError::InvalidValue)?,
            target: r8(r)?,
            yes: r8(r)?,
            no: r8(r)?,
            ends_in_s: r8(r)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoteResult {
    pub vote_id: u32,
    pub passed: bool,
}

impl Codec for VoteResult {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.vote_id.into(), 32)?;
        w.write_bool(self.passed)
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            vote_id: r.read_bits(32)? as u32,
            passed: r.read_bool()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TkPrompt {
    pub killer: u8,
    pub expires_s: u8,
    /// Opaque event ID the victim sends back in `ForgivePunish`.
    pub victim_event: u16,
}

impl Codec for TkPrompt {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.killer)?;
        w8(w, self.expires_s)?;
        w.write_bits(self.victim_event.into(), 16)
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            killer: r8(r)?,
            expires_s: r8(r)?,
            victim_event: r.read_bits(16)? as u16,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamBalanceNotice {
    pub code: TeamBalanceCode,
    /// Invite: `[target_team, expires_s]` (20% XP bonus).
    /// Moved: `[new_team | (flags << 8), effective_tick as i32]`.
    /// Denied/VoteDenied: `[rule_error, 0]`. LockRole: `[mode, seconds]`.
    /// AFK/TK warnings, AFK countdown and LockVehicle: `[seconds, 0]`.
    /// DriverVacant: `[vehicle_id, 0]`. Muted: `[player, 0]`. RepeatInput/AfkCancelled: `[0, 0]`.
    pub args: [i32; 2],
}

impl Codec for TeamBalanceNotice {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.code as u8)?;
        w.write_i(self.args[0].into(), 32)?;
        w.write_i(self.args[1].into(), 32)
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            code: TeamBalanceCode::from_u8(r8(r)?).ok_or(CodecError::InvalidValue)?,
            args: [r.read_i(32)? as i32, r.read_i(32)? as i32],
        })
    }
}

pub const MAP_VOTE_CHOICES: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapVoteCast {
    pub vote_id: u32,
    /// Index into `MapVoteState.candidates`, not a map ID.
    pub choice: u8,
}

fn map_choice(choice: u8) -> Result<u8, CodecError> {
    if usize::from(choice) < MAP_VOTE_CHOICES {
        Ok(choice)
    } else {
        Err(CodecError::InvalidValue)
    }
}

impl Codec for MapVoteCast {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.vote_id.into(), 32)?;
        w8(w, map_choice(self.choice)?)
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            vote_id: r.read_bits(32)? as u32,
            choice: map_choice(r8(r)?)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapVoteState {
    pub vote_id: u32,
    /// Map IDs as in `Welcome.map_id`, in a fixed order for the lifetime of this vote.
    pub candidates: [u8; MAP_VOTE_CHOICES],
    pub votes: [u8; MAP_VOTE_CHOICES],
    pub ends_in_s: u8,
}

impl Codec for MapVoteState {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.vote_id.into(), 32)?;
        for v in self.candidates.into_iter().chain(self.votes) {
            w8(w, v)?;
        }
        w8(w, self.ends_in_s)
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            vote_id: r.read_bits(32)? as u32,
            candidates: [r8(r)?, r8(r)?, r8(r)?],
            votes: [r8(r)?, r8(r)?, r8(r)?],
            ends_in_s: r8(r)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapVoteResult {
    pub vote_id: u32,
    /// The winning map ID, not a candidate index.
    pub winner: u8,
}

impl Codec for MapVoteResult {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.vote_id.into(), 32)?;
        w8(w, self.winner)
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            vote_id: r.read_bits(32)? as u32,
            winner: r8(r)?,
        })
    }
}
