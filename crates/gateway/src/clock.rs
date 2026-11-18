//! Wall clock with a manual mode so TTLs, rate-limit windows and heartbeat timeouts are testable.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Clock(Arc<Inner>);

enum Mode {
    Real,
    Manual,
}

struct Inner {
    mode: Mode,
    ms: AtomicU64,
}

impl Clock {
    pub fn real() -> Clock {
        Clock(Arc::new(Inner {
            mode: Mode::Real,
            ms: AtomicU64::new(0),
        }))
    }

    /// A clock that only moves with [`Clock::advance_ms`].
    pub fn manual(start_ms: u64) -> Clock {
        Clock(Arc::new(Inner {
            mode: Mode::Manual,
            ms: AtomicU64::new(start_ms),
        }))
    }

    pub fn advance_ms(&self, d: u64) {
        self.0.ms.fetch_add(d, Ordering::SeqCst);
    }

    /// Milliseconds since the Unix epoch.
    pub fn now_ms(&self) -> u64 {
        match self.0.mode {
            Mode::Real => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_millis() as u64),
            Mode::Manual => self.0.ms.load(Ordering::SeqCst),
        }
    }

    pub fn now_s(&self) -> u64 {
        self.now_ms() / 1000
    }
}
