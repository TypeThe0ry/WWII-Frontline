//! `nav_query` (ARCHITECTURE 16.2: <= 1 ms/tick for 60 bots): random start/goal pairs on the
//! Carentan 1/3 walk mesh. Plain `Instant` harness; writes `reports/m1-bots-nav.json`.
//! Run: `cargo bench -p frontline-bots --bench nav_query`.

use std::path::PathBuf;
use std::time::Instant;

use frontline_bots::nav::path::{PathFinder, PathResult};
use frontline_bots::nav::NavMesh;
use frontline_bots::TICK_HZ;

fn root() -> Option<PathBuf> {
    let mut d = std::env::current_dir().ok()?;
    loop {
        if d.join("data/maps/carentan/m1/navmesh.bin").is_file() {
            return Some(d);
        }
        if !d.pop() {
            return None;
        }
    }
}

fn main() {
    let Some(root) = root() else {
        eprintln!("nav_query: data/maps/carentan/m1/navmesh.bin missing; run `map-pipeline nav --slice m1`");
        return;
    };
    let mesh = NavMesh::load(&root.join("data/maps/carentan/m1/navmesh.bin")).expect("navmesh");
    // 60 deterministic pairs inside the slice rectangle (x 380..1560, z 520..1700)
    let mut s = 0x9E37_79B9_7F4A_7C15u64;
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f32 / (1u64 << 53) as f32
    };
    let mut pairs = Vec::new();
    while pairs.len() < 60 {
        let a = [380.0 + rnd() * 1180.0, 520.0 + rnd() * 1180.0];
        let b = [380.0 + rnd() * 1180.0, 520.0 + rnd() * 1180.0];
        if mesh.nearest(a, 2.0).is_some() && mesh.nearest(b, 2.0).is_some() {
            pairs.push((a, b));
        }
    }
    let mut pf = PathFinder::new();
    for (a, b) in &pairs {
        let _ = pf.find(&mesh, *a, *b, !0); // warm-up
    }
    let mut times = Vec::new();
    let (mut found, mut partial, mut none, mut expanded) = (0, 0, 0, Vec::new());
    for _ in 0..5 {
        for (a, b) in &pairs {
            let t = Instant::now();
            let r = pf.find(&mesh, *a, *b, !0);
            times.push(t.elapsed().as_secs_f64() * 1e3);
            expanded.push(pf.last_expanded);
            match r {
                PathResult::Found(_) => found += 1,
                PathResult::Partial(_) => partial += 1,
                PathResult::NoPath => none += 1,
            }
        }
    }
    times.sort_by(|a, b| a.total_cmp(b));
    expanded.sort_unstable();
    let med = times[times.len() / 2];
    let p99 = times[(times.len() as f64 * 0.99) as usize];
    // all 60 bots replanning at the 2 Hz cap: 120 queries/s spread over TICK_HZ ticks
    let per_tick = med * 60.0 * 2.0 / TICK_HZ as f64;
    let report = serde_json::json!({
        "schema": 1, "mesh_polys": mesh.poly_count(), "pairs": 60,
        "query_ms": { "median": med, "p99": p99, "max": times[times.len() - 1] },
        "expanded": { "median": expanded[expanded.len() / 2], "max": expanded[expanded.len() - 1] },
        "results": { "found": found / 5, "partial": partial / 5, "none": none / 5 },
        "tick_hz": TICK_HZ, "bots": 60, "replans_per_bot_per_s": 2,
        "ms_per_tick_at_2hz_replan": per_tick,
        "all_60_in_one_tick_ms": med * 60.0,
        "budget_ms_per_tick": 1.0,
        "note": "single thread, release profile; machine is the cloud container (4 cores), not the reference machine",
    });
    let out = root.join("reports/m1-bots-nav.json");
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
