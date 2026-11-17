// DATA_HASH machinery, shared by `build.rs` (computes the constant) and `data.rs`
// (re-computes it in tests). Pure std + serde_json + sha2; no sim types.
//
// Algorithm (ARCHITECTURE 4.4 / task M0-004): every `.json` under /data except
// `schemas/`, ordered by relative path (`/` separators, byte order); each file is parsed,
// re-serialised canonically (object keys sorted, no whitespace); SHA-256 over
// `path 0x00 canonical 0x0A` for each file; first 8 bytes little-endian -> u64.
// Pure re-formatting never changes the hash; any value (or path) change does.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, io, path::Path};

/// Canonical text of a JSON value: keys sorted, no insignificant whitespace.
pub fn canonical_json(v: &Value) -> String {
    let mut out = String::new();
    write_canonical(v, &mut out);
    out
}

fn write_canonical(v: &Value, out: &mut String) {
    match v {
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(x, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(k.clone()).to_string());
                out.push(':');
                write_canonical(&m[k], out);
            }
            out.push('}');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// Hash `(relative_path, raw_json_text)` pairs. Sorts by path itself.
pub fn hash_files(files: &[(String, String)]) -> Result<u64, String> {
    let mut sorted: Vec<&(String, String)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut h = Sha256::new();
    for (path, text) in sorted {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("{path}: invalid JSON: {e}"))?;
        h.update(path.as_bytes());
        h.update([0u8]);
        h.update(canonical_json(&v).as_bytes());
        h.update(*b"\n");
    }
    let d = h.finalize();
    let mut b = [0u8; 8];
    b.copy_from_slice(&d[..8]);
    Ok(u64::from_le_bytes(b))
}

/// Collect `(relative_path, text)` for every `.json` under `root` except `schemas/`.
pub fn collect_data_files(root: &Path) -> io::Result<Vec<(String, String)>> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let p = entry?.path();
            let rel = p
                .strip_prefix(root)
                .map_err(io::Error::other)?
                .to_string_lossy()
                .replace('\\', "/");
            if p.is_dir() {
                if rel == "schemas" {
                    continue;
                }
                walk(&p, root, out)?;
            } else if rel.ends_with(".json") {
                out.push((rel, fs::read_to_string(&p)?));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}
