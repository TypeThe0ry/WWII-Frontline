//! Authority-side cover from the authored map vectors, independent of render assets.

use std::path::Path;

use frontline_sim::ballistics::materials::{MaterialId, MaterialTable};
use frontline_sim::collision::{CollisionData, CoverSlab};
use frontline_sim::throwables::Occlusion;
use serde::Deserialize;

use super::MapError;

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
    style: String,
    levels: u8,
    gap: bool,
    height_m: f32,
    mound_width_m: f32,
    #[serde(rename = "building:material", alias = "material")]
    material: String,
}

pub(super) fn load(dir: &Path, col: &CollisionData) -> Result<Vec<CoverSlab>, MapError> {
    let root = if dir.file_name().is_some_and(|name| name == "m1") {
        dir.parent().unwrap_or(dir)
    } else {
        dir
    };
    let mats = MaterialTable::embedded();
    let material = |name| mats.id_of(name).expect("embedded cover material");
    let brick = material("brick");
    let stone = material("stone_concrete");
    let wood = material("thin_wood");
    let foliage = material("foliage");
    let mut cover = Vec::new();
    // Per-slice vectors override full-map vectors when available. M0 ships neither.
    for filename in ["buildings.geojson", "hedgerows.geojson"] {
        let local = dir.join(filename);
        let path = if local.is_file() {
            local
        } else {
            root.join(filename)
        };
        let data = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(MapError::Io {
                    path,
                    error: e.to_string(),
                })
            }
        };
        let data: Collection = serde_json::from_slice(&data).map_err(|e| MapError::BadJson {
            path: path.clone(),
            error: e.to_string(),
        })?;
        for Feature {
            geometry,
            properties: p,
        } in data.features
        {
            match geometry {
                Geometry::Polygon(rings) => {
                    let (width, mat, blast) = if p.material == "wood" {
                        (0.03, wood, Occlusion::Thin)
                    } else if p.material == "stone"
                        || p.style.starts_with("stone_")
                        || p.style == "church"
                    {
                        (0.30, stone, Occlusion::Solid)
                    } else {
                        (0.30, brick, Occlusion::Solid)
                    };
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
    Ok(cover)
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
    #[ignore = "requires raw Carentan collision from the M1 map bake"]
    fn carentan_wall_is_used_by_authoritative_projectiles() {
        let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
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
