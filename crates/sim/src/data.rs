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
    AmmoDef, BotsRulesDef, DataStatus, ExplosiveDef, Faction, GadgetDef, LoadoutDef, MaterialDef,
    MeleeDef, RespawnRulesDef, RoleDef, RulesDef, VehicleDef, WeaponDef,
};
/// SHA-256 (first 8 bytes, little-endian) of the canonicalised `/data` tree. Sent as
/// `Hello.data_hash`; client and server must agree (ARCHITECTURE 8.2).
pub use embedded::DATA_HASH;

use std::sync::OnceLock;

/// All embedded game data. Slices are `'static` (leaked once at first use).
#[derive(Debug)]
pub struct GameData {
    pub weapons: &'static [WeaponDef],
    /// Grenades, smoke and the Panzerfaust (`data/explosives/`, M1-008b).
    pub explosives: &'static [ExplosiveDef],
    pub melee: &'static [MeleeDef],
    pub ammo: &'static [AmmoDef],
    /// Penetration materials (`data/materials/`, M1-012).
    pub materials: &'static [MaterialDef],
    pub gadgets: &'static [GadgetDef],
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
            explosives: leak(embedded::EXPLOSIVES_JSON),
            melee: leak(embedded::MELEE_JSON),
            ammo: leak(embedded::AMMO_JSON),
            materials: leak(embedded::MATERIALS_JSON),
            gadgets: leak(embedded::GADGETS_JSON),
            roles: leak(embedded::ROLES_JSON),
            loadouts: leak(embedded::LOADOUTS_JSON),
            vehicles: leak(embedded::VEHICLES_JSON),
            rules: Box::leak(Box::new(RulesDef {
                respawn: serde_json::from_str(embedded::RESPAWN_JSON)
                    .expect("embedded data parsed at build time"),
                bots: serde_json::from_str(embedded::BOTS_JSON)
                    .expect("embedded data parsed at build time"),
                capture: serde_json::from_str(embedded::CAPTURE_JSON)
                    .expect("embedded data parsed at build time"),
                match_m1: serde_json::from_str(embedded::MATCH_M1_JSON)
                    .expect("embedded data parsed at build time"),
                supplies: serde_json::from_str(embedded::SUPPLIES_JSON)
                    .expect("embedded data parsed at build time"),
                squads: serde_json::from_str(embedded::SQUADS_JSON)
                    .expect("embedded data parsed at build time"),
                aoi: serde_json::from_str(embedded::AOI_JSON)
                    .expect("embedded data parsed at build time"),
            })),
            hash: DATA_HASH,
        })
    }

    pub fn weapon(&self, id: &str) -> Option<&'static WeaponDef> {
        self.weapons.iter().find(|w| w.id == id)
    }
    pub fn explosive(&self, id: &str) -> Option<&'static ExplosiveDef> {
        self.explosives.iter().find(|e| e.id == id)
    }
    pub fn melee_weapon(&self, id: &str) -> Option<&'static MeleeDef> {
        self.melee.iter().find(|m| m.id == id)
    }
    pub fn gadget(&self, id: &str) -> Option<&'static GadgetDef> {
        self.gadgets.iter().find(|g| g.id == id)
    }
    pub fn ammo_def(&self, id: &str) -> Option<&'static AmmoDef> {
        self.ammo.iter().find(|a| a.id == id)
    }
    pub fn material(&self, id: &str) -> Option<&'static MaterialDef> {
        self.materials.iter().find(|m| m.id == id)
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
    fn data_m1_008b_explosives_melee_ammo_load() {
        use super::defs::{ExplosiveClass, ExplosiveGameplay};
        let d = GameData::embedded();
        assert_eq!(d.explosives.len(), 5);
        assert_eq!(d.melee.len(), 6);
        assert_eq!(d.ammo.len(), 6);
        for e in d.explosives {
            // the typed gameplay variant agrees with the declared class
            let ok = matches!(
                (e.class, &e.gameplay),
                (ExplosiveClass::FragGrenade, ExplosiveGameplay::Frag(_))
                    | (ExplosiveClass::SmokeGrenade, ExplosiveGameplay::Smoke(_))
                    | (ExplosiveClass::AtRocket, ExplosiveGameplay::Rocket(_))
            );
            assert!(ok, "{}: class/gameplay mismatch", e.id);
        }
        let ExplosiveGameplay::Frag(g) = &d.explosive("us_mk2").unwrap().gameplay else {
            panic!("us_mk2 is a frag grenade")
        };
        assert_eq!((g.fuse_s, g.lethal_radius_m), (4.0, 3.2));
        assert_eq!(
            d.melee_weapon("us_m1_bayonet").unwrap().gameplay.damage_hp,
            90.0
        );
        assert_eq!(d.ammo_def("ger_9x19").unwrap().gameplay.drag_k_per_m, 0.002);
        // ids are unique across the weapon-slot id space (weapons + explosives + melee)
        let mut ids: Vec<&str> = d.weapons.iter().map(|w| w.id.as_str()).collect();
        ids.extend(d.explosives.iter().map(|e| e.id.as_str()));
        ids.extend(d.melee.iter().map(|m| m.id.as_str()));
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    #[test]
    fn roles_match_gd_4_4() {
        // (role, max_per_squad, squad_size_min); Crewman is team-capped instead (GAMEDESIGN 4.4).
        let d = GameData::embedded();
        for (id, cap, min) in [
            ("officer", 1, 2),
            ("rifleman", 6, 1),
            ("medic", 2, 2),
            ("support", 1, 3),
        ] {
            let r = d.role(id).unwrap_or_else(|| panic!("role {id}"));
            assert_eq!(r.max_per_squad, Some(cap), "{id} max_per_squad");
            assert_eq!(
                r.unlock.as_ref().unwrap().squad_size_min,
                min,
                "{id} squad_size_min"
            );
        }
        assert_eq!(d.role("crewman").unwrap().max_per_team, Some(8));
        assert_eq!(d.roles.len(), 5);
    }

    #[test]
    fn loadouts_reference_existing_data() {
        let d = GameData::embedded();
        assert_eq!(d.loadouts.len(), 13);
        let weapon_space = |id: &str| {
            d.weapon(id).is_some() || d.explosive(id).is_some() || d.melee_weapon(id).is_some()
        };
        let mut refs = 0;
        for l in d.loadouts {
            let mut ids = vec![l.primary.as_str()];
            ids.extend(l.secondary.as_deref());
            ids.extend(l.melee.as_deref());
            for id in ids {
                assert!(weapon_space(id), "{}: unknown weapon {id}", l.id);
                refs += 1;
            }
            for g in [&l.gadget_1, &l.gadget_2].into_iter().flatten() {
                assert!(
                    weapon_space(&g.id) || d.gadget(&g.id).is_some(),
                    "{}: {}",
                    l.id,
                    g.id
                );
                refs += 1;
            }
            for t in &l.tools {
                assert!(d.gadget(&t.id).is_some(), "{}: unknown tool {}", l.id, t.id);
                refs += 1;
            }
            assert_eq!(
                l.tags.contains(&"m1_only".to_string()),
                l.id == "ger_rifleman_pf60"
            );
        }
        assert!(refs > 60);
        // every role's loadouts exist and name that role and faction
        for r in d.roles {
            for (fac, o) in &r.faction_overrides {
                for lid in &o.loadout_ids {
                    let l = d.loadout(lid).unwrap();
                    assert_eq!(l.role, r.id);
                    assert_eq!(format!("{:?}", l.faction).to_lowercase(), *fac);
                }
            }
        }
    }

    #[test]
    fn m4a1_sherman_matches_gd_14() {
        let d = GameData::embedded();
        let v = d.vehicle("us_m4a1_75").expect("us_m4a1_75");
        let g = &v.gameplay;
        assert_eq!(g.hull_health_hp, 1600.0);
        assert_eq!(g.seats.len(), 3);
        // TM 9-2800 p. 109: maximum allowable speed is 24 mph.
        let historical_mps = 24.0 * 0.44704;
        assert_eq!(
            v.historical.max_speed_mps,
            defs::Hist::Known(historical_mps)
        );
        assert!((g.max_speed_mps.unwrap() - historical_mps * 0.75).abs() < 1e-6);
        assert_eq!(g.armor_zones.len(), 9);
        let plates: Vec<(f64, f64)> = g
            .armor_zones
            .iter()
            .map(|z| (z.thickness_mm, z.slope_deg))
            .collect();
        assert_eq!(
            plates,
            [
                (51.0, 56.0),
                (51.0, 45.0),
                (38.0, 0.0),
                (38.0, 0.0),
                (19.0, 90.0),
                (76.0, 30.0),
                (51.0, 25.0),
                (51.0, 25.0),
                (25.0, 90.0)
            ]
        );
        let hp = |id: &str| g.modules.iter().find(|m| m.id == id).unwrap().health_hp;
        assert_eq!(
            [
                hp("engine"),
                hp("track_l"),
                hp("track_r"),
                hp("turret_drive"),
                hp("main_gun"),
                hp("ammo_rack")
            ],
            [300.0, 250.0, 250.0, 200.0, 250.0, 150.0]
        );
        let main = g
            .weapons
            .iter()
            .find(|w| w["id"] == "main_gun_75mm_m3")
            .unwrap();
        assert_eq!(
            main["ammo_rounds"],
            serde_json::json!({"ap": 30, "he": 24, "wp": 6})
        );
        assert_eq!(main["reload_s"], 3.0);
        let rack = g.modules.iter().find(|m| m.id == "ammo_rack").unwrap();
        assert_eq!(rack.effects["at_0"]["cookoff_chance_pct"], 60.0);
        assert_eq!(g.gun_elevation_deg, Some([-10.0, 25.0]));
        assert_eq!(g.turret_traverse_deg_s, Some(24.0));
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
                "explosive" => drop(
                    serde_json::from_str::<ExplosiveDef>(&text)
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                ),
                "melee" => drop(
                    serde_json::from_str::<MeleeDef>(&text)
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                ),
                "gadget" => drop(
                    serde_json::from_str::<GadgetDef>(&text)
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                ),
                "ammo" => drop(
                    serde_json::from_str::<AmmoDef>(&text)
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
        assert_eq!(n, 11);
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
