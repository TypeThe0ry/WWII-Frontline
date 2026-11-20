use super::*;
use crate::data::defs::DataStatus;
use crate::vehicle::armor::{armor_hit_steps, ShellKind};

const US: &str = include_str!("../../../../../data/emplacements/us_at_57mm_m1.json");
const GER: &str = include_str!("../../../../../data/emplacements/ger_at_75mm_pak40.json");
const HZ: u8 = 30;

fn actor(player: PlayerId, team: Team, role_id: &str) -> Actor<'_> {
    Actor {
        player,
        team,
        role_id,
    }
}

fn us_gun() -> AtGunState {
    AtGunState::new(50, 1, Vec3::ZERO, 0.0, &AtGunDef::load(US).unwrap())
}

fn tick(g: &mut AtGunState, n: u32) {
    for _ in 0..n {
        g.tick();
    }
}

#[test]
fn at_gun_build_permission() {
    let us = AtGunDef::load(US).unwrap();
    let ger = AtGunDef::load(GER).unwrap();
    let mut guns = AtGuns::default();
    assert_eq!(us.meta.status, DataStatus::Initial);
    assert!(!us.meta.tunable.is_empty());
    assert_eq!(us.build.supplies, 200);
    assert_eq!(us.build.time_s, 20.0);
    assert_eq!(
        guns.check_build(&us, Team::Us, "engineer", 1),
        Err(BuildError::NotAntiTank)
    );
    assert_eq!(
        guns.check_build(&ger, Team::Us, "anti_tank", 1),
        Err(BuildError::WrongTeam)
    );
    for id in 1..4 {
        assert_eq!(guns.check_build(&us, Team::Us, "anti_tank", id), Ok(()));
        guns.list
            .insert(id, AtGunState::new(id, id, Vec3::ZERO, 0.0, &us));
        if id == 1 {
            assert_eq!(
                guns.check_build(&us, Team::Us, "anti_tank", 1),
                Err(BuildError::SectorCap)
            );
            assert_eq!(
                guns.check_build(&ger, Team::Ger, "anti_tank", 1),
                Err(BuildError::SectorCap)
            );
        }
    }
    assert_eq!(
        guns.check_build(&us, Team::Us, "anti_tank", 4),
        Err(BuildError::TeamCap)
    );
    guns.list.get_mut(&1).unwrap().damage(600.0);
    assert_eq!(guns.check_build(&us, Team::Us, "anti_tank", 1), Ok(()));
}

#[test]
fn at_gun_reload() {
    for (data, team, solo, trained, untrained) in
        [(US, Team::Us, 126, 90, 113), (GER, Team::Ger, 135, 96, 120)]
    {
        for (role, duration) in [
            (None, solo),
            (Some("engineer"), trained),
            (Some("rifleman"), untrained),
        ] {
            let mut g = AtGunState::new(50, 1, Vec3::ZERO, 0.0, &AtGunDef::load(data).unwrap());
            assert!(g.occupy(actor(1, team, "rifleman"), Seat::Gunner));
            if let Some(role) = role {
                assert!(g.occupy(actor(2, team, role), Seat::Loader));
            }
            let mut shells = Shells::default();
            let flash = g.fire(1, role, HZ, &mut shells).unwrap();
            assert_eq!(g.ready_in_ticks(), duration);
            assert_eq!((flash.visual_ticks, flash.exposure_ticks), (90, 150));
            assert_eq!(
                (flash.audience_radius_m, flash.error_radius_m),
                (350.0, 25.0)
            );
            assert_eq!(shells.list[0].pos, flash.pos);
            assert_eq!(g.ammo_ap, 17);
            assert_eq!(g.ammo_he, 8);
            tick(&mut g, duration - 1);
            assert!(g.fire(1, role, HZ, &mut shells).is_none());
            g.tick();
            assert!(g.select_ammo(1, Ammo::He, HZ));
            tick(&mut g, 29);
            assert!(g.fire(1, role, HZ, &mut shells).is_none());
            g.tick();
            assert!(g.fire(1, role, HZ, &mut shells).is_some());
            assert_eq!(g.ammo_he, 7);
            assert_eq!(shells.list[1].warhead, Warhead::He);
            assert!(g.select_ammo(1, Ammo::Ap, HZ));
            tick(&mut g, 30);
            assert!(g.fire(1, role, HZ, &mut shells).is_none());
            tick(&mut g, duration - 30);
            assert!(g.fire(1, role, HZ, &mut shells).is_some());
        }
    }
}

