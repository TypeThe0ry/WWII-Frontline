//! Native half of the M0-029 determinism/drift check.
//! Run: `cargo run --release -p frontline-sim-wasm --example determinism [-- OUT_DIR [TICKS [WORLD_TICKS]]]`
//! (defaults: `target/determinism`, 18000, same as TICKS). For seeds 1..=3 it generates the fixed input
//! sequence, runs `PredictSim` twice (native x2, per-tick `state_hash` compared), runs a 10-player
//! `World` twice per seed, and writes into OUT_DIR:
//!   inputs_seed{N}.bin      `[u32 count][18 B InputCmd x count]`
//!   native_hash_seed{N}.bin u64 LE per tick (`Predictor.stateHash()` equivalent)
//!   native_pos_seed{N}.f32  3 x f32 LE per tick
//!   native_seed{N}.json     native x2 result + coverage + timing
//!   native_meta.json        versions / DATA_HASH
//! Exit status 1 when native x2 differs (blocking defect).
#[path = "../tests/support/determinism.rs"]
mod det;

use std::fmt::Write as _;
use std::path::PathBuf;

fn first_diff_json(d: Option<usize>) -> String {
    d.map_or("null".into(), |t| (t + 1).to_string())
}

fn main() {
    let mut a = std::env::args().skip(1);
    let out = PathBuf::from(a.next().unwrap_or_else(|| "target/determinism".into()));
    let n: usize = a.next().map_or(det::TEN_MIN_TICKS, |s| s.parse().unwrap());
    let wn: usize = a.next().map_or(n, |s| s.parse().unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let Some(col) = det::terrain() else {
        eprintln!("M0 terrain missing: PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0");
        std::process::exit(2);
    };
    let mut failed = false;
    for seed in 1u64..=3 {
        let cmds = det::gen_inputs(seed, n);
        std::fs::write(
            out.join(format!("inputs_seed{seed}.bin")),
            det::inputs_to_bytes(&cmds),
        )
        .unwrap();
        let r1 = det::run_predict(seed, &col, &cmds, None);
        // Perturb the allocator between the two runs.
        let junk: Vec<Vec<u8>> = (0..64).map(|i| vec![i as u8; 1000 + i * 37]).collect();
        let r2 = det::run_predict(seed, &col, &cmds, None);
        drop(junk);
        let d = det::first_diff(&r1.hashes, &r2.hashes);
        failed |= d.is_some();
        let equal = r1
            .hashes
            .iter()
            .zip(&r2.hashes)
            .filter(|(x, y)| x == y)
            .count();
        let mut hb = Vec::with_capacity(n * 8);
        r1.hashes
            .iter()
            .for_each(|h| hb.extend_from_slice(&h.to_le_bytes()));
        std::fs::write(out.join(format!("native_hash_seed{seed}.bin")), hb).unwrap();
        let mut pb = Vec::with_capacity(n * 12);
        r1.pos
            .iter()
            .flatten()
            .for_each(|v| pb.extend_from_slice(&v.to_le_bytes()));
        std::fs::write(out.join(format!("native_pos_seed{seed}.f32")), pb).unwrap();

        let mut world = String::from("null");
        if wn > 0 {
            let (h1, t1) = det::run_world(seed, &col, wn);
            let (h2, t2) = det::run_world(seed, &col, wn);
            let wd = det::first_diff(&h1, &h2);
            failed |= wd.is_some();
            world = format!(
                "{{\"players\":10,\"ticks\":{wn},\"equal\":{},\"first_diff_tick\":{},\"final_hash\":\"{:#018x}\",\"elapsed_s\":[{t1:.3},{t2:.3}]}}",
                wd.is_none(),
                first_diff_json(wd),
                h1[wn - 1]
            );
        }
        let json = format!(
            "{{\"seed\":{seed},\"ticks\":{n},\"native_x2\":{{\"equal\":{},\"equal_ticks\":{equal},\"first_diff_tick\":{}}},\"final_hash\":\"{:#018x}\",\"elapsed_s\":[{:.3},{:.3}],\"spawn\":{:?},\"coverage\":{{\"blocked_ticks\":{},\"grounded_ticks\":{},\"chunks_visited\":{},\"shots\":{},\"max_speed_mps\":{:.3}}},\"world10\":{world}}}\n",
            d.is_none(),
            first_diff_json(d),
            r1.hashes[n - 1],
            r1.elapsed_s,
            r2.elapsed_s,
            r1.spawn,
            r1.blocked_ticks,
            r1.grounded_ticks,
            r1.chunks_visited,
            r1.shots,
            r1.max_speed
        );
        std::fs::write(out.join(format!("native_seed{seed}.json")), &json).unwrap();
        println!(
            "seed {seed}: native x2 {equal}/{n} ticks equal, {:.2}s / {:.2}s, final hash {:#018x}",
            r1.elapsed_s,
            r2.elapsed_s,
            r1.hashes[n - 1]
        );
    }
    let mut meta = String::new();
    let _ = writeln!(
        meta,
        "{{\"data_hash\":\"{:#018x}\",\"protocol_version\":{},\"rapier3d\":\"0.36.1\",\"rapier_features\":\"dim3,f32,std,enhanced-determinism\",\"tick_hz\":30,\"player\":{},\"reserve\":{}}}",
        frontline_sim::data::DATA_HASH,
        frontline_protocol::PROTOCOL_VERSION,
        det::PLAYER,
        det::RESERVE
    );
    std::fs::write(out.join("native_meta.json"), meta).unwrap();
    if failed {
        eprintln!("DETERMINISM FAILURE: native x2 differs (see native_seed*.json)");
        std::process::exit(1);
    }
}
