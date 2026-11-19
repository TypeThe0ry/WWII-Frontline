//! `headless-client` binary (M0-027).

use std::process::ExitCode;

use clap::Parser;
use frontline_headless::cli::Args;
use frontline_headless::map;
use frontline_headless::run::{build_ctx, run};
use frontline_headless::synth_input::kinematic_hash;

fn main() -> ExitCode {
    let args = Args::parse();
    if let Err(e) = args.validate() {
        eprintln!("error: {e}");
        return ExitCode::from(2);
    }
    if let Some(ticks) = args.hash_only {
        let wps = map::locate_data_dir(args.data_dir.as_deref(), &args.map)
            .and_then(|d| map::load(&d, &args.map))
            .map(|m| m.patrol_points)
            .unwrap_or_default();
        for i in 0..args.count {
            println!(
                "client {i}: {:016x}",
                kinematic_hash(args.seed, i, &wps, ticks)
            );
        }
        return ExitCode::SUCCESS;
    }
    let workers = args
        .workers
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(2, |n| n.get().min(4)));
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: tokio runtime: {e}");
            return ExitCode::from(2);
        }
    };
    let ctx = match build_ctx(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let report = rt.block_on(run(&args, ctx));
    let a = &report.aggregate;
    eprintln!(
        "{} clients ({} joined, {} failed) over {}: down P50 {:.1} / P99 {:.1} kbps (worst client P50 {:.1}), up P50 {:.1} kbps, \
         decode_errors {}, missing_baseline {}, disconnects {}, corrections {} (snaps {}, around death {}), deaths {}, respawns {}, \
         client CPU {:.0}% of one core",
        report.count, a.clients_joined, a.clients_failed, report.transport, a.down_kbps.p50, a.down_kbps.p99,
        a.down_kbps_p50_worst_client, a.up_kbps.p50, a.decode_errors, a.missing_baseline, a.disconnects,
        a.corrections.total, a.corrections.snaps, a.corrections.around_death, a.deaths, a.respawns,
        report.env.client_cpu_pct_of_one_core
    );
    for c in &report.per_client {
        if let Some(e) = &c.join_error {
            eprintln!("  client {} FAILED: {e}", c.id);
        } else if c.disconnects > 0 {
            eprintln!(
                "  client {} disconnected: {}",
                c.id,
                c.disconnect_reason.as_deref().unwrap_or("?")
            );
        }
    }
    if let Some(path) = &args.report {
        match serde_json::to_string_pretty(&report) {
            Ok(j) => {
                if let Err(e) = std::fs::write(path, j) {
                    eprintln!("error: cannot write {}: {e}", path.display());
                    return ExitCode::from(2);
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        }
    }
    let drive_completed = !args.drive_vehicle
        || report
            .per_client
            .first()
            .and_then(|c| c.vehicle.as_ref())
            .is_some_and(|v| v.completed());
    if !drive_completed {
        eprintln!(
            "  client 0 did not complete the drive scenario (board, move at least 1 m, reconcile, exit)"
        );
    }
    let disconnects_ok = a.disconnects == 0;
    #[cfg(feature = "anticheat-test")]
    let disconnects_ok = if args.anticheat_mix {
        report.per_client.iter().all(|c| match c.id {
            3 => {
                c.disconnects == 1
                    && c.disconnect_reason.as_deref() == Some("server Disconnect, reason 2")
            }
            4 => {
                c.disconnects == 1
                    && c.disconnect_reason.as_deref() == Some("server Disconnect, reason 1")
            }
            _ => c.disconnects == 0,
        })
    } else {
        disconnects_ok
    };
    if a.clients_failed > 0 || !disconnects_ok || a.decode_errors > 0 || !drive_completed {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
