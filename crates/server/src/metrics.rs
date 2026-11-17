//! Prometheus metrics (ARCHITECTURE §18.3) and the minimal `/metrics` HTTP responder.
//! The responder is hand-written on tokio TCP (no HTTP framework dependency).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use prometheus::{
    Encoder, Gauge, Histogram, HistogramOpts, IntCounter, IntGauge, IntGaugeVec, Opts, Registry,
    TextEncoder,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub const TICK_MS_BUCKETS: &[f64] = &[
    0.5, 1.0, 2.0, 4.0, 8.0, 12.0, 16.0, 20.0, 24.0, 28.0, 33.3, 50.0, 100.0,
];
pub const INTERVAL_ERR_MS_BUCKETS: &[f64] =
    &[0.05, 0.1, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0];

pub struct Metrics {
    pub registry: Registry,
    pub tick_ms: Histogram,
    pub tick_interval_error_ms: Histogram,
    pub tick_overrun_total: IntCounter,
    pub tick_skipped_slots_total: IntCounter,
    pub tick_overrun_consecutive: IntGauge,
    pub tick_overrun_consecutive_max: IntGauge,
    pub input_queue_full_total: IntCounter,
    pub players: IntGaugeVec,
    pub bots: IntGauge,
    pub uptime_seconds: Gauge,
    /// Latest published tick (for the report / smoke tests).
    pub ticks_run: AtomicU64,
    started: Instant,
}

impl Metrics {
    pub fn new() -> Arc<Metrics> {
        let registry = Registry::new();
        let hist = |name: &str, help: &str, buckets: &[f64]| {
            Histogram::with_opts(HistogramOpts::new(name, help).buckets(buckets.to_vec()))
                .expect("histogram opts")
        };
        let counter = |name: &str, help: &str| IntCounter::new(name, help).expect("counter");
        let gauge = |name: &str, help: &str| IntGauge::new(name, help).expect("gauge");
        let m = Metrics {
            tick_ms: hist(
                "server_tick_ms",
                "Wall time of one server tick's work (ms)",
                TICK_MS_BUCKETS,
            ),
            tick_interval_error_ms: hist(
                "server_tick_interval_error_ms",
                "Actual wake time minus scheduled deadline (ms)",
                INTERVAL_ERR_MS_BUCKETS,
            ),
            tick_overrun_total: counter(
                "server_tick_overrun_total",
                "Ticks whose work exceeded the tick period",
            ),
            tick_skipped_slots_total: counter(
                "server_tick_skipped_slots_total",
                "Schedule slots skipped (never caught up) after overruns",
            ),
            tick_overrun_consecutive: gauge(
                "server_tick_overrun_consecutive",
                "Current run of consecutive overrunning ticks",
            ),
            tick_overrun_consecutive_max: gauge(
                "server_tick_overrun_consecutive_max",
                "Longest run of consecutive overrunning ticks this match",
            ),
            input_queue_full_total: counter(
                "server_input_queue_full_total",
                "Inputs dropped because the input channel was full",
            ),
            players: IntGaugeVec::new(Opts::new("server_players", "Connected players"), &["team"])
                .expect("players"),
            bots: gauge("server_bots", "Active bots"),
            uptime_seconds: Gauge::new("server_uptime_seconds", "Process uptime").expect("uptime"),
            ticks_run: AtomicU64::new(0),
            started: Instant::now(),
            registry,
        };
        let r = &m.registry;
        r.register(Box::new(m.tick_ms.clone())).unwrap();
        r.register(Box::new(m.tick_interval_error_ms.clone()))
            .unwrap();
        r.register(Box::new(m.tick_overrun_total.clone())).unwrap();
        r.register(Box::new(m.tick_skipped_slots_total.clone()))
            .unwrap();
        r.register(Box::new(m.tick_overrun_consecutive.clone()))
            .unwrap();
        r.register(Box::new(m.tick_overrun_consecutive_max.clone()))
            .unwrap();
        r.register(Box::new(m.input_queue_full_total.clone()))
            .unwrap();
        r.register(Box::new(m.players.clone())).unwrap();
        r.register(Box::new(m.bots.clone())).unwrap();
        r.register(Box::new(m.uptime_seconds.clone())).unwrap();
        // Make both team series visible from the start.
        m.players.with_label_values(&["us"]).set(0);
        m.players.with_label_values(&["ger"]).set(0);
        Arc::new(m)
    }

    /// Prometheus text exposition.
    pub fn render(&self) -> String {
        self.uptime_seconds
            .set(self.started.elapsed().as_secs_f64());
        let mut buf = Vec::new();
        TextEncoder::new()
            .encode(&self.registry.gather(), &mut buf)
            .expect("encode");
        String::from_utf8(buf).unwrap_or_default()
    }

    pub fn ticks_run(&self) -> u64 {
        self.ticks_run.load(Ordering::Relaxed)
    }
}

/// Serves `GET /metrics` until `shutdown` resolves. Returns the bound port via `bound`.
pub async fn serve(
    listener: TcpListener,
    metrics: Arc<Metrics>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            _ = shutdown.changed() => break,
            acc = listener.accept() => {
                let Ok((mut sock, _)) = acc else { continue };
                let m = metrics.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let n = match tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        sock.read(&mut buf),
                    ).await {
                        Ok(Ok(n)) => n,
                        _ => return,
                    };
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let first = req.lines().next().unwrap_or("");
                    let (status, ctype, body) = if first.starts_with("GET /metrics") {
                        ("200 OK", "text/plain; version=0.0.4", m.render())
                    } else {
                        ("404 Not Found", "text/plain", "not found\n".to_string())
                    };
                    let resp = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = sock.write_all(resp.as_bytes()).await;
                    let _ = sock.shutdown().await;
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_required_series_with_enough_buckets() {
        let m = Metrics::new();
        m.tick_ms.observe(1.0);
        m.tick_interval_error_ms.observe(0.1);
        let text = m.render();
        for name in [
            "server_tick_ms_bucket",
            "server_tick_interval_error_ms_bucket",
            "server_tick_overrun_total",
            "server_tick_overrun_consecutive",
            "server_tick_overrun_consecutive_max",
            "server_input_queue_full_total",
            "server_players{team=\"us\"}",
            "server_bots",
            "server_uptime_seconds",
        ] {
            assert!(text.contains(name), "missing {name}");
        }
        let buckets = text
            .lines()
            .filter(|l| l.starts_with("server_tick_ms_bucket"))
            .count();
        assert!(buckets >= 10, "{buckets} buckets");
    }
}
