//! Per-client C2 budget control (M2-020). No time or I/O dependencies: the scheduler supplies
//! one sample per sim tick and the transport supplies optional capacity observations.

use std::collections::VecDeque;

use serde::Deserialize;

const HZ: f32 = 30.0;
const EVALUATE_EVERY: u8 = 10;
const ACK_WINDOW: usize = 30;
const ACK_HEALTHY: f32 = 0.95;
const HEALTHY_TICKS: u32 = 60;
const NOTICE_TICKS: u32 = 300;
const NOTICE_LOW: f32 = 0.70;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegulatorCfg {
    pub ack_low: f32,
    pub min_scale: f32,
    pub step_down: f32,
    pub step_up: f32,
    pub ewma_alpha: f32,
}

impl Default for RegulatorCfg {
    fn default() -> Self {
        let cfg: Self = serde_json::from_str(include_str!("../../config/server_net.json"))
            .expect("valid server_net.json");
        assert!((0.0..=ACK_HEALTHY).contains(&cfg.ack_low));
        assert!((0.4..=1.0).contains(&cfg.min_scale));
        for x in [cfg.step_down, cfg.step_up, cfg.ewma_alpha] {
            assert!(x > 0.0 && x <= 1.0);
        }
        cfg
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetNotice {
    Degraded(u8),
    Recovered,
}

#[derive(Debug)]
pub struct BandwidthRegulator {
    cfg: RegulatorCfg,
    pending: VecDeque<(u32, bool)>,
    ack_bits: u32,
    ack_count: usize,
    periods: u8,
    ewma_bytes_per_tick: f32,
    rtt_ms: f32,
    scale: f32,
    healthy_since: Option<u32>,
    last_notice: Option<u32>,
    announced_degraded: bool,
}

impl BandwidthRegulator {
    pub fn new(cfg: RegulatorCfg, base: usize) -> Self {
        Self {
            cfg,
            pending: VecDeque::new(),
            ack_bits: 0,
            ack_count: 0,
            periods: 0,
            ewma_bytes_per_tick: base as f32,
            rtt_ms: 0.0,
            scale: 1.0,
            healthy_since: None,
            last_notice: None,
            announced_degraded: false,
        }
    }

    /// Feed bytes offered to the snapshot transport this tick (zero on idle ticks).
    pub fn on_send(&mut self, bytes: usize) {
        self.ewma_bytes_per_tick += self.cfg.ewma_alpha * (bytes as f32 - self.ewma_bytes_per_tick);
    }

    pub fn track_snapshot(&mut self, tick: u32) {
        self.pending.push_back((tick, false));
    }

    /// C1 carries only the newest decoded C2 tick, not a bitmap or cumulative ACK.
    pub fn on_ack(&mut self, tick: u32) {
        if let Some((_, acked)) = self.pending.iter_mut().rev().find(|(t, _)| *t == tick) {
            *acked = true;
        }
    }

    pub fn on_rtt(&mut self, ms: f32) {
        if ms.is_finite() && ms > 0.0 {
            self.rtt_ms = if self.rtt_ms == 0.0 {
                ms
            } else {
                self.rtt_ms + self.cfg.ewma_alpha * (ms - self.rtt_ms)
            };
        }
    }

    pub fn ack_rate(&self) -> Option<f32> {
        (self.ack_count >= EVALUATE_EVERY as usize)
            .then(|| self.ack_bits.count_ones() as f32 / self.ack_count as f32)
    }

    /// Only count a missing ACK after the RTT and a jitter/input-cadence allowance. Evaluate
    /// on every tenth packet slot (even if the slot could not produce a packet).
    pub fn tick(
        &mut self,
        now: u32,
        packet_due: bool,
        capacity_bytes_per_sec: Option<u64>,
    ) -> bool {
        let grace = ((self.rtt_ms + 150.0) * HZ / 1000.0)
            .ceil()
            .clamp(6.0, 60.0) as u32;
        while self
            .pending
            .front()
            .is_some_and(|(t, _)| now.wrapping_sub(*t) >= grace)
        {
            let (_, acked) = self.pending.pop_front().expect("front exists");
            self.ack_bits = ((self.ack_bits << 1) | u32::from(acked)) & ((1 << ACK_WINDOW) - 1);
            self.ack_count = (self.ack_count + 1).min(ACK_WINDOW);
        }
        self.periods += u8::from(packet_due);
        if self.periods < EVALUATE_EVERY {
            return false;
        }
        self.periods = 0;
        // Do not use the seeded EWMA as congestion evidence before the receiver has samples.
        let Some(ack) = self.ack_rate() else {
            return true;
        };
        let short = capacity_bytes_per_sec
            .is_some_and(|cap| (cap as f32) < self.ewma_bytes_per_tick * HZ * 1.2);
        if ack < self.cfg.ack_low || short {
            self.healthy_since = None;
            self.scale = (self.scale - self.cfg.step_down).max(self.cfg.min_scale);
        } else if ack >= ACK_HEALTHY {
            let since = *self.healthy_since.get_or_insert(now);
            if now.wrapping_sub(since) >= HEALTHY_TICKS {
                self.scale = (self.scale + self.cfg.step_up).min(1.0);
                if self.scale > 1.0 - f32::EPSILON {
                    self.scale = 1.0;
                }
            }
        } else {
            self.healthy_since = None;
        }
        true
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn budget(&self, base: usize) -> usize {
        (base as f64 * f64::from(self.scale)).floor() as usize
    }

    /// Number of candidates to omit, rather than lowering retained candidates' frequencies.
    pub fn shed_far(&self, count: usize) -> usize {
        count - self.budget(count)
    }

    pub fn notice(&mut self, now: u32) -> Option<BudgetNotice> {
        if self
            .last_notice
            .is_some_and(|t| now.wrapping_sub(t) < NOTICE_TICKS)
        {
            return None;
        }
        let notice = if !self.announced_degraded && self.scale + f32::EPSILON < NOTICE_LOW {
            self.announced_degraded = true;
            BudgetNotice::Degraded((self.scale * 100.0).round() as u8)
        } else if self.announced_degraded && self.scale >= ACK_HEALTHY {
            self.announced_degraded = false;
            BudgetNotice::Recovered
        } else {
            return None;
        };
        self.last_notice = Some(now);
        Some(notice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn step(
        r: &mut BandwidthRegulator,
        t: u32,
        ack: bool,
        capacity: Option<u64>,
    ) -> Option<BudgetNotice> {
        let due = !t.is_multiple_of(3);
        if ack && t > 1 {
            r.on_ack(t - 1);
            r.on_ack(t - 1); // repeated latest ACK must not inflate the rate
        }
        r.tick(t, due, capacity);
        r.on_send(if due { 256 } else { 0 });
        if due {
            r.track_snapshot(t);
        }
        r.notice(t)
    }

    #[test]
    fn ack_and_capacity_trigger_clamped_linear_decrease_and_hysteretic_recovery() {
        for capacity in [None, Some(5000)] {
            let mut r = BandwidthRegulator::new(RegulatorCfg::default(), 1150);
            let mut last = 1.0;
            for t in 1..240 {
                step(&mut r, t, capacity.is_some(), capacity);
                assert!(r.scale() <= last && last - r.scale() <= 0.10001);
                last = r.scale();
            }
            assert_eq!(r.scale(), 0.4);
            assert_eq!(r.budget(1150), 460);
            assert_eq!(r.shed_far(22), 14);
            for t in 240..300 {
                step(&mut r, t, true, Some(100_000));
                assert_eq!(r.scale(), 0.4);
            }
            let recovered = (300..540).find(|&t| {
                step(&mut r, t, true, Some(100_000));
                r.scale() >= 0.95
            });
            assert!(recovered.is_some());
        }
    }

    #[test]
    fn ewma_includes_idle_ticks_and_clean_delay_does_not_scale_down() {
        let cfg = RegulatorCfg {
            ewma_alpha: 0.25,
            ..RegulatorCfg::default()
        };
        let mut r = BandwidthRegulator::new(cfg, 100);
        r.on_send(200);
        assert_eq!(r.ewma_bytes_per_tick, 125.0);
        r.on_send(0);
        assert_eq!(r.ewma_bytes_per_tick, 93.75);
        for t in 1u32..600 {
            r.on_rtt(150.0);
            if t > 5 {
                r.on_ack(t - 5);
            }
            let due = !t.is_multiple_of(3);
            r.tick(t, due, Some(100_000));
            if due {
                r.track_snapshot(t);
            }
            assert_eq!(r.scale(), 1.0);
        }
        assert_eq!(r.ack_rate(), Some(1.0));
    }

    #[test]
    fn notice_reports_transitions_at_most_once_per_ten_seconds() {
        let mut r = BandwidthRegulator::new(RegulatorCfg::default(), 1150);
        let mut events = Vec::new();
        for t in 1..1200 {
            if let Some(n) = step(&mut r, t, (130..470).contains(&t) || t >= 650, None) {
                events.push((t, n));
            }
        }
        assert!(
            matches!(
                events.as_slice(),
                [
                    (_, BudgetNotice::Degraded(60)),
                    (_, BudgetNotice::Recovered),
                    (_, BudgetNotice::Degraded(_)),
                    (_, BudgetNotice::Recovered)
                ]
            ),
            "{events:?}"
        );
        assert!(events.windows(2).all(|v| v[1].0 - v[0].0 >= 300));
    }

    proptest! {
        #[test]
        fn arbitrary_ack_sequences_respect_budget_bounds(samples in proptest::collection::vec((any::<bool>(), 0u32..100_000), 0..1200)) {
            let mut r = BandwidthRegulator::new(RegulatorCfg::default(), 1150);
            for (i, (ack, capacity)) in samples.into_iter().enumerate() {
                step(&mut r, i as u32 + 1, ack, Some(u64::from(capacity)));
                prop_assert!((0.4..=1.0).contains(&r.scale()));
                prop_assert!((460..=1150).contains(&r.budget(1150)));
            }
        }
    }
}
