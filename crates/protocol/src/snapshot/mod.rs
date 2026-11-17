//! Snapshot encoding and baseline deltas (ARCHITECTURE §6.5, §6.9.1, §8.2; task M0-006).
//!
//! # Packet layout (bit-packed, MSB first, zero padded to a byte)
//!
//! ```text
//! header   : see [`header`]   (type 4 | flags 4 | tick 32/16 | ack_input_seq 16 | slack 4 | baseline 5)
//! self     : see [`self_state`] (only if FLAG_HAS_SELF)
//! events   : n:6, then n x (event_id:16 | type:5 | payload)      see [`events`]
//! entities : n:7, then n records (delta / keyframe / remove)     see [`entity`]
//! trailer  : CRC-16/CCITT over all preceding bytes, big endian (only if FLAG_CRC)
//! ```
//!
//! Limits: packet <= 1200 B ([`MAX_PACKET`]); default encoder budget 1150 B
//! ([`DEFAULT_BUDGET`]); <= 127 entity records, <= 63 events. All field widths are documented in
//! the submodules (these are the "refinements" recorded in PLAN_ISSUES PI-002).
//!
//! # Baselines
//!
//! The client acknowledges the newest snapshot tick it decoded. The server keeps the states of
//! every unacknowledged snapshot (<= 64) and promotes an acknowledged one to the baseline.
//! A packet refers to the baseline by `tick - baseline_tick` (1..=32). Entities that were in
//! the baseline snapshot are sent as deltas, everything else (new entities, or all of them
//! when the ack lags more than 32 ticks or nothing was ever acked) as keyframes.

pub mod baseline;
pub mod crc;
pub mod decoder;
pub mod encoder;
pub mod entity;
pub mod events;
pub mod header;
pub mod self_state;

pub use baseline::{ClientBaselines, BASELINE_RING};
pub use decoder::{DecodedSnapshot, EntitiesSoA, SnapshotDecoder};
pub use encoder::{ClientId, SnapshotBuilder, SnapshotEncoder, Tick};
pub use entity::{EntityKind, EntityState, QPos, QVel, VehicleExtra};
pub use events::{Event, EventEntry};
pub use self_state::SelfState;

/// Maximum snapshot datagram size in bytes.
pub const MAX_PACKET: usize = 1200;
/// Default encoder budget in bytes (leaves room for transport framing).
pub const DEFAULT_BUDGET: usize = 1150;
/// Maximum entity records per packet (7-bit count).
pub const MAX_ENTITIES: usize = 127;
