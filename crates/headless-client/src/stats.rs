//! Bandwidth windows and percentile helpers (M0-027).
//!
//! Traffic is binned per second since the client joined. A bin's kbps is
//! `(payload bytes + WIRE_OVERHEAD * packets) * 8 / 1000`; the payload-only value is kept next to
//! it. Percentiles use the nearest-rank (ceil) rule, like the server's own report.

use serde::{Deserialize, Serialize};

/// Per-packet overhead of the line estimate (UDP/IP + QUIC/H3 framing, ARCHITECTURE §6.9.1).
pub const WIRE_OVERHEAD: u64 = 58;

/// `p50` / `p99` / `max` of a series (all zero for an empty one).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Dist {
    pub p50: f64,
    pub p99: f64,
    pub max: f64,
}

/// Nearest-rank percentile of an ascending slice; `q` in `0..=1`.
pub fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((sorted.len() as f64 - 1.0) * q).ceil() as usize;
    sorted[i.min(sorted.len() - 1)]
}

impl Dist {
    pub fn of(values: &[f64]) -> Dist {
        if values.is_empty() {
            return Dist::default();
        }
        let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Dist {
            p50: percentile(&v, 0.5),
            p99: percentile(&v, 0.99),
            max: v.last().copied().unwrap_or(0.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Bin {
    bytes: u64,
    packets: u64,
}

impl Bin {
    fn payload_kbps(&self) -> f64 {
        self.bytes as f64 * 8.0 / 1000.0
    }
    fn wire_kbps(&self) -> f64 {
        (self.bytes + WIRE_OVERHEAD * self.packets) as f64 * 8.0 / 1000.0
    }
}

/// One direction's per-second traffic.
#[derive(Debug, Clone, Default)]
pub struct Meter {
    bins: Vec<Bin>,
    pub total_bytes: u64,
    pub total_packets: u64,
}

impl Meter {
    /// Records one packet of `bytes` payload at `sec` seconds after the client started.
    pub fn add(&mut self, sec: usize, bytes: usize) {
        if self.bins.len() <= sec {
            self.bins.resize(sec + 1, Bin::default());
        }
        let b = &mut self.bins[sec];
        b.bytes += bytes as u64;
        b.packets += 1;
        self.total_bytes += bytes as u64;
        self.total_packets += 1;
    }

    /// Bins `[skip_start, total_secs - skip_end)`: seconds that were fully inside the run.
    fn window(&self, total_secs: usize, skip_start: usize, skip_end: usize) -> Vec<Bin> {
        let end = total_secs.saturating_sub(skip_end);
        (skip_start..end)
            .map(|s| self.bins.get(s).copied().unwrap_or_default())
            .collect()
    }

    /// `(line estimate, payload only)` distributions over the measured window.
    pub fn dists(&self, total_secs: usize, warmup: usize) -> (Dist, Dist) {
        let w = self.window(total_secs, warmup, 1);
        let wire: Vec<f64> = w.iter().map(Bin::wire_kbps).collect();
        let pay: Vec<f64> = w.iter().map(Bin::payload_kbps).collect();
        (Dist::of(&wire), Dist::of(&pay))
    }

    /// Raw per-second values (line estimate) over the measured window, for cross-client pooling.
    pub fn series_wire(&self, total_secs: usize, warmup: usize) -> Vec<f64> {
        self.window(total_secs, warmup, 1)
            .iter()
            .map(Bin::wire_kbps)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_edges() {
        assert_eq!(percentile(&[], 0.5), 0.0);
        assert_eq!(percentile(&[7.0], 0.99), 7.0);
        let v: Vec<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(percentile(&v, 0.5), 51.0); // ceil(99 * 0.5) = 50 -> value 51
        assert_eq!(percentile(&v, 0.99), 100.0);
        assert_eq!(percentile(&v, 1.0), 100.0);
        assert_eq!(percentile(&v, 0.0), 1.0);
    }

    #[test]
    fn dist_ignores_nan_and_sorts() {
        let d = Dist::of(&[3.0, f64::NAN, 1.0, 2.0]);
        assert_eq!(d.max, 3.0);
        assert_eq!(d.p50, 2.0);
        assert_eq!(Dist::of(&[]), Dist::default());
    }

    #[test]
    fn meter_wire_vs_payload_and_window() {
        let mut m = Meter::default();
        // 20 packets of 300 B in each of seconds 0..5; second 0 is warm-up, 4 is the partial tail.
        for s in 0..5 {
            for _ in 0..20 {
                m.add(s, 300);
            }
        }
        let (wire, pay) = m.dists(5, 1);
        // payload: 20*300*8/1000 = 48 kbps; wire adds 20*58*8/1000 = 9.28 kbps
        assert!((pay.p50 - 48.0).abs() < 1e-9, "{pay:?}");
        assert!((wire.p50 - 57.28).abs() < 1e-9, "{wire:?}");
        assert_eq!(m.series_wire(5, 1).len(), 3);
        assert_eq!(m.total_packets, 100);
    }

    #[test]
    fn empty_window_is_zero() {
        let m = Meter::default();
        assert_eq!(m.dists(2, 5).0, Dist::default());
    }

    #[test]
    fn meter_fills_gaps_with_zero() {
        let mut m = Meter::default();
        m.add(0, 100);
        m.add(4, 100);
        let s = m.series_wire(6, 0);
        assert_eq!(s.len(), 5);
        assert_eq!(s[2], 0.0);
    }
}
