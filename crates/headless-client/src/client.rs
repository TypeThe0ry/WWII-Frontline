//! One synthetic player: handshake, 30 Hz input, snapshot decoding, prediction + reconciliation,
//! statistics (M0-027 behaviours 2-4, 7).

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use arrayvec::ArrayVec;
use frontline_protocol::codec::CodecError;
use frontline_protocol::msg::c0::{
    decode_c0_server, encode_c0_client, ClientMsgC0, Disconnect, Hello, Ping, ServerMsgC0, Welcome,
};
use frontline_protocol::msg::input::{encode_input_bundle, InputBundle, InputCmd};
use frontline_protocol::snapshot::events::Event;
use frontline_protocol::snapshot::SnapshotDecoder;
use frontline_protocol::{Channel, PROTOCOL_VERSION};

use crate::clock::Clock;
use crate::map::MapData;
use crate::predictor::{new_sim, Predictor};
use crate::report::ClientReport;
use crate::stats::{Dist, Meter};
use crate::synth_input::{hash_cmd, Pose, SynthInput};
use crate::transport::{self, Incoming, Link, Target};
use crate::vehicle::Driver;

/// Everything shared by the clients of one process.
pub struct Ctx {
    pub target: Target,
    pub seed: u64,
    pub duration: Duration,
    pub warmup_s: u32,
    pub data_hash: u64,
    pub map: Option<Arc<MapData>>,
    pub empty_collision: frontline_sim::CollisionData,
    pub waypoints: Vec<[f32; 2]>,
    pub drive_vehicle: bool,
    /// Print every rollback to stderr (`HEADLESS_TRACE=1`).
    pub trace: bool,
}

/// Raw per-second series of one client, for pooling in the aggregate.
#[derive(Default)]
pub struct Series {
    pub down: Vec<f64>,
    pub up: Vec<f64>,
    pub rtt: Vec<f64>,
}

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const DEATH_WINDOW: Duration = Duration::from_secs(3);
const RESPAWN_WINDOW: Duration = Duration::from_secs(2);

fn failed(id: u32, bind: String, why: String) -> (ClientReport, Series) {
    (
        ClientReport {
            id,
            bind_ip: bind,
            join_error: Some(why),
            input_hash: "0000000000000000".into(),
            ..ClientReport::default()
        },
        Series::default(),
    )
}

async fn handshake(ctx: &Ctx, link: &mut Link) -> Result<(Welcome, Instant, Duration), String> {
    let hello = encode_c0_client(&ClientMsgC0::Hello(Hello {
        protocol_version: PROTOCOL_VERSION,
        data_hash: ctx.data_hash,
        ticket: Vec::new(),
        client_build: 1,
        caps: if ctx.target.kind == transport::Kind::Wt {
            0b10
        } else {
            0
        },
    }))
    .map_err(|e| format!("encode hello: {e:?}"))?;
    let sent = Instant::now();
    if !link.send(Channel::C0, hello) {
        return Err("connection closed before Hello".into());
    }
    let end = sent + HANDSHAKE_TIMEOUT;
    loop {
        let left = end.saturating_duration_since(Instant::now());
        match tokio::time::timeout(left, link.rx.recv()).await {
            Err(_) => return Err("handshake timeout (no Welcome)".into()),
            Ok(None) => return Err("connection dropped during handshake".into()),
            Ok(Some(Incoming::Closed(r))) => return Err(format!("closed during handshake: {r}")),
            Ok(Some(Incoming::Frame(Channel::C0, p))) => match decode_c0_server(&p) {
                Ok(ServerMsgC0::Welcome(w)) => {
                    let now = Instant::now();
                    return Ok((w, now, now.saturating_duration_since(sent)));
                }
                Ok(ServerMsgC0::Reject(r)) => return Err(format!("rejected: {:?}", r.reason)),
                Ok(ServerMsgC0::Disconnect(d)) => {
                    return Err(format!(
                        "disconnected during handshake (reason {})",
                        d.reason
                    ))
                }
                _ => {}
            },
            Ok(Some(Incoming::Frame(..))) => {}
        }
    }
}

struct Life {
    alive: bool,
    last_death: Option<Instant>,
    last_respawn: Option<Instant>,
}

