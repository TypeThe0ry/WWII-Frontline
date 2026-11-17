//! Server clock estimate and input lead (ARCHITECTURE §6.2; mirror of `client/src/net/clock.ts`).
//!
//! The server tick "now" is `tick_pong + 0.5 + (now - t_recv + rtt / 2) / TICK_MS`, taken from the
//! lowest-RTT sample among the last 8 Pongs (the Welcome counts as the first sample). The input
//! lead is `ceil(rtt / 2 / TICK_MS) + 1` ticks plus a bias nudged by the server's `input_slack`
//! report (target 1 tick).

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

pub const TICK_MS: f64 = 1000.0 / 30.0;
const WINDOW: usize = 8;
const TARGET_SLACK: f64 = 1.0;
const MAX_BIAS: i32 = 3;
const MIN_BIAS: i32 = -1;

#[derive(Clone, Copy)]
struct Sample {
    tick: u32,
    recv: Instant,
    rtt_ms: f64,
}

pub struct Clock {
    samples: VecDeque<Sample>,
    pending: HashMap<u32, Instant>,
    rtt_ewma: Option<f64>,
    origin: Instant,
    last_ct: u32,
    slack_ewma: Option<f64>,
    bias: i32,
    last_bias_change: Instant,
}

impl Clock {
    /// `recv` is when the Welcome arrived, `rtt` the handshake round trip.
    pub fn new(server_tick: u32, recv: Instant, rtt: Duration) -> Self {
        let rtt_ms = rtt.as_secs_f64() * 1000.0;
        let mut samples = VecDeque::new();
        samples.push_back(Sample {
            tick: server_tick,
            recv,
            rtt_ms,
        });
        Clock {
            samples,
            pending: HashMap::new(),
            rtt_ewma: Some(rtt_ms),
            origin: recv,
            last_ct: 0,
            slack_ewma: None,
            bias: 0,
            last_bias_change: recv,
        }
    }

    /// New `Ping.client_time` (strictly increasing ms since the clock started).
    pub fn ping(&mut self, now: Instant) -> u32 {
        let mut ct = now.saturating_duration_since(self.origin).as_millis() as u32;
        if ct <= self.last_ct {
            ct = self.last_ct + 1;
        }
        self.last_ct = ct;
        self.pending.insert(ct, now);
        if self.pending.len() > 64 {
            if let Some(&k) = self.pending.keys().min() {
                self.pending.remove(&k);
            }
        }
        ct
    }

    /// Feeds a Pong; returns the RTT sample in ms.
    pub fn on_pong(&mut self, client_time: u32, server_tick: u32, recv: Instant) -> Option<f64> {
        let sent = self.pending.remove(&client_time)?;
        let rtt_ms = recv.saturating_duration_since(sent).as_secs_f64() * 1000.0;
        self.rtt_ewma = Some(match self.rtt_ewma {
            None => rtt_ms,
            Some(p) => p + 0.1 * (rtt_ms - p),
        });
        self.samples.push_back(Sample {
            tick: server_tick,
            recv,
            rtt_ms,
        });
        while self.samples.len() > WINDOW {
            self.samples.pop_front();
        }
        Some(rtt_ms)
    }

    pub fn rtt_ms(&self) -> f64 {
        self.rtt_ewma.unwrap_or(0.0)
    }

    /// Estimated server tick at `now`, as a fraction.
    pub fn server_tick(&self, now: Instant) -> f64 {
        let best = self
            .samples
            .iter()
            .min_by(|a, b| a.rtt_ms.partial_cmp(&b.rtt_ms).unwrap_or(std::cmp::Ordering::Equal))
            .copied()
            .unwrap_or(Sample {
                tick: 0,
                recv: now,
                rtt_ms: 0.0,
            });
        let since = now.saturating_duration_since(best.recv).as_secs_f64() * 1000.0;
        f64::from(best.tick) + 0.5 + (since + best.rtt_ms / 2.0) / TICK_MS
    }

