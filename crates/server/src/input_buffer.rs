//! Per-player input buffer: an 8-slot ring indexed by `cmd.tick` (ARCHITECTURE §6.3).
//!
//! * Window: a command is accepted when `now - 2 <= tick <= now + 10`, where `now` is the tick
//!   the sim is about to run. Older is [`PushResult::TooOld`], newer [`PushResult::TooFar`].
//! * One slot per tick (`tick % 8`): a second command for the same tick is a
//!   [`PushResult::Duplicate`] (first one wins - a client cannot rewrite what it already sent).
//! * Ring wrap: a newer command may replace an older, not yet consumed one (it is counted by the
//!   caller as `ring_overwrite`); an older command never replaces a newer one.
//! * Arrival order is irrelevant: [`InputBuffer::take`] returns exactly the command of the
//!   requested tick.

use frontline_protocol::msg::input::InputCmd;

use crate::validate::Reject;

/// Slots in the ring.
pub const CAPACITY: usize = 8;
/// Oldest accepted tick is `now - MAX_PAST`.
pub const MAX_PAST: u32 = 2;
/// Newest accepted tick is `now + MAX_FUTURE`.
pub const MAX_FUTURE: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushResult {
    Accepted,
    /// Accepted, but replaced an older command that had not been consumed yet.
    AcceptedOverwrote,
    Duplicate,
    TooOld,
    TooFar,
    /// Refused by validation (see [`Reject`]).
    Invalid(Reject),
}

impl PushResult {
    pub fn is_accepted(self) -> bool {
        matches!(self, PushResult::Accepted | PushResult::AcceptedOverwrote)
    }

