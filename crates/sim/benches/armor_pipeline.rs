//! `armor_pipeline` (M1-023): full hit resolution on a Sherman (zone lookup, armour verdict,
//! internal path, module damage). Budget <= 10 us per hit.
//! Run: `cargo bench -p frontline-sim --features test-util --bench armor_pipeline`.
use std::time::Instant;

use frontline_sim::vehicle::shell::HitRolls;
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, SimConfig, Team, Vec3, World,
};

fn main() {
    let mut v = Vec::new();
    for bz in 0..6 {
        for bx in 0..30 {
            v.push(CollisionChunk::new(
                bx,
                bz,
                vec![5.0f32; 65 * 65],
                vec![0; 64 * 64],
            ));
        }
    }
    let col = CollisionData::from_chunks(v);
    let mut w = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 1,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        &col,
    );
    let id = w
        .spawn_vehicle("us_m4a1_75", Team::Us, Vec3::new(500.0, 6.5, 100.0), 0.0)
        .unwrap();
    let n = 200_000u32;
    let t0 = Instant::now();
    let mut pens = 0u32;
    for i in 0..n {
        let a = (i % 360) as f32 * 0.01745;
        let o = Vec3::new(500.0 + 80.0 * a.cos(), 6.5, 100.0 + 80.0 * a.sin());
        let d = (Vec3::new(500.0, 6.3, 100.0) - o).normalize();
        let r = w.debug_ap_hit(
            id,
            0,
            o,
            d,
            80.0,
            HitRolls {
                armor: 0.5,
                damage: 0.5,
                cookoff: 0.99,
                fire: 0.99,
            },
        );
        if r.is_some_and(|r| r.armor == Some(frontline_sim::vehicle::shell::ArmorKind::Pen)) {
            pens += 1;
        }
    }
    println!(
        "armor pipeline: {:.2} us/hit ({pens} pens of {n})",
        t0.elapsed().as_secs_f64() * 1e6 / f64::from(n)
    );
}
