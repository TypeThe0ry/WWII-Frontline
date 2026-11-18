//! Frontline wire protocol: the single definition of the on-the-wire format shared by
//! `server`, `headless-client` and `sim-wasm` (ARCHITECTURE §4.3.1, §8.2).
//!
//! # Versioning rules
//!
//! * [`PROTOCOL_VERSION`] must be strictly equal on both ends at handshake time.
//! * Fields are only ever **appended at the end** of a message; any other change
//!   (reordering, removing, changing a width or a meaning) is breaking and bumps
//!   [`PROTOCOL_VERSION`].
//! * Golden byte fixtures under `tests/golden/` guard against accidental format changes.
//!
//! # Safety properties
//!
//! `#![no_std]` (uses `alloc`), no I/O, raw-pointer-style code forbidden by the workspace lint
//! table (`[lints] workspace = true`). Decoding never panics and never
//! allocates more than the remaining input could justify (ARCHITECTURE §19.1): every length
//! prefix is bounds-checked against both a per-field maximum and the remaining bits.
#![no_std]

extern crate alloc;

pub mod bitio;
pub mod channel;
pub mod codec;
pub mod dto;
pub mod framing;
pub mod msg;
pub mod quant;
pub mod snapshot;

pub use bitio::{BitReader, BitWriter};
pub use channel::Channel;
pub use codec::{Codec, CodecError};

/// Wire protocol version (bumped on any incompatible change, see module docs).
pub const PROTOCOL_VERSION: u16 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_three() {
        assert_eq!(PROTOCOL_VERSION, 3);
    }
}