#[test]
fn at_gun_aim_and_reorientation() {
    let mut g = us_gun();
    assert!(!g.occupy(actor(1, Team::Ger, "anti_tank"), Seat::Gunner));
    assert!(g.occupy(actor(1, Team::Us, "anti_tank"), Seat::Gunner));
    assert!(!g.occupy(actor(2, Team::Us, "rifleman"), Seat::Gunner));
    assert!(g.aim(1, 90_f32.to_radians(), 40_f32.to_radians()));
    assert!((g.yaw.to_degrees() - 40.0).abs() < 1e-4);
    assert!((g.pitch.to_degrees() - 15.0).abs() < 1e-4);
    assert!(g.aim(1, -90_f32.to_radians(), -30_f32.to_radians()));
    assert!((g.yaw.to_degrees() + 40.0).abs() < 1e-4);
    assert!((g.pitch.to_degrees() + 5.0).abs() < 1e-4);
    assert!(!g.start_reorient(actor(2, Team::Us, "anti_tank"), 1.0, HZ));
    assert!(g.start_reorient(actor(2, Team::Us, "engineer"), 1.0, HZ));
    tick(&mut g, 89);
    assert_eq!(g.facing_center_yaw, 0.0);
    assert!(g.fire(1, None, HZ, &mut Shells::default()).is_none());
    g.tick();
    assert_eq!(g.facing_center_yaw, 1.0);
    assert!((g.yaw - (1.0 - 40_f32.to_radians())).abs() < 1e-5);
    assert!(g.start_reorient(actor(2, Team::Us, "engineer"), -1.0, HZ));
    g.leave(2);
    tick(&mut g, 90);
    assert_eq!(g.facing_center_yaw, 1.0);
    assert!(g.start_reorient(actor(2, Team::Us, "engineer"), 178_f32.to_radians(), HZ));
    tick(&mut g, 90);
    assert!(g.aim(1, -178_f32.to_radians(), 0.0));
    assert!((g.yaw.to_degrees() + 178.0).abs() < 1e-4);
}

#[test]
fn at_gun_penetration_vectors() {
    // The M1-023 hand table supplies each geometry. The two AT definitions use their own P100
    // and drop, so a borderline verdict can differ from the M61 reference.
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/armor_cases.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    let us = [
        "pen", "pen", "nopen", "pen", "pen", "nopen", "pen", "pen", "pen", "ricochet", "ricochet",
        "nopen",
    ];
    let mut ger = us;
    ger[2] = "pen";
    for (data, want, p100, drop, damage, blast, radius) in [
        (US, us, 92.0, 3.0, 350.0, 140.0, 4.0),
        (GER, ger, 96.0, 2.2, 400.0, 250.0, 7.0),
    ] {
        let def = AtGunDef::load(data).unwrap();
        let ap = shell_from_def(&def.ap, 1, def.team(), Vec3::ZERO, Vec3::Z);
        let he = shell_from_def(&def.he, 1, def.team(), Vec3::ZERO, Vec3::Z);
        assert_eq!(
            (
                ap.params.p100_mm,
                ap.params.drop_pct_per_100m,
                ap.params.structure_damage_hp
            ),
            (p100, drop, damage)
        );
        assert_eq!((he.blast_dmax, he.blast_radius), (blast, radius));
        for (case, expect) in cases[..12].iter().zip(want) {
            let float = |key: &str| case[key].as_f64().unwrap() as f32;
            let angle = float("phi_deg").to_radians();
            let travel = Vec3::new(libm::sinf(angle), 0.0, -libm::cosf(angle));
            let result = armor_hit_steps(
                &ap.params,
                float("dist_m"),
                Vec3::Z,
                travel,
                float("thickness_mm"),
                1.0,
            );
            let actual = match result.result {
                ArmorResult::Pen { .. } => "pen",
                ArmorResult::NoPen { .. } => "nopen",
                ArmorResult::Ricochet => "ricochet",
            };
            assert_eq!(actual, expect, "{} {}", def.id, case["id"]);
        }
    }
}

