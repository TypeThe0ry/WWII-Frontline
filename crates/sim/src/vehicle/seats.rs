//! Seats (GD 14.1): enter, switch (instant when slow, 2 s when moving), exit (0.8 s hold,
//! damage above 20 km/h). M1 allows any crewman of the team aboard (no squad claim yet).

use bevy_ecs::prelude::*;

use crate::ids::PlayerId;

pub const KPH: f32 = 1.0 / 3.6;
/// Switch instantly below this speed (5 km/h), else it takes [`SWITCH_S`].
pub const SWITCH_FREE_MPS: f32 = 5.0 * KPH;
pub const SWITCH_S: f32 = 2.0;
pub const EXIT_HOLD_S: f32 = 0.8;
/// Leaving faster than this hurts (20 km/h).
pub const EXIT_HURT_MPS: f32 = 20.0 * KPH;
pub const EXIT_DAMAGE_HP: f32 = 30.0;
/// How close to the hull a player must be to board.
pub const BOARD_RANGE_M: f32 = 6.0;

/// Which vehicle seat a player sits in (`None` = on foot).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InVehicle(pub Option<(u16, u8)>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occupant {
    pub player: PlayerId,
    /// `(target seat, completes at tick)`.
    pub switch: Option<(u8, u32)>,
    pub exit_at: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SeatError {
    NoSuchVehicle,
    NoSuchSeat,
    NotOnRoster,
    /// Only Crewmen board vehicles in M1.
    NotCrewman,
    WrongTeam,
    NotAlive,
    AlreadyAboard,
    NotAboard,
    TooFar,
    Occupied,
    /// An exit is already running.
    Exiting,
}
