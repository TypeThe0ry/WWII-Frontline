//! C0 control messages (reliable; ARCHITECTURE §6.8.1, §8.2).
//!
//! Wire form of a C0 message: `msg_type: u8` followed by the fields in declaration order,
//! the whole thing zero padded to a byte boundary. Fields may only be appended at the end.

use alloc::string::String;
use alloc::vec::Vec;

use crate::bitio::{BitReader, BitWriter};
use crate::channel::c0_type;
use crate::codec::{Codec, CodecError};

/// Maximum `Hello.ticket` length in bytes.
pub const MAX_TICKET_LEN: usize = 256;
const TICKET_LEN_BITS: u8 = 9;

/// Client understands the optional C3 tactical marker extension. Without this opt-in, the
/// server must send the original member-only C3 payload, even when markers are active.
pub const CAP_TEAM_MARKERS: u8 = 1 << 2;

fn w8(w: &mut BitWriter, v: u8) -> Result<(), CodecError> {
    w.write_bits(v as u64, 8)
}
fn r8(r: &mut BitReader<'_>) -> Result<u8, CodecError> {
    Ok(r.read_bits(8)? as u8)
}

/// Client -> server handshake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hello {
    pub protocol_version: u16,
    pub data_hash: u64,
    /// Opaque join ticket, at most [`MAX_TICKET_LEN`] bytes.
    pub ticket: Vec<u8>,
    pub client_build: u32,
    /// bit0 webgpu, bit1 webtransport, bit2 [`CAP_TEAM_MARKERS`].
    pub caps: u8,
}

impl Codec for Hello {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        if self.ticket.len() > MAX_TICKET_LEN {
            return Err(CodecError::TooLong);
        }
        w.write_bits(self.protocol_version as u64, 16)?;
        w.write_bits(self.data_hash, 64)?;
        w.write_bytes_len_prefixed(&self.ticket, TICKET_LEN_BITS)?;
        w.write_bits(self.client_build as u64, 32)?;
        w8(w, self.caps)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Hello {
            protocol_version: r.read_bits(16)? as u16,
            data_hash: r.read_bits(64)?,
            ticket: r.read_bytes_len_prefixed(TICKET_LEN_BITS, MAX_TICKET_LEN)?,
            client_build: r.read_bits(32)? as u32,
            caps: r8(r)?,
        })
    }
}

/// Server -> client handshake answer. LiveKit fields are appended in M2 (end only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Welcome {
    pub player_id: u8,
    pub server_tick: u32,
    pub tick_hz: u8,
    pub match_seed: u64,
    pub data_hash: u64,
    pub map_id: u8,
    pub mode: u8,
    pub team: u8,
    /// Suggested client lead over the server tick, in ticks.
    pub client_tick_offset: i16,
}

impl Codec for Welcome {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.player_id)?;
        w.write_bits(self.server_tick as u64, 32)?;
        w8(w, self.tick_hz)?;
        w.write_bits(self.match_seed, 64)?;
        w.write_bits(self.data_hash, 64)?;
        w8(w, self.map_id)?;
        w8(w, self.mode)?;
        w8(w, self.team)?;
        w.write_i(self.client_tick_offset as i64, 16)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Welcome {
            player_id: r8(r)?,
            server_tick: r.read_bits(32)? as u32,
            tick_hz: r8(r)?,
            match_seed: r.read_bits(64)?,
            data_hash: r.read_bits(64)?,
            map_id: r8(r)?,
            mode: r8(r)?,
            team: r8(r)?,
            client_tick_offset: r.read_i(16)? as i16,
        })
    }
}

/// Why a handshake was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RejectReason {
    VersionMismatch = 1,
    ServerFull = 2,
    Banned = 3,
    InvalidTicket = 4,
    DataHashMismatch = 5,
}

impl RejectReason {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            1 => Self::VersionMismatch,
            2 => Self::ServerFull,
            3 => Self::Banned,
            4 => Self::InvalidTicket,
            5 => Self::DataHashMismatch,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reject {
    pub reason: RejectReason,
}

impl Codec for Reject {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.reason as u8)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        let reason = RejectReason::from_u8(r8(r)?).ok_or(CodecError::InvalidTag)?;
        Ok(Reject { reason })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ping {
    pub client_time: u32,
}

impl Codec for Ping {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.client_time as u64, 32)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Ping {
            client_time: r.read_bits(32)? as u32,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pong {
    pub client_time: u32,
    pub server_tick: u32,
    pub server_time_ms: u32,
}

impl Codec for Pong {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.client_time as u64, 32)?;
        w.write_bits(self.server_tick as u64, 32)?;
        w.write_bits(self.server_time_ms as u64, 32)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Pong {
            client_time: r.read_bits(32)? as u32,
            server_tick: r.read_bits(32)? as u32,
            server_time_ms: r.read_bits(32)? as u32,
        })
    }
}

