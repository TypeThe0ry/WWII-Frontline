//! M0-032: own-game re-measurement of the M0 batch (REFERENCE_MEASUREMENTS 7.4).
//!
//! `OWN_REMEASURE_OUT=docs/measurements/own_m0.json cargo test -p frontline-sim --test own_remeasure -- --nocapture`
//! writes the JSON consumed by `python3 -m ref_analysis own-fill|own-table`. Without the variable the
//! numbers are only printed. The assertions check that the *measurement methods* recover the
//! configured values of `/data` (a method self-test), not any reference-game number.
mod common;
mod own_measure;

use own_measure::{measure_all, Entry};

fn get<'a>(v: &'a [Entry], id: &str) -> &'a Entry {
    v.iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("{id}"))
}

#[test]
fn own_remeasure_matches_configured_data_and_exports_json() {
    let all = measure_all();
    for e in &all {
        match e.median {
            Some(m) => println!(
                "{:<26} {:>12.5} {:<6} (n={}, data {:?})  {}",
                e.id, m, e.unit, e.n, e.data_value, e.notes
            ),
            None => println!("{:<26} unavailable: {}", e.id, e.notes),
        }
    }
    // Method self-test: measured == configured within one 30 Hz tick / the 7.3 tolerance.
    let tick = 1.0 / 30.0;
    let near = |id: &str, tol: f64| {
        let e = get(&all, id);
        let (m, d) = (e.median.unwrap(), e.data_value.unwrap());
        assert!(
            (m - d).abs() <= tol,
            "{id}: measured {m} vs data {d} (tol {tol})"
        );
    };
    for w in ["us_m1_garand", "ger_kar98k"] {
        near(&format!("W-01.{w}"), 0.5);
        near(&format!("W-03.{w}"), 0.5 * tick);
        near(&format!("W-04.{w}"), 0.5 * tick);
        near(&format!("W-07.{w}"), 0.5 * tick);
        near(&format!("W-22.{w}"), 1e9); // average velocity < muzzle velocity: only checked below
        near(&format!("W-30.{w}"), 0.0);
        let v = get(&all, &format!("W-22.{w}"));
        assert!(
            v.median.unwrap() < v.data_value.unwrap(),
            "drag slows the projectile"
        );
        // 7.3: statistical items within +-10 % of the configured cone.
        let hip = get(&all, &format!("W-19.{w}"));
        assert!(
            hip.max.unwrap() <= hip.data_value.unwrap() + 1e-3,
            "cone bound"
        );
        assert!(
            (hip.median.unwrap() / hip.data_value.unwrap() - std::f64::consts::FRAC_1_SQRT_2).abs()
                < 0.03
        );
        let kick = get(&all, &format!("W-15.{w}"));
        assert!((kick.median.unwrap() / kick.data_value.unwrap() - 1.0).abs() < 0.05);
    }
    near("W-02.us_m1_garand", 1e-9);
    near("W-13.ger_kar98k", 1e-9);
    near("M-01", 0.03 * 2.0);
    near("M-02", 0.03 * 4.4);
    near("M-03", 0.03 * 6.4);
    near("M-04", 0.03 * 2.4);
    near("M-12", 0.5 * tick);
    near("M-13", 0.5 * tick);
    assert!((get(&all, "M-19").median.unwrap() - 0.6).abs() < 0.02);
    // Acceleration: 95 % of the 0 -> 100 % ramp.
    let acc = get(&all, "M-24").median.unwrap();
    let want = 0.95 * get(&all, "M-24").data_value.unwrap();
    assert!((acc - want).abs() < tick, "{acc} vs {want}");
    // Shots to kill: exact integers.
    assert_eq!(get(&all, "H-01.us_m1_garand@50m").median, Some(2.0));
    assert_eq!(get(&all, "H-01.ger_kar98k@50m").median, Some(1.0));
    assert_eq!(get(&all, "H-02.us_m1_garand").median, Some(1.0));
    assert_eq!(get(&all, "H-03.us_m1_garand").median, Some(3.0));
    // Features that M0 does not have are reported as unavailable, never as zero.
    for id in [
        "M-05",
        "M-14",
        "M-17",
        "W-11.us_m1_garand",
        "W-27.ger_kar98k",
    ] {
        assert!(get(&all, id).median.is_none(), "{id}");
    }

    if let Some(path) = std::env::var_os("OWN_REMEASURE_OUT") {
        let json = serde_json::json!({
            "generator": "crates/sim/tests/own_remeasure.rs (M0-032)",
            "tick_hz": 30,
            "data_hash": format!("{:016x}", frontline_sim::GameData::embedded().hash),
            "entries": all,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap() + "\n").unwrap();
        println!("wrote {}", std::path::PathBuf::from(path).display());
    }
}

#[test]
fn measurement_is_deterministic() {
    let a = serde_json::to_string(&measure_all()).unwrap();
    let b = serde_json::to_string(&measure_all()).unwrap();
    assert_eq!(a, b);
}
