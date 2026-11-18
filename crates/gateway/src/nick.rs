//! Nickname validation: 3-20 characters after folding full-width forms (the NFKC subset the
//! server also applies, PI-073), no control or bidirectional-override characters.

pub fn validate_nick(raw: &str) -> Result<String, &'static str> {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        let c = match c {
            '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFF01 + 0x21).unwrap_or(c),
            '\u{3000}' => ' ',
            _ => c,
        };
        if c.is_control()
            || matches!(c,
                '\u{061C}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
        {
            return Err("bad_characters");
        }
        out.push(c);
    }
    let t = out.trim();
    let n = t.chars().count();
    if !(3..=20).contains(&n) {
        return Err("bad_length");
    }
    Ok(t.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_and_folds() {
        assert_eq!(validate_nick("  Gaius  ").unwrap(), "Gaius");
        assert_eq!(validate_nick("ＡＢＣ").unwrap(), "ABC");
        assert!(validate_nick(&"a".repeat(20)).is_ok());
    }

    #[test]
    fn rejects_bad() {
        assert_eq!(validate_nick("ab"), Err("bad_length"));
        assert_eq!(validate_nick(&"a".repeat(21)), Err("bad_length"));
        assert_eq!(validate_nick("abc\u{202E}"), Err("bad_characters"));
        assert_eq!(validate_nick("a\nbc"), Err("bad_characters"));
    }
}
