//! Frontline gateway (M1-028): server list, guest authentication, join tickets and the internal
//! match-server API. It holds no real-time match state and must not depend on `sim`, `server`
//! or `bots` (ARCHITECTURE §4.3.5, enforced by `tools/ci/check-deps.mjs`).

pub mod app;
pub mod auth;
pub mod clock;
pub mod config;
pub mod db;
mod metrics;
pub mod nick;
pub mod ratelimit;
pub mod registry;
pub mod routes;
