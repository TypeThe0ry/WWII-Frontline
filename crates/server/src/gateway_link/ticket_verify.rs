//! Join-ticket verification (ARCHITECTURE §19.2). Format (shared with the gateway, which signs):
//! `base64url(claims) "." base64url(Ed25519 signature)`, signature over `TICKET_DOMAIN || claims`.
//! Several public keys are accepted (current + previous) so keys can be rotated; the ticket
//! carries no key id, so every key is tried (PI-076).

use std::collections::{HashSet, VecDeque};

use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use frontline_protocol::dto::{TicketClaims, TICKET_DOMAIN};

pub const JTI_TTL_S: u64 = 120;
pub const JTI_CAPACITY: usize = 4096;
/// A ticket string is at most 256 bytes on the wire (Hello.ticket).
pub const MAX_TICKET_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketReject {
    Malformed,
    BadSignature,
    Expired,
    WrongMatch,
    Replayed,
    /// Registration has not delivered the key yet.
    NotReady,
}

impl TicketReject {
    pub const LABELS: &'static [&'static str] = &[
        "malformed",
        "bad_signature",
        "expired",
        "wrong_match",
        "replayed",
        "not_ready",
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Malformed => "malformed",
            Self::BadSignature => "bad_signature",
            Self::Expired => "expired",
            Self::WrongMatch => "wrong_match",
            Self::Replayed => "replayed",
            Self::NotReady => "not_ready",
        }
    }
}

/// Used `jti`s, kept for [`JTI_TTL_S`] and at most [`JTI_CAPACITY`] entries (oldest evicted).
pub struct JtiCache {
    order: VecDeque<(u64, [u8; 16])>,
    seen: HashSet<[u8; 16]>,
    cap: usize,
}

impl Default for JtiCache {
    fn default() -> Self {
        Self::with_capacity(JTI_CAPACITY)
    }
}

impl JtiCache {
    pub fn with_capacity(cap: usize) -> Self {
        JtiCache {
            order: VecDeque::new(),
            seen: HashSet::new(),
            cap: cap.max(1),
        }
    }

    pub fn len(&self) -> usize {
        self.seen.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }

    /// `true` when `jti` was new (and is now remembered).
    pub fn take(&mut self, jti: [u8; 16], now_s: u64) -> bool {
        while let Some(&(t, id)) = self.order.front() {
            if now_s.saturating_sub(t) < JTI_TTL_S {
                break;
            }
            self.order.pop_front();
            self.seen.remove(&id);
        }
        if self.seen.contains(&jti) {
            return false;
        }
        while self.seen.len() >= self.cap {
            match self.order.pop_front() {
                Some((_, id)) => {
                    self.seen.remove(&id);
                }
                None => break,
            }
        }
        self.order.push_back((now_s, jti));
        self.seen.insert(jti);
        true
    }
}

pub struct TicketVerifier {
    keys: Vec<VerifyingKey>,
    jti: JtiCache,
    /// Tolerated clock offset between gateway and server, seconds (default 0, PI-076).
    skew_s: u64,
}

impl TicketVerifier {
    pub fn new(keys: Vec<VerifyingKey>) -> Self {
        TicketVerifier {
            keys,
            jti: JtiCache::default(),
            skew_s: 0,
        }
    }

    pub fn with_cache(mut self, jti: JtiCache) -> Self {
        self.jti = jti;
        self
    }

    pub fn with_skew(mut self, skew_s: u64) -> Self {
        self.skew_s = skew_s;
        self
    }

    pub fn add_key(&mut self, k: VerifyingKey) {
        if !self.keys.contains(&k) {
            self.keys.push(k);
        }
    }

