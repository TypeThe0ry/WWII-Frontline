//! Time source abstraction and the absolute tick schedule (ARCHITECTURE §6.1).
//!
//! Waiting strategy: `SystemClock::sleep_until` sleeps (`std::thread::sleep`) until
//! `deadline - SPIN_MARGIN` and spins on `Instant::now()` for the last margin. Own ~20 lines
//! instead of the `spin_sleep` crate: no new dependency, and the margin is tunable (1 ms per
//! task spec; Linux `nanosleep` overshoot is typically 50-100 us, so 1 ms leaves headroom).

use std::time::{Duration, Instant};

/// Margin that is spun rather than slept.
pub const SPIN_MARGIN: Duration = Duration::from_millis(1);

pub trait Clock {
    fn now(&self) -> Instant;
    /// Blocks until `now() >= deadline` (returns immediately if already past).
    fn sleep_until(&self, deadline: Instant);
}

impl<T: Clock + ?Sized> Clock for &T {
    fn now(&self) -> Instant {
        (**self).now()
    }
    fn sleep_until(&self, deadline: Instant) {
        (**self).sleep_until(deadline)
    }
}

/// Real monotonic clock with hybrid sleep + spin.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep_until(&self, deadline: Instant) {
        let now = Instant::now();
        if let Some(coarse) = deadline.checked_duration_since(now) {
            if coarse > SPIN_MARGIN {
                std::thread::sleep(coarse - SPIN_MARGIN);
            }
        }
        while Instant::now() < deadline {
            std::hint::spin_loop();
        }
    }
}

/// Absolute tick schedule: deadlines are always `start + k * period` (no drift). After an
/// overrun, missed slots are skipped, never replayed (no catch-up burst).
#[derive(Debug, Clone, Copy)]
pub struct TickSchedule {
    start: Instant,
    period: Duration,
    /// Index of the next slot (deadline = start + slot * period).
    slot: u64,
}

impl TickSchedule {
    pub fn new(start: Instant, period: Duration) -> Self {
        TickSchedule {
            start,
            period,
            slot: 0,
        }
    }

    pub fn period(&self) -> Duration {
        self.period
    }

    fn at(&self, slot: u64) -> Instant {
        // Duration * u32 is exact in nanoseconds; slot fits in u32 for >4 years at 30 Hz.
        self.start + self.period.mul_f64(slot as f64)
    }

    /// Deadline of the tick about to be run (slot 0 = `start`).
    pub fn current_deadline(&self) -> Instant {
        self.at(self.slot)
    }

    /// Moves to the next slot strictly after `now`. Returns the number of skipped slots (slots
    /// whose deadline has already passed and which will never be run).
    pub fn advance(&mut self, now: Instant) -> u64 {
        self.slot += 1;
        let mut skipped = 0;
        if self.at(self.slot) <= now {
            let late = now.duration_since(self.start).as_nanos();
            let target = late / self.period.as_nanos().max(1) + 1;
            let target = target as u64;
            skipped = target.saturating_sub(self.slot);
            self.slot = target.max(self.slot);
        }
        skipped
    }

    pub fn next_deadline(&self) -> Instant {
        self.at(self.slot)
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::cell::Cell;

    /// Manually advanced clock; `sleep_until` jumps time to the deadline.
    pub struct FakeClock {
        base: Instant,
        offset_ns: Cell<u128>,
    }

    impl Default for FakeClock {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FakeClock {
        pub fn new() -> Self {
            FakeClock {
                base: Instant::now(),
                offset_ns: Cell::new(0),
            }
        }
        pub fn advance(&self, d: Duration) {
            self.offset_ns.set(self.offset_ns.get() + d.as_nanos());
        }
        pub fn since_base(&self, t: Instant) -> Duration {
            t.duration_since(self.base)
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> Instant {
            self.base + Duration::from_nanos(self.offset_ns.get() as u64)
        }
        fn sleep_until(&self, deadline: Instant) {
            let target = deadline.duration_since(self.base).as_nanos();
            if target > self.offset_ns.get() {
                self.offset_ns.set(target);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeClock;
    use super::*;

    const P: Duration = Duration::from_nanos(33_333_333);

    #[test]
    fn deadlines_are_on_the_grid_without_drift() {
        let c = FakeClock::new();
        let mut s = TickSchedule::new(c.now(), P);
        for k in 0..3000u64 {
            assert_eq!(c.since_base(s.current_deadline()), P.mul_f64(k as f64));
            c.sleep_until(s.current_deadline());
            c.advance(Duration::from_millis(2)); // tick work
            assert_eq!(s.advance(c.now()), 0);
        }
        // 3000 ticks at 30 Hz = 100 s, error from f64 rounding only.
        let d = c.since_base(s.current_deadline());
        assert!((d.as_secs_f64() - 100.0).abs() < 1e-3, "{d:?}");
    }

    #[test]
    fn overrun_skips_slots_and_stays_on_grid() {
        let c = FakeClock::new();
        let mut s = TickSchedule::new(c.now(), P);
        c.sleep_until(s.current_deadline()); // slot 0 at t=0
        c.advance(Duration::from_millis(50)); // 50 ms tick
        let skipped = s.advance(c.now());
        // slot 1 (33.3) is past, next free slot is 2 (66.7).
        assert_eq!(skipped, 1);
        assert_eq!(c.since_base(s.current_deadline()), P.mul_f64(2.0));
        assert!(s.current_deadline() > c.now());
    }
}
