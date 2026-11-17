//! WebSocket frame envelope (ARCHITECTURE §6.6.2): `[channel: u8][len: u16 LE][payload]`.
//!
//! Every WebSocket binary message carries exactly one frame. Pure functions, no I/O; decoding
//! untrusted bytes only ever returns a [`FrameError`].

use alloc::vec::Vec;
use core::fmt;

use crate::channel::Channel;

/// Header size in bytes.
pub const FRAME_HEADER_LEN: usize = 3;
/// Largest payload a frame can carry (`len` is a `u16`).
pub const MAX_FRAME_PAYLOAD: usize = u16::MAX as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// Shorter than the 3 byte header.
    Truncated,
    /// Unknown channel id.
    BadChannel,
    /// `len` does not match the number of payload bytes present.
    LengthMismatch,
    /// Payload longer than 65535 bytes (encode side).
    TooLong,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            FrameError::Truncated => "frame shorter than header",
            FrameError::BadChannel => "unknown channel",
            FrameError::LengthMismatch => "frame length field does not match payload",
            FrameError::TooLong => "payload exceeds 65535 bytes",
        })
    }
}

impl core::error::Error for FrameError {}

/// Builds a frame; fails when the payload does not fit the `u16` length.
pub fn encode_ws_frame(channel: Channel, payload: &[u8]) -> Result<Vec<u8>, FrameError> {
    if payload.len() > MAX_FRAME_PAYLOAD {
        return Err(FrameError::TooLong);
    }
    let mut v = Vec::with_capacity(FRAME_HEADER_LEN + payload.len());
    v.push(channel as u8);
    v.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    v.extend_from_slice(payload);
    Ok(v)
}

/// Parses one frame; the buffer must contain exactly the header plus `len` payload bytes.
pub fn decode_ws_frame(buf: &[u8]) -> Result<(Channel, &[u8]), FrameError> {
    if buf.len() < FRAME_HEADER_LEN {
        return Err(FrameError::Truncated);
    }
    let channel = Channel::from_u8(buf[0]).ok_or(FrameError::BadChannel)?;
    let len = u16::from_le_bytes([buf[1], buf[2]]) as usize;
    let payload = &buf[FRAME_HEADER_LEN..];
    if payload.len() != len {
        return Err(FrameError::LengthMismatch);
    }
    Ok((channel, payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn golden_bytes() {
        assert_eq!(
            encode_ws_frame(Channel::C0, &[0xAA, 0xBB]).unwrap(),
            [0x00, 0x02, 0x00, 0xAA, 0xBB]
        );
        assert_eq!(
            encode_ws_frame(Channel::C2, &[]).unwrap(),
            [0x02, 0x00, 0x00]
        );
        let big = alloc::vec![7u8; 0x0102];
        let f = encode_ws_frame(Channel::C4, &big).unwrap();
        assert_eq!(&f[..3], &[0x04, 0x02, 0x01]);
    }

    #[test]
    fn limits_and_errors() {
        let max = alloc::vec![0u8; MAX_FRAME_PAYLOAD];
        let f = encode_ws_frame(Channel::C3, &max).unwrap();
        assert_eq!(decode_ws_frame(&f).unwrap().1.len(), MAX_FRAME_PAYLOAD);
        let over = alloc::vec![0u8; MAX_FRAME_PAYLOAD + 1];
        assert_eq!(
            encode_ws_frame(Channel::C3, &over),
            Err(FrameError::TooLong)
        );
        assert_eq!(decode_ws_frame(&[]), Err(FrameError::Truncated));
        assert_eq!(decode_ws_frame(&[0, 0]), Err(FrameError::Truncated));
        assert_eq!(decode_ws_frame(&[9, 0, 0]), Err(FrameError::BadChannel));
        assert_eq!(decode_ws_frame(&[0, 1, 0]), Err(FrameError::LengthMismatch));
        assert_eq!(
            decode_ws_frame(&[0, 0, 0, 1]),
            Err(FrameError::LengthMismatch)
        );
    }

    proptest! {
        #[test]
        fn roundtrip(ch in 0u8..=5, payload in proptest::collection::vec(any::<u8>(), 0..2000)) {
            let c = Channel::from_u8(ch).unwrap();
            let f = encode_ws_frame(c, &payload).unwrap();
            let (c2, p2) = decode_ws_frame(&f).unwrap();
            prop_assert_eq!(c, c2);
            prop_assert_eq!(&payload[..], p2);
            // every strict prefix is rejected
            for n in 0..f.len() {
                prop_assert!(decode_ws_frame(&f[..n]).is_err());
            }
        }

        #[test]
        fn random_bytes_never_panic(buf in proptest::collection::vec(any::<u8>(), 0..64)) {
            let _ = decode_ws_frame(&buf);
        }
    }
}