/// Disconnect reason codes (the field is a raw `u8`; unknown codes are tolerated).
pub mod disconnect_reason {
    pub const NORMAL: u8 = 0;
    pub const TIMEOUT: u8 = 1;
    pub const KICKED: u8 = 2;
    pub const SERVER_SHUTDOWN: u8 = 3;
    pub const PROTOCOL_ERROR: u8 = 4;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disconnect {
    pub reason: u8,
}

impl Codec for Disconnect {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.reason)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Disconnect { reason: r8(r)? })
    }
}

/// String limits (ARCHITECTURE §19.1). Length prefix is 8 bits, in bytes.
pub const MAX_NICK_LEN: usize = 24;
pub const MAX_CHAT_LEN: usize = 200;
pub const MAX_REPORT_NOTE_LEN: usize = 120;
pub const MAX_NAME_KEY_LEN: usize = 32;

/// Write `s` as `len:8 | UTF-8 bytes`, rejecting more than `max` bytes.
pub fn write_str(w: &mut BitWriter, s: &str, max: usize) -> Result<(), CodecError> {
    if s.len() > max {
        return Err(CodecError::TooLong);
    }
    w.write_bytes_len_prefixed(s.as_bytes(), 8)
}

/// Read a string written by [`write_str`]; invalid UTF-8 is [`CodecError::InvalidValue`].
pub fn read_str(r: &mut BitReader<'_>, max: usize) -> Result<String, CodecError> {
    let b = r.read_bytes_len_prefixed(8, max)?;
    String::from_utf8(b).map_err(|_| CodecError::InvalidValue)
}

macro_rules! u8_enum {
    ($(#[$m:meta])* $name:ident { $($var:ident = $val:expr),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(u8)]
        pub enum $name { $($var = $val),+ }
        impl $name {
            pub fn from_u8(v: u8) -> Option<Self> {
                match v { $($val => Some(Self::$var),)+ _ => None }
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeployRequest {
    pub spawn_point: u16,
    pub role: u8,
    pub loadout: u8,
    pub squad: u8,
}
impl Codec for DeployRequest {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.spawn_point as u64, 16)?;
        w8(w, self.role)?;
        w8(w, self.loadout)?;
        w8(w, self.squad)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            spawn_point: r.read_bits(16)? as u16,
            role: r8(r)?,
            loadout: r8(r)?,
            squad: r8(r)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeployResult {
    pub ok: bool,
    pub spawn_tick: u32,
    pub reason: u8,
}
impl Codec for DeployResult {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bool(self.ok)?;
        w.write_bits(self.spawn_tick as u64, 32)?;
        w8(w, self.reason)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            ok: r.read_bool()?,
            spawn_tick: r.read_bits(32)? as u32,
            reason: r8(r)?,
        })
    }
}

u8_enum!(
    /// `lock`/`unlock`/`invite` are M1 additions (GD §4.3).
    SquadOpKind {
        Create = 1, Join = 2, Leave = 3, Kick = 4, Promote = 5, Rename = 6,
        Lock = 7, Unlock = 8, Invite = 9,
    }
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SquadOp {
    pub op: SquadOpKind,
    pub squad: u8,
    pub target: u8,
    pub name_key: String,
}
impl Codec for SquadOp {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.op as u8)?;
        w8(w, self.squad)?;
        w8(w, self.target)?;
        write_str(w, &self.name_key, MAX_NAME_KEY_LEN)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            op: SquadOpKind::from_u8(r8(r)?).ok_or(CodecError::InvalidTag)?,
            squad: r8(r)?,
            target: r8(r)?,
            name_key: read_str(r, MAX_NAME_KEY_LEN)?,
        })
    }
}

u8_enum!(ChatChannel { All = 0, Team = 1, Squad = 2, Command = 3 });