    /// The matching rejection reason, if any (for metrics).
    pub fn reject(self) -> Option<Reject> {
        match self {
            PushResult::Accepted => None,
            PushResult::AcceptedOverwrote => Some(Reject::RingOverwrite),
            PushResult::Duplicate => Some(Reject::Duplicate),
            PushResult::TooOld => Some(Reject::TooOld),
            PushResult::TooFar => Some(Reject::TooFar),
            PushResult::Invalid(r) => Some(r),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InputBuffer {
    slots: [Option<InputCmd>; CAPACITY],
}

impl Default for InputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl InputBuffer {
    pub fn new() -> Self {
        InputBuffer {
            slots: [None; CAPACITY],
        }
    }

    #[inline]
    fn idx(tick: u32) -> usize {
        tick as usize % CAPACITY
    }

    /// Window check only (no state change).
    pub fn check_window(tick: u32, now: u32) -> Result<(), PushResult> {
        if (tick as u64) + (MAX_PAST as u64) < now as u64 {
            Err(PushResult::TooOld)
        } else if tick as u64 > now as u64 + MAX_FUTURE as u64 {
            Err(PushResult::TooFar)
        } else {
            Ok(())
        }
    }

    /// Everything [`Self::push`] would check, without storing: window and slot conflict.
    pub fn classify(&self, tick: u32, now: u32) -> Result<(), PushResult> {
        Self::check_window(tick, now)?;
        match self.slots[Self::idx(tick)] {
            Some(c) if c.tick == tick => Err(PushResult::Duplicate),
            Some(c) if c.tick > tick => Err(PushResult::TooOld),
            _ => Ok(()),
        }
    }

    /// Stores `cmd` (already validated by the caller) if the window and ring rules allow it.
    pub fn push(&mut self, cmd: InputCmd, now: u32) -> PushResult {
        if let Err(r) = self.classify(cmd.tick, now) {
            return r;
        }
        let slot = &mut self.slots[Self::idx(cmd.tick)];
        let overwrote = matches!(slot, Some(old) if old.tick >= now);
        *slot = Some(cmd);
        if overwrote {
            PushResult::AcceptedOverwrote
        } else {
            PushResult::Accepted
        }
    }

    /// Removes and returns the command for `tick`, if one arrived.
    pub fn take(&mut self, tick: u32) -> Option<InputCmd> {
        let slot = &mut self.slots[Self::idx(tick)];
        match slot {
            Some(c) if c.tick == tick => slot.take(),
            _ => None,
        }
    }

    /// Number of buffered commands (including stale ones not yet overwritten).
    pub fn len(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        self.slots = [None; CAPACITY];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(tick: u32, seq: u16) -> InputCmd {
        InputCmd {
            seq,
            tick,
            move_x: 0,
            move_z: 0,
            yaw: 0,
            pitch: 0,
            buttons: 0,
            weapon_slot: 0,
            aux: 0,
            view_tick: 0,
        }
    }

    #[test]
    fn out_of_order_arrival_is_taken_in_tick_order() {
        let now = 100;
        for perm in [
            [0, 1, 2, 3, 4],
            [4, 3, 2, 1, 0],
            [2, 4, 0, 3, 1],
            [1, 0, 4, 2, 3],
        ] {
            let mut b = InputBuffer::new();
            for i in perm {
                assert_eq!(b.push(c(100 + i, i as u16), now), PushResult::Accepted);
            }
            for t in 100..105 {
                let got = b.take(t).expect("present");
                assert_eq!(got.tick, t);
                assert_eq!(got.seq as u32, t - 100);
            }
            assert!(b.is_empty());
        }
    }

    #[test]
    fn random_order_five_commands() {
        // Deterministic xorshift shuffle, 200 permutations.
        let mut x = 0x9E3779B97F4A7C15u64;
        for _ in 0..200 {
            let mut order = [0u32, 1, 2, 3, 4];
            for i in (1..5).rev() {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                order.swap(i, (x % (i as u64 + 1)) as usize);
            }
            let mut b = InputBuffer::new();
            for i in order {
                assert!(b.push(c(50 + i, i as u16), 50).is_accepted());
            }
            for t in 50..55 {
                assert_eq!(b.take(t).unwrap().tick, t);
            }
        }
    }

    #[test]
    fn duplicate_tick_is_refused_and_first_wins() {
        let mut b = InputBuffer::new();
        assert_eq!(b.push(c(10, 1), 10), PushResult::Accepted);
        assert_eq!(b.push(c(10, 1), 10), PushResult::Duplicate);
        assert_eq!(b.push(c(10, 2), 10), PushResult::Duplicate);
        assert_eq!(b.len(), 1);
        assert_eq!(b.take(10).unwrap().seq, 1);
        assert!(b.take(10).is_none(), "processed once");
    }

    #[test]
    fn window_edges() {
        let mut b = InputBuffer::new();
        let now = 100;
        assert_eq!(b.push(c(97, 1), now), PushResult::TooOld);
        assert_eq!(b.push(c(98, 2), now), PushResult::Accepted);
        assert_eq!(b.push(c(110, 3), now), PushResult::Accepted);
        assert_eq!(b.push(c(111, 4), now), PushResult::TooFar);
        assert_eq!(b.push(c(u32::MAX, 5), now), PushResult::TooFar);
        assert_eq!(b.push(c(0, 6), now), PushResult::TooOld);
    }

    #[test]
    fn window_near_zero_does_not_underflow() {
        let mut b = InputBuffer::new();
        assert_eq!(b.push(c(0, 1), 0), PushResult::Accepted);
        assert_eq!(b.push(c(10, 2), 0), PushResult::Accepted);
        assert_eq!(b.push(c(11, 3), 0), PushResult::TooFar);
        assert_eq!(InputBuffer::check_window(0, 2), Ok(()));
        assert_eq!(InputBuffer::check_window(0, 3), Err(PushResult::TooOld));
    }

    #[test]
    fn late_but_in_window_is_stored_and_never_taken_for_a_later_tick() {
        let mut b = InputBuffer::new();
        assert_eq!(b.push(c(99, 1), 100), PushResult::Accepted);
        assert!(b.take(100).is_none());
        assert_eq!(b.take(99).unwrap().seq, 1);
    }

    #[test]
    fn ring_wrap_newer_replaces_older_unconsumed() {
        let mut b = InputBuffer::new();
        let now = 100;
        assert_eq!(b.push(c(102, 1), now), PushResult::Accepted);
        // 110 maps to the same slot as 102 (110 % 8 == 102 % 8).
        assert_eq!(b.push(c(110, 2), now), PushResult::AcceptedOverwrote);
        assert!(b.take(102).is_none(), "old one is gone");
        assert_eq!(b.take(110).unwrap().seq, 2);
    }

    #[test]
    fn ring_wrap_older_never_replaces_newer() {
        let mut b = InputBuffer::new();
        let now = 100;
        assert_eq!(b.push(c(110, 2), now), PushResult::Accepted);
        assert_eq!(b.push(c(102, 1), now), PushResult::TooOld);
        assert_eq!(b.take(110).unwrap().seq, 2);
    }

    #[test]
    fn replacing_a_consumed_stale_entry_is_not_an_overwrite() {
        let mut b = InputBuffer::new();
        assert_eq!(b.push(c(100, 1), 100), PushResult::Accepted);
        // Not taken, but sim moved on: tick 100 < now.
        assert_eq!(b.push(c(108, 2), 105), PushResult::Accepted);
        assert_eq!(b.take(108).unwrap().seq, 2);
    }

    #[test]
    fn full_ring_of_eight_consecutive_ticks() {
        let mut b = InputBuffer::new();
        for i in 0..8 {
            assert_eq!(b.push(c(100 + i, i as u16), 100), PushResult::Accepted);
        }
        assert_eq!(b.len(), 8);
        for i in 0..8 {
            assert_eq!(b.take(100 + i).unwrap().seq as u32, i);
        }
    }

    #[test]
    fn take_of_missing_or_mismatched_tick_is_none() {
        let mut b = InputBuffer::new();
        b.push(c(100, 1), 100);
        assert!(b.take(101).is_none());
        assert!(b.take(108).is_none(), "same slot, other tick");
        assert!(b.take(92).is_none());
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn classify_matches_push() {
        let mut b = InputBuffer::new();
        b.push(c(105, 1), 100);
        assert_eq!(b.classify(105, 100), Err(PushResult::Duplicate));
        assert_eq!(b.classify(106, 100), Ok(()));
        assert_eq!(b.classify(200, 100), Err(PushResult::TooFar));
        assert_eq!(b.classify(10, 100), Err(PushResult::TooOld));
        assert_eq!(b.classify(106, 100), Ok(()));
        assert!(b.push(c(106, 2), 100).is_accepted());
    }

    #[test]
    fn push_result_reasons() {
        assert_eq!(PushResult::Accepted.reject(), None);
        assert_eq!(
            PushResult::AcceptedOverwrote.reject(),
            Some(Reject::RingOverwrite)
        );
        assert_eq!(PushResult::TooFar.reject(), Some(Reject::TooFar));
        assert_eq!(
            PushResult::Invalid(Reject::ReservedBits).reject(),
            Some(Reject::ReservedBits)
        );
    }

    #[test]
    fn clear_empties() {
        let mut b = InputBuffer::new();
        b.push(c(100, 1), 100);
        b.clear();
        assert!(b.is_empty());
    }
}
