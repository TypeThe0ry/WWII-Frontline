//! Logical channels (ARCHITECTURE §6.6.1) and stable `msg_type` numbers.
//!
//! `msg_type` numbers are part of the wire format: never renumber, only append.

/// Logical channels. The numeric value is the stable channel id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Channel {
    /// Control, reliable.
    C0 = 0,
    /// Client input, unreliable.
    C1 = 1,
    /// Snapshots, unreliable.
    C2 = 2,
    /// Team/world state, unreliable.
    C3 = 3,
    /// Reliable server events.
    C4 = 4,
    /// Reserved.
    C5 = 5,
}

impl Channel {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Channel::C0,
            1 => Channel::C1,
            2 => Channel::C2,
            3 => Channel::C3,
            4 => Channel::C4,
            5 => Channel::C5,
            _ => return None,
        })
    }
}

/// C0 `msg_type` values. One numbering shared by both directions (unique across them).
pub mod c0_type {
    pub const HELLO: u8 = 0x01;
    pub const WELCOME: u8 = 0x02;
    pub const REJECT: u8 = 0x03;
    pub const PING: u8 = 0x04;
    pub const PONG: u8 = 0x05;
    pub const DISCONNECT: u8 = 0x06;
    // M1 additions (PROTOCOL_VERSION 2). Chat is bidirectional and shares one number.
    pub const DEPLOY_REQUEST: u8 = 0x10;
    pub const DEPLOY_RESULT: u8 = 0x11;
    pub const SQUAD_OP: u8 = 0x12;
    pub const CHAT: u8 = 0x13;
    pub const MARKER_PLACE: u8 = 0x14;
    pub const SETTINGS: u8 = 0x15;
    pub const REPORT: u8 = 0x16;
    pub const PLACEMENT_REQUEST: u8 = 0x17;
    pub const PLACEMENT_RESULT: u8 = 0x18;
    pub const VOTE_START: u8 = 0x19;
    pub const VOTE_CAST: u8 = 0x1a;
    pub const TEAM_SWITCH_REQUEST: u8 = 0x1b;
    pub const FORGIVE_PUNISH: u8 = 0x1c;
    pub const MAP_VOTE_CAST: u8 = 0x1d;
}

/// C4 (reliable server events) `msg_type` values.
pub mod c4_type {
    pub const KILL: u8 = 0x01;
    pub const DEATH: u8 = 0x02;
    pub const REVIVE: u8 = 0x03;
    pub const SCORE_DELTA: u8 = 0x04;
    pub const OBJECTIVE_CHANGE: u8 = 0x05;
    pub const SPAWN_POINT_STATE: u8 = 0x06;
    pub const PLAYER_ROSTER: u8 = 0x07;
    pub const NOTICE: u8 = 0x08;
    pub const MATCH_PHASE: u8 = 0x09;
    pub const VOTE_STATE: u8 = 0x0a;
    pub const VOTE_RESULT: u8 = 0x0b;
    pub const TK_PROMPT: u8 = 0x0c;
    pub const TEAM_BALANCE_NOTICE: u8 = 0x0d;
    pub const MAP_VOTE_STATE: u8 = 0x0e;
    pub const MAP_VOTE_RESULT: u8 = 0x0f;
}

/// C1 `msg_type` values. Not written on the wire (the channel identifies the message, to keep
/// the `InputBundle` within its byte budget); defined for transports that multiplex.
pub mod c1_type {
    pub const INPUT_BUNDLE: u8 = 0x01;
}
