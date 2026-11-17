//! Terrain for client-side prediction and patrol points for the synthetic walker. A smaller twin
//! of the server's `world_loader` (the headless client must not depend on `server`).

use std::path::{Path, PathBuf};

use frontline_sim::{CollisionChunk, CollisionData};
use serde::Deserialize;

pub const REGEN_COMMAND: &str =
    "PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0";

pub struct MapData {
    pub collision: CollisionData,
    /// `(x, z)` of `spawns.json` `patrol_points`, in file order.
    pub patrol_points: Vec<[f32; 2]>,
    pub chunks: usize,
}

#[derive(Deserialize)]
struct SpawnsFile {
    #[serde(default)]
    patrol_points: Vec<Patrol>,
}
#[derive(Deserialize)]
struct Patrol {
    pos: [f32; 2],
}

/// `<explicit>` or the first ancestor of the cwd / executable holding `data/maps/<map>/meta.json`.
pub fn locate_data_dir(explicit: Option<&Path>, map: &str) -> Result<PathBuf, String> {
    let marker = |d: &Path| d.join("maps").join(map).join("meta.json").is_file();
    if let Some(d) = explicit {
        return if marker(d) {
            Ok(d.to_path_buf())
        } else {
            Err(format!("{} has no maps/{map}/meta.json", d.display()))
        };
    }
    let mut starts = Vec::new();
    if let Ok(c) = std::env::current_dir() {
        starts.push(c);
    }
    if let Some(p) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        starts.push(p);
    }
    for s in starts {
        for anc in s.ancestors() {
            let d = anc.join("data");
            if marker(&d) {
                return Ok(d);
            }
        }
    }
    Err(format!(
        "map '{map}' not found under any ancestor `data/`; regenerate the terrain with:\n  {REGEN_COMMAND}\nor pass --data-dir"
    ))
}

fn parse_chunk_name(name: &str) -> Option<(i32, i32)> {
    let rest = name.strip_prefix("c_")?.strip_suffix(".bin")?;
    let (bx, bz) = rest.split_once('_')?;
    Some((bx.parse().ok()?, bz.parse().ok()?))
}

pub fn load(data_dir: &Path, map: &str) -> Result<MapData, String> {
    let dir = data_dir.join("maps").join(map);
    let rd = std::fs::read_dir(dir.join("chunks"))
        .map_err(|e| format!("cannot read {}/chunks: {e}; {REGEN_COMMAND}", dir.display()))?;
    let mut names: Vec<(i32, i32, PathBuf)> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            parse_chunk_name(&n).map(|(bx, bz)| (bx, bz, e.path()))
        })
        .collect();
    if names.is_empty() {
        return Err(format!(
            "no collision chunks in {}; {REGEN_COMMAND}",
            dir.display()
        ));
    }
    names.sort();
    let mut chunks = Vec::with_capacity(names.len());
    for (bx, bz, p) in &names {
        let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
        chunks.push(
            CollisionChunk::from_bytes(*bx, *bz, &bytes)
                .map_err(|e| format!("{}: {e}", p.display()))?,
        );
    }
    let n = chunks.len();
    let spawns: SpawnsFile = std::fs::read(dir.join("spawns.json"))
        .map_err(|e| e.to_string())
        .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
        .map_err(|e| format!("spawns.json: {e}"))?;
    Ok(MapData {
        collision: CollisionData::from_chunks(chunks),
        patrol_points: spawns.patrol_points.iter().map(|p| p.pos).collect(),
        chunks: n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_names() {
        assert_eq!(parse_chunk_name("c_0_3.bin"), Some((0, 3)));
        assert_eq!(parse_chunk_name("c_-1_12.bin"), Some((-1, 12)));
        assert_eq!(parse_chunk_name("x_0_3.bin"), None);
    }

    #[test]
    fn missing_dir_names_regeneration() {
        let e = locate_data_dir(Some(Path::new("/nonexistent-frontline")), "m0").unwrap_err();
        assert!(e.contains("meta.json"), "{e}");
    }
}
