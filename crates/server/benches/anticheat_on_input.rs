//! Stage 1 input ingestion including the validator, metrics and anti-cheat policy.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use frontline_protocol::msg::input::{InputBundle, InputCmd};
use frontline_server::{metrics::Metrics, sim_bridge::SimBridge, validate::ValidateCfg};
use frontline_sim::{CollisionData, GameData, GameMode, MapId, SimConfig, Team, World};

fn fixture(players: u8) -> (SimBridge, World) {
    let mut world = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 1,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        &CollisionData::empty(),
    );
    let mut bridge = SimBridge::new(Metrics::new(), None, None, ValidateCfg::default());
    for id in 0..players {
        bridge.join(&mut world, id, Team::Us).unwrap();
    }
    (bridge, world)
}

fn cmd(tick: u32) -> InputCmd {
    InputCmd {
        seq: tick as u16,
        tick,
        move_x: 30,
        move_z: 120,
        yaw: tick.wrapping_mul(94) as u16,
        pitch: 0,
        buttons: 0,
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    }
}

fn anticheat_on_input(c: &mut Criterion) {
    let mut group = c.benchmark_group("anticheat_on_input");
    let (mut single, _) = fixture(1);
    let mut tick = 0u32;
    group.throughput(Throughput::Elements(1));
    group.bench_function("one_input", |b| {
        b.iter(|| {
            tick += 1;
            let command = cmd(tick);
            black_box(single.ingest_cmd(0, black_box(&command), tick));
            black_box(single.resolve(tick).len());
        });
    });

    let (mut full, _) = fixture(100);
    let mut tick = 0;
    group.throughput(Throughput::Elements(300));
    group.bench_function("100_players_3_inputs", |b| {
        b.iter(|| {
            tick += 3;
            let bundle = InputBundle {
                ack_snapshot_tick: 0,
                pred_hash: 0,
                cmds: [cmd(tick - 2), cmd(tick - 1), cmd(tick)]
                    .into_iter()
                    .collect(),
                fire: Default::default(),
            };
            // All 300 commands are fresh. Each tick's earlier two commands are usable
            // delayed inputs. Normal redundant bundles are cheaper to validate.
            for id in 0..100 {
                full.ingest_bundle(id, black_box(&bundle), tick);
            }
            black_box(full.resolve(tick).len());
        });
    });
    group.finish();
}

criterion_group!(benches, anticheat_on_input);
criterion_main!(benches);
