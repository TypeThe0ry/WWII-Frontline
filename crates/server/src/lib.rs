//! Frontline match server (library half, so tests and benches can drive the sim loop).

pub mod anticheat;
pub mod bot_host;
pub mod c0;
mod c4;
pub mod clock;
pub mod config;
pub mod gateway_link;
pub mod input_buffer;
pub mod logging;
pub mod metrics;
pub mod net;
mod report;
mod seat_input;
pub mod session;
pub mod shutdown;
pub mod sim_bridge;
pub mod sim_thread;
pub mod snapshot;
pub mod text;
pub mod validate;
pub mod world_loader;

/// Build-time facts printed at startup.
pub const GIT_SHA: &str = env!("FRONTLINE_GIT_SHA");
