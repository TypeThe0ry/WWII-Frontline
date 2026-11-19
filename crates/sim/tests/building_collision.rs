//! D2 acceptance through the actual shared player movement path, on Carentan's 1 m terrain.
mod common;

use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::Path;

use common::{cmd, move_params};
use frontline_protocol::quant::quant_yaw;
use frontline_sim::collision::{BuildingCollision, CHUNK_CELLS, CHUNK_SAMPLES};
use frontline_sim::{
    step_player, CollisionChunk, CollisionData, MoveParams, PlayerAuthState, Vec3,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct MapMeta {
    height: TerrainMeta,
}

#[derive(Deserialize)]
struct TerrainMeta {
    hmin_m: f32,
    hmax_m: f32,
    samples: usize,
    row_order: String,
}

#[derive(Deserialize)]
struct Document {
    buildings: Vec<Building>,
}

#[derive(Deserialize)]
struct Building {
    id: String,
    door: Door,
    ramp: Option<Ramp>,
}

#[derive(Deserialize)]
struct Door {
    center: [f32; 2],
    outward: [f32; 2],
}

#[derive(Deserialize)]
struct Ramp {
    start: [f32; 3],
    end: [f32; 3],
}

fn xz([x, z]: [f32; 2]) -> Vec3 {
    Vec3::new(x, 0.0, z)
}

fn flat(v: Vec3) -> Vec3 {
    v.with_y(0.0)
}

fn settle(col: &CollisionData, p: &MoveParams, pos: Vec3) -> PlayerAuthState {
    let mut state = PlayerAuthState::spawn(pos + Vec3::Y * 0.17, 0.0);
    for _ in 0..24 {
        state = step_player(&state, &cmd(0, 0, 0, 0, 0), p, col);
    }
    assert!(state.grounded, "failed to settle at {pos}: {state:?}");
    state
}

fn walk(
    col: &CollisionData,
    p: &MoveParams,
    mut state: PlayerAuthState,
    target: Vec3,
) -> PlayerAuthState {
    for _ in 0..105 {
        let offset = flat(target - state.pos);
        if offset.length() < 0.12 {
            break;
        }
        let yaw = quant_yaw(offset.x.atan2(offset.z), 16).code as u16;
        let throttle = (offset.length() * 300.0).clamp(36.0, 127.0) as i8;
        state = step_player(&state, &cmd(0, throttle, yaw, 0, 0), p, col);
        assert!(!state.blocked, "terrain unavailable at {:?}", state.pos);
    }
    state
}

fn terrain(dir: &Path, cases: &[(&Building, f32)]) -> CollisionData {
    let meta: MapMeta =
        serde_json::from_slice(&std::fs::read(dir.join("meta.json")).unwrap()).unwrap();
    let TerrainMeta {
        hmin_m,
        hmax_m,
        samples,
        row_order,
    } = meta.height;
    assert_eq!(row_order, "south_to_north");
    let r16 = std::fs::read(dir.join("heightmap_1m.r16")).unwrap();
    assert_eq!(r16.len(), samples * samples * 2);
    let mut keys = BTreeSet::new();
    for (b, _) in cases {
        let ramp = b.ramp.as_ref().unwrap();
        for [x, z] in [
            b.door.center,
            [ramp.start[0], ramp.start[2]],
            [ramp.end[0], ramp.end[2]],
        ] {
            let chunk = |v: f32| (v / CHUNK_CELLS as f32).floor() as i32;
            for bz in chunk(z - 3.0)..=chunk(z + 3.0) {
                for bx in chunk(x - 3.0)..=chunk(x + 3.0) {
                    keys.insert((bx, bz));
                }
            }
        }
    }
    CollisionData::from_chunks(keys.into_iter().map(|(bx, bz)| {
        let mut h = Vec::with_capacity(CHUNK_SAMPLES * CHUNK_SAMPLES);
        for dz in 0..CHUNK_SAMPLES {
            for dx in 0..CHUNK_SAMPLES {
                let x = bx as usize * CHUNK_CELLS + dx;
                let z = bz as usize * CHUNK_CELLS + dz;
                let off = (z * samples + x) * 2;
                let q = u16::from_le_bytes([r16[off], r16[off + 1]]);
                h.push(hmin_m + (q as f32 / 65_535.0) * (hmax_m - hmin_m));
            }
        }
        CollisionChunk::new(bx, bz, h, vec![0; CHUNK_CELLS * CHUNK_CELLS])
    }))
}

#[test]
fn carentan_threshold_both_ways_wall_and_upstairs() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/carentan");
    let path = dir.join("building-interiors.json");
    let doc = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            assert!(
                std::env::var_os("FRONTLINE_REQUIRE_BUILDING_COLLISION").is_none(),
                "{path:?}: {e}"
            );
            eprintln!("SKIPPED: {path:?} not present");
            return;
        }
        Err(e) => panic!("{path:?}: {e}"),
    };
    let meta: Document = serde_json::from_slice(&doc).unwrap();
    // Door-front jamb offset (positive = along the clockwise tangent). Selected for
    // unobstructed approaches and playable straight ramp landings, not for flat synthetic terrain.
    let cases = [
        ("way/154020056", -1.26),
        ("way/154023302", -1.26),
        ("way/154020233", -1.26),
    ]
    .map(|(id, side)| (meta.buildings.iter().find(|b| b.id == id).unwrap(), side));
    let mut col = terrain(&dir, &cases);
    col.install_building_collision(BuildingCollision::from_bytes(&doc).unwrap());
    let p = move_params();
    for (b, jamb_offset) in cases {
        let door = xz(b.door.center);
        let outward = xz(b.door.outward).normalize();
        let ground = |v: Vec3| v.with_y(col.height_at(v.x, v.z).unwrap());
        let outside = door + outward * 1.15;
        let inside = door - outward * 0.92;
        let target_reached = |phase, s: PlayerAuthState, target| {
            assert!(
                flat(s.pos - target).length() < 0.15,
                "{} {phase}: pos {} target {target}",
                b.id,
                s.pos
            );
        };
        let outdoor_spawn = settle(&col, &p, ground(outside));
        let entered = walk(&col, &p, outdoor_spawn, inside);
        target_reached("enter doorway", entered, inside);
        let exited = walk(&col, &p, entered, outside);
        target_reached("exit doorway", exited, outside);
        let jamb = door + Vec3::new(outward.z, 0.0, -outward.x) * jamb_offset;
        let start = settle(&col, &p, ground(jamb + outward * 1.15));
        let blocked = walk(&col, &p, start, jamb - outward * 0.92);
        let separation = flat(blocked.pos - jamb).dot(outward);
        assert!(
            (0.2..0.65).contains(&separation),
            "{} not held by jamb: start {} end {} separation {separation}",
            b.id,
            start.pos,
            blocked.pos
        );

        let ramp = b.ramp.as_ref().unwrap();
        let a = Vec3::from_array(ramp.start);
        let z = Vec3::from_array(ramp.end);
        let axis = flat(z - a).normalize();
        let foot = a - axis * 0.7;
        let landing = z + axis * 0.7;
        let reentered = walk(&col, &p, exited, inside);
        target_reached("re-enter doorway", reentered, inside);
        let approach = walk(&col, &p, reentered, foot);
        target_reached("approach ramp", approach, foot);
        assert!(
            (approach.pos.y - a.y).abs() < 0.25,
            "{} incorrect floor height: {} vs {}",
            b.id,
            approach.pos.y,
            a.y
        );
        let climbed = walk(&col, &p, approach, landing);
        target_reached("upstairs landing", climbed, landing);
        assert!(
            climbed.grounded && (climbed.pos.y - z.y).abs() < 0.20,
            "{} did not reach upper floor: {climbed:?}, target {landing}",
            b.id
        );
        println!(
            "{} entrance {} -> {} -> {}, blocked {}, ramp {} -> {}",
            b.id,
            outdoor_spawn.pos,
            entered.pos,
            exited.pos,
            blocked.pos,
            approach.pos,
            climbed.pos
        );
    }
}

