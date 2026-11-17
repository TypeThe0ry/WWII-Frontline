//! `frontline-server`: one process = one match (ARCHITECTURE §5.4).

use std::process::ExitCode;
use std::time::Duration;

use frontline_server::config::Config;
use frontline_server::shutdown::{wait_for_signal, StopFlag};
use frontline_server::sim_thread::{self, SimThreadConfig};
use frontline_server::{logging, metrics, world_loader, GIT_SHA};
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
    // Terrain + spawn points. A missing map is a startup error that names the regeneration
    // command; only the hidden --empty-world (tests) skips it.
    // --dev-echo never feeds inputs to the sim, so terrain is pointless there as well.
    let map_data = if cfg.empty_world || cfg.dev_echo {
        tracing::warn!(
            event = "empty_world",
            "--empty-world/--dev-echo: no terrain loaded (test scaffolding; players cannot move)"
        );
        None
    } else {
        let dir = world_loader::locate_data_dir(cfg.data_dir.as_deref(), &cfg.map)
            .map_err(|e| e.to_string())?;
        let m = world_loader::load_map(&dir, &cfg.map).map_err(|e| e.to_string())?;
        tracing::info!(
            event = "map_loaded",
            map = %cfg.map,
            chunks = m.chunk_count as u64,
            dir = %dir.display(),
            "terrain collision loaded"
        );
        Some(std::sync::Arc::new(m))
    };

    let game_data = frontline_sim::GameData::embedded();
    let inbound = frontline_server::net::conn::InboundQueue::new();
    let sessions = frontline_server::net::conn::Sessions::new(metrics.clone());
    if cfg.bots > 0 && map_data.is_none() {
        tracing::warn!(
            event = "bots_without_terrain",
            bots = cfg.bots as u64,
            "no terrain loaded: bots will stand still"
        );
    }
    let handle = sim_thread::spawn_with_bots(
        SimThreadConfig {
            match_seed: seed,
            map: MapId(0),
            mode,
            pin_core: cfg.pin_sim_core,
            map_data,
            // --dev-echo hands the inbound queue to the echo scaffold instead.
            net: (!cfg.dev_echo).then(|| frontline_server::sim_bridge::NetLink {
                inbound: inbound.rx.clone(),
                sessions: sessions.clone(),
            }),
        },
        cfg.bots,
        metrics.clone(),
        stop.clone(),
    )
    .map_err(|e| format!("cannot start sim thread: {e}"))?;

    let net_ctx = std::sync::Arc::new(frontline_server::net::conn::NetCtx {
        metrics: metrics.clone(),
        sessions: sessions.clone(),
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
            net_ctx.clone(),
        ))
        .map_err(|e| format!("cannot bind ws port {}: {e}", cfg.ws_port))?;
    let wt = if cfg.wt_enabled() {
        let identity = rt.block_on(load_identity(&cfg))?;
        let hash = frontline_server::net::wt::cert_hash(&identity);
        let l = rt
            .block_on(frontline_server::net::wt::WtListener::bind(
                std::net::SocketAddr::from(([0, 0, 0, 0], cfg.wt_port)),
                identity,
                net_ctx.clone(),
            ))
            .map_err(|e| format!("cannot bind wt port {} (udp): {e}", cfg.wt_port))?;
        tracing::info!(
            event = "wt_listening",
            port = l.local_addr().port() as u64,
            dev_cert = cfg.dev_cert,
            cert_sha256_hex = frontline_server::net::wt::hash_hex(&hash),
            cert_sha256_base64 = frontline_server::net::wt::hash_base64(&hash),
            "serving WebTransport"
        );
        Some(l)
    } else {
        tracing::info!(
            event = "wt_disabled",
            "no --cert/--key/--dev-cert: WebTransport not served (WebSocket only)"
        );
        None
    };
    let echo = cfg.dev_echo.then(|| {
        tracing::warn!(
            event = "dev_echo",
            "C1 -> C2 echo enabled (test scaffolding)"
        );
        frontline_server::net::echo::DevEcho::spawn(net_ctx.sessions.clone(), inbound.rx.clone())
    });
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
    if let Some(wt) = wt {
        rt.block_on(wt.shutdown());
    }
    if let Some(e) = echo {
        e.stop();
    }
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

/// `--cert/--key` PEM files, or a fresh development certificate for `--dev-cert`.
async fn load_identity(cfg: &Config) -> Result<wtransport::Identity, String> {
    if let (Some(c), Some(k)) = (&cfg.cert, &cfg.key) {
        wtransport::Identity::load_pemfiles(c, k)
            .await
            .map_err(|e| format!("cannot load --cert/--key: {e}"))
    } else {
        frontline_server::net::wt::dev_identity(frontline_server::net::wt::DEV_CERT_DAYS)
            .map_err(|e| format!("cannot generate dev certificate: {e}"))
    }
}
