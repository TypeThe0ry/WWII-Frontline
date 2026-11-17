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
}

/// C1 `msg_type` values. Not written on the wire (the channel identifies the message, to keep
/// the `InputBundle` within its byte budget); defined for transports that multiplex.
pub mod c1_type {
    pub const INPUT_BUNDLE: u8 = 0x01;
}
