//! Grenades and smoke (M1-015, GAMEDESIGN 6.9): cooking, flight with bounce, fuse, blast damage
//! and smoke clouds. `grenade` is the pure flight / fuse maths, `explosion` the blast curve and
//! victim selection, `smoke` the cloud state machine and the line-of-sight test. The ECS parts
//! (components, resources, the `throwables` system) live in `system`.

pub mod explosion;
pub mod grenade;
pub mod smoke;
pub mod system;

pub use explosion::{explosion_damage, Occlusion};
pub use smoke::{smoke_blocks_los, SmokeCloud, SmokeParams};
pub use system::{Gadgets, Smokes, ThrowableTable, Throwables};

/// `weapon` index reported in kill events for explosions (not an index of `GameData::weapons`).
pub const EXPLOSIVE_WEAPON_IDX: u16 = 0xFE;
