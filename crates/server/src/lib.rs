//! Frontline match server (library half, so tests and benches can drive the sim loop).

pub mod clock;
pub mod config;
pub mod input_buffer;
pub mod logging;
pub mod metrics;
pub mod net;
pub mod session;
pub mod shutdown;
pub mod sim_bridge;
pub mod sim_thread;
pub mod validate;
pub mod world_loader;

/// Build-time facts printed at startup.
pub const GIT_SHA: &str = env!("FRONTLINE_GIT_SHA");
