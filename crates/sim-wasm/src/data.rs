//! Data queries: `weaponDef`, `roleDef` (JSON bytes) and the weapon index used by `WeaponState`.

use wasm_bindgen::prelude::*;

use crate::js_err;

mod tables {
    include!(concat!(env!("OUT_DIR"), "/data_json.rs"));
}

fn lookup(table: &'static [(&'static str, &'static str)], id: &str) -> Result<Vec<u8>, String> {
    table
        .iter()
        .find(|(k, _)| *k == id)
        .map(|(_, v)| v.as_bytes().to_vec())
        .ok_or_else(|| format!("unknown data id {id:?}"))
}

pub(crate) fn weapon_def_core(id: &str) -> Result<Vec<u8>, String> {
    lookup(tables::WEAPON_JSON, id)
}

pub(crate) fn role_def_core(id: &str) -> Result<Vec<u8>, String> {
    lookup(tables::ROLE_JSON, id)
}

/// Weapon definition as UTF-8 JSON (without `_meta` and `historical._source`).
#[wasm_bindgen(js_name = weaponDef)]
pub fn weapon_def(id: &str) -> Result<Vec<u8>, JsError> {
    weapon_def_core(id).map_err(js_err)
}

/// Role definition as UTF-8 JSON (includes `capsule` and `movement`).
#[wasm_bindgen(js_name = roleDef)]
pub fn role_def(id: &str) -> Result<Vec<u8>, JsError> {
    role_def_core(id).map_err(js_err)
}

/// Index of a weapon in the game data: the `WeaponState.def_id` / `Predictor.equipWeapon` key.
#[wasm_bindgen(js_name = weaponIndex)]
pub fn weapon_index(id: &str) -> Result<u32, JsError> {
    frontline_sim::GameData::embedded()
        .weapons
        .iter()
        .position(|w| w.id == id)
        .map(|i| i as u32)
        .ok_or_else(|| js_err(format!("unknown weapon {id:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defs_are_json_and_stripped() {
        let w = weapon_def_core("us_m1_garand").unwrap();
        let v: serde_json_probe::V = serde_json_probe::parse(&w);
        assert!(v.has("gameplay") && !v.has("_meta"));
        let r = role_def_core("rifleman").unwrap();
        let text = String::from_utf8(r).unwrap();
        assert!(text.contains("\"capsule\"") && text.contains("height_stand_m"));
        assert!(weapon_def_core("nope").is_err());
    }

    // Tiny substring probe: avoids a serde_json dev-dependency for two assertions.
    mod serde_json_probe {
        pub struct V(pub String);
        pub fn parse(b: &[u8]) -> V {
            V(String::from_utf8(b.to_vec()).unwrap())
        }
        impl V {
            pub fn has(&self, k: &str) -> bool {
                self.0.contains(&format!("\"{k}\":"))
            }
        }
    }
}
