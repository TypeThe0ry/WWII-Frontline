//! Shared state and router assembly.

use std::sync::Arc;

use axum::Router;
use ed25519_dalek::SigningKey;

use crate::clock::Clock;
use crate::config::Config;
use crate::db::Database;
use crate::metrics::Metrics;
use crate::ratelimit::SlidingLimiter;

pub struct AppState {
    pub cfg: Config,
    pub clock: Clock,
    pub db: Database,
    pub(crate) metrics: Metrics,
    pub ip_limiter: SlidingLimiter<String>,
    pub account_limiter: SlidingLimiter<String>,
    pub ticket_key: SigningKey,
}

pub type Shared = Arc<AppState>;

impl AppState {
    pub async fn new(
        cfg: Config,
        clock: Clock,
        ticket_key: SigningKey,
    ) -> Result<Shared, sqlx::Error> {
        let db = Database::open(&cfg.database_url, ticket_key.verifying_key().to_bytes()).await?;
        Ok(Arc::new(AppState {
            ip_limiter: SlidingLimiter::new(cfg.ip_limit_per_min),
            account_limiter: SlidingLimiter::new(cfg.account_limit_per_min),
            cfg,
            clock,
            db,
            metrics: Metrics::default(),
            ticket_key,
        }))
    }
}

pub fn router(state: Shared) -> Router {
    crate::routes::router(state)
}
