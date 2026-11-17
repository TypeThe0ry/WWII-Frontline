//! `frontline-server`: one process = one match (ARCHITECTURE §5.4).

use std::process::ExitCode;
use std::time::Duration;

use frontline_server::config::Config;
use frontline_server::shutdown::{wait_for_signal, StopFlag};
use frontline_server::sim_thread::{self, SimThreadConfig};
use frontline_server::{logging, metrics, GIT_SHA};
use frontline_sim::{GameMode, MapId};

fn main() -> ExitCode {
    let cfg = match Config::try_parse_validated(std::env::args_os()) {
        Ok(c) => c,
        Err(e) => {
            // clap renders --help/--version through the same path; those are not errors.
            let help = e.starts_with("Usage")
                || e.contains("Print help")
                || e.contains("frontline-server 0");
            eprintln!("{}", e.trim_end());
            return if help {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            };
        }
    };
    if let Err(e) = logging::init(cfg.match_id(), &cfg.log_level, cfg.log_rate_limit) {
        eprintln!("error: cannot initialise logging: {e}");
        return ExitCode::from(2);
    }
    match run(cfg) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!(event = "fatal", error = %e, "server failed");
            ExitCode::FAILURE
        }
    }
}

fn run(cfg: Config) -> Result<(), String> {
    tracing::info!(
        event = "startup",
        git_sha = GIT_SHA,
        protocol_version = frontline_protocol::PROTOCOL_VERSION as u64,
        data_hash = format!("{:016x}", frontline_sim::GameData::embedded().hash),
        map = %cfg.map,
        mode = %cfg.mode,
        bots = cfg.bots as u64,
        "frontline-server starting"
    );
    if cfg.bots > 0 {
        tracing::warn!(
            event = "bots_not_available",
            requested = cfg.bots as u64,
            "bot subsystem arrives with M0-026; running without bots"
        );
    }
    tracing::info!(
        event = "wt_pending",
        wt_port = cfg.wt_port as u64,
        "WebTransport arrives with M0-017; port not bound"
    );

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("net")
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;

    let metrics = metrics::Metrics::new();
    let stop = StopFlag::new();
    let seed = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        cfg.match_id().hash(&mut h);
        h.finish()
    };
    let mode = match cfg.mode.as_str() {
        "frontline" => GameMode::Frontline,
        "assault" => GameMode::Assault,
        "skirmish" => GameMode::Skirmish,
        _ => GameMode::Sandbox,
    };
    let handle = sim_thread::spawn(
        SimThreadConfig {
            match_seed: seed,
            map: MapId(0),
            mode,
            pin_core: cfg.pin_sim_core,
        },
        metrics.clone(),
        stop.clone(),
    )
    .map_err(|e| format!("cannot start sim thread: {e}"))?;

    let game_data = frontline_sim::GameData::embedded();
    let inbound = frontline_server::net::conn::InboundQueue::new();
    let net_ctx = std::sync::Arc::new(frontline_server::net::conn::NetCtx {
        metrics: metrics.clone(),
        sessions: frontline_server::net::conn::Sessions::new(metrics.clone()),
        inbound: inbound.tx.clone(),
        commands: handle.commands_tx.clone(),
        data_hash: game_data.hash,
        match_seed: seed,
        tick_hz: 30,
        map_id: 0,
        mode: cfg.mode_id(),
        server_tick: &logging::CURRENT_TICK,
        started: std::time::Instant::now(),
        handshake_timeout: frontline_server::net::limits::HANDSHAKE_TIMEOUT,
    });
    let ws = rt
        .block_on(frontline_server::net::ws::WsListener::bind(
            ("0.0.0.0", cfg.ws_port),
            net_ctx,
        ))
        .map_err(|e| format!("cannot bind ws port {}: {e}", cfg.ws_port))?;
    tracing::info!(
        event = "ws_listening",
        port = ws.local_addr().port() as u64,
        "serving WebSocket"
    );

    let (sd_tx, sd_rx) = tokio::sync::watch::channel(false);
    let listener = rt
        .block_on(tokio::net::TcpListener::bind(("0.0.0.0", cfg.metrics_port)))
        .map_err(|e| format!("cannot bind metrics port {}: {e}", cfg.metrics_port))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    tracing::info!(
        event = "metrics_listening",
        port = port as u64,
        "serving /metrics"
    );
    let server = rt.spawn(metrics::serve(listener, metrics.clone(), sd_rx));

    let reason = rt.block_on(async {
        match cfg.duration {
            Some(d) => tokio::select! {
                s = wait_for_signal() => s,
                _ = tokio::time::sleep(Duration::from_secs_f64(d)) => "duration",
            },
            None => wait_for_signal().await,
        }
    });
    tracing::info!(event = "shutdown", reason, "stopping");
    stop.stop();
    rt.block_on(ws.shutdown());
    drop(inbound);
    let report = handle.join();
    let _ = sd_tx.send(true);
    let _ = rt.block_on(server);
    rt.shutdown_timeout(Duration::from_millis(50));

    if let (Some(path), Some(report)) = (&cfg.report, &report) {
        let json = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
        std::fs::write(path, json)
            .map_err(|e| format!("cannot write report {}: {e}", path.display()))?;
    }
    match &report {
        Some(r) => tracing::info!(
            event = "summary",
            ticks = r.ticks,
            tick_ms_p99 = r.tick_ms.p99,
            interval_err_ms_p99 = r.tick_interval_error_ms.p99,
            overruns = r.tick_overrun_total,
            "match ended"
        ),
        None => return Err("sim thread panicked".into()),
    }
    Ok(())
}
