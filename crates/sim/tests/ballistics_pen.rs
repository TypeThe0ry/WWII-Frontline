//! M1-012: ammunition drag integration, material penetration, ricochet.
use frontline_sim::ballistics::{
    ricochet, step_tick, trace_with_penetration, Layer, LayerWorld, MaterialId, MaterialTable,
    PenAmmo, PenResult,
};
use frontline_sim::data::defs::MaterialDef;
use frontline_sim::rng::{Purpose, Rng};
use frontline_sim::{GameData, Tick, Vec3};

const P_3006: f32 = 220.0;
const P_9X19: f32 = 60.0;
const P_50: f32 = 520.0;

/// Slabs perpendicular to the x axis: `(x_front_m, thickness_m, material)`; the ray travels +x-ish.
struct Slabs(Vec<(f32, f32, MaterialId)>);

impl LayerWorld for Slabs {
    fn first_layer(&self, origin: Vec3, dir: Vec3, max_t: f32) -> Option<Layer> {
        if dir.x <= 1e-6 {
            return None;
        }
        self.0
            .iter()
            .filter(|(x, ..)| *x >= origin.x)
            .filter_map(|&(x, th, material)| {
                let t0 = (x - origin.x) / dir.x;
                (t0 <= max_t).then(|| Layer {
                    material,
                    entry: origin + dir * t0,
                    exit: origin + dir * ((x + th - origin.x) / dir.x),
                    normal: -Vec3::X,
                })
            })
            .min_by(|a, b| a.entry.x.total_cmp(&b.entry.x))
    }
}

fn table() -> MaterialTable {
    MaterialTable::embedded()
}

fn shoot(t: &MaterialTable, slabs: &[(f32, f32, &str)], pen: f32, dir: Vec3) -> PenResult {
    let w = Slabs(
        slabs
            .iter()
            .map(|&(x, th, m)| (x, th, t.id_of(m).unwrap_or_else(|| panic!("{m}"))))
            .collect(),
    );
    let mut rng = Rng::for_(7, Tick(1), 1, Purpose::Penetration);
    trace_with_penetration(
        &w,
        t,
        Vec3::ZERO,
        dir.normalize(),
        100.0,
        &PenAmmo { pen_power: pen },
        &mut rng,
    )
}

#[test]
fn ballistics_500m() {
    // Garand: 853 m/s, k 0.00075 /m, horizontal shot at 30 Hz.
    let (mut p, mut v) = (Vec3::ZERO, Vec3::new(853.0, 0.0, 0.0));
    let mut ticks = 0;
    while p.x < 500.0 {
        let s = step_tick(p, v, 0.00075, 9.81, 1.0 / 30.0);
        p = s.to;
        v = s.v_end;
        ticks += 1;
    }
    let speed = v.length();
    let t = ticks as f32 / 30.0;
    assert!((speed / 586.0 - 1.0).abs() < 0.03, "speed {speed}");
    assert!((t / 0.72 - 1.0).abs() < 0.03, "time {t}");
    assert!((-p.y / 2.5 - 1.0).abs() < 0.06, "drop {}", -p.y);
}

#[test]
fn material_table_matches_the_design_numbers() {
    let t = table();
    assert_eq!(t.len(), 12);
    for (name, cost) in [
        ("flesh", 0.5),
        ("foliage", 0.3),
        ("glass", 0.2),
        ("thin_wood", 1.0),
        ("thick_wood", 1.5),
        ("brick", 3.0),
        ("stone_concrete", 6.0),
        ("sandbag", 5.0),
        ("thin_steel", 8.0),
        ("thick_steel", 20.0),
        ("snow", 2.0),
        ("water", 15.0),
    ] {
        let m = t.get(t.id_of(name).unwrap()).unwrap();
        assert_eq!(m.cost_per_mm, cost, "{name}");
    }
    // Ammunition data carries the penetration power.
    let d = GameData::embedded();
    assert_eq!(d.ammo_def("us_30_06").unwrap().gameplay.pen_power, 220.0);
    assert_eq!(d.ammo_def("us_50_bmg").unwrap().gameplay.pen_power, 520.0);
    assert_eq!(d.ammo_def("ger_9x19").unwrap().gameplay.pen_power, 60.0);
}

#[test]
fn wood_board_30mm() {
    let t = table();
    let board = [(5.0, 0.030, "thin_wood")];
    let r = shoot(&t, &board, P_3006, Vec3::X);
    assert!(!r.stopped);
    assert!(
        (r.dealt_mult - (220.0 - 30.0) / 220.0).abs() < 0.01 * 0.864,
        "{}",
        r.dealt_mult
    );
    let r = shoot(&t, &board, P_9X19, Vec3::X);
    assert!((r.dealt_mult - 0.5).abs() < 0.005, "{}", r.dealt_mult);
    assert_eq!(r.segment_count(), 2, "one layer: before and behind");
}

#[test]
fn sandbag_and_brick() {
    let t = table();
    let sand = [(5.0, 0.200, "sandbag")];
    for pen in [P_3006, P_50] {
        let r = shoot(&t, &sand, pen, Vec3::X);
        assert!(r.stopped, "sandbag stops {pen}");
    }
    let brick = [(5.0, 0.150, "brick")];
    assert!(shoot(&t, &brick, P_3006, Vec3::X).stopped);
    let r = shoot(&t, &brick, P_50, Vec3::X);
    assert!(!r.stopped);
    assert!((r.pen_left - 70.0).abs() < 1e-3, "{}", r.pen_left);
    assert_eq!(r.dealt_mult, 0.25, "floor of the layer multiplier");
}

