use frontline_bots::scheduler::BotsMetrics;
use prometheus::{Histogram, HistogramOpts, IntCounter, Registry};

use super::BOT_MS_BUCKETS;

pub(crate) struct BotScaleMetrics {
    tick: Histogram,
    path: Histogram,
    perception: Histogram,
    tactical: Histogram,
    deferred: IntCounter,
    timeout: IntCounter,
    stuck: IntCounter,
}

impl BotScaleMetrics {
    pub(super) fn new(registry: &Registry) -> Self {
        let hist = |name, help, buckets: &[f64]| {
            Histogram::with_opts(HistogramOpts::new(name, help).buckets(buckets.to_vec())).unwrap()
        };
        let counter = |name, help| IntCounter::new(name, help).unwrap();
        let metrics = Self {
            tick: hist(
                "bots_tick_ms",
                "Whole bot host tick including input ingestion (ms)",
                BOT_MS_BUCKETS,
            ),
            path: hist(
                "bots_path_ms",
                "Shared path queue and search per tick (ms)",
                BOT_MS_BUCKETS,
            ),
            perception: hist(
                "bots_perception_ms",
                "Shared perception build and soldier scans (ms)",
                BOT_MS_BUCKETS,
            ),
            tactical: hist(
                "bots_tactical_ms",
                "Soldier decision work after perception (ms)",
                BOT_MS_BUCKETS,
            ),
            deferred: counter(
                "bots_deferred_total",
                "Soldier decisions deferred to a subsequent tick",
            ),
            timeout: counter(
                "bots_think_timeout_total",
                "Timed-out soldier decisions rolled back",
            ),
            stuck: counter("bots_stuck_total", "Soldier stuck recoveries (B-21)"),
        };
        for h in [
            &metrics.tick,
            &metrics.path,
            &metrics.perception,
            &metrics.tactical,
        ] {
            registry.register(Box::new(h.clone())).unwrap();
        }
        for c in [&metrics.deferred, &metrics.timeout, &metrics.stuck] {
            registry.register(Box::new(c.clone())).unwrap();
        }
        metrics
    }

    pub(crate) fn observe(&self, sample: BotsMetrics, host_ns: u64) {
        self.tick.observe(host_ns as f64 / 1e6);
        self.path.observe(sample.path_ns as f64 / 1e6);
        self.perception.observe(sample.perception_ns as f64 / 1e6);
        self.tactical.observe(sample.tactical_ns as f64 / 1e6);
        self.deferred.inc_by(sample.deferred);
        self.timeout.inc_by(sample.timeouts);
        self.stuck.inc_by(sample.stuck);
    }
}
