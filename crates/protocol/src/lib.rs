//! Frontline wire protocol (skeleton, filled in by M0-005).
#![no_std]

extern crate alloc;

/// Wire protocol version (bumped on any incompatible change).
pub const PROTOCOL_VERSION: u16 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(PROTOCOL_VERSION, 1);
    }
}
