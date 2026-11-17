//! The [`Codec`] trait and [`CodecError`].

use core::fmt;

use crate::bitio::{BitReader, BitWriter};

/// Errors produced while encoding or decoding. Decoding untrusted bytes only ever yields
/// one of these, never a panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecError {
    /// Ran out of input bits.
    Eof,
    /// A value does not fit in the requested number of bits.
    Overflow,
    /// Unknown message type / enum discriminant.
    InvalidTag,
    /// A length prefix exceeds the allowed maximum (or the remaining input).
    TooLong,
    /// A value is outside its legal range (e.g. `move_x == -128`, bad bit count).
    InvalidValue,
    /// Non-padding bits left over after a complete message.
    Trailing,
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CodecError::Eof => "unexpected end of input",
            CodecError::Overflow => "value does not fit in bit width",
            CodecError::InvalidTag => "invalid message type or enum tag",
            CodecError::TooLong => "length exceeds limit",
            CodecError::InvalidValue => "value out of range",
            CodecError::Trailing => "trailing data after message",
        })
    }
}

impl core::error::Error for CodecError {}

/// Symmetric bit-level encoding of a value (ARCHITECTURE §8.2).
pub trait Codec {
    /// Append the encoding of `self` to `w`.
    fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError>;
    /// Read one value from `r`.
    fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError>
    where
        Self: Sized;
}
