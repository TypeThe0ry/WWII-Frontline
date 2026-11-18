//! Ground crate cap: the 41st crate of a team removes its oldest.

use super::SupplyWorld;
use crate::components::Team;

/// Removes the earliest-placed ground crates of `team` beyond `max`; returns their ids.
pub fn evict(sw: &mut SupplyWorld, team: Team, max: usize) -> Vec<u32> {
    let mut ground: Vec<(u32, u32)> = sw
        .crates
        .values()
        .filter(|c| c.team == team && c.carried_by.is_none())
        .map(|c| (c.seq, c.id))
        .collect();
    ground.sort_unstable();
    let mut out = Vec::new();
    while ground.len() > max {
        let (_, id) = ground.remove(0);
        sw.crates.remove(&id);
        out.push(id);
    }
    out
}
