//! Embeds /data into `sim`: type-checks every data file against the typed defs (a
//! mismatch fails the build), computes `DATA_HASH` and writes the canonical JSON that
//! `data.rs` loads at runtime. Full JSON Schema validation is `tools/data-validate`.

use std::{env, fs, path::PathBuf};

#[allow(dead_code)]
mod defs {
    include!("src/data_defs.rs");
}
#[allow(dead_code)]
mod hash {
    include!("src/data_hash.rs");
}

fn parse_typed(rel: &str, text: &str) -> Result<(), String> {
    macro_rules! typed {
        ($t:ty) => {
            serde_json::from_str::<$t>(text)
                .map(|_| ())
                .map_err(|e| format!("{rel}: {e}"))
        };
    }
    match rel.split('/').next().unwrap_or("") {
        "economy.json" if rel == "economy.json" => typed!(defs::EconomyDef),
        "weapons" if rel.matches('/').count() == 1 => typed!(defs::WeaponDef),
        "explosives" if rel.matches('/').count() == 1 => typed!(defs::ExplosiveDef),
        "melee" if rel.matches('/').count() == 1 => typed!(defs::MeleeDef),
        "ammo" if rel.matches('/').count() == 1 => typed!(defs::AmmoDef),
        "materials" if rel.matches('/').count() == 1 => typed!(defs::MaterialDef),
        "gadgets" if rel.matches('/').count() == 1 => typed!(defs::GadgetDef),
        "roles" if rel.matches('/').count() == 1 => typed!(defs::RoleDef),
        "loadouts" if rel.matches('/').count() == 1 => typed!(defs::LoadoutDef),
        "vehicles" if rel.matches('/').count() == 1 => typed!(defs::VehicleDef),
        "structures"
            if rel.starts_with("structures/resource_node_") && rel.matches('/').count() == 1 =>
        {
            typed!(defs::ResourceNodeDef)
        }
        "rules" if rel == "rules/respawn.json" => typed!(defs::RespawnRulesDef),
        "rules" if rel == "rules/bots.json" => typed!(defs::BotsRulesDef),
        "rules" if rel == "rules/capture.json" => typed!(defs::CaptureRulesDef),
        "rules" if rel == "rules/match_m1.json" => typed!(defs::MatchRulesDef),
        "rules" if rel == "rules/supplies.json" => typed!(defs::SuppliesRulesDef),
        "rules" if rel == "rules/squads.json" => typed!(defs::SquadsRulesDef),
        "rules" if rel == "rules/aoi.json" => typed!(defs::AoiRulesDef),
        "comms.json" => typed!(defs::CommsDef),
        "structures" if rel == "structures/outpost.json" => typed!(defs::OutpostDef),
        _ => Ok(()), // maps/, other rules/, ...: hashed only until their schemas land
    }
}

fn canonical_required(files: &[(String, String)], rel: &str) -> String {
    let (_, text) = files
        .iter()
        .find(|(r, _)| r == rel)
        .unwrap_or_else(|| panic!("data/{rel} is required"));
    let value = serde_json::from_str(text).expect("validated above");
    hash::canonical_json(&value)
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let data_root = manifest.join("../../data");
    println!("cargo:rerun-if-changed=../../data");
    println!("cargo:rerun-if-changed=src/data_defs.rs");
    println!("cargo:rerun-if-changed=src/data_hash.rs");

    let files =
        hash::collect_data_files(&data_root).unwrap_or_else(|e| panic!("reading /data: {e}"));
    for (rel, text) in &files {
        if let Err(e) = parse_typed(rel, text) {
            panic!("data validation failed: {e}");
        }
    }
    let data_hash =
        hash::hash_files(&files).unwrap_or_else(|e| panic!("data validation failed: {e}"));

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let mut embedded = format!("pub const DATA_HASH: u64 = {data_hash:#018x};\n");
    for (dir, konst) in [
        ("weapons", "WEAPONS_JSON"),
        ("explosives", "EXPLOSIVES_JSON"),
        ("melee", "MELEE_JSON"),
        ("ammo", "AMMO_JSON"),
        ("materials", "MATERIALS_JSON"),
        ("gadgets", "GADGETS_JSON"),
        ("roles", "ROLES_JSON"),
        ("loadouts", "LOADOUTS_JSON"),
        ("vehicles", "VEHICLES_JSON"),
    ] {
        let items: Vec<String> = files
            .iter()
            .filter(|(rel, _)| rel.starts_with(&format!("{dir}/")) && rel.matches('/').count() == 1)
            .map(|(_, text)| {
                let v: serde_json::Value = serde_json::from_str(text).expect("validated above");
                hash::canonical_json(&v)
            })
            .collect();
        let file = format!("{dir}.json");
        fs::write(out.join(&file), format!("[{}]", items.join(","))).expect("write embedded json");
        embedded.push_str(&format!(
            "pub const {konst}: &str = include_str!(concat!(env!(\"OUT_DIR\"), \"/{file}\"));\n"
        ));
    }
    let nodes = ["manpower", "munitions", "fuel"]
        .map(|kind| canonical_required(&files, &format!("structures/resource_node_{kind}.json")));
    fs::write(
        out.join("resource_nodes.json"),
        format!("[{}]", nodes.join(",")),
    )
    .expect("write embedded json");
    embedded.push_str(
        "pub const RESOURCE_NODES_JSON: &str = include_str!(concat!(env!(\"OUT_DIR\"), \"/resource_nodes.json\"));\n",
    );
    for (rel, name, konst) in [
        (
            "structures/repair_station.json",
            "repair_station.json",
            "REPAIR_STATION_JSON",
        ),
        (
            "emplacements/us_at_57mm_m1.json",
            "us_at_57mm_m1.json",
            "US_AT_JSON",
        ),
        (
            "emplacements/ger_at_75mm_pak40.json",
            "ger_at_75mm_pak40.json",
            "GER_AT_JSON",
        ),
        ("structures/outpost.json", "outpost.json", "OUTPOST_JSON"),
        ("economy.json", "economy.json", "ECONOMY_JSON"),
        ("rules/respawn.json", "respawn.json", "RESPAWN_JSON"),
        ("rules/bots.json", "bots.json", "BOTS_JSON"),
        ("rules/capture.json", "capture.json", "CAPTURE_JSON"),
        ("rules/match_m1.json", "match_m1.json", "MATCH_M1_JSON"),
        ("rules/supplies.json", "supplies.json", "SUPPLIES_JSON"),
        ("rules/squads.json", "squads.json", "SQUADS_JSON"),
        ("rules/aoi.json", "aoi.json", "AOI_JSON"),
        ("comms.json", "comms.json", "COMMS_JSON"),
    ] {
        fs::write(out.join(name), canonical_required(&files, rel)).expect("write embedded json");
        embedded.push_str(&format!(
            "pub const {konst}: &str = include_str!(concat!(env!(\"OUT_DIR\"), \"/{name}\"));\n"
        ));
    }
    fs::write(out.join("data_embedded.rs"), embedded).expect("write data_embedded.rs");
}
