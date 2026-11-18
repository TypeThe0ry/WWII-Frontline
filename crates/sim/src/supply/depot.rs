//! HQ Depot and point supply stores: timed crate generation.

use glam::Vec3;

use crate::components::Team;

/// HQ Supply Depot: one crate per 45 s while the stock is below 8.
#[derive(Debug, Clone, PartialEq)]
pub struct Depot {
    pub team: Team,
    pub pos: Vec3,
    pub stock: u32,
    pub timer: u32,
}

/// Strongpoint supply store: one crate per 90 s (cap 4), only while its objective is held;
/// changing hands empties it.
#[derive(Debug, Clone, PartialEq)]
pub struct SupplyPoint {
    pub objective: u16,
    pub pos: Vec3,
    pub owner: Option<Team>,
    pub stock: u32,
    pub timer: u32,
}

/// Advances a generation timer; `true` when a crate was produced.
pub fn tick_store(stock: &mut u32, timer: &mut u32, interval: u32, max: u32) -> bool {
    if *stock >= max {
        *timer = 0;
        return false;
    }
    *timer += 1;
    if *timer >= interval {
        *timer = 0;
        *stock += 1;
        return true;
    }
    false
}
