//! WebTransport wire framing, client side (a copy of the few lines the server keeps in
//! `net::datagram`; the headless client must not depend on `server`).
//!
//! * datagram: `[channel u8][payload]` (C1 up, C2/C3 down)
//! * stream frame (C0 bidi, C4 uni): `[len u16 LE][payload]`

use frontline_protocol::Channel;

pub fn encode_datagram(ch: Channel, payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(1 + payload.len());
    v.push(ch as u8);
    v.extend_from_slice(payload);
    v
}

pub fn decode_datagram(dg: &[u8]) -> Option<(Channel, &[u8])> {
    let ch = Channel::from_u8(*dg.first()?)?;
    Some((ch, &dg[1..]))
}

pub fn encode_stream_frame(payload: &[u8]) -> Option<Vec<u8>> {
    let len = u16::try_from(payload.len()).ok()?;
    let mut v = Vec::with_capacity(2 + payload.len());
    v.extend_from_slice(&len.to_le_bytes());
    v.extend_from_slice(payload);
    Some(v)
}

/// Reassembles `[len u16][payload]` frames from arbitrary stream chunks.
#[derive(Default)]
pub struct StreamFramer {
    buf: Vec<u8>,
}

impl StreamFramer {
    pub fn push(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    pub fn next_frame(&mut self) -> Option<Vec<u8>> {
        if self.buf.len() < 2 {
            return None;
        }
        let len = u16::from_le_bytes([self.buf[0], self.buf[1]]) as usize;
        if self.buf.len() < 2 + len {
            return None;
        }
        let frame = self.buf[2..2 + len].to_vec();
        self.buf.drain(..2 + len);
        Some(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datagram_roundtrip() {
        let d = encode_datagram(Channel::C1, &[9, 8]);
        assert_eq!(d, vec![1, 9, 8]);
        let (ch, p) = decode_datagram(&d).unwrap();
        assert_eq!(ch, Channel::C1);
        assert_eq!(p, &[9, 8]);
        assert!(decode_datagram(&[]).is_none());
        assert!(decode_datagram(&[200]).is_none());
    }

    #[test]
    fn framer_handles_split_and_merged_chunks() {
        let a = encode_stream_frame(&[1, 2, 3]).unwrap();
        let b = encode_stream_frame(&[]).unwrap();
        let mut all = a.clone();
        all.extend_from_slice(&b);
        let mut f = StreamFramer::default();
        f.push(&all[..2]);
        assert!(f.next_frame().is_none());
        f.push(&all[2..4]);
        assert!(f.next_frame().is_none());
        f.push(&all[4..]);
        assert_eq!(f.next_frame().unwrap(), vec![1, 2, 3]);
        assert_eq!(f.next_frame().unwrap(), Vec::<u8>::new());
        assert!(f.next_frame().is_none());
    }
}