/// Bidirectional. C->S: `sender` is ignored by the server (it uses the connection's player).
/// S->C: `sender` is the originating `player_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chat {
    pub channel: ChatChannel,
    pub text: String,
    pub sender: u8,
}
impl Codec for Chat {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.channel as u8)?;
        write_str(w, &self.text, MAX_CHAT_LEN)?;
        w8(w, self.sender)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            channel: ChatChannel::from_u8(r8(r)?).ok_or(CodecError::InvalidTag)?,
            text: read_str(r, MAX_CHAT_LEN)?,
            sender: r8(r)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkerPlace {
    /// Manual marker tag (see [`super::team::TeamMarkerKind`]).
    pub kind: u8,
    /// Normalized x/z: `map_origin + code / 65535 * map_extent` in world metres.
    pub pos: [u16; 2],
    /// Legacy field; ignored. The server sets lifespan by marker kind.
    pub ttl: u8,
}
impl Codec for MarkerPlace {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.kind)?;
        w.write_bits(self.pos[0] as u64, 16)?;
        w.write_bits(self.pos[1] as u64, 16)?;
        w8(w, self.ttl)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            kind: r8(r)?,
            pos: [r.read_bits(16)? as u16, r.read_bits(16)? as u16],
            ttl: r8(r)?,
        })
    }
}

/// Client preferences that never affect the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// Requested view distance in metres (server clamps).
    pub view_dist_m: u16,
    /// bit0: mute proximity voice.
    pub flags: u8,
}
impl Codec for Settings {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.view_dist_m as u64, 16)?;
        w8(w, self.flags)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            view_dist_m: r.read_bits(16)? as u16,
            flags: r8(r)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub target: u8,
    pub reason: u8,
    pub note: String,
}
impl Codec for Report {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.target)?;
        w8(w, self.reason)?;
        write_str(w, &self.note, MAX_REPORT_NOTE_LEN)
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            target: r8(r)?,
            reason: r8(r)?,
            note: read_str(r, MAX_REPORT_NOTE_LEN)?,
        })
    }
}

u8_enum!(PlacementSlot { Garrison = 0, Supply = 1, Ammo = 2 });

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementRequest {
    pub id: u16,
    pub action: PlacementAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementAction {
    Preview {
        slot: PlacementSlot,
        yaw: u16,
        pitch: i16,
    },
    Commit {
        preview_id: u16,
    },
}

impl Codec for PlacementRequest {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.id as u64, 16)?;
        match self.action {
            PlacementAction::Preview { slot, yaw, pitch } => {
                w8(w, slot as u8)?;
                w.write_bits(yaw as u64, 16)?;
                w.write_bits(pitch as u16 as u64, 16)
            }
            PlacementAction::Commit { preview_id } => {
                w8(w, 3)?;
                w.write_bits(preview_id as u64, 16)
            }
        }
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        let id = r.read_bits(16)? as u16;
        let kind = r8(r)?;
        let action = if kind == 3 {
            PlacementAction::Commit {
                preview_id: r.read_bits(16)? as u16,
            }
        } else {
            PlacementAction::Preview {
                slot: PlacementSlot::from_u8(kind).ok_or(CodecError::InvalidTag)?,
                yaw: r.read_bits(16)? as u16,
                pitch: r.read_bits(16)? as u16 as i16,
            }
        };
        Ok(Self { id, action })
    }
}

u8_enum!(PlacementReason {
    Ok = 0, NotAlive = 1, WrongRole = 2, NeedSquad = 3, NotSupport = 4, NotCarrying = 5,
    NoTarget = 6, NoBuildZone = 7, TooCloseHq = 8, TooCloseOwnGarrison = 9,
    TooCloseObjective = 10, Slope = 11, Water = 12, NoSupplies = 13, TeamCap = 14,
    SectorCap = 15, LimitPerLife = 16, Cooldown = 17, Stacked = 18, Rejected = 19,
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementResult {
    pub id: u16,
    pub reason: PlacementReason,
    pub supply_cost: u8,
    /// The surface position for a preview, in centimetres. Commit results omit it.
    pub target_cm: Option<[i32; 3]>,
}

impl Codec for PlacementResult {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.id as u64, 16)?;
        w8(w, self.reason as u8)?;
        w8(w, self.supply_cost)?;
        w8(w, u8::from(self.target_cm.is_some()))?;
        if let Some(target) = self.target_cm {
            for cm in target {
                w.write_bits(cm as u32 as u64, 32)?;
            }
        }
        Ok(())
    }

    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        let id = r.read_bits(16)? as u16;
        let reason = PlacementReason::from_u8(r8(r)?).ok_or(CodecError::InvalidTag)?;
        let supply_cost = r8(r)?;
        let target_cm = match r8(r)? {
            0 => None,
            1 => Some([
                r.read_bits(32)? as i32,
                r.read_bits(32)? as i32,
                r.read_bits(32)? as i32,
            ]),
            _ => return Err(CodecError::InvalidTag),
        };
        Ok(Self {
            id,
            reason,
            supply_cost,
            target_cm,
        })
    }
}

