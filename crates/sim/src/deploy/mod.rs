//! Deployment: squad -> role -> loadout -> spawn point, all validated here (M1-021b;
//! GAMEDESIGN 2.5, ARCHITECTURE 7.3). Network handling is M1-026.
pub mod validate;

pub use validate::{DeployDone, DeployError, DeployOk, DeployReq};

/// `DeployReq::spawn_point` value meaning "my team's HQ" (Garrison ids start at 0).
pub const SPAWN_HQ: u16 = 0xFFFF;