#[test]
fn at_most_three_segments() {
    let t = table();
    let stack: Vec<(f32, f32, &str)> = (0..4)
        .map(|i| (5.0 + i as f32, 0.005, "thin_wood"))
        .collect();
    let r = shoot(&t, &stack, P_50, Vec3::X);
    assert_eq!(r.segment_count(), 3);
    assert!(r.stopped, "the third layer stops the shot");
    let last = r.segments[2].unwrap();
    assert!((last.to.x - 7.0).abs() < 1e-4, "stopped at the third entry");
    // Open air: one segment to the end of the range.
    let r = shoot(&t, &[], P_3006, Vec3::X);
    assert_eq!(r.segment_count(), 1);
    assert!(!r.stopped && r.dealt_mult == 1.0);
}

#[test]
fn ricochet_angle_boundary_and_speed() {
    let t = table();
    let wall = [(5.0, 0.3, "brick")];
    let dir = |deg: f32| Vec3::new(deg.to_radians().sin(), deg.to_radians().cos(), 0.0);
    let r = shoot(&t, &wall, P_3006, dir(10.0));
    let (pos, out, k) = r.ricochet.expect("10 degrees bounces");
    assert!(r.stopped && r.segment_count() == 1);
    assert!((k - 0.632).abs() < 0.0065, "speed factor {k}");
    assert!(out.x < 0.0 && out.y > 0.0, "reflected back: {out:?}");
    assert!((pos.x - 5.0).abs() < 1e-3);
    let r = shoot(&t, &wall, P_3006, dir(20.0));
    assert!(r.ricochet.is_none(), "20 degrees does not");
    // Soft materials never bounce, even at 5 degrees.
    let r = shoot(&t, &[(5.0, 0.3, "thin_wood")], P_3006, dir(5.0));
    assert!(r.ricochet.is_none());
    assert!((ricochet::incidence_deg(dir(14.9), -Vec3::X) - 14.9).abs() < 0.01);
}

fn custom_table(json: &str) -> MaterialTable {
    let def: MaterialDef = serde_json::from_str(json).unwrap();
    MaterialTable::from_defs(&[def])
}

#[test]
fn stop_cap_wins_over_power_and_min_thickness_is_ignored() {
    let capped = custom_table(
        r#"{"id":"cap","name_key":"material.cap.name","gameplay":{"cost_per_mm":0.1,"stop_mm":50,"min_mm":0,
        "deflect_deg":0.5,"ricochet":false,"sound_class":"x","impact_fx_class":"x"},
        "_meta":{"status":"initial","tunable":[],"calibration_ref":null}}"#,
    );
    let id = capped.id_of("cap").unwrap();
    let mut rng = Rng::for_(1, Tick(1), 1, Purpose::Penetration);
    let go = |th: f32, rng: &mut _| {
        let w = Slabs(vec![(5.0, th, id)]);
        trace_with_penetration(
            &w,
            &capped,
            Vec3::ZERO,
            Vec3::X,
            100.0,
            &PenAmmo { pen_power: P_50 },
            rng,
        )
    };
    assert!(!go(0.049, &mut rng).stopped, "under the cap: cost 4.9 only");
    assert!(
        go(0.051, &mut rng).stopped,
        "over the cap: stopped despite 520 power"
    );
    // Shallow water does not count, deep water stops.
    let t = table();
    let r = shoot(&t, &[(5.0, 0.3, "water")], P_3006, Vec3::X);
    assert!(!r.stopped && r.dealt_mult == 1.0);
    assert!(shoot(&t, &[(5.0, 0.6, "water")], P_3006, Vec3::X).stopped);
}

#[test]
fn deflection_is_bounded_and_deterministic() {
    let t = table();
    let run = |mat: &str| -> Vec<Vec3> {
        let w = Slabs(vec![(5.0, 0.01, t.id_of(mat).unwrap())]);
        (0..2000u32)
            .map(|i| {
                let mut rng = Rng::for_(42, Tick(i), 3, Purpose::Penetration);
                let r = trace_with_penetration(
                    &w,
                    &t,
                    Vec3::ZERO,
                    Vec3::new(1.0, 0.3, 0.0).normalize(),
                    100.0,
                    &PenAmmo { pen_power: P_50 },
                    &mut rng,
                );
                r.segments[1].unwrap().dir
            })
            .collect()
    };
    let inc = Vec3::new(1.0, 0.3, 0.0).normalize();
    for (mat, max) in [("thin_steel", 1.5f32), ("thin_wood", 0.5)] {
        let a = run(mat);
        let widest = a
            .iter()
            .map(|d| d.dot(inc).clamp(-1.0, 1.0).acos().to_degrees())
            .fold(0.0f32, f32::max);
        assert!(
            widest <= max + 0.01 && widest > 0.5 * max,
            "{mat}: {widest}"
        );
        assert_eq!(a, run(mat), "same seed, same deflection");
    }
}