/// Client -> server C0 messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientMsgC0 {
    Hello(Hello),
    Ping(Ping),
    Disconnect(Disconnect),
    DeployRequest(DeployRequest),
    SquadOp(SquadOp),
    Chat(Chat),
    MarkerPlace(MarkerPlace),
    Settings(Settings),
    Report(Report),
    PlacementRequest(PlacementRequest),
}

/// Server -> client C0 messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerMsgC0 {
    Welcome(Welcome),
    Reject(Reject),
    Pong(Pong),
    Disconnect(Disconnect),
    DeployResult(DeployResult),
    Chat(Chat),
    PlacementResult(PlacementResult),
}

impl ClientMsgC0 {
    /// Stable `msg_type` of this message.
    pub fn msg_type(&self) -> u8 {
        match self {
            Self::Hello(_) => c0_type::HELLO,
            Self::Ping(_) => c0_type::PING,
            Self::Disconnect(_) => c0_type::DISCONNECT,
            Self::DeployRequest(_) => c0_type::DEPLOY_REQUEST,
            Self::SquadOp(_) => c0_type::SQUAD_OP,
            Self::Chat(_) => c0_type::CHAT,
            Self::MarkerPlace(_) => c0_type::MARKER_PLACE,
            Self::Settings(_) => c0_type::SETTINGS,
            Self::Report(_) => c0_type::REPORT,
            Self::PlacementRequest(_) => c0_type::PLACEMENT_REQUEST,
        }
    }
}

impl ServerMsgC0 {
    /// Stable `msg_type` of this message.
    pub fn msg_type(&self) -> u8 {
        match self {
            Self::Welcome(_) => c0_type::WELCOME,
            Self::Reject(_) => c0_type::REJECT,
            Self::Pong(_) => c0_type::PONG,
            Self::Disconnect(_) => c0_type::DISCONNECT,
            Self::DeployResult(_) => c0_type::DEPLOY_RESULT,
            Self::PlacementResult(_) => c0_type::PLACEMENT_RESULT,
            Self::Chat(_) => c0_type::CHAT,
        }
    }
}

impl Codec for ClientMsgC0 {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.msg_type())?;
        match self {
            Self::Hello(m) => m.encode(w),
            Self::Ping(m) => m.encode(w),
            Self::Disconnect(m) => m.encode(w),
            Self::DeployRequest(m) => m.encode(w),
            Self::SquadOp(m) => m.encode(w),
            Self::Chat(m) => m.encode(w),
            Self::MarkerPlace(m) => m.encode(w),
            Self::Settings(m) => m.encode(w),
            Self::Report(m) => m.encode(w),
            Self::PlacementRequest(m) => m.encode(w),
        }
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(match r8(r)? {
            c0_type::HELLO => Self::Hello(Hello::decode(r)?),
            c0_type::PING => Self::Ping(Ping::decode(r)?),
            c0_type::DISCONNECT => Self::Disconnect(Disconnect::decode(r)?),
            c0_type::DEPLOY_REQUEST => Self::DeployRequest(DeployRequest::decode(r)?),
            c0_type::SQUAD_OP => Self::SquadOp(SquadOp::decode(r)?),
            c0_type::CHAT => Self::Chat(Chat::decode(r)?),
            c0_type::MARKER_PLACE => Self::MarkerPlace(MarkerPlace::decode(r)?),
            c0_type::SETTINGS => Self::Settings(Settings::decode(r)?),
            c0_type::REPORT => Self::Report(Report::decode(r)?),
            c0_type::PLACEMENT_REQUEST => Self::PlacementRequest(PlacementRequest::decode(r)?),
            _ => return Err(CodecError::InvalidTag),
        })
    }
}

impl Codec for ServerMsgC0 {
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w8(w, self.msg_type())?;
        match self {
            Self::Welcome(m) => m.encode(w),
            Self::Reject(m) => m.encode(w),
            Self::Pong(m) => m.encode(w),
            Self::Disconnect(m) => m.encode(w),
            Self::DeployResult(m) => m.encode(w),
            Self::Chat(m) => m.encode(w),
            Self::PlacementResult(m) => m.encode(w),
        }
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(match r8(r)? {
            c0_type::WELCOME => Self::Welcome(Welcome::decode(r)?),
            c0_type::REJECT => Self::Reject(Reject::decode(r)?),
            c0_type::PONG => Self::Pong(Pong::decode(r)?),
            c0_type::DISCONNECT => Self::Disconnect(Disconnect::decode(r)?),
            c0_type::DEPLOY_RESULT => Self::DeployResult(DeployResult::decode(r)?),
            c0_type::CHAT => Self::Chat(Chat::decode(r)?),
            c0_type::PLACEMENT_RESULT => Self::PlacementResult(PlacementResult::decode(r)?),
            _ => return Err(CodecError::InvalidTag),
        })
    }
}