#[test]
fn terrain_snap_and_stance_do_not_pass_through_building_surfaces() {
    use frontline_sim::movement::move_capsule;
    use frontline_sim::Stance;

    let mut col = common::flat(1, 0.0);
    let buildings = BuildingCollision::from_bytes(
        br#"{"version":1,"buildings":[{"id":"room",
      "walls":[{"a":[10,10],"b":[16,10],"bottom":0,"top":3,"width":0.24}],
      "surfaces":[
        {"kind":"floor","vertices":[[10,0.15,10],[10,0.15,16],[16,0.15,10]],"indices":[0,1,2]},
        {"kind":"roof","vertices":[[10,1.65,10],[10,1.65,16],[16,1.65,10]],"indices":[0,1,2]}
      ]}]}"#,
    )
    .unwrap();
    let p = move_params();
    let foot = Vec3::new(12.0, 0.22, 12.0);
    let shift = Vec3::X * 0.04;
    let legacy = move_capsule(&col, &p, foot, p.height_crouch_m, shift, true);
    assert!((foot.y + legacy.translation.y - 0.03).abs() < 1e-4);
    col.install_building_collision(buildings);
    let on_floor = move_capsule(&col, &p, foot, p.height_crouch_m, shift, true);
    assert!((foot.y + on_floor.translation.y - 0.18).abs() < 1e-4);

    let mut state = PlayerAuthState::spawn(foot.with_y(0.18), 0.0);
    state.stance = Stance::Crouch;
    state.stance_from = Stance::Crouch;
    state.stance_blend = Stance::Crouch.pose();
    for _ in 0..24 {
        state = step_player(&state, &cmd(0, 0, 0, 0, 0), &p, &col);
    }
    assert_eq!(state.stance, Stance::Crouch);
    assert!((state.pos.y - 0.18).abs() < 0.03, "{state:?}");
    assert_eq!(col.height_at(12.0, 12.0), Some(0.0));
    assert!(
        col.raycast(foot, Vec3::Y, 5.0).is_none(),
        "terrain-only API"
    );
}

