//! `compute_damage` (M1-014 acceptance: <= 0.5 us per call) and `hitbox_record` (100 players
//! written into the history ring: <= 0.1 ms per tick). Run: `cargo bench -p frontline-sim --bench combat`.
use std::hint::black_box;
use std::time::Instant;

use frontline_sim::combat::damage::{compute_damage, DamageKind, HitInfo, TargetCtx};
use frontline_sim::combat::lagcomp_history::{LagCompHistory, PlayerHitboxes};
use frontline_sim::{GameData, HitZone, Tick, Vec3};

fn main() {
    let data = GameData::embedded();
    let def = data.weapon("us_m1_garand").expect("weapon");
    let n = 2_000_000u32;
    let t0 = Instant::now();
    let mut acc = 0.0f32;
    for i in 0..n {
        let r = compute_damage(
            def,
            &HitInfo {
                zone: HitZone::Torso,
                travelled_m: (i % 700) as f32,
                pen_mult: 1.0,
                kind: DamageKind::Bullet,
            },
            &TargetCtx {
                hp: 100.0,
                same_team: i % 3 == 0,
                helmet: true,
                friendly_fire: true,
            },
        );
        acc += black_box(r.damage);
    }
    let ns = t0.elapsed().as_nanos() as f64 / f64::from(n);
    println!("compute_damage: {ns:.1} ns/call (acc {acc})");

    let players: Vec<PlayerHitboxes> = (0..100u16)
        .map(|i| PlayerHitboxes {
            id: i,
            foot: Vec3::new(f32::from(i), 0.0, 0.0),
            blend: 0.0,
            head_dx: 0.0,
            head_dz: 0.0,
        })
        .collect();
    let mut h = LagCompHistory::new();
    let ticks = 20_000u32;
    let t0 = Instant::now();
    for t in 0..ticks {
        h.record(Tick(t), black_box(&players));
    }
    let us = t0.elapsed().as_secs_f64() * 1e6 / f64::from(ticks);
    println!("hitbox_record: {us:.2} us/tick (100 players)");
}
