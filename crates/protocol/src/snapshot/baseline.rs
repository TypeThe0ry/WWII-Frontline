//! Baseline bookkeeping on both ends (ARCHITECTURE §6.5.2).
//!
//! A packet carries **one** baseline reference: the tick of an earlier snapshot the client has
//! acknowledged. A delta record is only produced for entities that were part of that exact
//! snapshot; every other entity gets a keyframe. Refinement of the per-entity baseline text in
//! §6.5.2: this keeps encoder and decoder trivially in sync under packet loss (the decoder only
//! needs "the states of snapshot tick T").

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::snapshot::entity::EntityState;

/// Maximum baseline age in ticks (ring length, about 1 s at 30 Hz). Older -> keyframes.
pub const BASELINE_RING: u32 = 32;
/// Unacknowledged packets remembered per client.
pub(crate) const MAX_PENDING: usize = 64;
/// The 16-bit tick form is only used while the newest ack is younger than this.
pub(crate) const MAX_SHORT_TICK_AGE: u32 = 16_384;

/// Entity states of one sent/received snapshot, sorted by id.
#[derive(Debug, Clone)]
pub(crate) struct SnapshotStates {
    pub tick: u32,
    pub states: Vec<EntityState>,
}

/// Server-side baseline state for one client.
#[derive(Debug, Default)]
pub struct ClientBaselines {
    pub(crate) baseline: Option<SnapshotStates>,
    pub(crate) pending: VecDeque<SnapshotStates>,
    /// Tick of the newest acknowledged snapshot ever (survives lag-outs).
    pub(crate) last_acked: Option<u32>,
}

impl ClientBaselines {
    pub fn new() -> Self {
        Self::default()
    }

    /// Tick of the current baseline, if any.
    pub fn baseline_tick(&self) -> Option<u32> {
        self.baseline.as_ref().map(|b| b.tick)
    }

    /// The baseline usable for a packet at `tick` (exists, strictly older, within the ring).
    pub(crate) fn usable_baseline(&self, tick: u32) -> Option<&SnapshotStates> {
        let b = self.baseline.as_ref()?;
        let age = tick.wrapping_sub(b.tick);
        (age >= 1 && age <= BASELINE_RING).then_some(b)
    }

    /// The client acknowledged the snapshot sent at `ack_tick` (absolute tick). Unknown or
    /// stale acks are ignored.
    pub fn on_ack(&mut self, ack_tick: u32) {
        if let Some(b) = &self.baseline {
            // not newer than the current baseline (also covers wrap-around garbage)
            if ack_tick.wrapping_sub(b.tick) as i32 <= 0 {
                return;
            }
        }
        let Some(i) = self.pending.iter().position(|p| p.tick == ack_tick) else {
            return;
        };
        // promote it and forget everything older
        for _ in 0..i {
            self.pending.pop_front();
        }
        self.baseline = self.pending.pop_front();
        self.last_acked = Some(ack_tick);
    }

    pub(crate) fn push_sent(&mut self, tick: u32, mut states: Vec<EntityState>) {
        states.sort_by_key(|s| s.id);
        self.pending.retain(|p| p.tick != tick);
        self.pending.push_back(SnapshotStates { tick, states });
        while self.pending.len() > MAX_PENDING
            || self
                .pending
                .front()
                .is_some_and(|p| tick.wrapping_sub(p.tick) > 2 * BASELINE_RING)
        {
            self.pending.pop_front();
        }
    }
}

/// Client-side ring of recently decoded snapshots.
#[derive(Debug, Default)]
pub(crate) struct DecoderRing {
    pub entries: VecDeque<SnapshotStates>,
    pub newest: Option<u32>,
}

impl DecoderRing {
    pub fn get(&self, tick: u32) -> Option<&SnapshotStates> {
        self.entries.iter().find(|e| e.tick == tick)
    }

    pub fn push(&mut self, tick: u32, mut states: Vec<EntityState>) {
        states.sort_by_key(|s| s.id);
        self.entries.retain(|e| e.tick != tick);
        self.entries.push_back(SnapshotStates { tick, states });
        if self.newest.is_none_or(|n| tick.wrapping_sub(n) as i32 > 0) {
            self.newest = Some(tick);
        }
        let newest = self.newest.unwrap_or(tick);
        self.entries
            .retain(|e| newest.wrapping_sub(e.tick) as i32 <= 2 * BASELINE_RING as i32);
        while self.entries.len() > 2 * BASELINE_RING as usize + 2 {
            self.entries.pop_front();
        }
    }
}
