//! Authority checks on the actual world view, not on client-provided target hints.
mod common;

use frontline_sim::comms::ping::{identify, TargetKind};
use frontline_sim::{GameData, SpawnReq, Team, Vec3, World};

#[test]
fn ray_classifies_infantry_and_respects_terrain_los() {
    for blocked in [false, true] {
        let collision = common::synth_data(1, 1, &|_, z| {
            if blocked && (29.0..=31.0).contains(&z) {
                9.0
            } else {
                5.0
            }
        });
        let mut world = World::new(common::sim_cfg(GameData::embedded(), 1), &collision);
        for (player, team, z) in [(1, Team::Us, 15.0), (2, Team::Ger, 45.0)] {
            world.roster_add(player, team).unwrap();
            world
                .spawn_player(SpawnReq {
                    player,
                    team,
                    pos: Vec3::new(30.0, 5.05, z),
                    yaw: 0.0,
                })
                .unwrap();
        }
        world.step(&[]);
        let aimed = identify(&world, 1, 400.0);
        if blocked {
            assert_eq!(aimed, None);
        } else {
            let aimed = aimed.unwrap();
            assert_eq!(aimed.key.kind, TargetKind::Infantry);
            assert_eq!(aimed.team, Team::Ger);
            assert!(!aimed.downed);
            assert_eq!(identify(&world, 1, 29.0), None);
            assert_eq!(identify(&world, 2, 400.0), None);
        }
    }
}

#[test]
fn ray_classifies_vehicle_and_tracked_position() {
    let data = GameData::embedded();
    let collision = common::flat(1, 5.0);
    let mut world = World::new(common::sim_cfg(data, 1), &collision);
    world.roster_add(1, Team::Us).unwrap();
    world
        .spawn_player(SpawnReq {
            player: 1,
            team: Team::Us,
            pos: Vec3::new(30.0, 5.05, 15.0),
            yaw: 0.0,
        })
        .unwrap();
    let id = world
        .spawn_vehicle("us_m4a1_75", Team::Ger, Vec3::new(30.0, 6.65, 45.0), 0.0)
        .unwrap();
    world.step(&[]);
    let target = identify(&world, 1, 400.0).unwrap();
    assert_eq!(target.key.kind, TargetKind::Vehicle);
    assert_eq!(target.key.id, id);
    assert!(frontline_sim::comms::ping::observed_position(&world, 1, target.key, 400.0).is_some());
    assert!(frontline_sim::comms::ping::observed_position(&world, 1, target.key, 10.0).is_none());
}
