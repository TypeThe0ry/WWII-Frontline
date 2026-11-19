//! Report JSON (M0-027 behaviour 6). `server` is left `null` here; `scripts/summarize-loadtest.mjs`
//! merges the server's own report and `/metrics` scrapes into it.

use serde::{Deserialize, Serialize};

use crate::stats::Dist;

pub const REPORT_VERSION: u32 = 1;

/// Position-correction accounting of one client (the reconciler of ARCHITECTURE §6.4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Corrections {
    /// Rollbacks after the initial placement (error above 5 cm).
    pub total: u64,
    /// Of those, error >= 1 m (the browser client would snap instead of smoothing).
    pub snaps: u64,
    /// Of `total`: while dead, within 3 s after dying or within 2 s after respawning (teleports).
    pub around_death: u64,
    /// `pos_corr` received but within 5 cm: prediction kept.
    pub tolerance_kept: u64,
    /// First placement from the first `pos_corr` (not a correction).
    pub initial_syncs: u64,
    /// Acks the ring did not know (future or too old).
    pub unknown_acks: u64,
    /// Largest rollback error in metres.
    pub max_err_m: f64,
    /// Largest `pos_corr` error that stayed within the 5 cm tolerance.
    pub tolerance_max_err_m: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VehicleCorrections {
    pub comparisons: u64,
    /// Finite, same-tick errors >= 5 cm, including those >= 1 m (`snaps`).
    pub total: u64,
    pub snaps: u64,
    /// Re-anchors without a predicted pose at this tick; excluded from the error distribution.
    pub missing_ticks: u64,
    /// P50/P99/max of corrected position errors, or null when there were no finite corrections.
    pub error_m: Option<Dist>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VehicleReport {
    pub entries: u64,
    pub exits: u64,
    pub driver_vehicle_slot: Option<u8>,
    pub driver_snapshots: u64,
    pub driver_input_ticks: u64,
    pub last_vehicle_slot: u8,
    pub last_seat: u8,
    /// Largest horizontal displacement from the first authoritative pose of a driving stint.
    pub max_displacement_m: f32,
    pub corrections: VehicleCorrections,
}

impl VehicleReport {
    pub fn completed(&self) -> bool {
        self.entries > 0
            && self.exits > 0
            && self.last_vehicle_slot == 0
            && self.last_seat == 0
            && self.corrections.comparisons > 0
            && self.driver_input_ticks > 0
            && self.max_displacement_m >= 1.0
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientReport {
    /// Index of the client in this process.
    pub id: u32,
    /// Server-assigned player id (`None` when the join failed).
    pub player_id: Option<u8>,
    pub team: Option<u8>,
    pub join_error: Option<String>,
    /// Source address the client bound to.
    pub bind_ip: String,
    /// Seconds actually spent connected.
    pub connected_s: f64,
    /// Downlink (C2) / uplink (C1), line estimate (+58 B per packet) over per-second windows.
    pub down_kbps: Dist,
    pub up_kbps: Dist,
    /// Same windows, application payload only.
    pub down_payload_kbps: Dist,
    pub up_payload_kbps: Dist,
    pub rtt_ms: Dist,
    pub decode_errors: u64,
    pub missing_baseline: u64,
    pub disconnects: u64,
    pub disconnect_reason: Option<String>,
    pub snapshots: u64,
    pub snapshot_bytes_mean: f64,
    pub records: u64,
    pub keyframe_records: u64,
    /// `keyframe_records / records`.
    pub keyframe_ratio: f64,
    pub events: u64,
    pub reordered_snapshots: u64,
    pub bundles_sent: u64,
    pub cmds_generated: u64,
    pub corrections: Corrections,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vehicle: Option<VehicleReport>,
    /// Deaths / respawns seen in `self_state.health`.
    pub deaths: u64,
    pub respawns: u64,
    pub shots_fired: u64,
    /// Client-side R3 audit (`audit.rs`): enemy records checked / beyond 105.6 m / outside the rule.
    #[serde(default)]
    pub aoi_records_checked: u64,
    #[serde(default)]
    pub aoi_beyond_close: u64,
    #[serde(default)]
    pub aoi_leaks: u64,
    #[serde(default)]
    pub aoi_worst_leak_m: f32,
    /// Last `self_state` health (0..=255) and ammo, for sanity checks.
    pub final_health: Option<u8>,
    /// Final predicted position, metres.
    pub final_pos: Option<[f32; 3]>,
    /// Distance walked by the predicted position, metres.
    pub distance_m: f64,
    /// FNV-1a of the generated command sequence (tick-independent): equal for equal `--seed`.
    pub input_hash: String,
}

/// Totals over all clients. Distributions pool every client's per-second windows.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Aggregate {
    pub clients_joined: u32,
    pub clients_failed: u32,
    pub down_kbps: Dist,
    pub up_kbps: Dist,
    /// Worst single client P50 (the acceptance compares each client's P50).
    pub down_kbps_p50_worst_client: f64,
    pub up_kbps_p50_worst_client: f64,
    pub rtt_ms: Dist,
    pub decode_errors: u64,
    pub missing_baseline: u64,
    pub disconnects: u64,
    pub snapshots: u64,
    pub keyframe_ratio: f64,
    pub corrections: Corrections,
    pub deaths: u64,
    pub respawns: u64,
    pub shots_fired: u64,
    #[serde(default)]
    pub aoi_records_checked: u64,
    #[serde(default)]
    pub aoi_beyond_close: u64,
    #[serde(default)]
    pub aoi_leaks: u64,
    #[serde(default)]
    pub aoi_worst_leak_m: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Environment {
    pub cpus: usize,
    pub os: String,
    pub arch: String,
    pub build: String,
    pub client_pid: u32,
    /// Process CPU time / wall time, in percent of one core (headless itself, all threads).
    pub client_cpu_pct_of_one_core: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Report {
    pub report_version: u32,
    pub transport: String,
    pub url: String,
    pub count: u32,
    pub duration_s: f64,
    pub warmup_s: u32,
    pub seed: u64,
    pub predict: bool,
    pub env: Environment,
    pub per_client: Vec<ClientReport>,
    pub aggregate: Aggregate,
    /// Merged in by `scripts/summarize-loadtest.mjs`.
    pub server: Option<serde_json::Value>,
}

/// Builds the aggregate from per-client reports plus their pooled per-second series.
pub fn aggregate(
    clients: &[ClientReport],
    down_series: &[f64],
    up_series: &[f64],
    rtt_series: &[f64],
) -> Aggregate {
    let mut a = Aggregate {
        down_kbps: Dist::of(down_series),
        up_kbps: Dist::of(up_series),
        rtt_ms: Dist::of(rtt_series),
        ..Aggregate::default()
    };
    let (mut kf, mut rec) = (0u64, 0u64);
    for c in clients {
        if c.join_error.is_some() {
            a.clients_failed += 1;
        } else {
            a.clients_joined += 1;
        }
        a.down_kbps_p50_worst_client = a.down_kbps_p50_worst_client.max(c.down_kbps.p50);
        a.up_kbps_p50_worst_client = a.up_kbps_p50_worst_client.max(c.up_kbps.p50);
        a.decode_errors += c.decode_errors;
        a.missing_baseline += c.missing_baseline;
        a.disconnects += c.disconnects;
        a.snapshots += c.snapshots;
        kf += c.keyframe_records;
        rec += c.records;
        a.deaths += c.deaths;
        a.respawns += c.respawns;
        a.shots_fired += c.shots_fired;
        a.aoi_records_checked += c.aoi_records_checked;
        a.aoi_beyond_close += c.aoi_beyond_close;
        a.aoi_leaks += c.aoi_leaks;
        a.aoi_worst_leak_m = a.aoi_worst_leak_m.max(c.aoi_worst_leak_m);
        let k = &c.corrections;
        let t = &mut a.corrections;
        t.total += k.total;
        t.snaps += k.snaps;
        t.around_death += k.around_death;
        t.tolerance_kept += k.tolerance_kept;
        t.initial_syncs += k.initial_syncs;
        t.unknown_acks += k.unknown_acks;
        t.max_err_m = t.max_err_m.max(k.max_err_m);
        t.tolerance_max_err_m = t.tolerance_max_err_m.max(k.tolerance_max_err_m);
    }
    a.keyframe_ratio = if rec == 0 {
        0.0
    } else {
        kf as f64 / rec as f64
    };
    a
}

/// Process CPU seconds (user + system) from `/proc/self/stat`; `None` off Linux.
pub fn process_cpu_seconds() -> Option<f64> {
    let s = std::fs::read_to_string("/proc/self/stat").ok()?;
    // Fields after the parenthesised command name: state is field 3; utime/stime are 14/15.
    let rest = &s[s.rfind(')')? + 2..];
    let f: Vec<&str> = rest.split_whitespace().collect();
    let utime: f64 = f.get(11)?.parse().ok()?;
    let stime: f64 = f.get(12)?.parse().ok()?;
    Some((utime + stime) / 100.0) // USER_HZ is 100 on Linux
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Field names and nesting of the JSON consumers (summarize-loadtest.mjs, STATUS) rely on.
    #[test]
    fn report_schema() {
        let c = ClientReport {
            id: 3,
            down_kbps: Dist {
                p50: 20.0,
                p99: 30.0,
                max: 40.0,
            },
            ..ClientReport::default()
        };
        let r = Report {
            report_version: REPORT_VERSION,
            transport: "wt".into(),
            count: 1,
            per_client: vec![c.clone()],
            aggregate: aggregate(&[c], &[20.0, 30.0], &[5.0], &[1.0]),
            ..Report::default()
        };
        let v: serde_json::Value = serde_json::to_value(&r).unwrap();
        for k in [
            "transport",
            "count",
            "duration_s",
            "per_client",
            "server",
            "aggregate",
            "env",
            "report_version",
        ] {
            assert!(v.get(k).is_some(), "missing top-level {k}");
        }
        assert!(v["server"].is_null());
        let pc = &v["per_client"][0];
        for k in [
            "id",
            "down_kbps",
            "up_kbps",
            "rtt_ms",
            "decode_errors",
            "missing_baseline",
            "disconnects",
            "corrections",
            "input_hash",
        ] {
            assert!(pc.get(k).is_some(), "missing per_client.{k}");
        }
        for d in ["down_kbps", "up_kbps", "rtt_ms"] {
            for q in ["p50", "p99", "max"] {
                assert!(pc[d][q].is_number(), "{d}.{q}");
            }
        }
        // Round trip.
        let back: Report = serde_json::from_value(v).unwrap();
        assert_eq!(back.per_client[0].id, 3);
        assert_eq!(back.aggregate.clients_joined, 1);
        assert_eq!(back.aggregate.down_kbps.max, 30.0);
    }

    #[test]
    fn aggregate_sums_and_counts_failures() {
        let a = ClientReport {
            decode_errors: 2,
            disconnects: 1,
            down_kbps: Dist {
                p50: 10.0,
                ..Dist::default()
            },
            ..ClientReport::default()
        };
        let b = ClientReport {
            join_error: Some("reject".into()),
            decode_errors: 1,
            down_kbps: Dist {
                p50: 30.0,
                ..Dist::default()
            },
            ..ClientReport::default()
        };
        let g = aggregate(&[a, b], &[], &[], &[]);
        assert_eq!(g.decode_errors, 3);
        assert_eq!(g.disconnects, 1);
        assert_eq!(g.clients_failed, 1);
        assert_eq!(g.clients_joined, 1);
        assert_eq!(g.down_kbps_p50_worst_client, 30.0);
    }
}
