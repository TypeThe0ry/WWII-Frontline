//! Real scheduler and codec with deterministic C1 feedback. OS impairment is tested separately.
mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use common::synth::{new_world, Synth};
use frontline_protocol::msg::input::InputCmd;
use frontline_server::metrics::Metrics;
use frontline_server::net::{regulator::BudgetNotice, transport_stats::LinkSample};
use frontline_server::snapshot::{LinkInfo, SnapshotScheduler};
use frontline_sim::{SimEvent, Team, Vec3};

fn idle(_player: u8, tick: u32) -> InputCmd {
    InputCmd {
        seq: tick as u16,
        tick,
        move_x: 0,
        move_z: 0,
        yaw: 0,
        pitch: 0,
        buttons: 0,
        weapon_slot: 0,
        aux: 0,
        view_tick: 2,
    }
}

fn scene(link: LinkInfo) -> (Synth, Arc<Metrics>) {
    let mut s = Synth::new(new_world(None));
    let metrics = Metrics::new();
    s.sched = SnapshotScheduler::new(Some(metrics.clone()));
    let center = Vec3::new(1000.0, 0.0, 1000.0);
    s.add(0, Team::Us, center, 0.0, Some(link));
    let mut id = 1;
    for (dist, count) in [(20.0, 4), (64.0, 10), (150.0, 14), (300.0, 22)] {
        for i in 0..count {
            let a = i as f32 * std::f32::consts::TAU / count as f32;
            s.add(
                id,
                Team::Us,
                center + Vec3::new(a.cos() * dist, 0.0, a.sin() * dist),
                0.0,
                None,
            );
            id += 1;
        }
    }
    s.add(51, Team::Ger, center - Vec3::Z * 150.0, 0.0, None);
    (s, metrics)
}

fn sample(capacity: u64) -> LinkSample {
    LinkSample {
        capacity_bytes_per_sec: Some(capacity),
        rtt_micros: 150_000,
    }
}

#[test]
fn lossless_regulator_matches_m1_packet_path_byte_for_byte() {
    for link in [LinkInfo::DATAGRAM, LinkInfo::WS] {
        let (mut base, _) = scene(link);
        let (mut regulated, m) = scene(link.with_sample(sample(1_000_000)));
        let mut packets = 0;
        for _ in 0..600 {
            base.step(idle);
            regulated.step(idle);
            let a = base.outgoing().iter().map(|p| &p.bytes).collect::<Vec<_>>();
            let b = regulated
                .outgoing()
                .iter()
                .map(|p| &p.bytes)
                .collect::<Vec<_>>();
            assert_eq!(a, b, "tick {}", base.tick);
            packets += b.len();
            assert_eq!(
                regulated
                    .sched
                    .client(0)
                    .unwrap()
                    .regulator
                    .as_ref()
                    .unwrap()
                    .scale(),
                1.0
            );
            assert!(regulated.sched.drain_budget_notices().next().is_none());
        }
        assert_eq!(packets, if link.hz15 { 300 } else { 400 });
        assert!(m.net_budget_scale.get_sample_count() > 0);
        assert_eq!(m.net_regulated_clients.get(), 0);
        assert_eq!(m.aoi_enemy_leak_total.get(), 0);
    }
}

