//! Authority-side cover from the authored map vectors, independent of render assets.

use std::collections::BTreeMap;
use std::path::Path;

use frontline_sim::ballistics::materials::{MaterialId, MaterialTable};
use frontline_sim::collision::{CollisionData, CoverSlab};
use frontline_sim::throwables::Occlusion;
use serde::Deserialize;

use super::{map_asset_root, MapError};

#[derive(Deserialize)]
struct Collection {
    features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
    geometry: Geometry,
    properties: Properties,
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "coordinates")]
enum Geometry {
    Polygon(Vec<Vec<[f32; 2]>>),
    LineString(Vec<[f32; 2]>),
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Properties {
    #[serde(rename = "_osm_id")]
    id: serde_json::Value,
    style: String,
    levels: u8,
    gap: bool,
    height_m: f32,
    mound_width_m: f32,
    #[serde(rename = "building:material", alias = "material")]
    material: String,
}

pub(super) fn load(dir: &Path, col: &CollisionData) -> Result<Vec<CoverSlab>, MapError> {
    let root = map_asset_root(dir);
    let mats = MaterialTable::embedded();
    let material = |name| mats.id_of(name).expect("embedded cover material");
    let brick = material("brick");
    let stone = material("stone_concrete");
    let wood = material("thin_wood");
    let foliage = material("foliage");
    let wall_material = |p: &Properties| {
        if p.material == "wood" {
            (0.03, wood, Occlusion::Thin)
        } else if p.material == "stone" || p.style.starts_with("stone_") || p.style == "church" {
            (0.30, stone, Occlusion::Solid)
        } else {
            (0.30, brick, Occlusion::Solid)
        }
    };
    let mut cover = Vec::new();
    let managed = col.building_collision();
    let mut managed_materials = BTreeMap::new();
    // Per-slice vectors override full-map vectors when available. M0 ships neither.
    for filename in ["buildings.geojson", "hedgerows.geojson"] {
        let local = dir.join(filename);
        let path = if local.is_file() {
            local
        } else {
            root.join(filename)
        };
        let Some(data) = read_vectors(&path)? else {
            continue;
        };
        for Feature {
            geometry,
            properties: p,
        } in data.features
        {
            match geometry {
                Geometry::Polygon(rings) => {
                    let (width, mat, blast) = wall_material(&p);
                    if filename == "buildings.geojson" {
                        if let Some(id) = p.id.as_str().filter(|id| managed.walls(id).is_some()) {
                            managed_materials.insert(id.to_owned(), (mat, blast));
                            continue;
                        }
                    }
                    for ring in rings.iter().filter(|r| r.len() >= 3) {
                        for (&a, &b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
                            edge(
                                col,
                                &mut cover,
                                a,
                                b,
                                (f32::from(p.levels.max(1)) * 3.0, width, mat, blast),
                            );
                        }
                    }
                }
                Geometry::LineString(points) if !p.gap => {
                    for seg in points.windows(2) {
                        edge(
                            col,
                            &mut cover,
                            seg[0],
                            seg[1],
                            (p.height_m, p.mound_width_m, foliage, Occlusion::Clear),
                        );
                    }
                }
                Geometry::LineString(_) => {}
            }
        }
    }
    // A local vector override may subset a shared document. Use parent material metadata for
    // canonical IDs omitted locally, without restoring the parent's legacy slabs.
    let buildings_file = "buildings.geojson";
    if dir != root
        && dir.join(buildings_file).is_file()
        && managed
            .buildings()
            .any(|(id, _)| !managed_materials.contains_key(id))
    {
        if let Some(parent) = read_vectors(&root.join(buildings_file))? {
            for f in parent.features {
                if !matches!(f.geometry, Geometry::Polygon(_)) {
                    continue;
                }
                if let Some(id) = f
                    .properties
                    .id
                    .as_str()
                    .filter(|id| managed.walls(id).is_some())
                {
                    let (_, mat, blast) = wall_material(&f.properties);
                    managed_materials
                        .entry(id.to_owned())
                        .or_insert((mat, blast));
                }
            }
        }
    }
    // Canonical pieces are installed even if a slice-local vector list omits the building.
    // Missing material metadata has the legacy default (brick); dimensions come from the doc.
    for (id, walls) in managed.buildings() {
        let (mat, blast) = managed_materials
            .get(id)
            .copied()
            .unwrap_or((brick, Occlusion::Solid));
        cover.extend(
            walls
                .iter()
                .map(|&wall| CoverSlab::from_building_wall(wall, mat, blast)),
        );
    }
    Ok(cover)
}

fn read_vectors(path: &Path) -> Result<Option<Collection>, MapError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(MapError::Io {
                path: path.to_path_buf(),
                error: e.to_string(),
            })
        }
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| MapError::BadJson {
            path: path.to_path_buf(),
            error: e.to_string(),
        })
}

