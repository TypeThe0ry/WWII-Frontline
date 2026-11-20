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
    let _log = match logging::init(cfg.match_id(), &cfg.log_level, cfg.log_rate_limit) {
        Ok(log) => log,
        Err(e) => {
            eprintln!("error: cannot initialise logging: {e}");
            return ExitCode::from(2);
        }
    };
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

    let objectives = map_data.as_ref().map_or(0, |map| {
        map.objectives.len().min(usize::from(u16::MAX)) as u16
    });
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
            map: MapId(u16::from(cfg.map_id())),
            mode,
            pin_core: cfg.pin_sim_core,
            map_data,
            dev_spawn_vehicle: cfg.dev_spawn_vehicle,
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
    if cfg.bot_fill > 0 {
        let _ = handle
            .commands_tx
            .send(sim_thread::Command::SetBotFill(cfg.bot_fill));
    }

    let gate = cfg.gateway_enabled().then(|| {
        frontline_server::gateway_link::TicketGate::new(
            frontline_server::gateway_link::system_now_secs(),
        )
    });
    if gate.is_none() {
        tracing::warn!(
            event = "dev_no_gateway",
            "no GATEWAY_URL: not registering and NOT checking join tickets (development only)"
        );
    }
    let net_ctx = std::sync::Arc::new(frontline_server::net::conn::NetCtx {
        metrics: metrics.clone(),
        sessions: sessions.clone(),
        inbound: inbound.tx.clone(),
        commands: handle.commands_tx.clone(),
        data_hash: game_data.hash,
        match_seed: seed,
        tick_hz: 30,
        map_id: cfg.map_id(),
        mode: cfg.mode_id(),
        server_tick: &logging::CURRENT_TICK,
        started: std::time::Instant::now(),
        handshake_timeout: frontline_server::net::limits::HANDSHAKE_TIMEOUT,
        tickets: gate.clone(),
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

    // Register once everything is listening; a refused registration ends the process.
    let (hb_stop_tx, hb_stop_rx) = tokio::sync::watch::channel(false);
    let link = match &gate {
        Some(g) => Some(rt.block_on(start_gateway_link(
            &cfg,
            g,
            &ws,
            wt.as_ref(),
            &metrics,
            handle.commands_tx.clone(),
            hb_stop_rx,
        ))?),
        None => None,
    };

    let reason = rt.block_on(async {
        match cfg.duration {
            Some(d) => tokio::select! {
                s = wait_for_signal() => s,
                _ = tokio::time::sleep(Duration::from_secs_f64(d)) => "duration",
            },
            None => wait_for_signal().await,
        }
    });
    let _ = hb_stop_tx.send(true);
    tracing::info!(event = "shutdown", reason, "stopping");
    let (humans_at_end, bots_at_end) = gateway_population(&metrics);
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
    if let Some((client, id, report_worker)) = link {
        rt.block_on(async {
            let mut worker = report_worker;
            if tokio::time::timeout(
                frontline_server::gateway_link::client::REQUEST_TIMEOUT,
                &mut worker,
            )
            .await
            .is_err()
            {
                worker.abort();
                tracing::warn!(
                    event = "report_shutdown_deadline",
                    "dropping pending report submissions"
                );
            }
        });
        if let Some(report) = &report {
            let result = frontline_server::gateway_link::result::MatchResult::from_report(
                u16::from(humans_at_end),
                u16::from(bots_at_end),
                objectives,
                report,
            );
            if let Err(e) = rt.block_on(client.submit_result(&id, &result)) {
                match frontline_server::gateway_link::result::spool(&cfg.spool_dir, &id, &result) {
                    Ok(p) => {
                        tracing::warn!(event = "result_spooled", error = %e, path = %p.display(), "result not delivered; kept for a retry")
                    }
                    Err(io) => {
                        tracing::error!(event = "result_lost", error = %e, io = %io, "result not delivered and could not be spooled")
                    }
                }
            }
        }
    }
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

fn parse_hex16(s: &str) -> Option<[u8; 16]> {
    if s.len() != 32 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 16];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

/// Registers with the gateway (3 attempts), arms the ticket gate with the announced key and
/// starts the 1 s heartbeat. Returns the client and the gateway-assigned match id.
async fn start_gateway_link(
    cfg: &Config,
    gate: &frontline_server::gateway_link::SharedGate,
    ws: &frontline_server::net::ws::WsListener,
    wt: Option<&frontline_server::net::wt::WtListener>,
    metrics: &std::sync::Arc<metrics::Metrics>,
    commands: crossbeam_channel::Sender<sim_thread::Command>,
    hb_stop: tokio::sync::watch::Receiver<bool>,
) -> Result<
    (
        frontline_server::gateway_link::client::GatewayClient,
        String,
        tokio::task::JoinHandle<()>,
    ),
    String,
> {
    use frontline_server::gateway_link::{
        admin, client, heartbeat, reports, result, ticket_verify,
    };
    let url = cfg.gateway_url.as_deref().unwrap_or_default();
    let token = cfg.internal_api_token.as_deref().unwrap_or_default();
    let gw = client::GatewayClient::new(url, token)?;
    let pinned = match &cfg.ticket_public_key_file {
        Some(p) => Some(ticket_verify::parse_public_key(
            &std::fs::read_to_string(p)
                .map_err(|e| format!("cannot read TICKET_PUBLIC_KEY_FILE: {e}"))?,
        )?),
        None => None,
    };
    let req = client::RegisterReq {
        name: cfg.server_name.clone(),
        map: cfg.map.clone(),
        mode: cfg.mode.clone(),
        region: cfg.region.clone(),
        tags: Vec::new(),
        version: GIT_SHA.to_string(),
        protocol_version: frontline_protocol::PROTOCOL_VERSION,
        data_hash: frontline_sim::GameData::embedded().hash,
        cap: 16,
        addr: cfg.public_addr.clone(),
        ws_port: ws.local_addr().port(),
        wt_port: wt.map_or(0, |w| w.local_addr().port()),
    };
    let resp = gw
        .register_with_retry(&req, 3, Duration::from_millis(500))
        .await
        .map_err(|e| format!("gateway registration failed after 3 attempts: {e}"))?;
    let key = ticket_verify::parse_public_key(&resp.ticket_public_key)
        .map_err(|e| format!("gateway sent a bad ticket key: {e}"))?;
    if let Some(p) = pinned {
        if p != key {
            return Err(
                "gateway announced a ticket key that differs from TICKET_PUBLIC_KEY_FILE".into(),
            );
        }
    }
    let id = parse_hex16(&resp.match_id).ok_or("gateway sent a malformed match id")?;
    let match_id = frontline_server::gateway_link::lower_hex(&id);
    let (reports_tx, reports_rx) = tokio::sync::mpsc::channel(reports::QUEUE_CAPACITY);
    let (chat_updates, chat_rx) = tokio::sync::watch::channel(std::sync::Arc::default());
    commands
        .try_send(sim_thread::Command::EnableReports {
            reports: reports_tx,
            chat_updates,
        })
        .map_err(|_| "cannot enable reports on the sim thread")?;
    let report_worker = tokio::spawn(reports::run(
        gw.clone(),
        match_id.clone(),
        cfg.spool_dir.clone(),
        reports_rx,
    ));
    tokio::spawn(reports::run_live_chat(
        gw.clone(),
        match_id.clone(),
        chat_rx,
        hb_stop.clone(),
    ));
    gate.arm(ticket_verify::TicketVerifier::new(vec![key]), id);
    tokio::spawn(admin::run(
        gw.clone(),
        match_id.clone(),
        commands,
        hb_stop.clone(),
    ));
    metrics.gateway_registered.set(1);
    tracing::info!(
        event = "gateway_registered",
        gateway_match_id = %match_id,
        key_fp = %resp.ticket_key_fingerprint,
        "registered with the gateway"
    );
    let spool_dir = cfg.spool_dir.clone();
    let replay_client = gw.clone();
    tokio::spawn(async move { result::replay_spool(&spool_dir, &replay_client).await });
    let population = metrics.clone();
    let countdown = cfg.duration.map(Duration::from_secs_f64);
    let started = std::time::Instant::now();
    let source: heartbeat::Source = std::sync::Arc::new(move || {
        let (humans, bots) = gateway_population(&population);
        client::Heartbeat {
            humans,
            bots,
            phase: "playing".into(),
            time_left_s: countdown.map_or(0, |d| {
                d.saturating_sub(started.elapsed()).as_secs_f64().ceil() as u32
            }),
        }
    });
    tokio::spawn(heartbeat::run(
        gw.clone(),
        match_id.clone(),
        source,
        metrics.clone(),
        hb_stop,
        heartbeat::INTERVAL,
    ));
    Ok((gw, match_id, report_worker))
}

/// Use active participants rather than transport connections, which include pending Hello.
fn gateway_population(metrics: &metrics::Metrics) -> (u8, u8) {
    let humans = metrics.players.with_label_values(&["us"]).get()
        + metrics.players.with_label_values(&["ger"]).get();
    let bots = metrics.bots.get();
    (humans.clamp(0, 255) as u8, bots.clamp(0, 255) as u8)
}