#[test]
fn canonical_hash_and_clone_ignore_install_order_but_track_geometry() {
    use rapier3d::prelude::{Ray, Vector};
    use serde_json::json;

    let building = |id, x: u32| {
        json!({"id": id,
            "walls":[{"a":[x,10],"b":[x+4,10],"bottom":0,"top":3,"width":0.24}],
            "surfaces":[{"kind":"roof","vertices":[[x,3,10],[x,3,14],[x+4,3,10]],"indices":[0,1,2]}]
        })
    };
    let mut doc = json!({"version":1,"buildings":[building("z", 15), building("a", 10)]});
    let parse =
        |doc: &serde_json::Value| BuildingCollision::from_bytes(&serde_json::to_vec(doc).unwrap());
    let mut a = common::flat(1, 0.0);
    let legacy_hash = a.state_hash();
    a.install_building_collision(parse(&doc).unwrap());
    let mut b = CollisionData::empty();
    doc["buildings"].as_array_mut().unwrap().reverse();
    doc["buildings"][0]["probes"] = json!([[10, 1, 10]]);
    b.install_building_collision(parse(&doc).unwrap());
    b.insert_chunk(common::synth_chunk(0, 0, &|_, _| 0.0));
    let copied = b.clone();
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(b.state_hash(), copied.state_hash());
    let ray = Ray::new(Vector::new(11.0, 4.0, 11.0), -Vector::Y);
    let distance = |col: &CollisionData| {
        col.physics()
            .world()
            .query_pipeline()
            .cast_ray(&ray, 5.0, true)
            .unwrap()
            .1
    };
    assert_eq!(distance(&a), 1.0);
    assert_eq!(distance(&copied), distance(&a));
    doc["buildings"][0]["surfaces"][0]["vertices"][0][1] = json!(3.1);
    b.install_building_collision(parse(&doc).unwrap());
    assert_ne!(a.state_hash(), b.state_hash());
    b.install_building_collision(BuildingCollision::default());
    assert_eq!(b.state_hash(), legacy_hash);
    assert_eq!(distance(&b), 4.0);

    doc["buildings"][0]["surfaces"][0]["indices"][2] = json!(3);
    assert!(parse(&doc)
        .err()
        .unwrap()
        .to_string()
        .contains("index out of bounds"));
    doc["buildings"][0]["surfaces"][0]["indices"][2] = json!(0);
    assert!(parse(&doc)
        .err()
        .unwrap()
        .to_string()
        .contains("degenerate triangle"));
    let bytes = vec![b' '; frontline_sim::collision::MAX_BUILDING_COLLISION_BYTES + 1];
    assert!(BuildingCollision::from_bytes(&bytes)
        .err()
        .unwrap()
        .to_string()
        .contains("exceeds"));
}
