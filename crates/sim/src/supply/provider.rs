//! `SupplyProvider` over the ground crates: nearest first, ties by ascending crate id.

use glam::Vec3;

use super::{dist_xz, SupplyWorld};
use crate::supply_iface::SupplyProvider;

impl SupplyProvider for SupplyWorld {
    fn available(&self, pos: Vec3, r: f32) -> u32 {
        self.crates
            .values()
            .filter(|c| c.carried_by.is_none() && dist_xz(c.pos, pos) <= r)
            .map(|c| c.supplies)
            .sum()
    }

    fn consume(&mut self, pos: Vec3, r: f32, amount: u32) -> bool {
        if self.available(pos, r) < amount {
            return false;
        }
        let mut near: Vec<(f32, u32)> = self
            .crates
            .values()
            .filter(|c| c.carried_by.is_none() && c.supplies > 0 && dist_xz(c.pos, pos) <= r)
            .map(|c| (dist_xz(c.pos, pos), c.id))
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let mut need = amount;
        for (_, id) in near {
            if need == 0 {
                break;
            }
            let c = self.crates.get_mut(&id).unwrap();
            let take = need.min(c.supplies);
            c.supplies -= take;
            need -= take;
            if c.supplies == 0 {
                self.crates.remove(&id);
            }
        }
        true
    }
}
