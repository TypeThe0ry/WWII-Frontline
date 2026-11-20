#![cfg(feature = "test-util")]

mod common;

use common::{cmd, flat, sim_cfg};
use frontline_sim::spawn::garrison::SpawnRequestError;
use frontline_sim::spawn::outpost::OutpostError;
use frontline_sim::squad::{SquadKind, SquadOp};
use frontline_sim::{GameData, SpawnReq, Team, Vec3, World};

const SITE: Vec3 = Vec3::new(220.0, 5.0, 220.0);

fn fixture() -> World {
    let mut w = World::new(sim_cfg(GameData::embedded(), 1011), &flat(6, 5.0));
    w.set_hq(Team::Us, Vec3::new(28.0, 5.0, 28.0));
    w.set_hq(Team::Ger, Vec3::new(360.0, 5.0, 360.0));
    for player in 0..=4 {
        w.roster_add(player + 20, Team::Ger).unwrap();
        w.roster_add(player, Team::Us).unwrap();
    }
    w.roster_add(5, Team::Ger).unwrap();
    for player in 0..=5 {
        let team = if player == 5 { Team::Ger } else { Team::Us };
        w.spawn_player(SpawnReq {
            player,
            team,
            pos: if player >= 4 {
                Vec3::new(40.0, 5.0, 200.0)
            } else {
                SITE + Vec3::X * f32::from(player)
            },
            yaw: 0.0,
        })
        .unwrap();
    }
    w.squad_op(
        0,
        SquadOp::Create {
            kind: SquadKind::Infantry,
            suffix: String::new(),
        },
    )
    .unwrap();
    let squad = w.squads().squad_of(0).unwrap().id;
    for player in 1..=3 {
        w.squad_op(player, SquadOp::Join { squad }).unwrap();
    }
    w.set_role(0, Some("officer")).unwrap();
    step(&mut w, 2);
    w
}

fn step(w: &mut World, ticks: u32) {
    for _ in 0..ticks {
        w.step(&[]);
    }
}

#[test]
fn outpost_place_rules() {
    let mut w = fixture();
    w.debug_teleport(2, SITE - Vec3::X * 30.0);
    w.debug_teleport(3, SITE - Vec3::X * 40.0);
    assert_eq!(w.place_outpost(0, SITE), Err(OutpostError::NeedSquadmates));
    w.debug_teleport(2, SITE + Vec3::X * 2.0);
    w.debug_teleport(5, SITE + Vec3::X * 25.0);
    assert!(w.preview_outpost(0, SITE).is_ok());
    assert_eq!(w.place_outpost(0, SITE), Err(OutpostError::EnemyNearby));
    w.debug_teleport(5, Vec3::new(40.0, 5.0, 200.0));
    w.set_objectives(&[frontline_sim::rules::ObjectiveSpec {
        id: 1,
        pos: SITE + Vec3::Z * 69.0,
        radius_m: 10.0,
        owner: Some(Team::Ger),
    }]);
    assert_eq!(w.place_outpost(0, SITE), Err(OutpostError::TooCloseEnemySP));
    w.set_objectives(&[]);
    assert!(w.place_outpost(0, SITE).is_ok());
    w.step(&[(0, cmd(80, 0, 0, 0, 0))]);
    step(&mut w, 179);
    assert_eq!(w.outpost_list().count(), 0);
    w.debug_teleport(0, SITE);
    w.step(&[(0, cmd(0, 0, 0, 0, 0))]);
    w.place_outpost(0, SITE).unwrap();
    w.debug_set_hp(0, 95.0);
    step(&mut w, 180);
    assert_eq!(w.outpost_list().count(), 0);
    w.place_outpost(0, SITE).unwrap();
    step(&mut w, 179);
    assert_eq!(w.outpost_list().count(), 0);
    step(&mut w, 1);
    let id = w.outpost_list().next().unwrap().id;
    w.damage_outpost(id, 100.0, false);
    assert_eq!(w.outpost_list().next().unwrap().hp, 280.0);
    w.damage_outpost(id, 280.0, true);
    step(&mut w, 3599);
    assert_eq!(w.place_outpost(0, SITE), Err(OutpostError::Cooldown));
    step(&mut w, 1);
    assert!(w.preview_outpost(0, SITE).is_ok());
    step(&mut w, 1);
    w.place_outpost(0, SITE).unwrap();
}

