//! Game data embedded at build time from `/data` (ARCHITECTURE 4.4, 8.1, 8.5).
//!
//! `build.rs` parses every data file into the typed defs below (build fails on mismatch),
//! computes [`DATA_HASH`] and embeds canonical JSON; [`GameData::embedded`] materialises it.
//! `sim` does not use the data for any game logic yet (M0-007 onwards).

#[allow(clippy::module_inception)]
pub mod defs {
    include!("data_defs.rs");
}
pub mod hash {
    include!("data_hash.rs");
}
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/data_embedded.rs"));
}

pub use defs::{
    DataStatus, Faction, LoadoutDef, RespawnRulesDef, RoleDef, RulesDef, VehicleDef, WeaponDef,
};
/// SHA-256 (first 8 bytes, little-endian) of the canonicalised `/data` tree. Sent as
/// `Hello.data_hash`; client and server must agree (ARCHITECTURE 8.2).
pub use embedded::DATA_HASH;

use std::sync::OnceLock;

/// All embedded game data. Slices are `'static` (leaked once at first use).
#[derive(Debug)]
pub struct GameData {
    pub weapons: &'static [WeaponDef],
    pub roles: &'static [RoleDef],
    pub loadouts: &'static [LoadoutDef],
    pub vehicles: &'static [VehicleDef],
    /// Rule families from `data/rules/` (M0-013).
    pub rules: &'static RulesDef,
    pub hash: u64,
}

fn leak<T: serde::de::DeserializeOwned>(json: &str) -> &'static [T] {
    // Build-time parsing already succeeded for the same types, so this cannot fail.
    let v: Vec<T> = serde_json::from_str(json).expect("embedded data parsed at build time");
    Box::leak(v.into_boxed_slice())
}

impl GameData {
    /// The data embedded in this build.
    pub fn embedded() -> &'static GameData {
        static DATA: OnceLock<GameData> = OnceLock::new();
        DATA.get_or_init(|| GameData {
            weapons: leak(embedded::WEAPONS_JSON),
            roles: leak(embedded::ROLES_JSON),
            loadouts: leak(embedded::LOADOUTS_JSON),
            vehicles: leak(embedded::VEHICLES_JSON),
            rules: Box::leak(Box::new(RulesDef {
                respawn: serde_json::from_str(embedded::RESPAWN_JSON)
                    .expect("embedded data parsed at build time"),
            })),
            hash: DATA_HASH,
        })
    }

    pub fn weapon(&self, id: &str) -> Option<&'static WeaponDef> {
        self.weapons.iter().find(|w| w.id == id)
    }
    pub fn role(&self, id: &str) -> Option<&'static RoleDef> {
        self.roles.iter().find(|r| r.id == id)
    }
    pub fn loadout(&self, id: &str) -> Option<&'static LoadoutDef> {
        self.loadouts.iter().find(|l| l.id == id)
    }
    pub fn vehicle(&self, id: &str) -> Option<&'static VehicleDef> {
        self.vehicles.iter().find(|v| v.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::hash::{canonical_json, collect_data_files, hash_files};
    use super::*;
    use std::path::{Path, PathBuf};

    fn data_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    fn sample() -> Vec<(String, String)> {
        vec![
            (
                "weapons/a.json".into(),
                r#"{"id":"a","x_m":1.5,"list":[1,2,{"k":true}]}"#.into(),
            ),
            (
                "roles/r.json".into(),
                r#"{"id":"r","health_hp":100}"#.into(),
            ),
        ]
    }

    #[test]
    fn data_hash_stable() {
        let a = hash_files(&sample()).unwrap();
        // same content, again
        assert_eq!(a, hash_files(&sample()).unwrap());
        // file order does not matter
        let mut rev = sample();
        rev.reverse();
        assert_eq!(a, hash_files(&rev).unwrap());
        // re-indenting / key reordering does not change the hash
        let reformatted = vec![
            (
                "weapons/a.json".to_string(),
                "{\n  \"list\": [1, 2, {\n    \"k\": true }],\n  \"x_m\": 1.5,\n  \"id\": \"a\"\n}\n".to_string(),
            ),
            ("roles/r.json".to_string(), "{ \"health_hp\" : 100,\n\t\"id\": \"r\" }".to_string()),
        ];
        assert_eq!(a, hash_files(&reformatted).unwrap());
        // changing one value changes it
        let mut changed = sample();
        changed[0].1 = changed[0].1.replace("1.5", "1.6");
        assert_ne!(a, hash_files(&changed).unwrap());
        // renaming a file changes it
        let mut renamed = sample();
        renamed[1].0 = "roles/r2.json".into();
        assert_ne!(a, hash_files(&renamed).unwrap());
    }

    #[test]
    fn data_hash_matches_tree() {
        // The compiled-in constant equals a fresh walk of /data (also pins the walker).
        let files = collect_data_files(&data_root()).unwrap();
        assert!(files.iter().all(|(p, _)| !p.starts_with("schemas/")));
        assert_eq!(hash_files(&files).unwrap(), DATA_HASH);
        assert_eq!(GameData::embedded().hash, DATA_HASH);
    }

    #[test]
    fn data_canonical_json_sorted_compact() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{ "b": [1, 2], "a": {"z": 1, "y": "s"} }"#).unwrap();
        assert_eq!(canonical_json(&v), r#"{"a":{"y":"s","z":1},"b":[1,2]}"#);
    }

    #[test]
    fn data_fixtures_parse_into_typed_defs() {
        // Valid schema fixtures must also satisfy the typed (deny_unknown_fields) defs.
        let dir = data_root().join("schemas/fixtures/valid");
        let mut n = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let p = entry.unwrap().path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            let text = std::fs::read_to_string(&p).unwrap();
            match name.split('.').next().unwrap() {
                "weapon" => drop(
                    serde_json::from_str::<WeaponDef>(&text)
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                ),
                "role" => drop(
                    serde_json::from_str::<RoleDef>(&text)
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                ),
                "loadout" => drop(
                    serde_json::from_str::<LoadoutDef>(&text)
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                ),
                "vehicle" => drop(
                    serde_json::from_str::<VehicleDef>(&text)
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                ),
                _ => continue,
            }
            n += 1;
        }
        assert_eq!(n, 6);
    }

    #[test]
    fn data_typed_defs_reject_bad_input() {
        let dir = data_root().join("schemas/fixtures/valid");
        let text = std::fs::read_to_string(dir.join("weapon.us_fixture_rifle.json")).unwrap();
        // missing required field
        let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
        v["historical"].as_object_mut().unwrap().remove("_source");
        assert!(serde_json::from_value::<WeaponDef>(v).is_err());
        // unknown field
        let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
        v["gameplay"]["bonus_hp"] = 5.into();
        assert!(serde_json::from_value::<WeaponDef>(v).is_err());
    }

    #[test]
    fn data_embedded_excludes_schema_fixtures() {
        // Fixtures live under data/schemas/ and are never part of the embedded set.
        let d = GameData::embedded();
        assert!(d.weapon("us_fixture_rifle").is_none());
        assert_eq!(d.hash, DATA_HASH);
    }
}
