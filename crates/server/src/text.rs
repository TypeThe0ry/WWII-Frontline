//! Chat text hygiene (ARCHITECTURE §19.1, M1-026). Untrusted text is normalised, bounded and
//! stripped of anything that can spoof or break the UI before it is routed. Rendering is
//! plain text on the client.
//!
//! NFKC: the full normalisation needs a table crate that is not on the dependency allow-list
//! (ARCHITECTURE §4.5), so M1 folds the compatibility forms players actually type — full-width
//! ASCII (U+FF01..U+FF5E) and the ideographic space (U+3000) — and leaves the rest (PI-073).

use std::collections::VecDeque;

pub const MAX_CHAT_CHARS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatReject {
    Empty,
    TooLong,
    Control,
    Bidi,
    Duplicate,
}

impl ChatReject {
    pub const LABELS: [&'static str; 5] = ["empty", "too_long", "control", "bidi", "duplicate"];

    pub fn label(self) -> &'static str {
        match self {
            ChatReject::Empty => "empty",
            ChatReject::TooLong => "too_long",
            ChatReject::Control => "control",
            ChatReject::Bidi => "bidi",
            ChatReject::Duplicate => "duplicate",
        }
    }
}

fn is_bidi(c: char) -> bool {
    matches!(c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

fn fold(c: char) -> char {
    match c {
        '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFF01 + 0x21).unwrap_or(c),
        '\u{3000}' => ' ',
        _ => c,
    }
}

/// Normalises and validates one chat line: folds compatibility forms, trims, rejects control
/// characters (including newlines), bidirectional overrides and lines over 200 characters.
pub fn sanitize_chat(raw: &str) -> Result<String, ChatReject> {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        if is_bidi(c) {
            return Err(ChatReject::Bidi);
        }
        let c = fold(c);
        if c.is_control() || matches!(c, '\u{200B}'..='\u{200D}' | '\u{FEFF}') {
            return Err(ChatReject::Control);
        }
        out.push(c);
    }
    let t = out.trim();
    if t.is_empty() {
        return Err(ChatReject::Empty);
    }
    if t.chars().count() > MAX_CHAT_CHARS {
        return Err(ChatReject::TooLong);
    }
    Ok(t.to_owned())
}

/// Rejects a player repeating the same line within the last `keep` lines.
#[derive(Debug, Default)]
pub struct DupFilter {
    recent: VecDeque<String>,
}

impl DupFilter {
    pub const KEEP: usize = 3;

    pub fn check(&mut self, line: &str) -> Result<(), ChatReject> {
        let key = line.to_lowercase();
        if self.recent.iter().any(|r| *r == key) {
            return Err(ChatReject::Duplicate);
        }
        if self.recent.len() == Self::KEEP {
            self.recent.pop_front();
        }
        self.recent.push_back(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_and_trims() {
        assert_eq!(sanitize_chat("  hello team  ").unwrap(), "hello team");
        assert_eq!(sanitize_chat("前进！").unwrap(), "前进!");
    }

    #[test]
    fn folds_fullwidth_to_ascii() {
        assert_eq!(sanitize_chat("ＡＢＣ１２３").unwrap(), "ABC123");
        assert_eq!(sanitize_chat("ａ\u{3000}ｂ").unwrap(), "a b");
    }

    #[test]
    fn rejects_length_over_200_chars() {
        assert_eq!(sanitize_chat(&"a".repeat(200)).unwrap().len(), 200);
        assert_eq!(sanitize_chat(&"a".repeat(201)), Err(ChatReject::TooLong));
        // Characters, not bytes.
        assert!(sanitize_chat(&"我".repeat(200)).is_ok());
    }

    #[test]
    fn rejects_bidi_overrides() {
        for c in ['\u{202E}', '\u{202A}', '\u{2066}', '\u{200F}', '\u{061C}'] {
            assert_eq!(sanitize_chat(&format!("a{c}b")), Err(ChatReject::Bidi));
        }
    }

    #[test]
    fn rejects_control_and_zero_width() {
        for s in [
            "a\nb",
            "a\u{0}b",
            "a\tb",
            "a\u{7F}b",
            "a\u{200B}b",
            "a\u{FEFF}b",
        ] {
            assert_eq!(sanitize_chat(s), Err(ChatReject::Control), "{s:?}");
        }
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(sanitize_chat("   "), Err(ChatReject::Empty));
        assert_eq!(sanitize_chat(""), Err(ChatReject::Empty));
    }

    #[test]
    fn duplicate_filter_remembers_three_lines() {
        let mut d = DupFilter::default();
        assert!(d.check("gg").is_ok());
        assert_eq!(d.check("GG"), Err(ChatReject::Duplicate));
        assert!(d.check("a").is_ok() && d.check("b").is_ok() && d.check("c").is_ok());
        assert!(d.check("gg").is_ok()); // fell out of the window
    }
}
