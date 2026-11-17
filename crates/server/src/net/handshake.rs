//! Handshake decision logic (ARCHITECTURE §6.8.1), independent of the transport.

use frontline_protocol::msg::c0::{Hello, RejectReason};
use frontline_protocol::PROTOCOL_VERSION;

/// Checks a `Hello` against this server. M0 does not validate `ticket` (`--dev-no-ticket`
/// is the default; M1-029 implements it), only its length which the decoder already bounds.
pub fn check_hello(hello: &Hello, data_hash: u64) -> Result<(), RejectReason> {
    if hello.protocol_version != PROTOCOL_VERSION {
        return Err(RejectReason::VersionMismatch);
    }
    if hello.data_hash != data_hash {
        return Err(RejectReason::DataHashMismatch);
    }
    Ok(())
}

/// Label used for `net_rejects_total{reason}`.
pub fn reject_label(r: RejectReason) -> &'static str {
    match r {
        RejectReason::VersionMismatch => "version",
        RejectReason::DataHashMismatch => "data_hash",
        RejectReason::ServerFull => "server_full",
        RejectReason::Banned => "banned",
        RejectReason::InvalidTicket => "invalid_ticket",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(v: u16, h: u64) -> Hello {
        Hello {
            protocol_version: v,
            data_hash: h,
            ticket: vec![],
            client_build: 0,
            caps: 0,
        }
    }

    #[test]
    fn accepts_matching_and_rejects_mismatches() {
        assert_eq!(check_hello(&hello(PROTOCOL_VERSION, 9), 9), Ok(()));
        assert_eq!(
            check_hello(&hello(PROTOCOL_VERSION + 1, 9), 9),
            Err(RejectReason::VersionMismatch)
        );
        assert_eq!(
            check_hello(&hello(PROTOCOL_VERSION, 8), 9),
            Err(RejectReason::DataHashMismatch)
        );
    }
}
