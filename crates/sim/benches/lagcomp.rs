//! PI-072 sim-side costs. Run with `cargo bench -p frontline-sim --bench lagcomp`.

use std::hint::black_box;

use bevy_ecs::prelude::World;
use criterion::{criterion_group, criterion_main, Criterion};
use frontline_sim::combat::lagcomp::{resolve_fire, LagComp};
use frontline_sim::combat::lagcomp_history::HISTORY_TICKS;
use frontline_sim::components::{CurrentTick, EntityTable, Motion, MoveParamsRes, Transform};
use frontline_sim::data::GameData;
use frontline_sim::health::Health;
use frontline_sim::movement::{MoveParams, PlayerAuthState};
use frontline_sim::vehicle::{Vehicle, Vehicles};
use frontline_sim::{Team, Tick, Vec3};

fn benchmark(c: &mut Criterion) {
    let mut world = World::new();
    let data = GameData::embedded();
    let role = data.role("rifleman").unwrap();
    world.insert_resource(MoveParamsRes(Some(
        MoveParams::from_role(role, 30).unwrap(),
    )));
    world.insert_resource(CurrentTick(Tick(0)));
    world.insert_resource(LagComp::default());
    let mut table = EntityTable::default();
    for id in 0..100 {
        let pos = Vec3::X * f32::from(id);
        let mut auth = PlayerAuthState::spawn(pos, 1.0);
        auth.lean = 0.5;
        let entity = world
            .spawn((
                Transform {
                    pos,
                    yaw: auth.yaw,
                    pitch: auth.pitch,
                },
                Motion::from_auth(&auth),
                Health::full(100.0),
            ))
            .id();
        table.insert(id, entity);
    }
    world.insert_resource(table);
    let mut vehicles = Vehicles::default();
    for id in 100..140 {
        let pos = Vec3::X * f32::from(id);
        let vehicle =
            Vehicle::from_def(data, "us_m4a1_75", id, Team::Us, pos, glam::Quat::IDENTITY).unwrap();
        vehicles.list.insert(id, vehicle);
    }
    world.insert_resource(vehicles);
    let mut record = || {
        let mut tick = world.resource_mut::<CurrentTick>();
        tick.0 = tick.0.next();
        LagComp::record_world(&mut world);
    };
    for _ in 0..HISTORY_TICKS {
        record();
    }
    c.bench_function("record_history_100_players_40_vehicles", |b| {
        b.iter(&mut record)
    });
    c.bench_function("resolve_fire", |b| {
        b.iter(|| resolve_fire(black_box(12), black_box(Vec3::X), black_box(Vec3::ZERO)))
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
