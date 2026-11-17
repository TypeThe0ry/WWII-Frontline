//! Frontline deterministic simulation core (ARCHITECTURE 4.3.2, 8.1).
//!
//! One code base runs natively on the server and as WASM for client prediction. Forbidden
//! here: tokio, I/O, wall-clock time, `thread_rng`, std `HashMap`/`HashSet`, rayon/threads,
//! wasm-bindgen, platform `sin`/`cos` (use `libm`). `bevy_ecs` runs with the single-threaded
//! executor only. See [`components`] for the sorted-iteration rule.

pub mod collision;
pub mod components;
pub mod data;
pub mod hash;
pub mod ids;
pub mod movement;
#[cfg(feature = "client-predict")]
pub mod predict;
pub mod rng;
pub mod schedule;
pub mod step;
pub mod view;
pub mod world;

pub use collision::{CollisionChunk, CollisionData, CollisionError};
pub use components::Team;
pub use data::GameData;
pub use glam::Vec3;
pub use ids::{EntityId, PlayerId, Tick};
pub use movement::{step_player, MoveParams, PlayerAuthState, Stance};
pub use step::{RuleChange, SimEvent, StepOutput};
pub use view::WorldView;
pub use world::{GameMode, MapId, SimConfig, SpawnError, SpawnReq, World};