pub(crate) fn encode_whole<T: Codec>(m: &T) -> Result<Vec<u8>, CodecError> {
    let mut w = BitWriter::new();
    m.encode(&mut w)?;
    Ok(w.finish())
}

pub(crate) fn decode_whole<T: Codec>(bytes: &[u8]) -> Result<T, CodecError> {
    let mut r = BitReader::new(bytes);
    let m = T::decode(&mut r)?;
    r.finish()?;
    Ok(m)
}

pub fn encode_c0_client(m: &ClientMsgC0) -> Result<Vec<u8>, CodecError> {
    encode_whole(m)
}
pub fn decode_c0_client(bytes: &[u8]) -> Result<ClientMsgC0, CodecError> {
    decode_whole(bytes)
}
pub fn encode_c0_server(m: &ServerMsgC0) -> Result<Vec<u8>, CodecError> {
    encode_whole(m)
}
pub fn decode_c0_server(bytes: &[u8]) -> Result<ServerMsgC0, CodecError> {
    decode_whole(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_c0_matches_browser_bytes() {
        let preview = ClientMsgC0::PlacementRequest(PlacementRequest {
            id: 0x1234,
            action: PlacementAction::Preview {
                slot: PlacementSlot::Ammo,
                yaw: 0x4321,
                pitch: -4096,
            },
        });
        let bytes = [0x17, 0x12, 0x34, 2, 0x43, 0x21, 0xf0, 0];
        assert_eq!(encode_c0_client(&preview).unwrap(), bytes);
        assert_eq!(decode_c0_client(&bytes), Ok(preview));
        let commit = ClientMsgC0::PlacementRequest(PlacementRequest {
            id: 1,
            action: PlacementAction::Commit { preview_id: 0x1234 },
        });
        let bytes = [0x17, 0, 1, 3, 0x12, 0x34];
        assert_eq!(encode_c0_client(&commit).unwrap(), bytes);
        assert_eq!(decode_c0_client(&bytes), Ok(commit));
        let result = ServerMsgC0::PlacementResult(PlacementResult {
            id: 0x1234,
            reason: PlacementReason::NoBuildZone,
            supply_cost: 0,
            target_cm: Some([-100, 500, 2400]),
        });
        let bytes = [
            0x18, 0x12, 0x34, 7, 0, 1, 0xff, 0xff, 0xff, 0x9c, 0, 0, 1, 0xf4, 0, 0, 9, 0x60,
        ];
        assert_eq!(encode_c0_server(&result).unwrap(), bytes);
        assert_eq!(decode_c0_server(&bytes), Ok(result));
        assert!(decode_c0_server(&bytes[..17]).is_err());
        assert!(decode_c0_client(&[0x17, 0, 0, 4]).is_err());
    }

    #[test]
    fn ticket_limit() {
        let mk = |n| {
            ClientMsgC0::Hello(Hello {
                protocol_version: 1,
                data_hash: 7,
                ticket: alloc::vec![0xAB; n],
                client_build: 1,
                caps: 3,
            })
        };
        let ok = encode_c0_client(&mk(256)).unwrap();
        assert_eq!(decode_c0_client(&ok).unwrap(), mk(256));
        assert_eq!(encode_c0_client(&mk(257)), Err(CodecError::TooLong));
        // hand-craft a 257-byte ticket on the wire
        let mut w = BitWriter::new();
        w.write_bits(c0_type::HELLO as u64, 8).unwrap();
        w.write_bits(1, 16).unwrap();
        w.write_bits(7, 64).unwrap();
        w.write_bits(257, 9).unwrap();
        for _ in 0..257 {
            w.write_bits(0xAB, 8).unwrap();
        }
        w.write_bits(1, 32).unwrap();
        w.write_bits(3, 8).unwrap();
        assert_eq!(decode_c0_client(&w.finish()), Err(CodecError::TooLong));
    }

    #[test]
    fn bad_reject_reason() {
        assert_eq!(
            decode_c0_server(&[c0_type::REJECT, 0]),
            Err(CodecError::InvalidTag)
        );
        assert_eq!(
            decode_c0_server(&[c0_type::REJECT, 6]),
            Err(CodecError::InvalidTag)
        );
        assert_eq!(decode_c0_server(&[0xEE]), Err(CodecError::InvalidTag));
    }
}
