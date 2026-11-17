//! WebTransport wire framing (ARCHITECTURE §6.6.1), pure functions without I/O.
//!
//! * Datagram: `[channel u8][payload]`; the length is the datagram boundary. C1 travels
//!   client -> server, C2/C3 server -> client.
//! * Stream (C0 bidi stream, C4 uni stream, C5 bulk): `[len u16 LE][payload]`; the channel is
//!   implied by the stream, so the header is two bytes (not the three bytes of the WebSocket
//!   envelope in `frontline_protocol::framing`).

use bytes::{Buf, Bytes, BytesMut};
use frontline_protocol::Channel;

/// Preferred datagram payload budget (T7: <= 1200 B message MTU target, minus 1 B channel and
/// QUIC/H3 overhead headroom). The real limit is `max_datagram_size` of the live connection.
pub const PACKET_BUDGET_TARGET: usize = 1150;
/// Stream frame header size.
pub const STREAM_HEADER_LEN: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatagramError {
    Empty,
    BadChannel,
}

/// Builds `[channel][payload]`.
pub fn encode_datagram(ch: Channel, payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(1 + payload.len());
    v.push(ch as u8);
    v.extend_from_slice(payload);
    v
}

/// Splits a received datagram into channel and payload (zero copy).
pub fn decode_datagram(dg: &Bytes) -> Result<(Channel, Bytes), DatagramError> {
    let first = *dg.first().ok_or(DatagramError::Empty)?;
    let ch = Channel::from_u8(first).ok_or(DatagramError::BadChannel)?;
    Ok((ch, dg.slice(1..)))
}

/// Whether `[channel][payload]` fits a datagram of at most `max_datagram_size` bytes
/// (`None` = datagrams unavailable).
pub fn datagram_fits(max_datagram_size: Option<usize>, payload_len: usize) -> bool {
    matches!(max_datagram_size, Some(m) if payload_len < m)
}

/// `PACKET_BUDGET = min(1150, max_datagram_size - 1)` (M0-019); 0 when datagrams are unusable.
pub fn packet_budget(max_datagram_size: Option<usize>) -> usize {
    match max_datagram_size {
        Some(m) => PACKET_BUDGET_TARGET.min(m.saturating_sub(1)),
        None => 0,
    }
}

/// Builds `[len u16 LE][payload]`; `None` when the payload does not fit the length field.
pub fn encode_stream_frame(payload: &[u8]) -> Option<Vec<u8>> {
    let len = u16::try_from(payload.len()).ok()?;
    let mut v = Vec::with_capacity(STREAM_HEADER_LEN + payload.len());
    v.extend_from_slice(&len.to_le_bytes());
    v.extend_from_slice(payload);
    Some(v)
}

/// Reassembles `[len u16][payload]` frames from arbitrary stream chunks (half / merged packets).
#[derive(Debug)]
pub struct StreamFramer {
    buf: BytesMut,
    max_payload: usize,
}

/// A frame header announced a payload above the configured maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameTooLong(pub usize);

impl StreamFramer {
    pub fn new(max_payload: usize) -> Self {
        StreamFramer {
            buf: BytesMut::new(),
            max_payload,
        }
    }

