//! Per-client snapshot bookkeeping: baselines (the protocol crate's [`ClientBaselines`]), the
//! acknowledgement bridge (16-bit ack tick -> absolute tick, event confirmation, `pred_hash`
//! comparison) and the bandwidth meter.
//!
//! PI-023 contract: the client reports the low 16 bits of the newest snapshot tick it decoded
//! (`ack_snapshot_tick`) together with `pred_hash` of *its* position right after that snapshot's
//! `ack_input_seq`. The server remembers `(tick, hash of the position it sent)` for every packet
//! and compares; a mismatch (and the very first packet) puts `pos_corr` into every packet until
//! the client acknowledges a packet that carried it.

use std::collections::VecDeque;

use frontline_protocol::snapshot::ClientBaselines;

use super::events_out::EventWindow;
use super::priority::PriorityState;

/// Sent-packet records kept for the `pred_hash` comparison (> baseline ring of 32 ticks).
const HIST_LEN: usize = 96;

#[derive(Debug, Clone, Copy)]
struct SentRec {
    tick: u32,
    hash: u8,
}

/// State of the position-correction protocol.
#[derive(Debug, Clone, Copy)]
pub struct Corr {
    /// `pos_corr` goes into every packet while this is set.
    pub pending: bool,
    /// Tick of the first packet that carried the current correction.
    pub first_sent: Option<u32>,
    /// Corrections started (mismatches found), including the initial one.
    pub started_total: u64,
}

/// Sliding 1 s window of sent bytes (30 ticks) for the kbps metrics.
#[derive(Debug, Clone, Default)]
pub struct BandwidthMeter {
    bytes: [u32; 30],
    pkts: [u8; 30],
    idx: usize,
    n: usize,
}

/// Per-packet transport overhead used for the "on the wire" estimate (ARCHITECTURE §6.9.1).
pub const WIRE_OVERHEAD_B: f64 = 58.0;