    pub fn verify(
        &mut self,
        raw: &[u8],
        now_s: u64,
        match_id: [u8; 16],
    ) -> Result<TicketClaims, TicketReject> {
        if raw.is_empty() || raw.len() > MAX_TICKET_BYTES {
            return Err(TicketReject::Malformed);
        }
        let s = std::str::from_utf8(raw).map_err(|_| TicketReject::Malformed)?;
        let (c, sg) = s.split_once('.').ok_or(TicketReject::Malformed)?;
        let e = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let bytes = e.decode(c).map_err(|_| TicketReject::Malformed)?;
        let sig: [u8; 64] = e
            .decode(sg)
            .map_err(|_| TicketReject::Malformed)?
            .try_into()
            .map_err(|_| TicketReject::Malformed)?;
        let sig = Signature::from_bytes(&sig);
        let mut msg = Vec::with_capacity(TICKET_DOMAIN.len() + bytes.len());
        msg.extend_from_slice(TICKET_DOMAIN);
        msg.extend_from_slice(&bytes);
        if !self.keys.iter().any(|k| k.verify(&msg, &sig).is_ok()) {
            return Err(TicketReject::BadSignature);
        }
        let claims = TicketClaims::decode(&bytes).map_err(|_| TicketReject::Malformed)?;
        if now_s >= claims.exp.saturating_add(self.skew_s) {
            return Err(TicketReject::Expired);
        }
        if claims.match_id != match_id {
            return Err(TicketReject::WrongMatch);
        }
        if !self.jti.take(claims.jti, now_s) {
            return Err(TicketReject::Replayed);
        }
        Ok(claims)
    }
}

