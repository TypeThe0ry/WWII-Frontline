//! Frontline bots (M0-026): a basic soldier bot.
//!
//! A bot is a client without a network connection: it reads the world only through the
//! read-only [`WorldView`](frontline_sim::WorldView), and its only output is an
//! [`InputCmd`](frontline_protocol::msg::input::InputCmd) that the host (`crates/server`,
//! `bot_host.rs`) pushes through the same validation path as a human's input. This crate never
//! touches a mutable `World` and does not depend on `server` (ARCHITECTURE 4.5).
//!
//! * [`grid_nav`]: 4 m walkability grid + deterministic 8-neighbour A\* (no navmesh before M1).
//! * [`perception`]: view cone, range, line of sight, distance-dependent recognition.
//! * [`aim`]: aim point, ballistic drop, gaussian aim error, turn-rate limiting.
//! * [`soldier`]: the Patrol / Engage / Reload / Dead state machine ([`SoldierBrain`]).
//!
//! All decisions are deterministic: randomness comes from `frontline_sim::rng::Rng::for_`
//! keyed by the match seed, the tick and the bot; nothing reads a clock or a system RNG.

pub mod aim;
pub mod brain;
pub mod ctx;
pub mod difficulty;
pub mod grid_nav;
pub mod perception;
pub mod soldier;

pub use brain::{BotBrain, BrainStats};
pub use ctx::BotCtx;
pub use difficulty::{Difficulty, Tuning, TICK_HZ};
pub use grid_nav::{NavGrid, PathFinder};
pub use soldier::{BotMap, SoldierBrain};