#[test]
fn outpost_lifecycle_headless_squad() {
    let mut w = fixture();
    w.place_outpost(0, SITE).unwrap();
    step(&mut w, 180);
    let id = w.outpost_list().next().unwrap().id;
    let mut twin = fixture();
    twin.place_outpost(0, SITE).unwrap();
    step(&mut twin, 180);
    assert_eq!(w.state_hash(), twin.state_hash());
    w.squad_op(0, SquadOp::Promote { target: 1 }).unwrap();
    w.debug_kill(0);
    step(&mut w, 1);
    assert_eq!(w.outpost_list().next().unwrap().id, id);
    w.squad_op(1, SquadOp::AcceptLead).unwrap();
    assert_eq!(w.squads().squad_of(1).unwrap().leader, 1);
    w.debug_kill(4);
    step(&mut w, 1);
    assert_eq!(w.request_spawn(4, id), Err(SpawnRequestError::NoSuchPoint));
    for wave in 0..6 {
        for p in 1..=3 {
            w.debug_kill(p);
        }
        step(&mut w, 1);
        for p in 1..=3 {
            assert_eq!(w.request_spawn(p, id), Ok(360));
        }
        step(&mut w, 359);
        for p in 1..=3 {
            assert!(w.player_life(p).unwrap().is_dead());
        }
        step(&mut w, 1);
        for p in 1..=3 {
            assert!(w.player_state(p).unwrap().pos.distance(SITE) < 8.01);
            assert_eq!(w.spawn_protection_left(p), 90);
        }
        if wave < 5 {
            assert_eq!(
                w.outpost_list().next().unwrap().spawns_left,
                18 - 3 * (wave + 1)
            );
        }
        // Let the spawn shield expire before the next synthetic KIA.
        step(&mut w, 90);
    }
    assert_eq!(w.outpost_list().count(), 0);
    w.debug_kill(1);
    step(&mut w, 1);
    assert_eq!(w.request_spawn(1, id), Err(SpawnRequestError::NoSuchPoint));
}

#[test]
fn outpost_lifecycle_overrun_and_expiry() {
    let mut w = fixture();
    w.place_outpost(0, SITE).unwrap();
    step(&mut w, 180);
    let point = *w.outpost_list().next().unwrap();
    assert_eq!(point.expires_tick - w.tick().0, 18_000);
    for p in 0..=3 {
        w.debug_teleport(p, SITE + Vec3::Z * 40.0);
    }
    w.debug_teleport(5, SITE + Vec3::Z * 10.0);
    w.debug_kill(2);
    step(&mut w, 449);
    assert!(w.outpost_list().next().unwrap().blocked);
    // New KIA while the site is blocked rejects an explicit spawn request.
    w.debug_kill(3);
    w.debug_teleport(1, SITE + Vec3::X);
    step(&mut w, 1);
    assert_eq!(
        w.request_spawn(3, point.id),
        Err(SpawnRequestError::Blocked)
    );
    w.debug_teleport(1, SITE + Vec3::Z * 40.0);
    step(&mut w, 449);
    assert_eq!(w.outpost_list().count(), 1);
    step(&mut w, 1);
    assert_eq!(w.outpost_list().count(), 0);

    let mut w = fixture();
    w.place_outpost(0, SITE).unwrap();
    step(&mut w, 180);
    step(&mut w, 17_999);
    assert_eq!(w.outpost_list().count(), 1);
    step(&mut w, 1);
    assert_eq!(w.outpost_list().count(), 0);
}

#[test]
fn build_and_fire_at_gun_headless() {
    use frontline_protocol::msg::input::buttons;
    use frontline_sim::build::{BuildError, BuildState, StructureKind};
    use frontline_sim::emplacement::at_gun::Seat;
    use frontline_sim::SimEvent;

    let mut w = fixture();
    let site = SITE + Vec3::X * 2.0;
    assert_eq!(
        w.place_facility(2, StructureKind::AtGun, site, 0.0),
        Err(BuildError::WrongRole)
    );
    w.set_role(2, Some("anti_tank")).unwrap();
    w.set_role(3, Some("engineer")).unwrap();
    w.debug_add_crate(Team::Us, site, 320);
    let gun = w
        .place_facility(2, StructureKind::AtGun, site, 0.0)
        .unwrap();
    assert_eq!(w.supply_crates()[0].supplies, 120);
    w.debug_teleport(3, site + Vec3::X * 5.0);
    let station = w
        .place_facility(3, StructureKind::RepairStation, site + Vec3::X * 5.0, 0.0)
        .unwrap();
    step(&mut w, 449);
    assert_eq!(
        w.structures().find(|s| s.id == station).unwrap().state,
        BuildState::Building
    );
    step(&mut w, 1);
    assert_eq!(
        w.structures()
            .find(|s| s.id == station)
            .unwrap()
            .repair_station()
            .unwrap()
            .supplies(),
        200
    );
    step(&mut w, 149);
    assert!(w.at_gun(gun).is_none());
    step(&mut w, 1);
    assert!(w.enter_at_gun(2, gun, Seat::Gunner));
    let tank = w
        .spawn_vehicle(
            "us_m4a1_75",
            Team::Ger,
            site + Vec3::new(0., 1., 16.),
            std::f32::consts::FRAC_PI_2,
        )
        .unwrap();
    let mut shots = 0;
    let mut destroyed = false;
    for _ in 0..2250 {
        let tick = w.step(&[(2, cmd(0, 0, 0, buttons::FIRE, 0))]);
        for e in tick.events {
            if let SimEvent::MuzzleFlash(flash) = e {
                assert_eq!(
                    (flash.gun, flash.visual_ticks, flash.exposure_ticks),
                    (gun, 90, 150)
                );
                assert_eq!(flash.error_radius_m, 25.0);
                shots += 1;
            }
            if e == (SimEvent::VehicleKnockedOut { vehicle: tank }) {
                destroyed = true;
            }
        }
        if destroyed {
            break;
        }
    }
    assert!(destroyed, "Sherman not knocked out after {shots} shots");
    assert!((1..=18).contains(&shots));
    eprintln!("M2-011 integrated Sherman knockout after {shots} shots");
}
