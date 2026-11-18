//! Shared state and router assembly.

use std::sync::Arc;

use axum::Router;
use ed25519_dalek::SigningKey;

use crate::clock::Clock;
use crate::config::Config;
use crate::ratelimit::SlidingLimiter;
use crate::registry::Registry;

pub struct AppState {
    pub cfg: Config,
    pub clock: Clock,
    pub registry: Registry,
    pub ip_limiter: SlidingLimiter<String>,
    pub account_limiter: SlidingLimiter<String>,
    pub ticket_key: SigningKey,
}

pub type Shared = Arc<AppState>;

impl AppState {
    pub fn new(cfg: Config, clock: Clock, ticket_key: SigningKey) -> Shared {
        Arc::new(AppState {
            ip_limiter: SlidingLimiter::new(cfg.ip_limit_per_min),
            account_limiter: SlidingLimiter::new(cfg.account_limit_per_min),
            cfg,
            clock,
            registry: Registry::default(),
            ticket_key,
        })
    }
}

pub fn router(state: Shared) -> Router {
    crate::routes::router(state)
}
