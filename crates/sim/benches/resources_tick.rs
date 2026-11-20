//! PI-109 workstation measurement of the resource rules body, excluding the rest of World::step.
use std::sync::OnceLock;
use std::time::Instant;

use frontline_sim::rules::resource_node::ResourceSite;
use frontline_sim::rules::resources::ResourceKind;
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, ObjectiveSpec, SimConfig, SpawnReq,
    Team, Vec3, World,
};

fn clock_ns() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_nanos() as u64
}

fn main() {
    let collision = CollisionData::from_chunks(
        (0..4).map(|x| CollisionChunk::new(x, 0, vec![0.0; 65 * 65], vec![0; 64 * 64])),
    );
    let mut w = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 9,
            map: MapId(0),
            mode: GameMode::Frontline,
            data: GameData::embedded(),
        },
        &collision,
    );
    w.set_objectives(
        &(0..6)
            .map(|id| ObjectiveSpec {
                id,
                pos: Vec3::new(35.0 * f32::from(id) + 25.0, 0.0, 44.0),
                radius_m: 8.0,
                owner: Some(if id < 3 { Team::Us } else { Team::Ger }),
            })
            .collect::<Vec<_>>(),
    );
    for sector in 0..6 {
        for kind in ResourceKind::ALL {
            let id = sector * 3 + kind as u16;
            assert!(w.install_resource_site(ResourceSite {
                id,
                sector,
                kind,
                pos: Vec3::new(
                    35.0 * f32::from(sector) + 25.0,
                    0.0,
                    42.0 + 3.0 * f32::from(kind as u8)
                )
            }));
            w.spawn_node(id, kind, if sector < 3 { Team::Us } else { Team::Ger }, &[])
                .unwrap();
        }
    }
    for player in 0..100 {
        // All actors outside every node radius, forcing the full scan for each node every tick.
        w.spawn_player(SpawnReq {
            player,
            team: if player % 2 == 0 { Team::Us } else { Team::Ger },
            pos: Vec3::new(2.0 * f32::from(player) + 4.0, 0.01, 5.0),
            yaw: 0.0,
        })
        .unwrap();
    }
    w.enable_step_profiling(clock_ns);
    for _ in 0..300 {
        w.step(&[]);
    }
    let mut ns = Vec::with_capacity(3600);
    let mut minute_ns = Vec::new();
    for _ in 0..3600 {
        let out = w.step(&[]);
        let elapsed = w.step_timings().unwrap().resources;
        ns.push(elapsed);
        if out.resource_history.is_some() {
            minute_ns.push(elapsed);
        }
    }
    assert_eq!(w.resource_nodes().count(), 18);
    assert_eq!(w.view().players().len(), 100);
    assert_eq!(minute_ns.len(), 2);
    ns.sort_unstable();
    let us = |value: u64| value as f64 / 1000.0;
    println!("resources_tick (100 players, 18 nodes, {} ticks): p50 {:.3} us, p99 {:.3} us, max {:.3} us; minute boundaries {:?} us", ns.len(), us(ns[ns.len()/2]), us(ns[ns.len()*99/100]), us(*ns.last().unwrap()), minute_ns.into_iter().map(us).collect::<Vec<_>>());
}