#[test]
fn at_gun_shield_and_destruction() {
    let mut g = us_gun();
    assert!(g.occupy(actor(1, Team::Us, "anti_tank"), Seat::Gunner));
    let center = Vec3::new(0.0, 0.95, 0.7);
    for (deg, blocked) in [(0.0_f32, true), (60.0, true), (60.1, false), (180.0, false)] {
        let dir = -forward(deg.to_radians());
        assert_eq!(
            g.raycast_shield(center - dir * 2.0, dir, 3.0).is_some(),
            blocked,
            "{deg}"
        );
    }
    assert!(g
        .raycast_shield(center + Vec3::X + Vec3::Z * 2.0, -Vec3::Z, 3.0)
        .is_none());
    let origin = center + Vec3::Z * 2.0;
    let dir = -Vec3::Z;
    let hit = g.raycast_shell(origin, dir, 3.0).unwrap();
    let def = AtGunDef::load(GER).unwrap();
    let mut shell = shell_from_def(&def.ap, 60, Team::Ger, origin, dir);
    shell.params.p100_mm = 6.0;
    assert!(matches!(
        g.apply_shell_hit(hit, &shell, dir, 1.0),
        Some(ArmorResult::NoPen { .. })
    ));
    assert_eq!(g.hp, 600.0);
    shell.params.p100_mm = 96.0;
    assert!(matches!(
        g.apply_shell_hit(hit, &shell, dir, 1.0),
        Some(ArmorResult::Pen { .. })
    ));
    assert_eq!(g.hp, 200.0);
    shell.warhead = Warhead::Heat;
    shell.params.kind = ShellKind::Heat;
    assert!(matches!(
        g.apply_shell_hit(hit, &shell, dir, 1.0),
        Some(ArmorResult::Pen { .. })
    ));
    assert_eq!(g.hp, 0.0);
    assert_eq!(g.gunner, Some(1)); // World cleanup needs the occupants to eject them.
    assert!(g.fire(1, None, HZ, &mut Shells::default()).is_none());
    assert!(g.raycast_shell(origin, dir, 3.0).is_none());
}

#[test]
fn at_gun_resupply_ap_and_hash() {
    let mut guns = AtGuns::default();
    let mut g = us_gun();
    assert_eq!(
        g.resupply_ap(|_, _| panic!("full weapon must not debit supplies")),
        0
    );
    assert!(g.occupy(actor(1, Team::Us, "anti_tank"), Seat::Gunner));
    let mut shells = Shells::default();
    for _ in 0..18 {
        assert!(g.fire(1, None, HZ, &mut shells).is_some());
        tick(&mut g, 126);
    }
    assert!(g.fire(1, None, HZ, &mut shells).is_none());
    assert_eq!(g.resupply_ap(|_, _| false), 0);
    guns.list.insert(g.id, g);
    let hash = |guns: &AtGuns| {
        let mut h = HashWriter::new();
        guns.hash_into(&mut h);
        h.finish()
    };
    let before = hash(&guns);
    let g = guns.list.get_mut(&50).unwrap();
    assert_eq!(
        g.resupply_ap(|rounds, supplies| (rounds, supplies) == (8, 100)),
        8
    );
    assert_eq!(g.resupply_ap(|_, _| true), 8);
    assert_eq!(g.resupply_ap(|rounds, _| rounds == 2), 2);
    assert_eq!(g.ammo_ap, 18);
    assert_eq!(g.ammo_he, 8);
    assert_ne!(hash(&guns), before);
}