#[test]
fn floor_retains_t3_frequency_and_recovers_via_full_records() {
    let (mut s, m) = scene(LinkInfo::DATAGRAM.with_sample(sample(1_000)));
    s.ack_delay = 5;
    let mut extra_removed = 0;
    for _ in 0..300 {
        s.step(idle);
        extra_removed += s
            .outgoing()
            .iter()
            .map(|_| s.client(0).log.last().unwrap().removed)
            .sum::<usize>();
    }
    assert!(extra_removed > 0);
    let scale = |s: &Synth| {
        s.sched
            .client(0)
            .unwrap()
            .regulator
            .as_ref()
            .unwrap()
            .scale()
    };
    assert_eq!(scale(&s), 0.4);
    let start = s.client(0).log.len();
    let starved_before = m.aoi_starved_total.get();
    for _ in 0..1800 {
        s.step(idle);
        assert_eq!(scale(&s), 0.4);
    }
    let mut updates: BTreeMap<u8, Vec<u32>> = BTreeMap::new();
    for p in &s.client(0).log[start..] {
        assert!(p.bytes <= 460);
        for &id in &p.ids {
            if (29..=50).contains(&id) {
                updates.entry(id).or_default().push(p.tick);
            }
        }
    }
    assert_eq!(updates.len(), 8, "keep floor(22 * 0.4) T3 players");
    assert!(updates.values().all(|ts| ts.len() >= 119));
    let mut gaps = updates
        .values()
        .flat_map(|ts| ts.windows(2).map(|t| t[1] - t[0]))
        .collect::<Vec<_>>();
    gaps.sort_unstable();
    let p99 = gaps[(gaps.len() * 99).div_ceil(100) - 1];
    let starved = m.aoi_starved_total.get() - starved_before;
    eprintln!(
        "T3 updates: {} entities, {} intervals, p99={} ticks, starved={starved}/min",
        updates.len(),
        gaps.len(),
        p99
    );
    assert!(p99 <= 18);
    assert_eq!(starved, 0);
    assert!(m.net_far_shed_total.get() > 0);
    assert_eq!(m.net_regulated_clients.get(), 1);
    assert_eq!(m.aoi_enemy_leak_total.get(), 0);

    let at_floor: BTreeSet<_> = updates.keys().copied().collect();
    let start = s.client(0).log.len();
    s.client_mut(0).link = LinkInfo::DATAGRAM.with_sample(sample(100_000));
    let mut recovered = None;
    for tick in 1..=300 {
        s.step(idle);
        if scale(&s) >= 0.95 {
            recovered = Some(tick);
            assert_eq!(
                s.sched.drain_budget_notices().collect::<Vec<_>>(),
                vec![(0, BudgetNotice::Recovered)]
            );
            break;
        }
    }
    eprintln!("capacity restored to scale >=0.95 after {recovered:?} ticks");
    assert!(recovered.is_some());
    let mut restored = BTreeSet::new();
    for p in &s.client(0).log[start..] {
        for &id in &p.ids {
            if (29..=50).contains(&id) && !at_floor.contains(&id) && restored.insert(id) {
                assert!(p.kf_ids.contains(&id), "re-entry of {id} needs a keyframe");
            }
        }
    }
    assert_eq!(restored.len(), 14);
}

#[test]
#[ignore = "manual release measurement: 30 simulated minutes, 100 players, 20 decoders"]
fn dense_150_events_per_second_scheduler_soak() {
    const TICKS: u32 = 54_000;
    let mut s = Synth::new(new_world(None));
    let center = Vec3::new(1000.0, 0.0, 1000.0);
    let link = LinkInfo::DATAGRAM.with_sample(sample(100_000));
    for i in 0..100u8 {
        let dist = if i < 20 {
            10.0
        } else {
            [25.0, 65.0, 140.0, 280.0][i as usize % 4]
        };
        let angle = std::f32::consts::TAU * f32::from(i) / 100.0;
        s.add(
            i,
            Team::Us,
            center + Vec3::new(dist * angle.sin(), 0.0, dist * angle.cos()),
            0.0,
            (i < 20).then_some(link),
        );
    }
    let impacts = std::array::from_fn::<_, 5, _>(|i| SimEvent::Impact {
        pos: center + Vec3::X * i as f32,
        normal: Vec3::Y,
        material: 0,
    });
    for t in 1..=TICKS {
        s.step_with_events(idle, &impacts);
        if t.is_multiple_of(30) {
            for c in &mut s.clients {
                assert_eq!(
                    s.sched
                        .client(c.player)
                        .unwrap()
                        .regulator
                        .as_ref()
                        .unwrap()
                        .scale(),
                    1.0
                );
                c.log.clear();
                c.events.clear();
                c.seen_events.clear();
                c.event_arrivals.clear();
            }
        }
    }
    assert!(s
        .clients
        .iter()
        .all(|c| c.packets_ok == 36_000 && c.missing_baseline == 0));
    let wire = &mut s.sched.stats.kbps_wire;
    assert_eq!(wire.len(), 36_000);
    wire.sort_unstable_by(f32::total_cmp);
    let p99 = wire[(wire.len() * 99).div_ceil(100) - 1];
    eprintln!(
        "dense fixture: {TICKS} ticks, {} generated events, {} wire estimates, P99={p99:.3} kbps",
        TICKS * 5,
        wire.len()
    );
    assert!(p99 <= 400.0);
}
