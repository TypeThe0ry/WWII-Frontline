//! Guest session token: `base64url(header).base64url(payload).base64url(HMAC-SHA256)`.
//! Payload `{sub, iat, exp}` only, no PII; TTL 24 h.

use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use super::{b64, ct_eq};

pub const TOKEN_TTL_S: u64 = 24 * 3600;
const HEADER: &str = r#"{"alg":"HS256","typ":"JWT"}"#;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claims {
    /// Guest id, 32 lower-case hex characters (128 bit).
    pub sub: String,
    pub iat: u64,
    pub exp: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TokenError {
    Malformed,
    BadSignature,
    Expired,
}

fn sign(secret: &[u8], signing_input: &str) -> Vec<u8> {
    let mut m = Hmac::<Sha256>::new_from_slice(secret).expect("hmac accepts any key length");
    m.update(signing_input.as_bytes());
    m.finalize().into_bytes().to_vec()
}

pub fn issue(secret: &[u8], guest_id: &str, now_s: u64) -> String {
    let claims = Claims {
        sub: guest_id.to_owned(),
        iat: now_s,
        exp: now_s + TOKEN_TTL_S,
    };
    let e = b64();
    let input = format!(
        "{}.{}",
        e.encode(HEADER),
        e.encode(serde_json::to_vec(&claims).expect("claims serialise"))
    );
    let sig = sign(secret, &input);
    format!("{input}.{}", e.encode(sig))
}

/// Prevents exposing the guest account ID to a game server or linking it across matches.
pub(crate) fn match_guest(secret: &[u8], match_id: &[u8; 16], guest_id: &str) -> [u8; 8] {
    let mut m = Hmac::<Sha256>::new_from_slice(secret).expect("hmac accepts any key length");
    m.update(b"frontline-match-guest-v2\0");
    m.update(match_id);
    m.update(guest_id.as_bytes());
    let mut id = [0; 8];
    id.copy_from_slice(&m.finalize().into_bytes()[..8]);
    id
}

pub fn verify(secret: &[u8], token: &str, now_s: u64) -> Result<Claims, TokenError> {
    let mut parts = token.split('.');
    let (Some(h), Some(p), Some(s), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(TokenError::Malformed);
    };
    let e = b64();
    if e.decode(h).map_err(|_| TokenError::Malformed)? != HEADER.as_bytes() {
        return Err(TokenError::Malformed);
    }
    let given = e.decode(s).map_err(|_| TokenError::Malformed)?;
    let want = sign(secret, &format!("{h}.{p}"));
    if !ct_eq(&given, &want) {
        return Err(TokenError::BadSignature);
    }
    let claims: Claims = serde_json::from_slice(&e.decode(p).map_err(|_| TokenError::Malformed)?)
        .map_err(|_| TokenError::Malformed)?;
    if now_s >= claims.exp {
        return Err(TokenError::Expired);
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    const K: &[u8] = b"0123456789abcdef0123456789abcdef";

    #[test]
    fn ttl_is_24_hours() {
        let t = issue(K, "aa", 1_000);
        let c = verify(K, &t, 1_000).unwrap();
        assert_eq!(c.exp - c.iat, 86_400);
        assert!(verify(K, &t, 1_000 + 86_399).is_ok());
        assert_eq!(verify(K, &t, 1_000 + 86_400), Err(TokenError::Expired));
    }

    #[test]
    fn tamper_and_wrong_key_fail() {
        let t = issue(K, "aa", 1_000);
        assert_eq!(
            verify(b"other-secret-other-secret-other!", &t, 1_001),
            Err(TokenError::BadSignature)
        );
        let mut bad = t.clone();
        bad.replace_range(t.len() - 3..t.len() - 2, "A");
        assert!(verify(K, &bad, 1_001).is_err());
        assert_eq!(verify(K, "a.b", 1), Err(TokenError::Malformed));
        assert_eq!(verify(K, "", 1), Err(TokenError::Malformed));
    }
}
