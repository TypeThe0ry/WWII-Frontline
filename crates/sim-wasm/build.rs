//! Embeds the client-visible part of `/data` (weapons and roles) as JSON for
//! `weaponDef(id)` / `roleDef(id)`. `sim` only exposes typed (Deserialize-only) defs, so the
//! JSON is re-read from the same files `sim` hashes; the research bookkeeping that the client
//! never needs (`_meta`, `historical._source`) is dropped to keep the wasm small.
use std::{env, fs, path::PathBuf};

fn collect(dir: &PathBuf, strip_source: bool) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    for p in files {
        let text = fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        let mut v: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        let obj = v.as_object_mut().expect("data file is an object");
        obj.remove("_meta");
        if strip_source {
            if let Some(h) = obj.get_mut("historical").and_then(|h| h.as_object_mut()) {
                h.remove("_source");
            }
        }
        let id = obj["id"].as_str().expect("data file has an id").to_string();
        out.push((id, serde_json::to_string(&v).expect("serialise")));
        println!("cargo:rerun-if-changed={}", p.display());
    }
    out
}

fn table(name: &str, rows: &[(String, String)]) -> String {
    let mut s = format!("pub static {name}: &[(&str, &str)] = &[\n");
    for (id, json) in rows {
        s.push_str(&format!("    ({id:?}, {json:?}),\n"));
    }
    s.push_str("];\n");
    s
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let data = manifest.join("../../data");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../data/weapons");
    println!("cargo:rerun-if-changed=../../data/roles");
    let weapons = collect(&data.join("weapons"), true);
    let roles = collect(&data.join("roles"), false);
    let src = format!(
        "{}{}",
        table("WEAPON_JSON", &weapons),
        table("ROLE_JSON", &roles)
    );
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("data_json.rs");
    fs::write(out, src).expect("write data_json.rs");
}
