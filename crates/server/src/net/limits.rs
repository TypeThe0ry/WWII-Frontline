//! Network limits (ARCHITECTURE §19.1). Every inbound byte is untrusted.

use std::time::{Duration, Instant};

/// Handshake (HTTP upgrade + `Hello`) must complete within this long of the TCP accept.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
/// Concurrent connections per client IP.
pub const MAX_CONN_PER_IP: usize = 4;
/// Concurrent connections in total (= PlayerId space, ARCHITECTURE §8.1).
pub const MAX_CONNECTIONS: usize = 128;
/// Largest accepted inbound frame payload (generous bound for C0/C1 messages).
pub const MAX_INBOUND_PAYLOAD: usize = 4096;
/// Per-connection send/receive buffer cap; a reliable backlog above this is a slow client.
pub const MAX_BUFFERED_BYTES: usize = 64 * 1024;
/// Unreliable (C2/C3) queue capacity per client; the oldest packet is dropped when full.
pub const UNRELIABLE_QUEUE_CAP: usize = 4;
/// When this many bytes are already in flight/queued towards the socket, only the newest
/// unreliable packet is kept.
pub const WRITE_BACKLOG_LIMIT: usize = 8 * 1024;
/// Sustained inbound frames per second per connection (C1 at 30-60 Hz + pings fit easily).
pub const RATE_FRAMES_PER_SEC: f64 = 150.0;
/// Burst allowance of the inbound frame bucket.
pub const RATE_BURST_FRAMES: f64 = 300.0;

/// Sustained C0 `Ping` messages per second per connection (M0-018). Excess pings are dropped.
pub const PING_PER_SEC: f64 = 5.0;
/// Sustained other C0 messages per second per connection (deploy/chat arrive in M1).
pub const C0_OTHER_PER_SEC: f64 = 10.0;

/// Token bucket over inbound frames.
#[derive(Debug)]
pub struct RateLimiter {
    tokens: f64,
    last: Instant,
    rate: f64,
    burst: f64,
}

impl RateLimiter {
    pub fn new(rate: f64, burst: f64, now: Instant) -> Self {
        RateLimiter {
            tokens: burst,
            last: now,
            rate,
            burst,
        }
    }

    /// Returns `false` when the frame exceeds the allowance.
    pub fn allow(&mut self, now: Instant) -> bool {
        let dt = now.saturating_duration_since(self.last).as_secs_f64();
        self.last = now;
        self.tokens = (self.tokens + dt * self.rate).min(self.burst);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_allows_burst_then_refills() {
        let t0 = Instant::now();
        let mut r = RateLimiter::new(10.0, 5.0, t0);
        for _ in 0..5 {
            assert!(r.allow(t0));
        }
        assert!(!r.allow(t0));
        assert!(r.allow(t0 + Duration::from_millis(150)));
        assert!(!r.allow(t0 + Duration::from_millis(150)));
    }
}
