//! Translate resolved seat-button levels into authoritative boarding and exit requests.

use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::vehicle::seats::{SeatError, BOARD_RANGE_M};
use frontline_sim::{EntityId, PlayerId, World};

#[derive(Default)]
pub(crate) struct SeatInput {
    // One bit per valid session slot (0..=127).
    held: u128,
}

impl SeatInput {
    pub(crate) fn forget(&mut self, player: PlayerId) {
        self.held &= !1u128.checked_shl(u32::from(player)).unwrap_or(0);
    }

    pub(crate) fn apply(&mut self, world: &mut World, inputs: &[(PlayerId, InputCmd)]) {
        let previous = std::mem::take(&mut self.held);
        for &(player, cmd) in inputs {
            let Some(bit) = 1u128.checked_shl(u32::from(player)) else {
                continue;
            };
            let held = cmd.buttons & buttons::VEHICLE_SEAT != 0;
            let was_held = previous & bit != 0;
            if held {
                self.held |= bit;
                if world.seat_of(player).is_some() {
                    // Repeated calls preserve the original simulation deadline.
                    let _ = world.seat_exit_begin(player);
                } else if !was_held {
                    board_nearest(world, player);
                }
            } else if was_held {
                world.seat_exit_cancel(player);
            }
        }
    }
}

fn board_nearest(world: &mut World, player: PlayerId) {
    let Some(id) = nearest_vehicle(world, player) else {
        return;
    };
    for seat in 0..3 {
        if world.seat_enter(player, id, seat) != Err(SeatError::Occupied) {
            break;
        }
    }
}

fn nearest_vehicle(world: &World, player: PlayerId) -> Option<EntityId> {
    let team = world.squads().entry(player)?.team;
    let pos = world.player_state(player)?.pos;
    world
        .vehicle_ids()
        .into_iter()
        .filter_map(|id| {
            let vehicle = world.vehicle(id)?;
            if vehicle.team != team {
                return None;
            }
            let distance_sq = vehicle.pos.distance_squared(pos);
            // Match seat_enter: the boarding allowance is measured from the hull.
            let range = BOARD_RANGE_M + vehicle.params.half_extents.z;
            (distance_sq <= range * range).then_some((id, distance_sq))
        })
        .min_by(|(a_id, a_dist), (b_id, b_dist)| a_dist.total_cmp(b_dist).then(a_id.cmp(b_id)))
        .map(|(id, _)| id)
}