impl Life {
    fn death_window(&self, now: Instant) -> bool {
        !self.alive
            || self
                .last_death
                .is_some_and(|t| now.saturating_duration_since(t) < DEATH_WINDOW)
            || self
                .last_respawn
                .is_some_and(|t| now.saturating_duration_since(t) < RESPAWN_WINDOW)
    }
}

/// Runs client `id` until `ctx.duration` has passed since its handshake, or the connection ends.
pub async fn run_client(ctx: Arc<Ctx>, id: u32) -> (ClientReport, Series) {
    let bind = ctx
        .target
        .bind_ip
        .map_or_else(|| "any".to_string(), |ip| ip.to_string());
    let mut link = match transport::connect(&ctx.target).await {
        Ok(l) => l,
        Err(e) => return failed(id, bind, format!("connect: {e}")),
    };
    let (welcome, t_start, hs_rtt) = match handshake(&ctx, &mut link).await {
        Ok(x) => x,
        Err(e) => return failed(id, bind, e),
    };
    let mut rep = ClientReport {
        id,
        player_id: Some(welcome.player_id),
        team: Some(welcome.team),
        bind_ip: bind,
        ..ClientReport::default()
    };
    let mut clock = Clock::new(welcome.server_tick, t_start, hs_rtt);
    let empty = frontline_sim::CollisionData::empty();
    let collision = ctx.map.as_ref().map_or(&empty, |m| &m.collision);
    let first_seq: u16 = 1;
    let mut pred = Predictor::new(
        welcome.match_seed,
        welcome.map_id,
        welcome.player_id,
        collision,
        first_seq,
    );
    let mut synth = SynthInput::new(ctx.seed, id, &ctx.waypoints);
    let mut driver_hash = synth.input_hash();
    let mut driver = ctx.drive_vehicle.then(|| {
        Driver::new(new_sim(
            welcome.match_seed,
            welcome.map_id,
            welcome.player_id,
            collision,
        ))
    });
    let mut dec = SnapshotDecoder::new();
    // team 0 = US, 1 = GER; the snapshot's `flags` bit 7 is the faction (PI-024). Tolerances: see audit.rs.
    let mut audit = crate::audit::AoiAudit::new(welcome.team & 1, 4.0, 5.0);
    // the client's predicted position is only trustworthy for the audit while it agrees with the server: skip 2 s after a death, a
    // respawn or a server position correction of more than 5 m (a teleport of the player makes the predicted pose stale)
    let mut audit_skip_until = Instant::now() + Duration::from_secs(3); // also the first seconds after joining (spawn pose not yet confirmed)
    let mut life = Life {
        alive: true,
        last_death: None,
        last_respawn: None,
    };

    let mut down = Meter::default();
    let mut up = Meter::default();
    let mut rtts: Vec<f64> = Vec::new();
    let mut seq = first_seq;
    let mut next_tick: Option<u32> = None;
    let mut history: VecDeque<InputCmd> = VecDeque::with_capacity(4);
    let mut ack_tick_low: u16 = 0;
    let mut report_hash: u8 = 0;
    let mut newest_snapshot: Option<u32> = None;
    let mut pings_sent = 0u32;
    let mut next_ping = t_start;
    let mut seen_events: VecDeque<u16> = VecDeque::with_capacity(300);
    let mut snapshot_bytes_sum = 0u64;
    let mut last_pos: Option<frontline_sim::Vec3> = None;
    let mut last_health: Option<u8> = None;
    let end = t_start + ctx.duration;
    let mut iv = tokio::time::interval(Duration::from_secs_f64(1.0 / 30.0));
    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut ended_by_us = false;

    'run: loop {
        tokio::select! {
            _ = tokio::time::sleep_until(end.into()) => { ended_by_us = true; break 'run; }
            inc = link.rx.recv() => {
                let now = Instant::now();
                let sec = now.saturating_duration_since(t_start).as_secs() as usize;
                match inc {
                    None => { rep.disconnects += 1; rep.disconnect_reason = Some("link dropped".into()); break 'run; }
                    Some(Incoming::Closed(r)) => { rep.disconnects += 1; rep.disconnect_reason.get_or_insert(r); break 'run; }
                    Some(Incoming::Frame(Channel::C0, p)) => match decode_c0_server(&p) {
                        Ok(ServerMsgC0::Pong(pg)) => {
                            if let Some(r) = clock.on_pong(pg.client_time, pg.server_tick, now) { rtts.push(r); }
                        }
                        Ok(ServerMsgC0::Disconnect(d)) => {
                            rep.disconnects += 1;
                            rep.disconnect_reason = Some(format!("server Disconnect, reason {}", d.reason));
                            break 'run;
                        }
                        Ok(_) => {}
                        Err(_) => rep.decode_errors += 1,
                    },
                    Some(Incoming::Frame(Channel::C2, p)) => {
                        down.add(sec, p.len());
                        match dec.decode(&p) {
                            Ok(s) => {
                                let stale = newest_snapshot.is_some_and(|n| (s.tick.wrapping_sub(n) as i32) <= 0);
                                if stale { rep.reordered_snapshots += 1; continue; }
                                newest_snapshot = Some(s.tick);
                                rep.snapshots += 1;
                                snapshot_bytes_sum += p.len() as u64;
                                rep.records += s.entities.len() as u64;
                                rep.keyframe_records += s.entities.keyframe.iter().filter(|&&k| k).count() as u64;
                                clock.on_slack(s.input_slack, now);
                                for e in &s.events {
                                    if seen_events.contains(&e.id) { continue; }
                                    if seen_events.len() >= 256 { seen_events.pop_front(); }
                                    seen_events.push_back(e.id);
                                    rep.events += 1;
                                    if matches!(e.event, Event::Shot { shooter, .. } if shooter == welcome.player_id) {
                                        rep.shots_fired += 1;
                                    }
                                }
                                let mut corr = None;
                                if let Some(ss) = s.self_state {
                                    last_health = Some(ss.health);
                                    let alive_now = ss.health > 0;
                                    if life.alive && !alive_now { rep.deaths += 1; life.last_death = Some(now); }
                                    if !life.alive && alive_now { rep.respawns += 1; life.last_respawn = Some(now); }
                                    life.alive = alive_now;
                                    corr = ss.pos_corr.map(|q| q.to_meters());
                                    let jump = match (corr, pred.pos()) { (Some(c), Some(p)) => (frontline_sim::Vec3::new(c[0], c[1], c[2]) - p).length() > 5.0, _ => false };
                                    if jump || life.last_death == Some(now) || life.last_respawn == Some(now) { audit_skip_until = now + Duration::from_secs(2); }
                                }
                                if let Some(d) = &mut driver {
                                    if d.seated() { audit_skip_until = now + Duration::from_secs(2); }
                                    d.on_snapshot(&s);
                                    if d.seated() { audit_skip_until = now + Duration::from_secs(2); }
                                }
                                audit.tick = s.tick;
                                if life.alive && now >= audit_skip_until { if let Some(p) = pred.pos() { audit.check(p, &s.entities); } else { audit.skipped += 1; } } else { audit.skipped += 1; }
                                let before = pred.stats.total;
                                report_hash = pred.on_self(s.ack_input_seq, corr, life.alive, life.death_window(now));
                                if ctx.trace && pred.stats.total != before {
                                    eprintln!(
                                        "[client {id} +{:.2}s] rollback #{}: err {:.3} m, snapshot tick {}, ack_seq {}, health {:?}, alive {}, corr {:?}",
                                        now.saturating_duration_since(t_start).as_secs_f64(),
                                        pred.stats.total, pred.last_err_m, s.tick, s.ack_input_seq,
                                        last_health, life.alive, corr,
                                    );
                                }
                                ack_tick_low = s.tick as u16;
                            }
                            Err(CodecError::MissingBaseline) => rep.missing_baseline += 1,
                            Err(_) => rep.decode_errors += 1,
                        }
                    }
                    Some(Incoming::Frame(..)) => {}
                }
            }
            _ = iv.tick() => {
                let now = Instant::now();
                let sec = now.saturating_duration_since(t_start).as_secs() as usize;
                if now >= next_ping {
                    let ct = clock.ping(now);
                    if let Ok(b) = encode_c0_client(&ClientMsgC0::Ping(Ping { client_time: ct })) {
                        if !link.send(Channel::C0, b) { rep.disconnects += 1; rep.disconnect_reason = Some("send failed".into()); break 'run; }
                    }
                    pings_sent += 1;
                    next_ping = now + if pings_sent < 5 { Duration::from_millis(100) } else { Duration::from_secs(1) };
                }
                let target = clock.input_tick(now);
                let mut t = *next_tick.get_or_insert(target);
                let mut generated = 0u32;
                while t <= target && generated < 30 {
                    let pose = Pose { xz: pred.pos().map(|p| (p.x, p.z)), alive: life.alive };
                    let cmd = if let Some(d) = &mut driver {
                        let c = d.cmd(seq, t, life.alive && last_health.is_some(), end.saturating_duration_since(now));
                        driver_hash = hash_cmd(driver_hash, &c);
                        c
                    } else {
                        synth.next(seq, t, pose)
                    };
                    // Do not interpret throttle as an infantry movement command while seated.
                    let foot_cmd = if driver.as_ref().is_some_and(Driver::seated) {
                        InputCmd { move_x: 0, move_z: 0, buttons: 0, ..cmd }
                    } else { cmd };
                    pred.push(&foot_cmd);
                    audit.push_orientation(frontline_protocol::quant::dequant_yaw(cmd.yaw, 16), frontline_protocol::quant::pitch_i16_to_rad(cmd.pitch));
                    if let (Some(a), Some(b)) = (last_pos, pred.pos()) { rep.distance_m += f64::from((a - b).length()); }
                    last_pos = pred.pos();
                    if history.len() == 3 { history.pop_front(); }
                    history.push_back(cmd);
                    seq = seq.wrapping_add(1);
                    t += 1;
                    generated += 1;
                    rep.cmds_generated += 1;
                }
                next_tick = Some(t);
                if generated > 0 {
                    let mut cmds = ArrayVec::new();
                    for c in &history { cmds.push(*c); }
                    let bundle = InputBundle { ack_snapshot_tick: ack_tick_low, pred_hash: report_hash, cmds, fire: ArrayVec::new() };
                    match encode_input_bundle(&bundle) {
                        Ok(b) => {
                            up.add(sec, b.len());
                            rep.bundles_sent += 1;
                            if !link.send(Channel::C1, b) { rep.disconnects += 1; rep.disconnect_reason = Some("send failed".into()); break 'run; }
                        }
                        Err(_) => rep.decode_errors += 0, // cannot happen for valid commands; checked in tests
                    }
                }
            }
        }
    }

    if ended_by_us {
        if let Ok(b) = encode_c0_client(&ClientMsgC0::Disconnect(Disconnect { reason: 0 })) {
            link.send(Channel::C0, b);
        }
        // give the writer task a moment to flush the goodbye before the link is dropped
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    let connected = Instant::now().saturating_duration_since(t_start);
    let total_secs = connected.as_secs() as usize;
    let warm = ctx.warmup_s as usize;
    let (dw, dp) = down.dists(total_secs, warm);
    let (uw, upay) = up.dists(total_secs, warm);
    rep.connected_s = connected.as_secs_f64();
    rep.down_kbps = dw;
    rep.down_payload_kbps = dp;
    rep.up_kbps = uw;
    rep.up_payload_kbps = upay;
    rep.rtt_ms = Dist::of(&rtts);
    rep.snapshot_bytes_mean = if rep.snapshots == 0 {
        0.0
    } else {
        snapshot_bytes_sum as f64 / rep.snapshots as f64
    };
    rep.keyframe_ratio = if rep.records == 0 {
        0.0
    } else {
        rep.keyframe_records as f64 / rep.records as f64
    };
    rep.corrections = pred.stats;
    rep.aoi_records_checked = audit.records_checked;
    rep.aoi_beyond_close = audit.beyond_close;
    rep.aoi_leaks = audit.leaks;
    rep.aoi_worst_leak_m = audit.worst_leak_m;
    rep.final_health = last_health;
    rep.final_pos = pred.pos().map(|p| [p.x, p.y, p.z]);
    rep.input_hash = format!(
        "{:016x}",
        if driver.is_some() {
            driver_hash
        } else {
            synth.input_hash()
        }
    );
    rep.vehicle = driver.map(Driver::finish);
    let series = Series {
        down: down.series_wire(total_secs, warm),
        up: up.series_wire(total_secs, warm),
        rtt: rtts,
    };
    (rep, series)
}
