//! M1-023c: AP hits on a real Sherman: zone selection, outcome and module damage.
mod common;

use common::*;
use frontline_sim::vehicle::modules::ModuleId;
use frontline_sim::vehicle::shell::{ArmorKind, HitRolls};
use frontline_sim::{GameData, Team, Vec3, World};

fn world() -> (World, u16) {
    let mut w = World::new(
        sim_cfg(GameData::embedded(), 3),
        &synth_data(30, 6, &|_, _| 5.0),
    );
    let v = w
        .spawn_vehicle("us_m4a1_75", Team::Us, Vec3::new(500.0, 6.5, 100.0), 0.0)
        .unwrap();
    (w, v)
}

const MID: HitRolls = HitRolls {
    armor: 0.5,
    damage: 0.5,
    cookoff: 0.99,
    fire: 0.99,
};

#[test]
fn side_hit_at_100m_penetrates_and_hurts_the_hull() {
    let (mut w, v) = world();
    // Shooter on the +X side at 100 m, firing -X at hull height.
    let r = w
        .debug_ap_hit(
            v,
            0,
            Vec3::new(600.0, 6.5, 100.0),
            Vec3::new(-1.0, 0.0, 0.0),
            100.0,
            MID,
        )
        .unwrap();
    assert_eq!(r.armor, Some(ArmorKind::Pen));
    assert!(r.hull_damage > 150.0, "{}", r.hull_damage);
    assert!(w.vehicle(v).unwrap().hull_hp < 1600.0 - 150.0);
}

#[test]
fn glacis_at_600m_does_not_penetrate_and_scratches_hull() {
    let (mut w, v) = world();
    // From the front (+Z of the hull) at 600 m, aimed slightly down onto the upper glacis.
    let r = w
        .debug_ap_hit(
            v,
            0,
            Vec3::new(500.0, 8.0, 700.0),
            Vec3::new(0.0, -0.0025, -1.0).normalize(),
            600.0,
            MID,
        )
        .unwrap();
    assert_ne!(r.armor, Some(ArmorKind::Pen), "{r:?}");
    if r.armor == Some(ArmorKind::NoPen) {
        assert!((r.hull_damage - 12.0).abs() < 1e-3);
    }
}

#[test]
fn rear_hit_through_the_engine_damages_the_engine() {
    let (mut w, v) = world();
    let r = w
        .debug_ap_hit(
            v,
            0,
            Vec3::new(500.0, 6.2, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            100.0,
            MID,
        )
        .unwrap();
    assert_eq!(r.armor, Some(ArmorKind::Pen));
    assert!(r.module_damage[ModuleId::Engine as usize] > 300.0);
    assert!(w.vehicle(v).unwrap().modules.hp[ModuleId::Engine as usize] <= 0.0);
}
