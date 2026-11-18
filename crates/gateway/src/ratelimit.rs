//! Sliding-window limiter, in memory (single instance). ARCHITECTURE §19.3: per IP 10/min for
//! login + ticket, per account 6/min for tickets.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::Mutex;

pub const WINDOW_MS: u64 = 60_000;

pub struct SlidingLimiter<K: Hash + Eq + Clone> {
    limit: u32,
    hits: Mutex<HashMap<K, VecDeque<u64>>>,
}

impl<K: Hash + Eq + Clone> SlidingLimiter<K> {
    pub fn new(limit: u32) -> Self {
        SlidingLimiter {
            limit,
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// Records a hit at `now_ms`. `Err(retry_after_s)` when over the limit (hit not recorded).
    pub fn check(&self, key: &K, now_ms: u64) -> Result<(), u64> {
        let mut g = self.hits.lock().unwrap_or_else(|e| e.into_inner());
        if g.len() > 50_000 {
            g.retain(|_, q| {
                q.back()
                    .is_some_and(|t| now_ms.saturating_sub(*t) < WINDOW_MS)
            });
        }
        let q = g.entry(key.clone()).or_default();
        while q
            .front()
            .is_some_and(|t| now_ms.saturating_sub(*t) >= WINDOW_MS)
        {
            q.pop_front();
        }
        if q.len() as u32 >= self.limit {
            let oldest = *q.front().unwrap_or(&now_ms);
            let wait = (oldest + WINDOW_MS).saturating_sub(now_ms);
            return Err(wait.div_ceil(1000).max(1));
        }
        q.push_back(now_ms);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eleventh_hit_in_a_minute_is_refused_then_recovers() {
        let l = SlidingLimiter::<u8>::new(10);
        for i in 0..10 {
            assert!(l.check(&1, i * 100).is_ok());
        }
        let retry = l.check(&1, 1_000).unwrap_err();
        assert!((58..=60).contains(&retry), "{retry}");
        assert!(l.check(&2, 1_000).is_ok(), "other keys are independent");
        assert!(
            l.check(&1, 60_000).is_ok(),
            "window slid past the first hit"
        );
    }
}
