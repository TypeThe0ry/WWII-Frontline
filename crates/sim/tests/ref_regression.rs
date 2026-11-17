//! M0-032: regression of calibrated / locked `/data` fields against the reference measurements
//! (REFERENCE_MEASUREMENTS 7.5). **No reference number is hard-coded here**: the test reads, from
//! every `data/weapons/*.json` and `data/roles/*.json`, the optional block
//!
//! ```json
//! "_meta": { "status": "calibrated|locked", "ref": {
//!     "gameplay.fire.cap_rpm": { "ref_id": "W-01", "ref_version": "<build>", "ref_median": <number>,
//!                                "kind": "timing|speed|distance|angle|stat|ratio|count|curve",
//!                                "state": "calibrated|locked", "ref_unc": <number, optional>,
//!                                "tol_abs": <number, optional override> } } }
//! ```
//!
//! and for each entry in state `locked` re-measures the quantity with `own_measure` (the same
//! definition as the recording analysis) and requires it to lie within the 7.3 tolerance of
//! `ref_median` (or `tol_abs`). The block is written by the B part of M0-032 once recordings exist
//! (schema + `data_defs.rs` get the optional `_meta.ref` field in the same change, PLAN_ISSUES
//! PI-027); until then no file carries it and the data-driven test passes with 0 locked entries.
mod common;
mod own_measure;

use std::path::PathBuf;

use own_measure::{measure_all, Entry};
use serde_json::{json, Value};

const FRAME_60_S: f64 = 1.0 / 60.0;

/// REFERENCE_MEASUREMENTS 7.3 (mirror of `tools/ref-analysis/ref_analysis/stats.py::tolerance_abs`).
fn tolerance_abs(kind: &str, target: f64, ref_unc: f64) -> Result<f64, String> {
    let a = target.abs();
    let t = match kind {
        "timing" => (0.05 * a).max(FRAME_60_S),
        "speed" => 0.03 * a,
        "distance" => (0.05 * a).max(1.0),
        "angle" => (0.05 * a).max(0.1),
        "stat" | "ratio" | "curve" => 0.10 * a,
        "count" => return Ok(0.0),
        other => return Err(format!("unknown tolerance kind {other:?}")),
    };
    Ok(t.max(ref_unc.abs()))
}

/// Checks one `_meta.ref` entry against the own re-measurement. `key` is the own entry id.
fn check_entry(own: &[Entry], key: &str, r: &Value) -> Result<(), String> {
    let kind = r["kind"].as_str().ok_or("missing kind")?;
    let target = r["ref_median"].as_f64().ok_or("missing ref_median")?;
    let unc = r["ref_unc"].as_f64().unwrap_or(0.0);
    let tol = match r.get("tol_abs").and_then(Value::as_f64) {
        Some(t) => t,
        None => tolerance_abs(kind, target, unc)?,
    };
    let e = own
        .iter()
        .find(|e| e.id == key)
        .ok_or_else(|| format!("no own measurement {key:?}"))?;
    let m = e
        .median
        .ok_or_else(|| format!("{key}: not measurable in the sim ({})", e.notes))?;
    if (m - target).abs() <= tol + 1e-12 {
        Ok(())
    } else {
        Err(format!(
            "{key}: own {m} vs ref_median {target} (tolerance {tol}, kind {kind})"
        ))
    }
}

fn data_files() -> Vec<(String, PathBuf)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let mut v = Vec::new();
    for dir in ["weapons", "roles"] {
        let mut names: Vec<_> = std::fs::read_dir(root.join(dir))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        names.sort();
        v.extend(names.into_iter().map(|p| (dir.to_string(), p)));
    }
    v
}

