//! Gateway <-> match server DTOs shared by both sides (M1-028, ARCHITECTURE §4.5, §19.2).
//! Structures and canonical byte encodings only: no cryptography lives in `protocol`.

use alloc::string::String;
use alloc::vec::Vec;

/// Longest nickname in bytes (20 characters of up to 4 bytes would not fit a `u8` length, the
/// validators bound it far lower; the codec bound is the hard limit).
pub const MAX_NICK_BYTES: usize = 80;
/// Join ticket lifetime in seconds.
pub const TICKET_TTL_S: u64 = 60;
/// Canonical encoding version byte.
pub const TICKET_FORMAT: u8 = 1;
/// Domain separator prepended to the claim bytes before signing.
pub const TICKET_DOMAIN: &[u8] = b"FLTK1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketClaims {
    pub player_id: u8,
    pub match_id: [u8; 16],
    pub nick: String,
    /// Expiry, seconds since the Unix epoch.
    pub exp: u64,
    pub jti: [u8; 16],
    pub is_admin: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DtoError {
    Short,
    BadVersion,
    BadNick,
    Trailing,
}

impl TicketClaims {
    /// `[ver:1][player:1][match:16][exp:8 BE][jti:16][admin:1][nick_len:1][nick]`.
    pub fn encode(&self) -> Vec<u8> {
        let nick = self.nick.as_bytes();
        let n = nick.len().min(MAX_NICK_BYTES);
        let mut v = Vec::with_capacity(44 + n);
        v.push(TICKET_FORMAT);
        v.push(self.player_id);
        v.extend_from_slice(&self.match_id);
        v.extend_from_slice(&self.exp.to_be_bytes());
        v.extend_from_slice(&self.jti);
        v.push(u8::from(self.is_admin));
        v.push(n as u8);
        v.extend_from_slice(&nick[..n]);
        v
    }

    pub fn decode(b: &[u8]) -> Result<Self, DtoError> {
        if b.len() < 44 {
            return Err(DtoError::Short);
        }
        if b[0] != TICKET_FORMAT {
            return Err(DtoError::BadVersion);
        }
        let n = usize::from(b[43]);
        if n > MAX_NICK_BYTES {
            return Err(DtoError::BadNick);
        }
        if b.len() < 44 + n {
            return Err(DtoError::Short);
        }
        if b.len() > 44 + n {
            return Err(DtoError::Trailing);
        }
        let nick = core::str::from_utf8(&b[44..44 + n]).map_err(|_| DtoError::BadNick)?;
        let mut match_id = [0u8; 16];
        match_id.copy_from_slice(&b[2..18]);
        let mut exp = [0u8; 8];
        exp.copy_from_slice(&b[18..26]);
        let mut jti = [0u8; 16];
        jti.copy_from_slice(&b[26..42]);
        Ok(TicketClaims {
            player_id: b[1],
            match_id,
            nick: String::from(nick),
            exp: u64::from_be_bytes(exp),
            jti,
            is_admin: b[42] != 0,
        })
    }
}

/// One row of the server list (GAMEDESIGN §2.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerInfo {
    pub match_id: [u8; 16],
    pub name: String,
    pub map: String,
    pub mode: String,
    pub phase: String,
    pub time_left_s: u32,
    pub humans: u8,
    pub bots: u8,
    pub cap: u8,
    pub region: String,
    pub tags: Vec<String>,
    pub version: String,
    pub compatible: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims() -> TicketClaims {
        TicketClaims {
            player_id: 7,
            match_id: [3; 16],
            nick: String::from("Gaius 前进"),
            exp: 1_700_000_060,
            jti: [9; 16],
            is_admin: true,
        }
    }

    #[test]
    fn round_trip_and_golden_prefix() {
        let c = claims();
        let b = c.encode();
        assert_eq!(&b[..2], &[1, 7]);
        assert_eq!(TicketClaims::decode(&b).unwrap(), c);
    }

    #[test]
    fn rejects_malformed() {
        let b = claims().encode();
        assert_eq!(TicketClaims::decode(&b[..10]), Err(DtoError::Short));
        assert_eq!(
            TicketClaims::decode(&b[..b.len() - 1]),
            Err(DtoError::Short)
        );
        let mut t = b.clone();
        t.push(0);
        assert_eq!(TicketClaims::decode(&t), Err(DtoError::Trailing));
        let mut v = b.clone();
        v[0] = 2;
        assert_eq!(TicketClaims::decode(&v), Err(DtoError::BadVersion));
        let mut u = b;
        *u.last_mut().unwrap() = 0xFF;
        assert_eq!(TicketClaims::decode(&u), Err(DtoError::BadNick));
    }
}
