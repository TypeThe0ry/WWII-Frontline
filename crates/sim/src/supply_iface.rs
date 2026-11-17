//! Supply source seen by the spawn rules (M1-019). M1-020 provides the real implementation
//! (crates, vehicles, depots); until then the world holds [`NoSupply`] and tests bring a stub.

use glam::Vec3;

pub trait SupplyProvider: Send {
    /// Supplies available within `r` metres of `pos`.
    fn available(&self, pos: Vec3, r: f32) -> u32;
    /// Takes `amount` Supplies from sources within `r` metres of `pos`; `false` (and nothing
    /// taken) when there are not enough.
    fn consume(&mut self, pos: Vec3, r: f32, amount: u32) -> bool;
}

/// No supplies anywhere (the default until M1-020).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoSupply;

impl SupplyProvider for NoSupply {
    fn available(&self, _: Vec3, _: f32) -> u32 {
        0
    }
    fn consume(&mut self, _: Vec3, _: f32, amount: u32) -> bool {
        amount == 0
    }
}