#[test]
fn locked_data_fields_stay_within_reference_tolerance() {
    let own = measure_all();
    let (mut locked, mut calibrated) = (0, 0);
    let mut errors = Vec::new();
    for (dir, path) in data_files() {
        let j: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let id = j["id"].as_str().unwrap().to_string();
        let meta = &j["_meta"];
        let refs = meta["ref"].as_object().cloned().unwrap_or_default();
        if meta["status"] == "locked" && !refs.values().any(|r| r["state"] == "locked") {
            errors.push(format!(
                "{}: status 'locked' without a locked _meta.ref entry",
                path.display()
            ));
        }
        for (field, r) in &refs {
            match r["state"].as_str() {
                Some("locked") => {
                    locked += 1;
                    let ref_id = r["ref_id"].as_str().unwrap_or("?");
                    let key = if dir == "weapons" {
                        format!("{ref_id}.{id}")
                    } else {
                        ref_id.to_string()
                    };
                    if let Err(e) = check_entry(&own, &key, r) {
                        errors.push(format!("{} [{field}]: {e}", path.display()));
                    }
                }
                Some("calibrated") => calibrated += 1,
                s => errors.push(format!("{} [{field}]: bad state {s:?}", path.display())),
            }
        }
    }
    println!(
        "ref_regression: {locked} locked and {calibrated} calibrated reference entries in /data"
    );
    assert!(
        errors.is_empty(),
        "reference regression failed:\n{}",
        errors.join("\n")
    );
}

#[test]
fn check_entry_passes_inside_and_fails_outside_the_tolerance() {
    let own = measure_all();
    // Fixture reference values are derived from our own measurement, never typed in.
    let m02 = own.iter().find(|e| e.id == "M-02").unwrap().median.unwrap();
    let ok = |kind: &str, delta: f64| {
        check_entry(
            &own,
            "M-02",
            &json!({"kind": kind, "ref_median": m02 + delta, "state": "locked"}),
        )
    };
    assert!(ok("speed", 0.0).is_ok());
    assert!(ok("speed", 0.9 * 0.03 * m02).is_ok(), "inside +-3 %");
    assert!(ok("speed", 1.2 * 0.03 * m02).is_err(), "outside +-3 %");
    assert!(
        ok("stat", 1.2 * 0.03 * m02).is_ok(),
        "stat tolerance is +-10 %"
    );
    // Reference uncertainty relaxes the tolerance; an explicit tol_abs overrides it.
    let r = json!({"kind": "speed", "ref_median": m02 + 0.3, "ref_unc": 0.5});
    assert!(check_entry(&own, "M-02", &r).is_ok());
    let r = json!({"kind": "speed", "ref_median": m02 + 0.3, "tol_abs": 0.1});
    assert!(check_entry(&own, "M-02", &r).is_err());
    // Shots to kill: exactly equal.
    let h = |t: f64| {
        check_entry(
            &own,
            "H-01.us_m1_garand@50m",
            &json!({"kind": "count", "ref_median": t}),
        )
    };
    let stk = own
        .iter()
        .find(|e| e.id == "H-01.us_m1_garand@50m")
        .unwrap()
        .median
        .unwrap();
    assert!(h(stk).is_ok());
    assert!(h(stk + 1.0).is_err());
    // A quantity the M0 sim cannot measure can never be locked; unknown ids and kinds are errors.
    assert!(check_entry(&own, "M-05", &json!({"kind": "speed", "ref_median": 1.0})).is_err());
    assert!(check_entry(&own, "X-99", &json!({"kind": "speed", "ref_median": 1.0})).is_err());
    assert!(check_entry(&own, "M-02", &json!({"kind": "bogus", "ref_median": 1.0})).is_err());
}

#[test]
fn tolerance_table_mirrors_section_7_3() {
    let t = |k: &str, x: f64| tolerance_abs(k, x, 0.0).unwrap();
    assert!((t("timing", 0.2) - 1.0 / 60.0).abs() < 1e-12);
    assert!((t("timing", 1.0) - 0.05).abs() < 1e-12);
    assert!((t("speed", 4.4) - 0.132).abs() < 1e-12);
    assert!((t("distance", 10.0) - 1.0).abs() < 1e-12);
    assert!((t("distance", 100.0) - 5.0).abs() < 1e-12);
    assert!((t("angle", 1.0) - 0.1).abs() < 1e-12);
    assert!((t("stat", 3.0) - 0.3).abs() < 1e-12);
    assert_eq!(t("count", 2.0), 0.0);
    assert!((tolerance_abs("speed", 4.4, 0.5).unwrap() - 0.5).abs() < 1e-12);
}
