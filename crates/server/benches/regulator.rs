use criterion::{criterion_group, criterion_main, Criterion};
use frontline_server::net::regulator::{BandwidthRegulator, RegulatorCfg};
use std::hint::black_box;

fn regulator(c: &mut Criterion) {
    let cfg = RegulatorCfg::default();
    let mut clients: Vec<_> = (0..100)
        .map(|_| BandwidthRegulator::new(cfg, 1150))
        .collect();
    let mut tick = 1u32;
    c.bench_function("regulator/100_clients_per_tick", |b| {
        b.iter(|| {
            for r in &mut clients {
                let due = !tick.is_multiple_of(3);
                let ack = tick.wrapping_sub(2);
                if !ack.is_multiple_of(3) && !ack.is_multiple_of(23) {
                    r.on_ack(ack);
                }
                r.on_rtt(100.0);
                r.tick(tick, due, Some(10_000));
                if due {
                    r.track_snapshot(tick);
                }
                r.on_send(if due { 300 } else { 0 });
                black_box((r.budget(1150), r.shed_far(22), r.notice(tick)));
            }
            tick = tick.wrapping_add(1);
        });
    });
}

criterion_group!(benches, regulator);
criterion_main!(benches);
