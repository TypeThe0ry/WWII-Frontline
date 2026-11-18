//! 1 s heartbeat task. The gateway delists a server after 5 s of silence; failures never end
//! the match.

use std::sync::Arc;
use std::time::Duration;

use super::client::{GatewayClient, Heartbeat};
use crate::metrics::Metrics;

pub const INTERVAL: Duration = Duration::from_secs(1);
/// Consecutive failures before a warning is logged (once per streak).
pub const WARN_AFTER: u32 = 3;

pub type Source = Arc<dyn Fn() -> Heartbeat + Send + Sync>;

/// Runs until `stop` flips to `true`.
pub async fn run(
    client: GatewayClient,
    match_id: String,
    source: Source,
    metrics: Arc<Metrics>,
    mut stop: tokio::sync::watch::Receiver<bool>,
    interval: Duration,
) {
    let mut tick = tokio::time::interval(interval);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut failures = 0u32;
    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = stop.changed() => return,
        }
        match client.heartbeat(&match_id, &source()).await {
            Ok(()) => {
                if failures >= WARN_AFTER {
                    tracing::info!(
                        event = "gateway_heartbeat_recovered",
                        "gateway reachable again"
                    );
                }
                failures = 0;
                metrics
                    .gateway_heartbeat_total
                    .with_label_values(&["ok"])
                    .inc();
            }
            Err(e) => {
                failures += 1;
                metrics
                    .gateway_heartbeat_total
                    .with_label_values(&["error"])
                    .inc();
                if failures == WARN_AFTER {
                    tracing::warn!(
                        event = "gateway_heartbeat_failing",
                        error = %e,
                        "heartbeats to the gateway are failing; the match continues"
                    );
                }
            }
        }
    }
}
