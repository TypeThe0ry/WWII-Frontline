//! AOI v0 and snapshot sending (task M0-019; ARCHITECTURE §6.5.4, §6.9, §6.10).
//!
//! * [`aoi_grid`]: 64 m grid.
//! * [`priority`]: rate / importance / R3 candidate rules and the accumulator order.
//! * [`baseline_book`]: per-client ack, baseline, `pred_hash` and bandwidth bookkeeping.
//! * [`events_out`]: C2 event routing and 3-packet redundancy.
//! * [`self_state`]: the receiving player's own state.
//! * [`scheduler`]: the per-tick pass that ties them together.
//!
//! R3 (enemy information) is implemented from the start: an enemy is only a candidate within
//! 96 m, or within 400 m and inside the view cone widened by 30 degrees; enemies that leave
//! are removed from the client's view with a repeated `entity_remove`. (The task card
//! postponed R3 to M1-024; the coordinator ruled it applies now.)

pub mod aoi_grid;
pub mod baseline_book;
pub mod events_out;
mod occlusion;
pub mod priority;
pub mod scheduler;
pub mod self_state;
pub mod team_channel;
pub(crate) mod world_state;

pub use scheduler::{
    ClientIn, LinkInfo, Outgoing, SchedStats, SnapshotScheduler, DEGRADE_HOLD_TICKS, PACKET_BUDGET,
};
mod far_admission;
