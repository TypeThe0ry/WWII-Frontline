//! Prometheus metrics (ARCHITECTURE §18.3) and the minimal `/metrics` HTTP responder.
//! The responder is hand-written on tokio TCP (no HTTP framework dependency).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use prometheus::{
    Encoder, Gauge, Histogram, HistogramOpts, IntCounter, IntCounterVec, IntGauge, IntGaugeVec,
    Opts, Registry, TextEncoder,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub const TICK_MS_BUCKETS: &[f64] = &[
    0.5, 1.0, 2.0, 4.0, 8.0, 12.0, 16.0, 20.0, 24.0, 28.0, 33.3, 50.0, 100.0,
];
/// Buckets of `bot_think_ms` / `bot_path_ms` (budget: 1 ms for 10 bots, 3 ms for 60).
pub const BOT_MS_BUCKETS: &[f64] = &[0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 3.0, 5.0, 10.0];
pub const INTERVAL_ERR_MS_BUCKETS: &[f64] =
    &[0.05, 0.1, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0];

/// Label values of `net_rejects_total{reason}`.
pub const REJECT_REASONS: &[&str] = &[
    "version",
    "data_hash",
    "server_full",
    "ip_limit",
    "conn_limit",
    "rate_limit",
    "slow_client",
];

pub struct Metrics {
    pub registry: Registry,
    pub tick_ms: Histogram,
    pub tick_interval_error_ms: Histogram,
    pub tick_overrun_total: IntCounter,
    pub tick_skipped_slots_total: IntCounter,
    pub tick_overrun_consecutive: IntGauge,
    pub tick_overrun_consecutive_max: IntGauge,
    pub input_queue_full_total: IntCounter,
    /// Inputs refused, clamped or dropped by the session layer (M0-018), by reason.
    pub input_rejected_total: IntCounterVec,
    /// Input commands that passed validation and entered a buffer.
    pub input_accepted_total: IntCounter,
    /// Ticks in which a joined player had no input of its own (repeat / neutral was used).
    pub input_missing_ticks_total: IntCounter,
    /// Players whose input stream stalled for >= 9 ticks (one count per stall episode).
    pub session_stall_total: IntCounter,
    /// Connections closed by the server because of 30 consecutive input violations.
    pub session_kicked_total: IntCounter,
    pub players: IntGaugeVec,
    pub bots: IntGauge,
    /// Wall time of one tick's bot step: all `think` calls + input ingestion (ms; M0-026).
    pub bot_think_ms: Histogram,
    /// Time of that step spent in path queries (ms).
    pub bot_path_ms: Histogram,
    pub uptime_seconds: Gauge,
    // ---- net (M0-016) ----
    pub net_connections: IntGauge,
    pub net_rejects_total: IntCounterVec,
    pub net_bytes_in_total: IntCounter,
    pub net_bytes_out_total: IntCounter,
    pub net_frames_malformed_total: IntCounter,
    pub net_handshake_timeout_total: IntCounter,
    pub net_snapshots_dropped_total: IntCounter,
    pub net_c1_inbound_total: IntCounter,
    // ---- WebTransport (M0-017) ----
    /// Open connections per transport (`ws` / `wt`).
    pub net_transport_connections: IntGaugeVec,
    /// Outbound C2/C3 packets dropped because they exceed the connection's `max_datagram_size`.
    pub net_datagram_oversize_total: IntCounter,
    /// Datagrams received (C1 candidates, valid or not).
    pub net_datagrams_in_total: IntCounter,
    /// Datagrams sent (C2/C3).
    pub net_datagrams_out_total: IntCounter,
    // ---- snapshots / AOI (M0-019) ----
    /// Size of each C2 snapshot payload (bytes).
    pub snapshot_bytes: Histogram,
    /// Payload / budget of each snapshot.
    pub snapshot_budget_utilization: Histogram,
    /// Entities that did not fit a packet (budget or record limit).
    pub aoi_starved_total: IntCounter,
    /// Enemies sent outside the R3 rules (must stay 0; audit).
    pub aoi_enemy_leak_total: IntCounter,
    /// Fire commands whose view tick lay more than 8 ticks back (rewind clamped; M1-025).
    pub lagcomp_clamped_total: IntCounter,
    /// Wall time of steps 6+7 (AOI + encode) per tick (ms).
    pub aoi_encode_ms: Histogram,
    pub snapshots_sent_total: IntCounter,
    pub snapshot_keyframe_records_total: IntCounter,
    pub snapshot_delta_records_total: IntCounter,
    /// Ticks spent in the reduced snapshot rate (3 consecutive tick overruns).
    pub snapshot_degraded_ticks_total: IntCounter,
    /// Per-client downstream rate over a sliding 1 s window (kbps), sampled once per second per
    /// client; `kind` = `payload` (snapshot bytes) or `wire` (payload + 58 B per packet).
    pub net_bytes_out_per_client_kbps: prometheus::HistogramVec,
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
            input_rejected_total: IntCounterVec::new(
                Opts::new(
                    "input_rejected_total",
                    "Inputs rejected, clamped or dropped by validation",
                ),
                &["reason"],
            )
            .expect("input rejected"),
            input_accepted_total: counter(
                "input_accepted_total",
                "Input commands accepted into a player's buffer",
            ),
            input_missing_ticks_total: counter(
                "input_missing_ticks_total",
                "Player-ticks resolved without an input of their own",
            ),
            session_stall_total: counter(
                "session_stall_total",
                "Input stalls (>= 9 ticks without input)",
            ),
            session_kicked_total: counter(
                "session_kicked_total",
                "Connections closed after 30 consecutive input violations",
            ),
            players: IntGaugeVec::new(Opts::new("server_players", "Connected players"), &["team"])
                .expect("players"),
            bots: gauge("server_bots", "Active bots"),
            bot_think_ms: hist(
                "bot_think_ms",
                "Bot step wall time per tick: think + input ingestion (ms)",
                BOT_MS_BUCKETS,
            ),
            bot_path_ms: hist(
                "bot_path_ms",
                "Path-query time per tick inside the bot step (ms)",
                BOT_MS_BUCKETS,
            ),
            uptime_seconds: Gauge::new("server_uptime_seconds", "Process uptime").expect("uptime"),
            net_connections: gauge("net_connections", "Open client connections"),
            net_rejects_total: IntCounterVec::new(
                Opts::new("net_rejects_total", "Connections/handshakes rejected"),
                &["reason"],
            )
            .expect("rejects"),
            net_bytes_in_total: counter("net_bytes_in_total", "Bytes received from clients"),
            net_bytes_out_total: counter("net_bytes_out_total", "Bytes sent to clients"),
            net_frames_malformed_total: counter(
                "net_frames_malformed_total",
                "Inbound frames that were malformed, oversize or illegal (connection closed)",
            ),
            net_handshake_timeout_total: counter(
                "net_handshake_timeout_total",
                "Connections closed for not completing the handshake in time",
            ),
            net_snapshots_dropped_total: counter(
                "net_snapshots_dropped_total",
                "Unreliable (C2/C3) packets dropped in favour of newer ones",
            ),
            net_c1_inbound_total: counter(
                "net_c1_inbound_total",
                "Valid C1 input bundles received",
            ),
            net_transport_connections: IntGaugeVec::new(
                Opts::new(
                    "net_transport_connections",
                    "Open client connections by transport",
                ),
                &["transport"],
            )
            .expect("transport connections"),
            net_datagram_oversize_total: counter(
                "net_datagram_oversize_total",
                "Outbound datagrams dropped for exceeding max_datagram_size",
            ),
            net_datagrams_in_total: counter("net_datagrams_in_total", "Datagrams received"),
            net_datagrams_out_total: counter("net_datagrams_out_total", "Datagrams sent"),
            snapshot_bytes: hist(
                "snapshot_bytes",
                "Size of each snapshot payload (bytes)",
                &[
                    50.0, 100.0, 150.0, 200.0, 300.0, 400.0, 600.0, 800.0, 1000.0, 1150.0, 1200.0,
                ],
            ),
            snapshot_budget_utilization: hist(
                "snapshot_budget_utilization",
                "Snapshot payload divided by the packet budget",
                &[0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0],
            ),
            aoi_starved_total: counter(
                "aoi_starved_total",
                "Candidate entities that did not fit into a snapshot packet",
            ),
            aoi_enemy_leak_total: counter(
                "aoi_enemy_leak_total",
                "Enemy records sent outside the R3 view-cone / 96 m rule (audit; must be 0)",
            ),
            lagcomp_clamped_total: counter(
                "lagcomp_clamped_total",
                "Fire commands whose lag-compensation rewind was clamped to 8 ticks",
            ),
            aoi_encode_ms: hist(
                "aoi_encode_ms",
                "AOI selection + snapshot encoding per tick (ms)",
                &[0.1, 0.25, 0.5, 1.0, 1.5, 2.0, 2.6, 4.0, 8.0, 16.0],
            ),
            snapshots_sent_total: counter("snapshots_sent_total", "Snapshot packets produced"),
            snapshot_keyframe_records_total: counter(
                "snapshot_keyframe_records_total",
                "Entity records sent as keyframes",
            ),
            snapshot_delta_records_total: counter(
                "snapshot_delta_records_total",
                "Entity records sent as deltas",
            ),
            snapshot_degraded_ticks_total: counter(
                "snapshot_degraded_ticks_total",
                "Ticks in which the snapshot rate was reduced to 15 Hz",
            ),
            net_bytes_out_per_client_kbps: prometheus::HistogramVec::new(
                HistogramOpts::new(
                    "net_bytes_out_per_client_kbps",
                    "Per-client downstream rate over 1 s windows (kbps)",
                )
                .buckets(vec![
                    8.0, 16.0, 32.0, 54.0, 64.0, 96.0, 128.0, 192.0, 256.0, 400.0, 512.0,
                ]),
                &["kind"],
            )
            .expect("per-client kbps"),
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
        for c in [
            Box::new(m.input_rejected_total.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(m.input_accepted_total.clone()),
            Box::new(m.input_missing_ticks_total.clone()),
            Box::new(m.session_stall_total.clone()),
            Box::new(m.session_kicked_total.clone()),
        ] {
            r.register(c).unwrap();
        }
        for reason in crate::validate::Reject::LABELS {
            m.input_rejected_total.with_label_values(&[reason]);
        }
        r.register(Box::new(m.players.clone())).unwrap();
        r.register(Box::new(m.bots.clone())).unwrap();
        r.register(Box::new(m.bot_think_ms.clone())).unwrap();
        r.register(Box::new(m.bot_path_ms.clone())).unwrap();
        r.register(Box::new(m.uptime_seconds.clone())).unwrap();
        for c in [
            Box::new(m.net_connections.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(m.net_rejects_total.clone()),
            Box::new(m.net_bytes_in_total.clone()),
            Box::new(m.net_bytes_out_total.clone()),
            Box::new(m.net_frames_malformed_total.clone()),
            Box::new(m.net_handshake_timeout_total.clone()),
            Box::new(m.net_snapshots_dropped_total.clone()),
            Box::new(m.net_c1_inbound_total.clone()),
            Box::new(m.net_transport_connections.clone()),
            Box::new(m.net_datagram_oversize_total.clone()),
            Box::new(m.net_datagrams_in_total.clone()),
            Box::new(m.net_datagrams_out_total.clone()),
        ] {
            r.register(c).unwrap();
        }
        for c in [
            Box::new(m.snapshot_bytes.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(m.snapshot_budget_utilization.clone()),
            Box::new(m.aoi_starved_total.clone()),
            Box::new(m.aoi_enemy_leak_total.clone()),
            Box::new(m.lagcomp_clamped_total.clone()),
            Box::new(m.aoi_encode_ms.clone()),
            Box::new(m.snapshots_sent_total.clone()),
            Box::new(m.snapshot_keyframe_records_total.clone()),
            Box::new(m.snapshot_delta_records_total.clone()),
            Box::new(m.snapshot_degraded_ticks_total.clone()),
            Box::new(m.net_bytes_out_per_client_kbps.clone()),
        ] {
            r.register(c).unwrap();
        }
        for kind in ["ws", "wt"] {
            m.net_transport_connections.with_label_values(&[kind]);
        }
        for reason in REJECT_REASONS {
            m.net_rejects_total.with_label_values(&[reason]);
        }
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
            "net_connections",
            "net_rejects_total{reason=\"version\"}",
            "net_frames_malformed_total",
            "net_handshake_timeout_total",
            "net_snapshots_dropped_total",
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