    pub fn push(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Bytes buffered towards an incomplete frame.
    pub fn buffered(&self) -> usize {
        self.buf.len()
    }

    /// Next complete frame payload, `Ok(None)` when more bytes are needed. A header announcing
    /// more than `max_payload` fails immediately (without waiting for the body), so the buffer
    /// never grows beyond `max_payload + header + one chunk`.
    pub fn next_frame(&mut self) -> Result<Option<Bytes>, FrameTooLong> {
        if self.buf.len() < STREAM_HEADER_LEN {
            return Ok(None);
        }
        let len = u16::from_le_bytes([self.buf[0], self.buf[1]]) as usize;
        if len > self.max_payload {
            return Err(FrameTooLong(len));
        }
        if self.buf.len() < STREAM_HEADER_LEN + len {
            return Ok(None);
        }
        self.buf.advance(STREAM_HEADER_LEN);
        Ok(Some(self.buf.split_to(len).freeze()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datagram_roundtrip_and_errors() {
        let d = Bytes::from(encode_datagram(Channel::C2, &[1, 2, 3]));
        assert_eq!(&d[..], &[2, 1, 2, 3]);
        let (ch, p) = decode_datagram(&d).unwrap();
        assert_eq!(ch, Channel::C2);
        assert_eq!(&p[..], &[1, 2, 3]);
        // Empty payload is legal; empty datagram and unknown channel are not.
        let (ch, p) = decode_datagram(&Bytes::from_static(&[1])).unwrap();
        assert_eq!((ch, p.len()), (Channel::C1, 0));
        assert_eq!(
            decode_datagram(&Bytes::new()).unwrap_err(),
            DatagramError::Empty
        );
        assert_eq!(
            decode_datagram(&Bytes::from_static(&[9, 1])).unwrap_err(),
            DatagramError::BadChannel
        );
    }

    #[test]
    fn max_datagram_size_adaptation() {
        // 1 byte of the datagram is the channel id.
        assert!(datagram_fits(Some(1200), 1199));
        assert!(!datagram_fits(Some(1200), 1200));
        assert!(!datagram_fits(None, 0));
        assert_eq!(packet_budget(Some(1200)), 1150);
        assert_eq!(packet_budget(Some(1100)), 1099);
        assert_eq!(packet_budget(Some(1)), 0);
        assert_eq!(packet_budget(Some(0)), 0);
        assert_eq!(packet_budget(None), 0);
    }

    #[test]
    fn stream_frame_golden_and_limits() {
        assert_eq!(encode_stream_frame(&[7, 8]).unwrap(), [2, 0, 7, 8]);
        assert_eq!(encode_stream_frame(&[]).unwrap(), [0, 0]);
        let big = vec![0u8; 0x0102];
        assert_eq!(&encode_stream_frame(&big).unwrap()[..2], &[0x02, 0x01]);
        assert!(encode_stream_frame(&vec![0u8; 65_536]).is_none());
        assert!(encode_stream_frame(&vec![0u8; 65_535]).is_some());
    }

    #[test]
    fn framer_handles_split_and_merged_chunks() {
        let mut stream = vec![];
        let msgs: Vec<Vec<u8>> = (0..50u8).map(|i| vec![i; (i as usize * 7) % 40]).collect();
        for m in &msgs {
            stream.extend(encode_stream_frame(m).unwrap());
        }
        // Every chunk size from 1 byte (worst split) to the whole stream (all merged).
        for chunk in [1usize, 2, 3, 5, 17, 64, stream.len()] {
            let mut f = StreamFramer::new(4096);
            let mut out = vec![];
            for c in stream.chunks(chunk) {
                f.push(c);
                while let Some(p) = f.next_frame().unwrap() {
                    out.push(p.to_vec());
                }
            }
            assert_eq!(out, msgs, "chunk size {chunk}");
            assert_eq!(f.buffered(), 0);
        }
    }

    #[test]
    fn framer_rejects_oversize_header_without_waiting_for_the_body() {
        let mut f = StreamFramer::new(4096);
        f.push(&[0x01, 0x10]); // len = 4097
        assert_eq!(f.next_frame(), Err(FrameTooLong(4097)));
        let mut f = StreamFramer::new(4096);
        f.push(&[0x00, 0x10]); // len = 4096: fine, needs the body
        assert_eq!(f.next_frame(), Ok(None));
        f.push(&vec![0u8; 4096]);
        assert_eq!(f.next_frame().unwrap().unwrap().len(), 4096);
        // A single header byte is not enough to decide.
        let mut f = StreamFramer::new(10);
        f.push(&[0xFF]);
        assert_eq!(f.next_frame(), Ok(None));
    }
}
