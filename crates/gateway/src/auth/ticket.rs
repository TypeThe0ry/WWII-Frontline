//! Join tickets: `base64url(claims) "." base64url(Ed25519 signature)`; the signature covers
//! `TICKET_DOMAIN || claims`. TTL 60 s, single use (the verifier keeps a [`JtiCache`]).

use std::collections::HashMap;

use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use frontline_protocol::dto::{TicketClaims, TICKET_DOMAIN, TICKET_TTL_S};

use super::b64;

#[derive(Debug, PartialEq, Eq)]
pub enum TicketError {
    Malformed,
    BadSignature,
    Expired,
    Replayed,
    WrongMatch,
}

fn message(claims: &[u8]) -> Vec<u8> {
    let mut m = Vec::with_capacity(TICKET_DOMAIN.len() + claims.len());
    m.extend_from_slice(TICKET_DOMAIN);
    m.extend_from_slice(claims);
    m
}

/// Issues a ticket valid for [`TICKET_TTL_S`] seconds from `now_s`.
pub fn issue(key: &SigningKey, mut claims: TicketClaims, now_s: u64) -> String {
    claims.exp = now_s + TICKET_TTL_S;
    let bytes = claims.encode();
    let sig = key.sign(&message(&bytes));
    let e = b64();
    format!("{}.{}", e.encode(&bytes), e.encode(sig.to_bytes()))
}

/// Remembers used `jti`s until their ticket expires.
#[derive(Default)]
pub struct JtiCache {
    used: HashMap<[u8; 16], u64>,
}

impl JtiCache {
    fn take(&mut self, jti: [u8; 16], exp: u64, now_s: u64) -> bool {
        self.used.retain(|_, e| *e > now_s);
        self.used.insert(jti, exp).is_none()
    }
}

/// What the match server does with a ticket: signature, expiry, match, single use.
pub fn verify(
    key: &VerifyingKey,
    ticket: &str,
    now_s: u64,
    match_id: &[u8; 16],
    cache: &mut JtiCache,
) -> Result<TicketClaims, TicketError> {
    let (c, s) = ticket.split_once('.').ok_or(TicketError::Malformed)?;
    let e = b64();
    let bytes = e.decode(c).map_err(|_| TicketError::Malformed)?;
    let sig: [u8; 64] = e
        .decode(s)
        .map_err(|_| TicketError::Malformed)?
        .try_into()
        .map_err(|_| TicketError::Malformed)?;
    key.verify(&message(&bytes), &Signature::from_bytes(&sig))
        .map_err(|_| TicketError::BadSignature)?;
    let claims = TicketClaims::decode(&bytes).map_err(|_| TicketError::Malformed)?;
    if now_s >= claims.exp {
        return Err(TicketError::Expired);
    }
    if &claims.match_id != match_id {
        return Err(TicketError::WrongMatch);
    }
    if !cache.take(claims.jti, claims.exp, now_s) {
        return Err(TicketError::Replayed);
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn claims() -> TicketClaims {
        TicketClaims {
            player_id: 5,
            match_id: [1; 16],
            nick: "Gaius".into(),
            exp: 0,
            jti: [2; 16],
            is_admin: false,
        }
    }

    #[test]
    fn ticket_ttl_is_60_seconds() {
        let t = issue(&key(), claims(), 1_000);
        let mut c = JtiCache::default();
        let ok = verify(&key().verifying_key(), &t, 1_059, &[1; 16], &mut c).unwrap();
        assert_eq!(ok.exp, 1_060);
        assert_eq!(ok.nick, "Gaius");
    }

    #[test]
    fn ticket_expired_by_one_second_fails() {
        let t = issue(&key(), claims(), 1_000);
        let mut c = JtiCache::default();
        assert_eq!(
            verify(&key().verifying_key(), &t, 1_061, &[1; 16], &mut c),
            Err(TicketError::Expired)
        );
    }

    #[test]
    fn ticket_replay_is_refused() {
        let t = issue(&key(), claims(), 1_000);
        let mut c = JtiCache::default();
        let pk = key().verifying_key();
        assert!(verify(&pk, &t, 1_001, &[1; 16], &mut c).is_ok());
        assert_eq!(
            verify(&pk, &t, 1_002, &[1; 16], &mut c),
            Err(TicketError::Replayed)
        );
    }

    #[test]
    fn ticket_tamper_one_byte_fails() {
        let t = issue(&key(), claims(), 1_000);
        let pk = key().verifying_key();
        for i in 0..t.len() {
            let mut b = t.clone().into_bytes();
            b[i] = if b[i] == b'A' { b'B' } else { b'A' };
            let s = String::from_utf8(b).unwrap();
            let mut c = JtiCache::default();
            assert!(
                verify(&pk, &s, 1_001, &[1; 16], &mut c).is_err(),
                "byte {i}"
            );
        }
    }

    #[test]
    fn ticket_for_another_match_or_key_fails() {
        let t = issue(&key(), claims(), 1_000);
        let mut c = JtiCache::default();
        assert_eq!(
            verify(&key().verifying_key(), &t, 1_001, &[9; 16], &mut c),
            Err(TicketError::WrongMatch)
        );
        let other = SigningKey::from_bytes(&[8u8; 32]).verifying_key();
        assert_eq!(
            verify(&other, &t, 1_001, &[1; 16], &mut c),
            Err(TicketError::BadSignature)
        );
    }
}
