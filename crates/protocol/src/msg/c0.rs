//! C0 control messages (reliable; ARCHITECTURE §6.8.1, §8.2).
//!
//! Wire form of a C0 message: `msg_type: u8` followed by the fields in declaration order,
//! the whole thing zero padded to a byte boundary. Fields may only be appended at the end.

use alloc::vec::Vec;

use crate::bitio::{BitReader, BitWriter};
use crate::channel::c0_type;
use crate::codec::{Codec, CodecError};

/// Maximum `Hello.ticket` length in bytes.
pub const MAX_TICKET_LEN: usize = 256;
const TICKET_LEN_BITS: u8 = 9;

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
    /// bit0 webgpu, bit1 webtransport.
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

/// Client -> server C0 messages (M0 subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientMsgC0 {
    Hello(Hello),
    Ping(Ping),
    Disconnect(Disconnect),
}

/// Server -> client C0 messages (M0 subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerMsgC0 {
    Welcome(Welcome),
    Reject(Reject),
    Pong(Pong),
    Disconnect(Disconnect),
}

impl ClientMsgC0 {
    /// Stable `msg_type` of this message.
    pub fn msg_type(&self) -> u8 {
        match self {
            Self::Hello(_) => c0_type::HELLO,
            Self::Ping(_) => c0_type::PING,
            Self::Disconnect(_) => c0_type::DISCONNECT,
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
        }
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(match r8(r)? {
            c0_type::HELLO => Self::Hello(Hello::decode(r)?),
            c0_type::PING => Self::Ping(Ping::decode(r)?),
            c0_type::DISCONNECT => Self::Disconnect(Disconnect::decode(r)?),
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
        }
    }
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        Ok(match r8(r)? {
            c0_type::WELCOME => Self::Welcome(Welcome::decode(r)?),
            c0_type::REJECT => Self::Reject(Reject::decode(r)?),
            c0_type::PONG => Self::Pong(Pong::decode(r)?),
            c0_type::DISCONNECT => Self::Disconnect(Disconnect::decode(r)?),
            _ => return Err(CodecError::InvalidTag),
        })
    }
}

fn encode_whole<T: Codec>(m: &T) -> Result<Vec<u8>, CodecError> {
    let mut w = BitWriter::new();
    m.encode(&mut w)?;
    Ok(w.finish())
}

fn decode_whole<T: Codec>(bytes: &[u8]) -> Result<T, CodecError> {
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
