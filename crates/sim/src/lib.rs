//! Frontline deterministic simulation core (ARCHITECTURE 4.3.2, 8.1).
//!
//! One code base runs natively on the server and as WASM for client prediction. Forbidden
//! here: tokio, I/O, wall-clock time, `thread_rng`, std `HashMap`/`HashSet`, rayon/threads,
//! wasm-bindgen, platform `sin`/`cos` (use `libm`). `bevy_ecs` runs with the single-threaded
//! executor only. See [`components`] for the sorted-iteration rule.

pub mod ballistics;
pub mod collision;
pub mod combat;
pub mod components;
pub mod data;
pub mod hash;
pub mod health;
pub mod ids;
pub mod medical;
pub mod movement;
#[cfg(feature = "client-predict")]
pub mod predict;
pub mod projectile;
pub mod respawn;
pub mod rng;
pub mod rules;
pub mod schedule;
pub mod step;
pub mod suppression;
pub mod throwables;
pub mod view;
pub mod weapon;
pub mod world;

pub use collision::{CollisionChunk, CollisionData, CollisionError};
pub use components::Team;
pub use data::GameData;
pub use glam::Vec3;
pub use health::{Health, LifeState, LifeStateView};
pub use ids::{EntityId, PlayerId, Tick};
pub use movement::{step_player, step_player_ex, MoveCtx, MoveParams, PlayerAuthState, Stance};
pub use respawn::{respawn_time, RespawnPolicy, RespawnQueue};
pub use rules::{MatchPhaseKind, ObjectiveSpec, VictoryReason};
pub use step::{RuleChange, SimEvent, StepOutput};
pub use view::WorldView;
pub use weapon::{HitZone, WeaponState};
pub use world::{GameMode, MapId, SimConfig, SpawnError, SpawnReq, World};
