//! Runs all clients of one invocation and builds the report.

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::cli::{spread_ip, Args};
use crate::client::{run_client, Ctx, Series};
use crate::map;
use crate::report::{aggregate, process_cpu_seconds, Environment, Report, REPORT_VERSION};
use crate::transport::{parse_cert_hash, Kind, Target};

/// Builds the shared context (loads terrain for prediction when available).
pub fn build_ctx(args: &Args) -> Result<Ctx, String> {
    let cert_hash = args.cert_hash.as_deref().map(parse_cert_hash).transpose()?;
    let loaded = match map::locate_data_dir(args.data_dir.as_deref(), &args.map)
        .and_then(|d| map::load(&d, &args.map))
    {
        Ok(m) => Some(Arc::new(m)),
        Err(e) if args.drive_vehicle || args.deploy_hq => {
            return Err(format!(
                "driving and HQ deployment probes require terrain: {e}"
            ))
        }
        Err(e) => {
            eprintln!("warning: no terrain, prediction runs on an empty world (players will not move): {e}");
            None
        }
    };
    let waypoints = loaded
        .as_ref()
        .map(|m| m.patrol_points.clone())
        .unwrap_or_default();
    Ok(Ctx {
        target: Target {
            kind: Kind::from(args.transport),
            addr: args.url.clone(),
            cert_hash,
            bind_ip: None,
            connect_timeout: Duration::from_secs(5),
        },
        seed: args.seed,
        duration: Duration::from_secs_f64(args.duration),
        warmup_s: args.warmup,
        data_hash: frontline_sim::GameData::embedded().hash,
        map: loaded,
        empty_collision: frontline_sim::CollisionData::empty(),
        waypoints,
        deploy_hq: args.deploy_hq,
        drive_vehicle: args.drive_vehicle,
        anticheat_control: args.anticheat_control,
        #[cfg(feature = "anticheat-test")]
        anticheat_mix: args.anticheat_mix,
        trace: std::env::var("HEADLESS_TRACE").is_ok_and(|v| v == "1"),
    })
}

/// Starts `args.count` clients (staggered by `--ramp-ms`), waits for all, returns the report.
pub async fn run(args: &Args, ctx: Ctx) -> Report {
    let spread = !args.no_bind_spread && args.host_is_loopback();
    let cpu0 = process_cpu_seconds();
    let wall0 = Instant::now();
    let ctx = Arc::new(ctx);
    let mut tasks = Vec::new();
    for i in 0..args.count {
        let mut c = Ctx {
            target: ctx.target.clone(),
            seed: ctx.seed,
            duration: ctx.duration,
            warmup_s: ctx.warmup_s,
            data_hash: ctx.data_hash,
            map: ctx.map.clone(),
            empty_collision: frontline_sim::CollisionData::empty(),
            waypoints: ctx.waypoints.clone(),
            deploy_hq: ctx.deploy_hq,
            drive_vehicle: ctx.drive_vehicle && i == 0,
            anticheat_control: ctx.anticheat_control,
            #[cfg(feature = "anticheat-test")]
            anticheat_mix: ctx.anticheat_mix,
            trace: ctx.trace,
        };
        if spread {
            c.target.bind_ip = Some(spread_ip(i));
        }
        let c = Arc::new(c);
        tasks.push(tokio::spawn(run_client(c, i)));
        if args.ramp_ms > 0 && i + 1 < args.count {
            tokio::time::sleep(Duration::from_millis(args.ramp_ms)).await;
        }
    }
    let mut per_client = Vec::new();
    let (mut down, mut up, mut rtt): (Vec<f64>, Vec<f64>, Vec<f64>) = (vec![], vec![], vec![]);
    for (i, t) in tasks.into_iter().enumerate() {
        match t.await {
            Ok((r, s)) => {
                let Series {
                    down: d,
                    up: u,
                    rtt: t,
                } = s;
                down.extend(d);
                up.extend(u);
                rtt.extend(t);
                per_client.push(r);
            }
            Err(e) => {
                let mut r = crate::report::ClientReport {
                    id: i as u32,
                    ..Default::default()
                };
                r.join_error = Some(format!("client task panicked: {e}"));
                per_client.push(r);
            }
        }
    }
    let wall = wall0.elapsed().as_secs_f64();
    let cpu = match (cpu0, process_cpu_seconds()) {
        (Some(a), Some(b)) if wall > 0.0 => (b - a) / wall * 100.0,
        _ => 0.0,
    };
    let agg = aggregate(&per_client, &down, &up, &rtt);
    Report {
        report_version: REPORT_VERSION,
        transport: ctx.target.kind.name().into(),
        url: args.url.clone(),
        count: args.count,
        duration_s: args.duration,
        warmup_s: args.warmup,
        seed: args.seed,
        predict: true,
        env: Environment {
            cpus: std::thread::available_parallelism().map_or(1, |n| n.get()),
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            build: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
            .into(),
            client_pid: std::process::id(),
            client_cpu_pct_of_one_core: cpu,
        },
        per_client,
        aggregate: agg,
        server: None,
    }
}
