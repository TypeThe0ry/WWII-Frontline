//! Team-scoped construction payments against the M1-020 ground crate ledger.

use glam::Vec3;

use crate::components::Team;
use crate::supply::{SupplyCrate, SupplyWorld};

const BUILD_SUPPLY_RANGE_M: f32 = 15.0;

fn sources(
    supplies: &SupplyWorld,
    team: Team,
    pos: Vec3,
) -> impl Iterator<Item = (f32, &SupplyCrate)> {
    supplies.crates.values().filter_map(move |c| {
        let distance_sq = (c.pos.x - pos.x).powi(2) + (c.pos.z - pos.z).powi(2);
        (c.team == team
            && c.carried_by.is_none()
            && c.supplies > 0
            && c.hp > 0.0
            && distance_sq <= BUILD_SUPPLY_RANGE_M * BUILD_SUPPLY_RANGE_M)
            .then_some((distance_sq, c))
    })
}

pub(super) fn can_pay(supplies: &SupplyWorld, team: Team, pos: Vec3, amount: u32) -> bool {
    let mut remaining = amount;
    for (_, c) in sources(supplies, team, pos) {
        remaining = remaining.saturating_sub(c.supplies);
        if remaining == 0 {
            break;
        }
    }
    remaining == 0
}

/// Called immediately after `can_pay` succeeds with exclusive access to the same ledger.
pub(super) fn pay(supplies: &mut SupplyWorld, team: Team, pos: Vec3, mut amount: u32) {
    if amount == 0 {
        return;
    }
    let mut nearest: Vec<_> = sources(supplies, team, pos)
        .map(|(d, c)| (d, c.id))
        .collect();
    nearest.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    for (_, id) in nearest {
        let c = supplies.crates.get_mut(&id).unwrap();
        let taken = amount.min(c.supplies);
        c.supplies -= taken;
        amount -= taken;
        if c.supplies == 0 {
            supplies.crates.remove(&id);
        }
        if amount == 0 {
            break;
        }
    }
    debug_assert_eq!(amount, 0, "construction was preflighted on this ledger");
}
