//! M1-034 integration test (`cargo test -p frontline-sim --test map_zones_load`):
//! parses real `data/maps/carentan/m1/{zones,spawns}.json` and installs the active strongpoints
//! and HQs into `frontline_sim::World`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use frontline_sim::{
    CollisionData, GameData, GameMode, MapId, ObjectiveSpec, SimConfig, Team, World,
};
use glam::Vec3;
use serde::Deserialize;

#[derive(Deserialize)]
struct ZonesFile {
    schema: String,
    lines: Vec<ZoneLine>,
    m1_active: Vec<String>,
    playable_polygon: Vec<[f32; 2]>,
    out_of_bounds_buffer_m: f32,
}

#[derive(Deserialize)]
struct ZoneLine {
    id: String,
    points: Vec<ZonePoint>,
}

#[derive(Deserialize)]
struct ZonePoint {
    id: String,
    pos: [f32; 2],
    radius_m: f32,
    snap_m: f32,
}

#[derive(Deserialize)]
struct SpawnsFile {
    schema: String,
    hq: BTreeMap<String, SpawnPoint>,
    garrison_candidates: BTreeMap<String, GarrisonCandidate>,
    kinds_supported: Vec<String>,
}

#[derive(Deserialize)]
struct SpawnPoint {
    pos: [f32; 2],
    radius_m: f32,
}

#[derive(Deserialize)]
struct GarrisonCandidate {
    pos: [f32; 2],
    hq_distance_m: f32,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn map_zones_load_carentan_m1() {
    let m1 = repo_root().join("data/maps/carentan/m1");
    let zones: ZonesFile =
        serde_json::from_slice(&std::fs::read(m1.join("zones.json")).expect("read zones.json"))
            .expect("parse zones.json");
    let spawns: SpawnsFile =
        serde_json::from_slice(&std::fs::read(m1.join("spawns.json")).expect("read spawns.json"))
            .expect("parse spawns.json");

    assert_eq!(zones.schema, "frontline.map.zones/1");
    assert_eq!(spawns.schema, "frontline.map.spawns/1");
    assert_eq!(zones.out_of_bounds_buffer_m, 64.0);
    assert!(zones.playable_polygon.len() >= 4);
    assert_eq!(zones.m1_active, vec!["L1a", "L2a", "L3a"]);
    assert!(spawns.kinds_supported.iter().any(|k| k == "hq"));
    assert!(spawns.kinds_supported.iter().any(|k| k == "garrison"));

    let xs: Vec<f32> = zones.playable_polygon.iter().map(|p| p[0]).collect();
    let zs: Vec<f32> = zones.playable_polygon.iter().map(|p| p[1]).collect();
    let (xmin, xmax) = (
        xs.iter().copied().fold(f32::INFINITY, f32::min),
        xs.iter().copied().fold(f32::NEG_INFINITY, f32::max),
    );
    let (zmin, zmax) = (
        zs.iter().copied().fold(f32::INFINITY, f32::min),
        zs.iter().copied().fold(f32::NEG_INFINITY, f32::max),
    );

    let mut active_specs = Vec::new();
    for line in &zones.lines {
        assert!(!line.id.is_empty());
        for pt in &line.points {
            assert!(pt.snap_m <= 60.0, "{} snap_m={} > 60", pt.id, pt.snap_m);
            if zones.m1_active.contains(&pt.id) {
                assert!(
                    pt.pos[0] >= xmin
                        && pt.pos[0] <= xmax
                        && pt.pos[1] >= zmin
                        && pt.pos[1] <= zmax,
                    "{} outside playable_polygon",
                    pt.id
                );
                active_specs.push(ObjectiveSpec {
                    id: (active_specs.len() + 1) as u16,
                    pos: Vec3::new(pt.pos[0], 0.0, pt.pos[1]),
                    radius_m: pt.radius_m,
                    owner: None,
                });
            }
        }
    }
    assert_eq!(active_specs.len(), 3);

    for (side, gar) in &spawns.garrison_candidates {
        let hq = spawns.hq.get(side).expect("matching HQ");
        let dist = ((gar.pos[0] - hq.pos[0]).powi(2) + (gar.pos[1] - hq.pos[1]).powi(2)).sqrt();
        assert!(
            dist >= 200.0 && gar.hq_distance_m >= 200.0,
            "{side} garrison distance {dist:.1} m < 200 m"
        );
    }

    let collision = CollisionData::empty();
    let cfg = SimConfig {
        tick_hz: 30,
        match_seed: 42,
        map: MapId(1),
        mode: GameMode::Frontline,
        data: GameData::embedded(),
    };
    let mut world = World::new(cfg, &collision);
    world.set_objectives(&active_specs);
    let us_hq = spawns.hq.get("US").expect("US HQ");
    let ger_hq = spawns.hq.get("GER").expect("GER HQ");
    assert!(us_hq.radius_m >= 20.0 && ger_hq.radius_m >= 20.0);
    world.set_hq(Team::Us, Vec3::new(us_hq.pos[0], 0.0, us_hq.pos[1]));
    world.set_hq(Team::Ger, Vec3::new(ger_hq.pos[0], 0.0, ger_hq.pos[1]));
    assert_eq!(world.objective_list().len(), 3);
    assert!(world.has_hq(Team::Us) && world.has_hq(Team::Ger));
    let _ = world.step(&[]);
}
