//! Stage-4 and stage-5 CPU budgets, sampled inside a full World tick.
#[path = "../tests/common/mod.rs"]
mod common;

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use common::{flat, sim_cfg};
use criterion::{criterion_group, criterion_main, Criterion};
use frontline_sim::build::StructureKind;
use frontline_sim::emplacement::at_gun::{Actor, AtGunState, Seat};
use frontline_sim::projectile::Projectiles;
use frontline_sim::squad::{SquadKind, SquadOp};
use frontline_sim::vehicle::shells::Shells;
use frontline_sim::{GameData, ObjectiveSpec, SpawnReq, Team, Vec3, World, WorldView};

fn ns() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_nanos() as u64
}

fn fixture() -> World {
    let mut w = World::new(sim_cfg(GameData::embedded(), 1011), &flat(12, 5.0));
    let site = |i: u16| Vec3::new(110.0 + 180.0 * f32::from(i), 5.0, 200.0);
    w.set_objectives(
        &(0..3)
            .map(|id| ObjectiveSpec {
                id,
                pos: site(id) + Vec3::Z * 60.0,
                radius_m: 16.0,
                owner: Some(Team::Us),
            })
            .collect::<Vec<_>>(),
    );
    for i in 0..18u8 {
        for (team, offset) in [(Team::Us, 0), (Team::Ger, 32)] {
            let player = i + offset;
            let center = site(u16::from(i / 6));
            let pos = center
                + match (team, i % 6) {
                    (Team::Ger, n) => Vec3::new(f32::from(n), 0.0, 75.0),
                    (_, 1) => Vec3::X * 4.0,
                    (_, 2) => Vec3::X * -4.0,
                    (_, n) => Vec3::Z * f32::from(n),
                };
            w.roster_add(player, team).unwrap();
            w.spawn_player(SpawnReq {
                player,
                team,
                pos,
                yaw: 0.0,
            })
            .unwrap();
            let leader = player - i % 6;
            if player == leader {
                w.squad_op(
                    player,
                    SquadOp::Create {
                        kind: SquadKind::Infantry,
                        suffix: String::new(),
                    },
                )
                .unwrap();
            } else {
                let squad = w.squads().squad_of(leader).unwrap().id;
                w.squad_op(player, SquadOp::Join { squad }).unwrap();
            }
        }
    }
    for i in 0..3u8 {
        let player = 6 * i;
        let pos = site(u16::from(i));
        w.set_role(player, Some("officer")).unwrap();
        w.set_role(player + 1, Some("engineer")).unwrap();
        w.set_role(player + 2, Some("anti_tank")).unwrap();
        w.debug_add_crate(Team::Us, pos, 320);
        w.place_outpost(player, pos).unwrap();
        w.place_facility(
            player + 1,
            StructureKind::RepairStation,
            pos + Vec3::X * 4.,
            0.,
        )
        .unwrap();
        w.place_facility(player + 2, StructureKind::AtGun, pos - Vec3::X * 4., 0.)
            .unwrap();
        let tank = w
            .spawn_vehicle("us_m4a1_75", Team::Us, pos + Vec3::new(9., 0., 3.), 0.)
            .unwrap();
        w.debug_ap_hit(
            tank,
            0,
            pos + Vec3::new(109.0, 1.5, 3.0),
            Vec3::NEG_X,
            100.0,
            frontline_sim::vehicle::shell::HitRolls {
                armor: 0.5,
                damage: 0.5,
                cookoff: 0.99,
                fire: 0.99,
            },
        );
    }
    for _ in 0..600 {
        w.step(&[]);
    }
    assert_eq!(w.outpost_list().count(), 3);
    assert_eq!(w.structures().count(), 6);
    w.enable_step_profiling(ns);
    w
}

fn flight_sample(world: &World) -> (Projectiles, Shells) {
    let mut bullets = Projectiles::default();
    let shooter = world
        .players()
        .iter()
        .find(|p| p.player == 32)
        .unwrap()
        .entity;
    let weapon = GameData::embedded()
        .weapons
        .iter()
        .position(|w| w.id == "ger_kar98k")
        .unwrap();
    for i in 0..399u16 {
        // Miss the facilities while exercising their nearest-hit scans at all three sites.
        let x = 110.0 + 180.0 * f32::from(i % 3) + 0.1 * f32::from(i % 17);
        bullets.spawn(
            shooter,
            Team::Ger,
            weapon as u16,
            Vec3::new(x, 8.0, 215.0),
            Vec3::NEG_Z * 750.0,
        );
    }
    let gun = world.structures().find_map(|s| world.at_gun(s.id)).unwrap();
    let mut ready = AtGunState::new(
        gun.id,
        gun.sector,
        gun.pos,
        gun.yaw,
        GameData::embedded().at_gun(gun.team),
    );
    let mut shells = Shells::default();
    assert!(ready.occupy(
        Actor {
            player: 2,
            team: Team::Us,
            role_id: "anti_tank"
        },
        Seat::Gunner
    ));
    assert!(ready.fire(2, None, 30, &mut shells).is_some());
    (bullets, shells)
}

fn bench(c: &mut Criterion) {
    c.bench_function("outpost_repair_atgun", |b| {
        b.iter_custom(|iterations| {
            let mut world = fixture();
            let mut measured = 0u64;
            for i in 0..iterations {
                if i > 0 && i % 15_000 == 0 {
                    world = fixture();
                }
                world.step(&[]);
                let perf = world.step_timings().unwrap();
                measured += perf.outpost_rules + perf.facility_rules;
            }
            Duration::from_nanos(measured)
        })
    });
    c.bench_function("outpost_400_projectile_scan", |b| {
        b.iter_custom(|iterations| {
            let mut world = fixture();
            let mut measured = 0u64;
            for i in 0..iterations {
                if i > 0 && i % 15_000 == 0 {
                    world = fixture();
                }
                let (projectiles, shells) = flight_sample(&world);
                world.debug_seed_facility_flight(projectiles, shells);
                world.step(&[]);
                let perf = world.step_timings().unwrap();
                measured += perf.projectiles + perf.rockets + perf.shells + perf.throwables;
            }
            Duration::from_nanos(measured)
        })
    });
}

criterion_group! { name = rules; config = Criterion::default().sample_size(10).measurement_time(Duration::from_secs(2)); targets = bench }
criterion_main!(rules);
