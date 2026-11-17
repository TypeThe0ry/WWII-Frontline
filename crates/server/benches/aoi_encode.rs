//! `aoi_encode` (task M0-019; ARCHITECTURE §6.10 stages 8 + 9): wall time of one scheduler pass
//! (AOI selection + snapshot encoding, serial on the sim thread) with 10 / 20 / 100 clients on
//! the M0 terrain, every player moving and firing (`gen_cmd`-like script). Writes
//! `reports/baselines/server-m0.json` (merging into an existing file).
//!
//! Needs the git-ignored terrain: `PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0
//! --seed 1 --out data/maps/m0`. Run: `cargo bench -p frontline-server --bench aoi_encode`.

use std::path::PathBuf;
use std::time::Instant; // benches are outside the sim purity scan

use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_server::snapshot::{ClientIn, LinkInfo, SnapshotScheduler};
use frontline_server::world_loader;
use frontline_sim::{GameData, GameMode, MapId, SimConfig, SpawnReq, Team, World};

fn gen_cmd(player: u8, tick: u32) -> InputCmd {
    let phase = ((tick / 24) + player as u32) % 5;
    let (mx, mz): (i8, i8) = match phase {
        0 => (0, 127),
        1 => (90, 90),
        2 => (127, 0),
        3 => (0, 0),
        _ => (-60, 100),
    };
    let mut b = 0u16;
    if phase == 0 && tick % 48 < 24 {
        b |= buttons::SPRINT;
    }
    if phase == 4 {
        b |= buttons::CROUCH;
    }
    if player.is_multiple_of(2) && (30..40).contains(&(tick % 90)) {
        b |= buttons::FIRE;
    }
    InputCmd {
        seq: tick as u16,
        tick,
        move_x: mx,
        move_z: mz,
        yaw: (player as u32 * 5000 + tick * 40) as u16,
        pitch: ((tick as i32 % 200) * 20 - 2000) as i16,
        buttons: b,
        weapon_slot: 0,
        aux: 0,
        view_tick: 2,
    }
}

fn q(v: &[f64], p: f64) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    s[(((s.len() - 1) as f64) * p).ceil() as usize]
}

struct Res {
    n: usize,
    p50: f64,
    p99: f64,
    max: f64,
    mean: f64,
    per_client_us: f64,
    bytes: f64,
    records: f64,
}

/// `dense`: everybody on one team (no R3 trimming, every player is a candidate).
fn run(n: usize, ticks: u32, map: &world_loader::LoadedMap, dense: bool) -> Res {
    let team_of = |i: usize| {
        if dense || i.is_multiple_of(2) {
            Team::Us
        } else {
            Team::Ger
        }
    };
    let mut w = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 3,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        &map.collision,
    );
    map.apply_to(&mut w);
    for i in 0..n {
        let team = team_of(i);
        let (pos, yaw) = map.join_spawn(i as u8, team);
        w.spawn_player(SpawnReq {
            player: i as u8,
            team,
            pos,
            yaw,
        })
        .unwrap();
    }
    let mut sched = SnapshotScheduler::new(None);
    let mut out = Vec::new();
    let mut ms = Vec::new();
    let (mut bytes, mut pk) = (0usize, 0usize);
    let mut ack = vec![(0u16, 0u8); n];
    for _ in 0..ticks {
        let t = w.tick().0 + 1;
        let inputs: Vec<_> = w
            .view()
            .players()
            .iter()
            .map(|p| (p.player, gen_cmd(p.player, t)))
            .collect();
        let ev = w.step(&inputs).events;
        let cin: Vec<ClientIn> = (0..n)
            .map(|i| ClientIn {
                player: i as u8,
                team: team_of(i),
                ack_low: ack[i].0,
                pred_hash: ack[i].1,
                ack_input_seq: t as u16,
                input_slack: 1,
                link: LinkInfo::DATAGRAM,
            })
            .collect();
        out.clear();
        let t0 = Instant::now();
        sched.tick(&w, &ev, &cin, t, &mut out);
        let e = t0.elapsed().as_secs_f64() * 1e3;
        for o in &out {
            bytes += o.bytes.len();
            pk += 1;
            // perfect acknowledgement one tick later: ack the tick, report the right hash
            let s = w.player_state(o.player).unwrap();
            ack[o.player as usize] = (
                t as u16,
                frontline_protocol::snapshot::pred_hash::pred_hash(s.pos.x, s.pos.y, s.pos.z),
            );
        }
        ms.push(e);
    }
    let warm = ms.len() / 5;
    let ms = &ms[warm..];
    let mean = ms.iter().sum::<f64>() / ms.len() as f64;
    let st = &sched.stats;
    Res {
        n,
        p50: q(ms, 0.5),
        p99: q(ms, 0.99),
        max: q(ms, 1.0),
        mean,
        per_client_us: mean * 1e3 / (n as f64 * 2.0 / 3.0),
        bytes: bytes as f64 / pk.max(1) as f64,
        records: (st.keyframe_records + st.delta_records) as f64 / st.packets.max(1) as f64,
    }
}

fn main() {
    let dir = match world_loader::locate_data_dir(None, "m0") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("SKIPPED: {e}");
            return;
        }
    };
    let map = world_loader::load_map(&dir, "m0").expect("terrain");
    let mut json = Vec::new();
    for (n, ticks, dense) in [
        (10usize, 1800u32, false),
        (20, 1800, false),
        (100, 900, false),
        (100, 900, true),
    ] {
        let r = run(n, ticks, &map, dense);
        eprintln!(
            "aoi_encode {:>3} clients{}: AOI+encode per tick P50 {:.3} / P99 {:.3} / max {:.3} ms (mean {:.3}), {:.1} us per client packet, mean packet {:.0} B, {:.1} entity records per packet",
            r.n,
            if dense {
                " (one team, no R3 trimming)"
            } else {
                ""
            },
            r.p50,
            r.p99,
            r.max,
            r.mean,
            r.per_client_us,
            r.bytes,
            r.records
        );
        json.push((
            if dense {
                format!("{}_dense", r.n)
            } else {
                r.n.to_string()
            },
            r,
        ));
    }
    // write reports/baselines/server-m0.json (merge)
    // `FRONTLINE_BASELINE_DIR` redirects the output (the perf regression check must not overwrite the baseline).
    let path = std::env::var_os("FRONTLINE_BASELINE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reports/baselines")
        })
        .join("server-m0.json");
    let mut root: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let mut obj = serde_json::Map::new();
    for (n, r) in &json {
        obj.insert(
            format!("clients_{n}"),
            serde_json::json!({
                "tick_ms_p50": r.p50, "tick_ms_p99": r.p99, "tick_ms_max": r.max, "tick_ms_mean": r.mean,
                "us_per_client_packet": r.per_client_us, "mean_packet_bytes": r.bytes, "entity_records_per_packet": r.records,
            }),
        );
    }
    root["aoi_encode"] = serde_json::Value::Object(obj);
    root["aoi_encode"]["_note"] = serde_json::json!(
        "AOI selection + snapshot encoding wall time per tick (sim thread, serial), moving+firing players on the M0 terrain; budget ARCHITECTURE 6.10 stages 8+9 = 2.0 + 0.6 ms at ~67 packets per tick"
    );
    std::fs::create_dir_all(path.parent().unwrap()).ok();
    std::fs::write(&path, serde_json::to_string_pretty(&root).unwrap() + "\n")
        .expect("write baseline");
    eprintln!("wrote {}", path.display());
}