/// Parses a base64url (or standard, padded) 32-byte Ed25519 public key.
pub fn parse_public_key(text: &str) -> Result<VerifyingKey, String> {
    let t = text.trim();
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(t.trim_end_matches('='))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(t))
        .map_err(|_| "ticket public key is not base64".to_string())?;
    let arr: [u8; 32] = raw
        .try_into()
        .map_err(|_| "ticket public key must be 32 bytes".to_string())?;
    VerifyingKey::from_bytes(&arr).map_err(|_| "ticket public key is not a valid point".into())
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    pub fn key(n: u8) -> SigningKey {
        SigningKey::from_bytes(&[n; 32])
    }

    pub fn issue(k: &SigningKey, mut c: TicketClaims, now_s: u64) -> Vec<u8> {
        c.exp = now_s + 60;
        let bytes = c.encode();
        let mut m = TICKET_DOMAIN.to_vec();
        m.extend_from_slice(&bytes);
        let e = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        format!("{}.{}", e.encode(&bytes), e.encode(k.sign(&m).to_bytes())).into_bytes()
    }

    pub fn claims(jti: u8) -> TicketClaims {
        TicketClaims {
            player_id: 5,
            match_id: [1; 16],
            nick: "Gaius".into(),
            exp: 0,
            jti: [jti; 16],
            is_admin: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::*;
    use super::*;

    fn v() -> TicketVerifier {
        TicketVerifier::new(vec![key(7).verifying_key()])
    }

    #[test]
    fn ticket_verify_accepts_until_59s() {
        let t = issue(&key(7), claims(1), 1000);
        let c = v().verify(&t, 1059, [1; 16]).unwrap();
        assert_eq!((c.nick.as_str(), c.player_id), ("Gaius", 5));
    }

    #[test]
    fn ticket_verify_expired_at_60_and_61s() {
        let t = issue(&key(7), claims(1), 1000);
        assert_eq!(v().verify(&t, 1060, [1; 16]), Err(TicketReject::Expired));
        assert_eq!(v().verify(&t, 1061, [1; 16]), Err(TicketReject::Expired));
    }

    #[test]
    fn ticket_verify_skew_extends_expiry_only_when_configured() {
        let t = issue(&key(7), claims(1), 1000);
        assert!(v().with_skew(2).verify(&t, 1061, [1; 16]).is_ok());
    }

    #[test]
    fn ticket_verify_replay_refused() {
        let t = issue(&key(7), claims(1), 1000);
        let mut x = v();
        assert!(x.verify(&t, 1001, [1; 16]).is_ok());
        assert_eq!(x.verify(&t, 1002, [1; 16]), Err(TicketReject::Replayed));
    }

    #[test]
    fn ticket_verify_tamper_every_byte() {
        let t = issue(&key(7), claims(1), 1000);
        for i in 0..t.len() {
            let mut b = t.clone();
            b[i] = if b[i] == b'A' { b'B' } else { b'A' };
            assert!(v().verify(&b, 1001, [1; 16]).is_err(), "byte {i}");
        }
    }

    #[test]
    fn ticket_verify_wrong_match_key_and_size() {
        let t = issue(&key(7), claims(1), 1000);
        assert_eq!(v().verify(&t, 1001, [9; 16]), Err(TicketReject::WrongMatch));
        let mut other = TicketVerifier::new(vec![key(8).verifying_key()]);
        assert_eq!(
            other.verify(&t, 1001, [1; 16]),
            Err(TicketReject::BadSignature)
        );
        assert_eq!(v().verify(&[], 1, [1; 16]), Err(TicketReject::Malformed));
        assert_eq!(
            v().verify(&[b'a'; 257], 1, [1; 16]),
            Err(TicketReject::Malformed)
        );
        assert_eq!(
            v().verify(b"no-dot", 1, [1; 16]),
            Err(TicketReject::Malformed)
        );
    }

    #[test]
    fn ticket_verify_key_rotation_accepts_old_and_new() {
        let old = issue(&key(7), claims(1), 1000);
        let new = issue(&key(8), claims(2), 1000);
        let mut x = TicketVerifier::new(vec![key(8).verifying_key()]);
        assert_eq!(
            x.verify(&old, 1001, [1; 16]),
            Err(TicketReject::BadSignature)
        );
        x.add_key(key(7).verifying_key());
        assert!(x.verify(&old, 1001, [1; 16]).is_ok());
        assert!(x.verify(&new, 1001, [1; 16]).is_ok());
    }

    #[test]
    fn jti_cache_evicts_oldest_at_capacity_and_expires() {
        let mut c = JtiCache::with_capacity(3);
        for i in 0..3u8 {
            assert!(c.take([i; 16], 10 + u64::from(i)));
        }
        assert!(c.take([9; 16], 20));
        assert_eq!(c.len(), 3);
        // [0] was evicted, so it counts as new again; [2] is still known.
        assert!(!c.take([2; 16], 21));
        assert!(c.take([0; 16], 21));
        // Everything older than 120 s is gone.
        assert!(c.take([2; 16], 21 + JTI_TTL_S + 1));
        let mut big = JtiCache::default();
        for i in 0..10_000u32 {
            let mut id = [0u8; 16];
            id[..4].copy_from_slice(&i.to_le_bytes());
            assert!(big.take(id, 5));
        }
        assert_eq!(big.len(), JTI_CAPACITY);
    }

    #[test]
    fn jti_cache_expires_at_120s_exactly() {
        let mut cache = JtiCache::default();
        assert!(cache.take([1; 16], 100));
        assert!(!cache.take([1; 16], 219));
        assert!(cache.take([1; 16], 220));
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "run in isolation to measure process RSS growth per full cache"]
    fn jti_cache_memory_budget() {
        fn rss_bytes() -> usize {
            std::fs::read_to_string("/proc/self/status")
                .unwrap()
                .lines()
                .find_map(|line| line.strip_prefix("VmRSS:"))
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap()
                .parse::<usize>()
                .unwrap()
                * 1024
        }
        // Use many simultaneous caches to amortize RSS accounting and page granularity.
        const CACHES: usize = 64;
        let before = rss_bytes();
        let mut caches = Vec::with_capacity(CACHES);
        for _ in 0..CACHES {
            let mut cache = JtiCache::default();
            for i in 0..JTI_CAPACITY {
                assert!(cache.take((i as u128).to_le_bytes(), 1));
            }
            caches.push(cache);
        }
        let per_cache = rss_bytes().saturating_sub(before) / CACHES;
        assert_eq!(std::hint::black_box(&caches)[0].len(), JTI_CAPACITY);
        eprintln!(
            "full JTI cache: {per_cache} B RSS growth per cache (average across {CACHES} caches)"
        );
        assert!((1..1_000_000).contains(&per_cache));
    }

    #[test]
    fn parse_public_key_accepts_url_safe() {
        let pk = key(7).verifying_key();
        let s = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(pk.as_bytes());
        assert_eq!(parse_public_key(&s).unwrap(), pk);
        assert!(parse_public_key("zz").is_err());
    }
}