    /// Ticks the newest command should lead the server's current tick by.
    pub fn lead_ticks(&self) -> i64 {
        ((self.rtt_ms() / 2.0 / TICK_MS).ceil() as i64) + 1 + i64::from(self.bias)
    }

    /// Tick the next command must carry when generated at `now`.
    pub fn input_tick(&self, now: Instant) -> u32 {
        (self.server_tick(now).floor() as i64 + self.lead_ticks()).max(0) as u32
    }

    /// `input_slack` of a snapshot: how many ticks before execution the latest input arrived.
    pub fn on_slack(&mut self, slack: u8, now: Instant) {
        let s = f64::from(slack);
        let e = match self.slack_ewma {
            None => s,
            Some(p) => p + 0.2 * (s - p),
        };
        self.slack_ewma = Some(e);
        if now.saturating_duration_since(self.last_bias_change) < Duration::from_secs(2) {
            return;
        }
        if e < TARGET_SLACK - 0.5 && self.bias < MAX_BIAS {
            self.bias += 1;
            self.last_bias_change = now;
        } else if e > TARGET_SLACK + 2.0 && self.bias > MIN_BIAS {
            self.bias -= 1;
            self.last_bias_change = now;
        }
    }

    pub fn bias(&self) -> i32 {
        self.bias
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_advances_with_time_and_adds_half_rtt() {
        let t0 = Instant::now();
        let c = Clock::new(100, t0, Duration::from_millis(20));
        let a = c.server_tick(t0);
        // 100 + 0.5 + (0 + 10 ms) / 33.33
        assert!((a - (100.5 + 10.0 / TICK_MS)).abs() < 1e-9);
        let b = c.server_tick(t0 + Duration::from_millis(1000));
        assert!((b - a - 30.0).abs() < 1e-6);
    }

    #[test]
    fn lead_follows_rtt() {
        let t0 = Instant::now();
        let c = Clock::new(0, t0, Duration::from_millis(1));
        assert_eq!(c.lead_ticks(), 1);
        let c = Clock::new(0, t0, Duration::from_millis(100));
        assert_eq!(c.lead_ticks(), 3); // ceil(50 / 33.3) + 1
    }

    #[test]
    fn pong_matches_ping_and_picks_min_rtt() {
        let t0 = Instant::now();
        let mut c = Clock::new(0, t0, Duration::from_millis(50));
        let ct = c.ping(t0);
        let r = c.on_pong(ct, 10, t0 + Duration::from_millis(5)).unwrap();
        assert!((r - 5.0).abs() < 0.5);
        assert!(c.on_pong(ct, 10, t0).is_none(), "duplicate pong");
        // lowest-RTT sample (the pong) drives the estimate now
        let est = c.server_tick(t0 + Duration::from_millis(5));
        assert!((est - (10.5 + 2.5 / TICK_MS)).abs() < 0.2, "{est}");
    }

    #[test]
    fn ping_times_strictly_increase() {
        let t0 = Instant::now();
        let mut c = Clock::new(0, t0, Duration::ZERO);
        let a = c.ping(t0);
        let b = c.ping(t0);
        assert!(b > a);
    }

    #[test]
    fn slack_bias_moves_after_cooldown() {
        let t0 = Instant::now();
        let mut c = Clock::new(0, t0, Duration::ZERO);
        for i in 0..20 {
            c.on_slack(0, t0 + Duration::from_millis(100 * i));
        }
        assert_eq!(c.bias(), 0, "inside the 2 s cooldown nothing moves");
        c.on_slack(0, t0 + Duration::from_secs(3));
        assert_eq!(c.bias(), 1);
        for k in 0..10 {
            c.on_slack(0, t0 + Duration::from_secs(6 + 3 * k));
        }
        assert_eq!(c.bias(), MAX_BIAS);
    }
}
