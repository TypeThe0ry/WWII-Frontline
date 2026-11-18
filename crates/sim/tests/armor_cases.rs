//! `armor_cases` (M1-023): hand-calculated AP armour verdicts from
//! `tests/fixtures/armor_cases.json` (every step written out in the fixture), penetration
//! table, ricochet thresholds and the overmatch rule.
use frontline_sim::vehicle::armor::{
    armor_hit, armor_hit_steps, penetration_at, ArmorResult, ShellKind, ShellParams,
};
use frontline_sim::Vec3;

fn ap() -> ShellParams {
    ShellParams {
        kind: ShellKind::Ap,
        caliber_mm: 75.0,
        p100_mm: 82.0,
        drop_pct_per_100m: 3.5,
        structure_damage_hp: 400.0,
    }
}

fn heat() -> ShellParams {
    ShellParams {
        kind: ShellKind::Heat,
        caliber_mm: 60.0,
        p100_mm: 200.0,
        drop_pct_per_100m: 0.0,
        structure_damage_hp: 900.0,
    }
}

fn geom(phi_deg: f32) -> (Vec3, Vec3) {
    let p = phi_deg.to_radians();
    (Vec3::new(0.0, 0.0, 1.0), Vec3::new(p.sin(), 0.0, -p.cos()))
}

fn kind(r: &ArmorResult) -> &'static str {
    match r {
        ArmorResult::Pen { .. } => "pen",
        ArmorResult::NoPen { .. } => "nopen",
        ArmorResult::Ricochet => "ricochet",
    }
}

fn load() -> serde_json::Value {
    let s = include_str!("fixtures/armor_cases.json");
    serde_json::from_str(s).unwrap()
}

#[test]
fn hand_calculated_cases_match() {
    let v = load();
    let cases = v["cases"].as_array().unwrap();
    assert!(cases.len() >= 12);
    let mut seen = std::collections::BTreeSet::new();
    for c in cases {
        let id = c["id"].as_str().unwrap();
        let (n, vh) = geom(c["phi_deg"].as_f64().unwrap() as f32);
        let s = armor_hit_steps(
            &ap(),
            c["dist_m"].as_f64().unwrap() as f32,
            n,
            vh,
            c["thickness_mm"].as_f64().unwrap() as f32,
            1.0,
        );
        let e = &c["expect"];
        assert_eq!(kind(&s.result), e["result"].as_str().unwrap(), "{id}");
        assert!(
            (s.phi_eff_deg - e["phi_eff_deg"].as_f64().unwrap() as f32).abs() < 0.01,
            "{id} phi_eff"
        );
        assert!(
            (s.p_mm - e["p_mm"].as_f64().unwrap() as f32).abs() < 0.01,
            "{id} P"
        );
        if let Some(t) = e["t_eff_mm"].as_f64() {
            assert!(
                (s.t_eff - t as f32).abs() < 0.01,
                "{id} t_eff {} vs {t}",
                s.t_eff
            );
        }
        seen.insert(e["result"].as_str().unwrap().to_string());
        if c["note"].as_str().unwrap().contains("overmatch") {
            assert!(s.overmatch, "{id}");
            seen.insert("overmatch".into());
        }
    }
    for k in ["pen", "nopen", "ricochet", "overmatch"] {
        assert!(seen.contains(k), "fixture lacks {k}");
    }
}

#[test]
fn critical_cases_flip_across_luck_band() {
    let v = load();
    for crit in v["critical"].as_array().unwrap() {
        let id = crit["id"].as_str().unwrap();
        let c = v["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap();
        let (n, vh) = geom(c["phi_deg"].as_f64().unwrap() as f32);
        let d = c["dist_m"].as_f64().unwrap() as f32;
        let t = c["thickness_mm"].as_f64().unwrap() as f32;
        let lo = armor_hit(
            &ap(),
            d,
            n,
            vh,
            t,
            crit["factor_lo"].as_f64().unwrap() as f32,
        );
        let hi = armor_hit(
            &ap(),
            d,
            n,
            vh,
            t,
            crit["factor_hi"].as_f64().unwrap() as f32,
        );
        assert_eq!(kind(&lo), crit["result_lo"].as_str().unwrap());
        assert_eq!(kind(&hi), crit["result_hi"].as_str().unwrap());
    }
}

#[test]
fn m61_penetration_table() {
    for (d, want) in [(100.0, 82.0), (300.0, 76.3), (600.0, 67.7), (50.0, 82.0)] {
        let p = penetration_at(&ap(), d);
        assert!((p - want).abs() <= 0.1, "{d}: {p}");
    }
    // Floor at 30% of P100.
    assert!((penetration_at(&ap(), 5000.0) - 24.6).abs() < 1e-3);
}

#[test]
fn ricochet_thresholds() {
    // AP: phi_eff = phi - 5.
    let (n, v) = geom(72.9);
    assert!(!matches!(
        armor_hit(&ap(), 100.0, n, v, 76.0, 1.0),
        ArmorResult::Ricochet
    ));
    let (n, v) = geom(73.05);
    assert!(matches!(
        armor_hit(&ap(), 100.0, n, v, 76.0, 1.0),
        ArmorResult::Ricochet
    ));
    // HEAT: no normalization, limit 75.
    let (n, v) = geom(74.9);
    assert!(!matches!(
        armor_hit(&heat(), 50.0, n, v, 51.0, 1.0),
        ArmorResult::Ricochet
    ));
    let (n, v) = geom(75.05);
    assert!(matches!(
        armor_hit(&heat(), 50.0, n, v, 51.0, 1.0),
        ArmorResult::Ricochet
    ));
}

#[test]
fn overmatch_never_ricochets() {
    let (n, v) = geom(80.0);
    let r = armor_hit(&ap(), 100.0, n, v, 9.5, 1.0);
    assert!(matches!(r, ArmorResult::Pen { .. }), "{r:?}");
}

#[test]
fn heat_pen_ignores_distance_and_normalization() {
    let (n, v) = geom(56.0);
    let s = armor_hit_steps(&heat(), 60.0, n, v, 51.0, 1.0);
    assert!((s.phi_eff_deg - 56.0).abs() < 0.01);
    assert!(matches!(s.result, ArmorResult::Pen { .. }));
    assert!((penetration_at(&heat(), 80.0) - 200.0).abs() < 1e-3);
}
