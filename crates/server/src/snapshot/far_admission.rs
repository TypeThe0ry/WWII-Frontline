//! Bandwidth admission for the far tiers, after visibility filtering. Retaining an incumbent
//! before an entrant avoids repeated remove/keyframe churn with a stable candidate set.

use super::priority::{Cand, Slot, SlotState};
use crate::net::regulator::BandwidthRegulator;

pub(super) const T2: u8 = 2;
pub(super) const T3: u8 = 3;

pub(super) fn tier(dist: f32) -> u8 {
    if dist > 192.0 {
        T3
    } else if dist > 96.0 {
        T2
    } else {
        0
    }
}

pub(super) fn trim(
    cands: &mut Vec<Cand>,
    tiers: &[u8],
    slots: &[Slot],
    force_full: &mut [bool],
    regulator: &BandwidthRegulator,
) -> usize {
    let t2 = cands.iter().filter(|c| tiers[c.id as usize] == T2).count();
    let t3 = cands.iter().filter(|c| tiers[c.id as usize] == T3).count();
    let shed = regulator.shed_far(t2 + t3);
    let keep_t3 = t3 - regulator.shed_far(t3);
    // Round T3 down first, then give the remaining slots to T2. Neither tier is starved
    // entirely just because the other tier happened to have at least the combined cap.
    let mut left = [t2 + t3 - shed - keep_t3, keep_t3];
    cands.sort_unstable_by_key(|c| (slots[c.id as usize].state != SlotState::InRange, c.id));
    cands.retain(|c| {
        let id = c.id as usize;
        let tier = tiers[id];
        if tier == 0 {
            return true;
        }
        let n = &mut left[(tier - T2) as usize];
        if *n > 0 {
            *n -= 1;
            true
        } else {
            force_full[id] = true;
            false
        }
    });
    shed
}
