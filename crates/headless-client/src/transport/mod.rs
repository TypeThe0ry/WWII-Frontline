//! Client transports (WebSocket and WebTransport) behind one small interface: `send` queues a
//! message on a channel, `recv` yields what the server sent. Wire formats are the ones of
//! `protocol::framing` (WS envelope) and ARCHITECTURE §6.6.1 (WT streams + datagrams).

pub mod framing;
pub mod ws;
pub mod wt;

use std::net::IpAddr;
use std::time::Duration;

use frontline_protocol::Channel;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Ws,
    Wt,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Ws => "ws",
            Kind::Wt => "wt",
        }
    }
}

#[derive(Debug)]
pub enum Incoming {
    Frame(Channel, Vec<u8>),
    /// The connection ended (reason is diagnostic text).
    Closed(String),
}

/// An established connection. Dropping it closes the connection.
pub struct Link {
    out: mpsc::UnboundedSender<(Channel, Vec<u8>)>,
    pub rx: mpsc::UnboundedReceiver<Incoming>,
    tasks: Vec<JoinHandle<()>>,
}

impl Link {
    pub(crate) fn new(
        out: mpsc::UnboundedSender<(Channel, Vec<u8>)>,
        rx: mpsc::UnboundedReceiver<Incoming>,
        tasks: Vec<JoinHandle<()>>,
    ) -> Self {
        Link { out, rx, tasks }
    }

    /// Queues `payload` on `ch`; `false` when the connection is gone.
    pub fn send(&self, ch: Channel, payload: Vec<u8>) -> bool {
        self.out.send((ch, payload)).is_ok()
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        for t in &self.tasks {
            t.abort();
        }
    }
}

/// How to reach the server.
#[derive(Clone, Debug)]
pub struct Target {
    pub kind: Kind,
    /// `host:port` (WS port for `Ws`, WT port for `Wt`).
    pub addr: String,
    /// SHA-256 of the server certificate (WebTransport).
    pub cert_hash: Option<[u8; 32]>,
    /// Local address to bind (distinct loopback addresses dodge the server's per-IP limit).
    pub bind_ip: Option<IpAddr>,
    pub connect_timeout: Duration,
}

pub async fn connect(t: &Target) -> Result<Link, String> {
    match t.kind {
        Kind::Ws => ws::connect(t).await,
        Kind::Wt => wt::connect(t).await,
    }
}

/// Hex (64 chars) or base64 (standard / url-safe, padded or not) SHA-256 fingerprint.
pub fn parse_cert_hash(s: &str) -> Result<[u8; 32], String> {
    let s = s.trim();
    let bytes: Vec<u8> = if s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
        (0..32)
            .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?
    } else {
        base64_decode(s)
            .ok_or_else(|| "cert hash is neither 64 hex chars nor base64".to_string())?
    };
    bytes
        .try_into()
        .map_err(|_| "cert hash must be 32 bytes (SHA-256)".to_string())
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.trim_end_matches('=').bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cert_hash_hex_and_base64_agree() {
        let hex = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
        let h = parse_cert_hash(hex).unwrap();
        assert_eq!(h[0], 0x00);
        assert_eq!(h[31], 0xff);
        // base64 of the same 32 bytes
        let b64 = "ABEiM0RVZneImaq7zN3u/wARIjNEVWZ3iJmqu8zd7v8=";
        assert_eq!(parse_cert_hash(b64).unwrap(), h);
        assert_eq!(
            parse_cert_hash("ABEiM0RVZneImaq7zN3u_wARIjNEVWZ3iJmqu8zd7v8").unwrap(),
            h
        );
        assert!(parse_cert_hash("zz").is_err());
        assert!(parse_cert_hash("AAAA").is_err());
    }
}
