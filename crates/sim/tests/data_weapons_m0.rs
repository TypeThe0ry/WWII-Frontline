//! M0-008: the two M0 rifles parse into `GameData`, values equal GAMEDESIGN 6.2-6.6,
//! and the torso shots-to-down (STK) table keeps the "2 hits down" principle (6.6).

use frontline_sim::data::defs::{Hist, HistSource};
use frontline_sim::data::{DataStatus, Faction, GameData, WeaponDef};

fn falloff_mult(w: &WeaponDef, d: f64) -> f64 {
    let pts = &w.gameplay.falloff;
    if d <= pts[0].dist_m {
        return pts[0].mult;
    }
    for pair in pts.windows(2) {
        if d <= pair[1].dist_m {
            let t = (d - pair[0].dist_m) / (pair[1].dist_m - pair[0].dist_m);
            return pair[0].mult + t * (pair[1].mult - pair[0].mult);
        }
    }
    pts[pts.len() - 1].mult // held after the last point (GAMEDESIGN 6.6)
}

/// Torso hits needed to bring a 100 HP infantryman to HP <= 0 at distance `d`.
fn stk_torso(w: &WeaponDef, d: f64) -> u32 {
    let per_hit = w.gameplay.damage_torso_hp * falloff_mult(w, d) * w.gameplay.part_mult.torso;
    let mut hp = 100.0;
    let mut n = 0;
    while hp > 0.0 {
        hp -= per_hit;
        n += 1;
    }
    n
}

#[test]
fn data_weapons_m0_load() {
    let gd = GameData::embedded();
    let g = gd.weapon("us_m1_garand").expect("us_m1_garand");
    let k = gd.weapon("ger_kar98k").expect("ger_kar98k");

    assert_eq!(g.faction, Faction::Us);
    assert_eq!(k.faction, Faction::Ger);
    assert_eq!(g.class, "rifle_semi");
    assert_eq!(k.class, "rifle_bolt");
    // M0-031: historical blocks reviewed (gameplay stays tunable / uncalibrated).
    assert_eq!(g.meta.status, DataStatus::Reviewed);
    assert_eq!(k.meta.status, DataStatus::Reviewed);
    assert_eq!(g.historical.magazine_capacity, Hist::Known(8));
    assert_eq!(g.historical.muzzle_velocity_mps, Hist::Known(853.0));
    assert_eq!(g.historical.rate_of_fire_rpm_practical, Hist::Known(24.0));
    assert_eq!(g.historical.mass_kg, Hist::Known(4.3));
    assert_eq!(g.historical.effective_range_m, Some(Hist::Known(460.0)));
    assert_eq!(k.historical.magazine_capacity, Hist::Known(5));
    assert_eq!(k.historical.mass_kg, Hist::Known(3.9));
    for w in [g, k] {
        let HistSource::Records(recs) = &w.historical.source else {
            panic!("reviewed weapon needs structured _source");
        };
        assert!(recs.iter().all(|r| !r.locator.is_empty()));
    }
    assert!(g.meta.unverified.is_empty());
    assert_eq!(
        k.meta.unverified,
        [
            "historical.muzzle_velocity_mps",
            "historical.rate_of_fire_rpm_practical"
        ]
    );

    let (gg, kg) = (&g.gameplay, &k.gameplay);
    assert_eq!(gg.damage_torso_hp, 70.0);
    assert_eq!(kg.damage_torso_hp, 100.0);
    assert_eq!(gg.fire.cap_rpm, Some(120.0));
    assert_eq!(kg.fire.bolt_cycle_ms, Some(1500.0));
    assert_eq!(
        (gg.reload.tactical_ms, gg.reload.empty_ms),
        (3400.0, 2800.0)
    );
    assert_eq!(gg.reload.per_round_ms, None);
    assert_eq!(
        (kg.reload.tactical_ms, kg.reload.empty_ms),
        (3000.0, 3000.0)
    );
    assert_eq!(kg.reload.per_round_ms, Some(650.0));
    assert_eq!((gg.ads_ms, kg.ads_ms), (300.0, 300.0));
    assert_eq!((gg.pen_power, kg.pen_power), (220.0, 230.0));
    assert_eq!(gg.ballistics.drag_k_per_m, 0.00075);
    assert_eq!(kg.ballistics.drag_k_per_m, 0.00068);
    assert_eq!(gg.ballistics.muzzle_velocity_mps, 853.0);
    assert_eq!(kg.ballistics.muzzle_velocity_mps, 760.0);
    assert_eq!(
        (
            gg.recoil.vkick_deg,
            gg.recoil.hsigma_deg,
            gg.recoil.recover_deg_s
        ),
        (1.6, 0.25, 12.0)
    );
    assert_eq!(
        (
            kg.recoil.vkick_deg,
            kg.recoil.hsigma_deg,
            kg.recoil.recover_deg_s
        ),
        (3.2, 0.20, 10.0)
    );
    assert_eq!((gg.spread.hip_deg, gg.spread.ads_deg), (3.5, 0.15));
    assert_eq!((kg.spread.hip_deg, kg.spread.ads_deg), (4.0, 0.08));
    assert_eq!(
        (gg.recoil.bloom_per_shot_deg, gg.recoil.bloom_decay_deg_s),
        (0.40, 3.0)
    );
    assert_eq!(kg.recoil.bloom_per_shot_deg, 0.0);
    for w in [g, k] {
        let p = &w.gameplay.part_mult;
        assert_eq!(
            (p.head, p.neck, p.torso, p.pelvis, p.limb),
            (3.0, 2.0, 1.0, Some(0.9), 0.65)
        );
        assert_eq!(w.gameplay.mag_reserve, 6);
        let fx = w.fx.as_ref().expect("fx");
        assert_eq!((fx.muzzle_flash_ms, fx.tracer), (Some(50.0), Some(false)));
    }
    let f = |w: &WeaponDef| -> Vec<(f64, f64)> {
        w.gameplay
            .falloff
            .iter()
            .map(|p| (p.dist_m, p.mult))
            .collect()
    };
    assert_eq!(f(g), [(0.0, 1.0), (100.0, 1.0), (600.0, 0.72)]);
    assert_eq!(f(k), [(0.0, 1.0), (200.0, 1.0), (800.0, 0.80)]);
}

/// Snapshot of the GAMEDESIGN 6.6 TTK baseline: Garand 2 hits at every range, Kar98k 1 hit
/// out to F0 (200 m) and 2 hits beyond. Tuning must not silently break this.
#[test]
fn data_weapons_m0_stk_snapshot() {
    let gd = GameData::embedded();
    let g = gd.weapon("us_m1_garand").unwrap();
    let k = gd.weapon("ger_kar98k").unwrap();
    for d in [0.0, 100.0, 300.0, 600.0, 800.0, 1000.0] {
        assert_eq!(stk_torso(g, d), 2, "garand at {d} m");
    }
    for d in [0.0, 100.0, 200.0] {
        assert_eq!(stk_torso(k, d), 1, "kar98k at {d} m");
    }
    for d in [201.0, 300.0, 500.0, 800.0, 1000.0] {
        assert_eq!(stk_torso(k, d), 2, "kar98k at {d} m");
    }
}