fn edge(
    col: &CollisionData,
    cover: &mut Vec<CoverSlab>,
    a: [f32; 2],
    b: [f32; 2],
    (height, width, material, blast): (f32, f32, MaterialId, Occlusion),
) {
    let mid = [a[0] + (b[0] - a[0]) * 0.5, a[1] + (b[1] - a[1]) * 0.5];
    let samples = [a, mid, b]
        .into_iter()
        .filter_map(|[x, z]| col.height_at(x, z));
    let (lo, hi) = samples.fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), y| {
        (lo.min(y), hi.max(y))
    });
    if let Some(slab) = CoverSlab::new(a, b, lo - 0.4, hi + height, width, material, blast) {
        cover.push(slab);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_loader::load_map;
    use frontline_sim::{
        GameData, GameMode, MapId, SimConfig, SimEvent, SpawnReq, Team, Vec3, World,
    };

    #[test]
    fn m1_building_doc_replaces_legacy_wall_with_aperture() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("maps/carentan");
        let m1 = root.join("m1");
        std::fs::create_dir_all(m1.join("chunks")).unwrap();
        // Valid flat zero-height FCOL chunk, with projectiles/LOS operated by the real map loader.
        let mut chunk = b"FCOL\x01\x00\x41\x00".to_vec();
        chunk.resize(16 + 65 * 65 * 2 + 64 * 64, 0);
        std::fs::write(m1.join("chunks/c_0_0.bin"), chunk).unwrap();
        std::fs::write(
            m1.join("spawns.json"),
            r#"{"hq":{"us":{"pos":[5,5]},"ger":{"pos":[60,60]}}}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("buildings.geojson"),
            r#"{"features":[
          {"geometry":{"type":"Polygon","coordinates":[[[27,32],[37,32],[37,42],[27,42],[27,32]]]},
           "properties":{"_osm_id":"way/door","building:material":"wood"}},
          {"geometry":{"type":"Polygon","coordinates":[[[42,32],[50,32],[50,42],[42,42],[42,32]]]},
           "properties":{"_osm_id":7,"style":"church"}}
        ]}"#,
        )
        .unwrap();
        let legacy = load_map(tmp.path(), "carentan/m1").unwrap().collision;
        let ray = |x, y| (Vec3::new(x, y, 31.0), Vec3::new(x, y, 33.0));
        let (from, to) = ray(32.0, 1.2);
        assert_eq!(legacy.cover_count(), 8);
        assert_eq!(legacy.physics().world().colliders.len(), 1);
        assert!(legacy.raycast_cover_segment(from, to).is_some());
        assert!(legacy.terrain_los(from, to));
        assert!(!legacy.los(from, to));

        let filename = "building-interiors.json";
        std::fs::write(root.join(filename), r#"{"version":1,"buildings":[{"id":"way/door",
            "walls":[
              {"a":[27,32],"b":[31.4,32],"bottom":-0.4,"top":3,"width":0.24},
              {"a":[32.6,32],"b":[37,32],"bottom":-0.4,"top":3,"width":0.24},
              {"a":[31.4,32],"b":[32.6,32],"bottom":2.2,"top":3,"width":0.24},
              {"a":[37,32],"b":[37,42],"bottom":-0.4,"top":3,"width":0.24},
              {"a":[37,42],"b":[27,42],"bottom":-0.4,"top":3,"width":0.24},
              {"a":[27,42],"b":[27,32],"bottom":-0.4,"top":3,"width":0.24}],
            "surfaces":[{"kind":"roof","vertices":[[27,3,32],[27,3,42],[37,3,32]],"indices":[0,1,2]}]
        }]}"#).unwrap();
        let current = load_map(tmp.path(), "carentan/m1").unwrap().collision;
        assert_eq!(current.cover_count(), 10);
        assert_eq!(current.physics().world().colliders.len(), 8);
        assert_eq!(current.height_at(32.0, 32.0), legacy.height_at(32.0, 32.0));
        for (a, b) in [(from, to), (to, from)] {
            assert!(current.raycast_cover_segment(a, b).is_none());
            assert_eq!(current.blast_cover(a, b), Occlusion::Clear);
            assert!(current.los(a, b));
        }
        let mats = MaterialTable::embedded();
        for (x, y, material, blast) in [
            (30.8, 1.2, "thin_wood", Occlusion::Thin), // adjacent wall
            (32.0, 2.5, "thin_wood", Occlusion::Thin), // lintel
            (45.0, 1.2, "stone_concrete", Occlusion::Solid), // unrelated legacy wall
        ] {
            let (a, b) = ray(x, y);
            let hit = current.raycast_cover_segment(a, b).unwrap();
            assert_eq!(hit.material, mats.id_of(material).unwrap().0);
            assert_eq!(current.blast_cover(a, b), blast);
            assert!(current
                .raycast(a, (b - a).normalize(), a.distance(b))
                .is_none());
        }
        let (a, b) = ray(30.8, 1.2);
        let distance = |col: &CollisionData| col.raycast_cover_segment(a, b).unwrap().t;
        assert!(
            (distance(&current) - 0.88).abs() < 1e-5,
            "canonical width, not legacy wood width"
        );

        // Local vectors may omit shared IDs without losing canonical wall materials.
        std::fs::write(m1.join("buildings.geojson"), br#"{"features":[]}"#).unwrap();
        let subset = load_map(tmp.path(), "carentan/m1").unwrap().collision;
        assert_eq!(subset.cover_count(), 6);
        assert_eq!(subset.blast_cover(a, b), Occlusion::Thin);
        std::fs::remove_file(m1.join("buildings.geojson")).unwrap();

        // The local doc overrides the parent, including an explicitly empty set; a bad local
        // file is reported, not silently replaced with the parent's collision.
        let local = m1.join(filename);
        std::fs::write(&local, br#"{"version":1,"buildings":[]}"#).unwrap();
        let overridden = load_map(tmp.path(), "carentan/m1").unwrap().collision;
        assert_eq!(overridden.state_hash(), legacy.state_hash());
        std::fs::write(&local, br#"{"version":2,"buildings":[]}"#).unwrap();
        let err = load_map(tmp.path(), "carentan/m1").unwrap_err().to_string();
        assert!(
            err.contains(local.to_str().unwrap()) && err.contains("unsupported"),
            "{err}"
        );
    }

    #[test]
    #[ignore = "requires raw Carentan collision from the M1 map bake"]
    fn carentan_wall_is_used_by_authoritative_projectiles() {
        let data_dir = std::env::var_os("FRONTLINE_DATA_DIR").map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"),
            std::path::PathBuf::from,
        );
        let map = load_map(&data_dir, "carentan/m1").unwrap();
        let raw: Collection =
            super::super::json(&data_dir.join("maps/carentan/buildings.geojson")).unwrap();
        let solid = MaterialTable::embedded();
        let ray = raw
            .features
            .into_iter()
            .filter_map(|f| match f.geometry {
                Geometry::Polygon(rings) => rings.into_iter().next(),
                _ => None,
            })
            .find_map(|ring| {
                ring.windows(2).find_map(|e| {
                    let [a, b] = [e[0], e[1]].map(|[x, z]| Vec3::new(x, 0.0, z));
                    if a.distance(b) < 2.0 {
                        return None;
                    }
                    let mid = (a + b) * 0.5;
                    let eye = mid + Vec3::Y * (map.collision.height_at(mid.x, mid.z)? + 1.2);
                    let n = Vec3::Y.cross(b - a).normalize();
                    let (from, to) = (eye - n * 1.6, eye + n * 1.6);
                    let hit = map.collision.raycast_cover_segment(from, to)?;
                    (map.collision.terrain_los(from, to)
                        && matches!(
                            solid.name(MaterialId(hit.material)),
                            Some("brick" | "stone_concrete")
                        ))
                    .then_some((from, to, hit.material))
                })
            })
            .expect("an unobstructed ray to a masonry edge in the loaded slice");
        let (from, to, material) = ray;
        assert_eq!(map.collision.blast_cover(from, to), Occlusion::Solid);
        assert!(!map.collision.los(from, to));
        let data = GameData::embedded();
        let mut world = World::new(
            SimConfig {
                tick_hz: 30,
                match_seed: 1,
                map: MapId(0),
                mode: GameMode::Sandbox,
                data,
            },
            &map.collision,
        );
        world
            .spawn_player(SpawnReq {
                player: 0,
                team: Team::Us,
                pos: from,
                yaw: 0.0,
            })
            .unwrap();
        let weapon = data
            .weapons
            .iter()
            .position(|w| w.id == "us_m1_garand")
            .unwrap() as u16;
        world
            .spawn_projectile(0, weapon, from, (to - from).normalize() * 850.0)
            .unwrap();
        let out = world.step(&[]);
        assert!(out
            .events
            .iter()
            .any(|e| matches!(e, SimEvent::Impact { material: m, .. } if *m == material)));
        assert!(world.projectiles().is_empty());
        println!(
            "Carentan: {} terrain chunks, {} installed graybox slabs, {} wall impact",
            map.chunk_count,
            map.collision.cover_count(),
            solid.name(MaterialId(material)).unwrap()
        );
    }
}
