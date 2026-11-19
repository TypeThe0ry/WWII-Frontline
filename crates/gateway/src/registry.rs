//! Wire match IDs are opaque 128-bit values encoded as lower-case hex.

pub type MatchId = [u8; 16];

pub fn to_hex(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn from_hex(s: &str) -> Option<MatchId> {
    if s.len() != 32 || !s.is_ascii() {
        return None;
    }
    let mut out = [0u8; 16];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}
