use std::time::{Duration, Instant};

pub const DEGRADE_AFTER_OVERRUNS: u32 = 3;
pub const DEGRADE_RECOVERY: Duration = Duration::from_secs(2);

#[derive(Debug, Default)]
pub struct OverrunGuard {
    consecutive: u32,
    last_overrun: Option<Instant>,
}

impl OverrunGuard {
    pub fn record(&mut self, elapsed: Duration, budget: Duration, now: Instant) -> u32 {
        if elapsed > budget {
            self.consecutive = self.consecutive.saturating_add(1);
            self.last_overrun = Some(now);
        } else {
            self.consecutive = 0;
        }
        self.consecutive
    }

    pub fn degraded(&self, now: Instant) -> bool {
        self.consecutive >= DEGRADE_AFTER_OVERRUNS
            && self
                .last_overrun
                .is_some_and(|last| now.duration_since(last) < DEGRADE_RECOVERY)
    }

    pub fn consecutive(&self) -> u32 {
        self.consecutive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_consecutive_overruns_degrade_and_recover_after_two_seconds() {
        let start = Instant::now();
        let mut guard = OverrunGuard::default();
        for tick in 0..3u32 {
            assert_eq!(
                guard.record(
                    Duration::from_millis(40),
                    Duration::from_millis(33),
                    start + Duration::from_millis(u64::from(tick) * 33),
                ),
                tick + 1
            );
        }
        assert!(guard.degraded(start + Duration::from_millis(99)));
        assert!(!guard.degraded(start + Duration::from_millis(99) + DEGRADE_RECOVERY));
    }

    #[test]
    fn an_on_budget_tick_breaks_the_consecutive_run() {
        let now = Instant::now();
        let mut guard = OverrunGuard::default();
        for _ in 0..2 {
            guard.record(Duration::from_millis(40), Duration::from_millis(33), now);
        }
        assert_eq!(
            guard.record(Duration::from_millis(33), Duration::from_millis(33), now),
            0
        );
        assert!(!guard.degraded(now));
    }
}
