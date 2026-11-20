//! Snapshot congestion observations. A missing capacity is not a zero-capacity link.

use wtransport::Connection;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinkSample {
    pub capacity_bytes_per_sec: Option<u64>,
    pub rtt_micros: u64,
}

pub(crate) fn webtransport_sample(conn: &Connection) -> LinkSample {
    let path = conn.quic_connection().stats().path;
    let rtt_micros = path.rtt.as_micros().min(u64::MAX as u128) as u64;
    LinkSample {
        capacity_bytes_per_sec: (path.cwnd > 0)
            .then(|| path.cwnd.saturating_mul(1_000_000).checked_div(rtt_micros))
            .flatten(),
        rtt_micros,
    }
}