#[test]
fn at_gun_shell_flight_intercepts_and_knocks_out_sherman() {
    use crate::collision::{CHUNK_CELLS, CHUNK_SAMPLES};
    use crate::components::{
        CollisionRes, CurrentTick, EntityTable, EventBuf, PlayerIndex, SimCfgRes, SimRng,
    };
    use crate::health::HitLog;
    use crate::ids::Tick;
    use crate::spawn::outpost::{OutpostState, OutpostStatus, Outposts};
    use crate::throwables::system::Smokes;
    use crate::vehicle::{shell_flight, yaw_quat, Vehicle, Vehicles};
    use crate::{CollisionChunk, CollisionData, GameData, SimEvent};
    use bevy_ecs::prelude::{Schedule, World};

    let data = GameData::embedded();
    let ger = AtGunDef::load(GER).unwrap();
    let us = AtGunDef::load(US).unwrap();
    let mut ecs = World::new();
    let mut schedule = Schedule::default();
    schedule.add_systems(shell_flight);
    ecs.insert_resource(SimCfgRes { data, hz: HZ });
    let ground = CollisionChunk::new(
        0,
        0,
        vec![0.0; CHUNK_SAMPLES * CHUNK_SAMPLES],
        vec![0; CHUNK_CELLS * CHUNK_CELLS],
    );
    ecs.insert_resource(CollisionRes(CollisionData::from_chunks([ground])));
    ecs.insert_resource(CurrentTick(Tick(0)));
    ecs.insert_resource(EntityTable::default());
    ecs.insert_resource(PlayerIndex::default());
    ecs.insert_resource(EventBuf::default());
    ecs.insert_resource(SimRng {
        match_seed: 3,
        draws: 0,
    });
    ecs.insert_resource(HitLog::default());
    ecs.insert_resource(Smokes::default());
    ecs.insert_resource(Shells::default());

    let mut vehicles = Vehicles::default();
    vehicles.list.insert(
        3,
        Vehicle::from_def(
            data,
            "us_m4a1_75",
            3,
            Team::Us,
            Vec3::new(0.0, 1.0, 16.0),
            yaw_quat(core::f32::consts::FRAC_PI_2),
        )
        .unwrap(),
    );
    ecs.insert_resource(vehicles);

    let mut guns = AtGuns::default();
    guns.list.insert(
        2,
        AtGunState::new(2, 2, Vec3::Z * 8.0, core::f32::consts::PI, &us),
    );
    ecs.insert_resource(guns);
    let mut shooter = AtGunState::new(1, 1, Vec3::ZERO, 0.0, &ger);
    assert!(shooter.occupy(actor(1, Team::Ger, "anti_tank"), Seat::Gunner));
    let initial_hull = ecs.resource::<Vehicles>().list[&3].hull_hp;
    let mut shots = 0;
    loop {
        assert!(shooter
            .fire(1, None, HZ, &mut ecs.resource_mut::<Shells>())
            .is_some());
        shots += 1;
        for _ in 0..2 {
            let mut tick = ecs.resource_mut::<CurrentTick>();
            tick.0 = tick.0.next();
            schedule.run(&mut ecs);
        }
        let target = &ecs.resource::<Vehicles>().list[&3];
        assert!(ecs.resource::<Shells>().list.is_empty());
        if shots <= 2 {
            assert_eq!(target.hull_hp, initial_hull);
            assert_eq!(
                ecs.resource::<AtGuns>().list[&2].hp,
                if shots == 1 { 200.0 } else { 0.0 }
            );
        }
        if target.knocked_out {
            break;
        }
        assert!(shots < 18);
        let reload = shooter.ready_in_ticks();
        tick(&mut shooter, reload);
    }
    assert!(shots > 2);
    assert!(ecs
        .resource::<EventBuf>()
        .0
        .contains(&SimEvent::VehicleKnockedOut { vehicle: 3 }));

    let mut outposts = Outposts::default();
    for (id, team) in [(0x8000, Team::Ger), (0x8001, Team::Us)] {
        outposts.list.insert(
            id,
            OutpostState {
                id,
                team,
                squad: id,
                pos: Vec3::new(0.0, 0.0, 14.0),
                hp: 100.0,
                spawns_left: 18,
                expires_tick: 18_000,
                blocked: false,
                overrun_ticks: 0,
            },
        );
    }
    ecs.insert_resource(outposts);
    assert!(shooter.select_ammo(1, Ammo::He, HZ));
    let reload = shooter.ready_in_ticks();
    tick(&mut shooter, reload);
    assert!(shooter
        .fire(1, None, HZ, &mut ecs.resource_mut::<Shells>())
        .is_some());
    let impact_tick = ecs.resource::<CurrentTick>().0.next().0;
    ecs.resource_mut::<CurrentTick>().0 = Tick(impact_tick);
    schedule.run(&mut ecs);
    let outposts = ecs.resource::<Outposts>();
    assert_eq!(outposts.list[&0x8000].hp, 100.0);
    assert!(!outposts.list.contains_key(&0x8001));
    assert_eq!(outposts.cooldown[&0x8001], impact_tick + 3_600);
    assert!(ecs.resource::<EventBuf>().0.iter().any(|event| matches!(event,
        SimEvent::OutpostChanged { point, status: OutpostStatus::Destroyed } if point.id == 0x8001
    )));
}
