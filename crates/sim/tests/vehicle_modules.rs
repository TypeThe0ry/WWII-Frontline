//! M1-023c: module HP, threshold effects and the ammo-rack cook-off roll.
use frontline_sim::rng::{unit_f32, Purpose, Rng};
use frontline_sim::vehicle::modules::{ModuleId, ModuleSpecs, Modules};
use frontline_sim::{GameData, Tick};

fn specs() -> ModuleSpecs {
    ModuleSpecs::from_def(GameData::embedded().vehicle("us_m4a1_75").unwrap())
}

#[test]
fn hp_table_matches_data() {
    let s = specs();
    let m = Modules::new(&s);
    assert_eq!(m.hp, [300.0, 250.0, 250.0, 200.0, 250.0, 150.0]);
}

#[test]
fn engine_at_half_hp_slows_to_60_percent() {
    let s = specs();
    let mut m = Modules::new(&s);
    assert_eq!(m.effects(&s).speed_mult, 1.0);
    m.damage(&s, ModuleId::Engine, 150.0, 1.0, 1.0);
    let e = m.effects(&s);
    assert_eq!(e.speed_mult, 0.6);
    assert_eq!(e.accel_mult, 0.6);
    assert!(!e.immobilized);
    m.damage(&s, ModuleId::Engine, 150.0, 1.0, 1.0);
    assert!(m.effects(&s).immobilized);
}

#[test]
fn track_effects_are_per_side() {
    let s = specs();
    let mut m = Modules::new(&s);
    m.damage(&s, ModuleId::TrackL, 130.0, 1.0, 1.0);
    let e = m.effects(&s);
    assert_eq!((e.thrust_mult_l, e.thrust_mult_r), (0.6, 1.0));
    m.damage(&s, ModuleId::TrackL, 200.0, 1.0, 1.0);
    let e = m.effects(&s);
    assert_eq!(e.speed_mult, 0.3);
}

#[test]
fn turret_drive_and_gun_effects() {
    let s = specs();
    let mut m = Modules::new(&s);
    m.damage(&s, ModuleId::TurretDrive, 100.0, 1.0, 1.0);
    assert_eq!(m.effects(&s).traverse_mult, 0.5);
    assert!(!m.effects(&s).turret_jammed);
    m.damage(&s, ModuleId::TurretDrive, 100.0, 1.0, 1.0);
    assert!(m.effects(&s).turret_jammed);
    m.damage(&s, ModuleId::MainGun, 125.0, 1.0, 1.0);
    let e = m.effects(&s);
    assert_eq!((e.reload_mult, e.spread_mult), (1.4, 1.5));
    assert!(e.can_fire);
    m.damage(&s, ModuleId::MainGun, 125.0, 1.0, 1.0);
    assert!(!m.effects(&s).can_fire);
}

#[test]
fn damage_to_dead_module_is_ignored() {
    let s = specs();
    let mut m = Modules::new(&s);
    assert!(m.damage(&s, ModuleId::AmmoRack, 999.0, 0.0, 0.0).destroyed);
    assert_eq!(
        m.damage(&s, ModuleId::AmmoRack, 10.0, 0.0, 0.0),
        Default::default()
    );
}

#[test]
fn cookoff_rate_is_60_percent() {
    let s = specs();
    let mut hits = 0;
    for i in 0..1000u32 {
        let mut rng = Rng::for_(7, Tick(i), 1, Purpose::Armor);
        let u = unit_f32(&mut rng);
        let mut m = Modules::new(&s);
        if m.damage(&s, ModuleId::AmmoRack, 150.0, u, 1.0).cookoff {
            hits += 1;
        }
    }
    assert!((580..=620).contains(&hits), "{hits}");
}

mod internal {
    use frontline_sim::vehicle::internal::{internal_damage, internal_len};
    use frontline_sim::vehicle::modules::ModuleId;
    use frontline_sim::Vec3;

    #[test]
    fn internal_length_formula() {
        assert!((internal_len(0.0) - 0.6).abs() < 1e-6);
        assert!((internal_len(40.0) - 1.6).abs() < 1e-6);
        assert_eq!(internal_len(1000.0), 3.0);
    }

    #[test]
    fn side_hit_through_the_ammo_rack_and_driver() {
        // From the left side at z = 1.4 (driver's row), travelling -X through the driver.
        let d = internal_damage(
            Vec3::new(1.5, 0.9, 1.4),
            Vec3::new(-1.0, 0.0, 0.0),
            40.0,
            400.0,
            [true; 3],
        );
        assert!((d.crew[0] - 100.0).abs() < 1e-3, "driver {}", d.crew[0]);
        assert_eq!(d.crew[1], 0.0);
        assert!((d.hull - 200.0).abs() < 1e-3);
    }

    #[test]
    fn module_takes_full_damage_and_hull_adds_20_percent() {
        // Through the engine from the rear, along the hull axis.
        let d = internal_damage(
            Vec3::new(0.0, 0.7, -2.7),
            Vec3::new(0.0, 0.0, 1.0),
            40.0,
            400.0,
            [true; 3],
        );
        assert!((d.module[ModuleId::Engine as usize] - 400.0).abs() < 1e-3);
        assert!((d.hull - (200.0 + 80.0)).abs() < 1e-3);
    }

    #[test]
    fn turret_spillover_hits_the_other_turret_crewman() {
        // Straight down the gunner's seat from above: gunner direct 25%, commander 8%.
        let d = internal_damage(
            Vec3::new(0.0, 2.2, 0.2),
            Vec3::new(0.0, -1.0, 0.0),
            40.0,
            400.0,
            [true; 3],
        );
        assert!((d.crew[1] - 100.0).abs() < 1e-3);
        assert!((d.crew[2] - 32.0).abs() < 1e-3, "{}", d.crew[2]);
        assert_eq!(d.crew[0], 0.0);
    }
}