impl BandwidthMeter {
    /// Records one tick; every 30th call returns `(payload_kbps, wire_kbps)` of the last second.
    pub fn tick(&mut self, bytes: usize, packets: usize) -> Option<(f64, f64)> {
        self.bytes[self.idx] = bytes as u32;
        self.pkts[self.idx] = packets as u8;
        self.idx = (self.idx + 1) % 30;
        self.n += 1;
        if self.n.is_multiple_of(30) {
            let b: u64 = self.bytes.iter().map(|&x| x as u64).sum();
            let p: u64 = self.pkts.iter().map(|&x| x as u64).sum();
            let payload = b as f64 * 8.0 / 1000.0;
            let wire = (b as f64 + p as f64 * WIRE_OVERHEAD_B) * 8.0 / 1000.0;
            Some((payload, wire))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct ClientBook {
    pub baselines: ClientBaselines,
    pub prio: PriorityState,
    pub events: EventWindow,
    hist: VecDeque<SentRec>,
    pub corr: Corr,
    pub last_sent_tick: Option<u32>,
    /// Newest acknowledged absolute tick.
    pub last_ack: Option<u32>,
    pub meter: BandwidthMeter,
    /// Bytes / packets handed out this tick (fed to the meter).
    pub tick_bytes: usize,
    pub tick_packets: usize,
    /// Counters for reports and tests.
    pub packets: u64,
    pub bytes: u64,
    pub keyframe_records: u64,
    pub delta_records: u64,
    pub starved: u64,
    pub joined_tick: u32,
}

impl ClientBook {
    pub fn new(now: u32) -> Self {
        ClientBook {
            baselines: ClientBaselines::new(),
            prio: PriorityState::default(),
            events: EventWindow::default(),
            hist: VecDeque::new(),
            corr: Corr {
                pending: true, // the client's start position is a guess (PI-023)
                first_sent: None,
                started_total: 1,
            },
            last_sent_tick: None,
            last_ack: None,
            meter: BandwidthMeter::default(),
            tick_bytes: 0,
            tick_packets: 0,
            packets: 0,
            bytes: 0,
            keyframe_records: 0,
            delta_records: 0,
            starved: 0,
            joined_tick: now,
        }
    }

    /// Absolute tick for a 16-bit ack: the newest tick <= the last sent one with those low bits.
    pub fn extend_ack(&self, low: u16) -> Option<u32> {
        let last = self.last_sent_tick?;
        Some(last.wrapping_sub(last.wrapping_sub(low as u32) & 0xFFFF))
    }

    /// Remembers what a packet told the client about its own position.
    pub fn record_sent(&mut self, tick: u32, hash: u8) {
        self.hist.push_back(SentRec { tick, hash });
        while self.hist.len() > HIST_LEN {
            self.hist.pop_front();
        }
        self.last_sent_tick = Some(tick);
        if self.corr.pending && self.corr.first_sent.is_none() {
            self.corr.first_sent = Some(tick);
        }
    }

    /// Processes the ack/hash a client reported in its latest input bundle. Idempotent: only a
    /// newer ack than the previous one has an effect. Returns the absolute acked tick when new.
    pub fn on_client_report(&mut self, ack_low: u16, pred_hash: u8) -> Option<u32> {
        let a = self.extend_ack(ack_low)?;
        let rec = self.hist.iter().find(|r| r.tick == a).copied()?;
        if self
            .last_ack
            .is_some_and(|l| (a.wrapping_sub(l) as i32) <= 0)
        {
            return None;
        }
        self.last_ack = Some(a);
        self.baselines.on_ack(a);
        self.events.on_ack(a);
        let matches = rec.hash == pred_hash;
        if self.corr.pending {
            let carried = self
                .corr
                .first_sent
                .is_some_and(|f| (a.wrapping_sub(f) as i32) >= 0);
            if carried && matches {
                self.corr.pending = false;
                self.corr.first_sent = None;
            }
        } else if !matches {
            self.corr.pending = true;
            self.corr.first_sent = None;
            self.corr.started_total += 1;
        }
        Some(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extend_ack_handles_wrap() {
        let mut b = ClientBook::new(0);
        b.record_sent(70_000, 0);
        assert_eq!(b.extend_ack((70_000u32 & 0xFFFF) as u16), Some(70_000));
        assert_eq!(
            b.extend_ack(((70_000u32 - 5) & 0xFFFF) as u16),
            Some(69_995)
        );
        // a low value that is "ahead" of the last sent one maps to the previous 65536 window
        assert_eq!(
            b.extend_ack(((70_000u32 + 5) & 0xFFFF) as u16),
            Some(70_005 - 65_536)
        );
    }

    #[test]
    fn first_packet_correction_is_cleared_by_a_matching_ack() {
        let mut b = ClientBook::new(0);
        assert!(b.corr.pending);
        b.record_sent(10, 77);
        assert_eq!(b.corr.first_sent, Some(10));
        b.record_sent(12, 78);
        // client hash does not match the packet it acked: stays pending
        assert_eq!(b.on_client_report(10, 1), Some(10));
        assert!(b.corr.pending);
        // a later ack with the matching hash clears it
        assert_eq!(b.on_client_report(12, 78), Some(12));
        assert!(!b.corr.pending);
    }

    #[test]
    fn mismatch_after_sync_starts_a_new_correction_once() {
        let mut b = ClientBook::new(0);
        b.record_sent(10, 5);
        b.on_client_report(10, 5);
        assert!(!b.corr.pending);
        b.record_sent(12, 9);
        b.on_client_report(12, 1); // wrong
        assert!(b.corr.pending);
        assert_eq!(b.corr.started_total, 2);
        // same ack again: no effect
        assert_eq!(b.on_client_report(12, 1), None);
        assert_eq!(b.corr.started_total, 2);
    }

    #[test]
    fn meter_reports_once_per_second() {
        let mut m = BandwidthMeter::default();
        let mut out = None;
        for _ in 0..30 {
            out = m.tick(100, 1);
        }
        let (p, w) = out.unwrap();
        assert!((p - 24.0).abs() < 1e-9); // 3000 B/s = 24 kbps
        assert!((w - (3000.0 + 30.0 * 58.0) * 8.0 / 1000.0).abs() < 1e-9);
    }
}
