//! Match server <-> gateway link (M1-029, ARCHITECTURE §3.3/§19.2): registration, 1 s heartbeat,
//! join-ticket verification and the end-of-match result. The `gateway` crate is *not* a
//! dependency; the shared wire types live in `protocol::dto`, the JSON bodies below.

pub mod admin;
pub mod client;
pub mod heartbeat;
pub mod reports;
pub mod result;
pub mod ticket_verify;

use std::sync::{Arc, RwLock};

use ticket_verify::{TicketReject, TicketVerifier};

/// Wall clock in whole seconds; replaceable so tests can place themselves anywhere.
pub type NowSecs = Arc<dyn Fn() -> u64 + Send + Sync>;

pub fn system_now_secs() -> NowSecs {
    Arc::new(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    })
}

/// What the handshake asks: "is this ticket good?". Empty until registration delivered the
/// key (connections that arrive earlier are refused with `not_ready`).
pub struct TicketGate {
    verifier: RwLock<Option<(TicketVerifier, [u8; 16])>>,
    now: NowSecs,
}

/// A verified ticket's identity, as far as the session layer uses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    pub gateway_player: u8,
    pub jti: [u8; 16],
    pub nick: String,
    pub is_admin: bool,
}

impl TicketGate {
    pub fn new(now: NowSecs) -> Arc<TicketGate> {
        Arc::new(TicketGate {
            verifier: RwLock::new(None),
            now,
        })
    }

    /// Installs the verifier once the match id and key are known.
    pub fn arm(&self, verifier: TicketVerifier, match_id: [u8; 16]) {
        *self.verifier.write().unwrap_or_else(|e| e.into_inner()) = Some((verifier, match_id));
    }

    pub fn is_armed(&self) -> bool {
        self.verifier
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    pub fn check(&self, raw: &[u8]) -> Result<Admitted, TicketReject> {
        // A write lock: verification mutates the replay cache.
        let mut g = self.verifier.write().unwrap_or_else(|e| e.into_inner());
        let Some((v, id)) = g.as_mut() else {
            return Err(TicketReject::NotReady);
        };
        let id = *id;
        let c = v.verify(raw, (self.now)(), id)?;
        Ok(Admitted {
            gateway_player: c.player_id,
            jti: c.jti,
            nick: c.nick,
            is_admin: c.is_admin,
        })
    }
}

/// Shared handle type used by `NetCtx`.
pub type SharedGate = Arc<TicketGate>;

pub fn lower_hex(id: &[u8; 16]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(32);
    for b in id {
        write!(hex, "{b:02x}").expect("writing to String cannot fail");
    }
    hex
}

pub fn parse_lower_hex(s: &str) -> Option<[u8; 16]> {
    if s.len() != 32
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    let mut id = [0; 16];
    for (byte, pair) in id.iter_mut().zip(s.as_bytes().chunks_exact(2)) {
        let digit = |b: u8| {
            if b.is_ascii_digit() {
                b - b'0'
            } else {
                b - b'a' + 10
            }
        };
        *byte = digit(pair[0]) * 16 + digit(pair[1]);
    }
    Some(id)
}
