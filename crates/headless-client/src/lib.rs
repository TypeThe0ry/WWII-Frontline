//! Frontline headless client (M0-027): synthetic players over the real transports, for load and
//! integration tests. Depends on `sim` and `protocol` only (never on `server`, ARCHITECTURE §4.5).

#[cfg(feature = "anticheat-test")]
mod anticheat;

pub mod audit;
pub mod cli;
pub mod client;
pub mod clock;
pub mod map;
pub mod predictor;
pub mod report;
pub mod run;
pub mod stats;
pub mod synth_input;
pub mod transport;
mod vehicle;
