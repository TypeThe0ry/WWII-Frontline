//! Snapshot byte budgets (ARCHITECTURE §6.9.1), the in-memory 60 s client/server replay, golden
//! bytes and (ignored) timing. Numbers are printed; run with `-- --nocapture`.

use frontline_protocol::snapshot::*;

const DT_PATTERN: [u32; 2] = [1, 2]; // 20 Hz snapshots on a 30 Hz sim: +1, +2 ticks alternating

fn walker(i: usize, x: f32, yaw: f32) -> EntityState {
    EntityState {
        id: (i * 3) as u8,
        kind: EntityKind::Player,
        pos: QPos::from_meters(x + i as f32 * 5.0, 12.0, 300.0 + i as f32 * 2.0),
        yaw: frontline_protocol::quant::quant_yaw(yaw + i as f32, 10).code,
        pitch: 32,
        vel: None,
        anim: 1,
        stance: 0,
        weapon: 2,
        flags: 0,
        health_q: 255,
        vehicle: None,
    }
}

#[derive(Default, Debug)]
struct Stats {
    delta_bits: usize,
    delta_n: usize,
    key_bits: usize,
    key_n: usize,
    packet_bytes: Vec<usize>,
}

impl Stats {
    fn delta_avg_b(&self) -> f64 {
        self.delta_bits as f64 / self.delta_n.max(1) as f64 / 8.0
    }
    fn key_avg_b(&self) -> f64 {
        self.key_bits as f64 / self.key_n.max(1) as f64 / 8.0
    }
}

/// 60 s replay: `n` entities walking 4 m/s on a straight line, optionally turning, in-memory
/// server -> client with acks arriving `ack_delay` snapshots late. `near(i)` picks the class.
fn replay(n: usize, turning: bool, ack_delay: usize, near: impl Fn(usize) -> bool) -> Stats {
    let mut enc = SnapshotEncoder::new();
    let mut dec = SnapshotDecoder::new();
    let mut st = Stats::default();
    let (mut tick, mut t_s) = (1000u32, 0f32);
    let mut acks: Vec<(usize, u32)> = Vec::new();
    for snap in 0..1200usize {
        let dt = DT_PATTERN[snap % 2];
        tick += dt;
        t_s += dt as f32 / 30.0;
        let x = 100.0 + 4.0 * t_s;
        let yaw = if turning { 0.2 * t_s } else { 0.0 };
        let mut b = enc.begin(1, tick, DEFAULT_BUDGET);
        let mut want = Vec::new();
        for i in 0..n {
            let s = walker(i, x, yaw);
            assert!(b.add_entity(&s, near(i)).unwrap());
            want.push((s, near(i)));
        }
        let (k0, d0) = b.record_counts();
        assert_eq!(k0 + d0, n);
        if snap > 0 && k0 == 0 {
            st.delta_bits += b.entity_bits();
            st.delta_n += n;
        } else if k0 == n {
            st.key_bits += b.entity_bits();
            st.key_n += n;
        }
        let bytes = b.finish();
        st.packet_bytes.push(bytes.len());
        let d = dec.decode(&bytes).unwrap();
        assert_eq!(d.entities.len(), n);
        for (i, (s, nr)) in want.iter().enumerate() {
            let got = d.entities.state(i).unwrap();
            assert_eq!(got.pos, s.pos);
            if *nr || snap == 0 {
                assert_eq!(got.yaw, s.yaw);
            }
        }
        acks.push((snap + ack_delay, d.tick));
        for (due, t) in acks.iter().filter(|(due, _)| *due == snap) {
            let _ = due;
            enc.on_ack(1, *t);
        }
    }
    st
}

#[test]
fn record_sizes_walking_4ms() {
    let near_pos = replay(40, false, 0, |_| true);
    let near_turn = replay(40, true, 0, |_| true);
    let far_turn = replay(40, true, 0, |_| false);
    let near_lag = replay(40, true, 3, |_| true);
    println!(
        "near delta, straight walk, ack lag <=2 ticks: {:.2} B/record\n\
         near delta, turning (yaw+pos): {:.2} B/record\n\
         far  delta, turning: {:.2} B/record\n\
         near delta, turning, ack 3 snapshots late (~100 ms RTT): {:.2} B/record\n\
         keyframe: {:.2} B/record",
        near_pos.delta_avg_b(),
        near_turn.delta_avg_b(),
        far_turn.delta_avg_b(),
        near_lag.delta_avg_b(),
        near_pos.key_avg_b()
    );
    assert!(near_pos.delta_avg_b() <= 10.0);
    assert!(near_turn.delta_avg_b() <= 10.0);
    assert!(far_turn.delta_avg_b() <= 9.0);
    assert!(near_lag.delta_avg_b() <= 10.0);
    assert!(near_pos.key_avg_b() <= 13.0, "{}", near_pos.key_avg_b());
}

#[test]
fn self_state_size() {
    let mut enc = SnapshotEncoder::new();
    for (pos_corr, expected_bits) in [(None, 56), (Some(QPos { x: 1, y: 2, z: 3 }), 101)] {
        for (vehicle_slot, seat) in [(0, 0), (128, 0), (255, 2)] {
            let mut b = enc.begin(1, 1, DEFAULT_BUDGET);
            let before = b.bits_used();
            let s = SelfState {
                pos_corr,
                ammo_in_mag: 30,
                ammo_reserve: 120,
                health: 100,
                stamina: 40,
                vehicle_slot,
                seat,
                ..Default::default()
            };
            assert!(b.add_self(&s).unwrap());
            assert_eq!(b.bits_used() - before, expected_bits);
        }
    }
}

#[test]
fn typical_40_entity_snapshot() {
    // 4 near + 10 mid (near format) + 26 far, deltas, with self_state and 5 Shot events
    let mut enc = SnapshotEncoder::new();
    let mut dec = SnapshotDecoder::new();
    let mut tick = 500;
    let mut sizes = Vec::new();
    let evs: Vec<EventEntry> = (0..5)
        .map(|i| EventEntry {
            id: i,
            event: Event::Shot {
                shooter: 1,
                weapon: 2,
                muzzle: [1, 2, 3],
                dir_yaw: 5,
                dir_pitch: 6,
            },
        })
        .collect();
    for snap in 0..200 {
        tick += DT_PATTERN[snap % 2];
        let t = tick as f32 / 30.0;
        let mut b = enc.begin(1, tick, DEFAULT_BUDGET);
        b.add_self(&SelfState::default()).unwrap();
        b.add_events(&evs).unwrap();
        for i in 0..40 {
            let s = walker(i, 100.0 + 4.0 * t, 0.2 * t);
            assert!(b.add_entity(&s, i < 14).unwrap());
        }
        let bytes = b.finish();
        if snap > 2 {
            sizes.push(bytes.len());
        }
        let d = dec.decode(&bytes).unwrap();
        enc.on_ack(1, d.tick);
    }
    let max = *sizes.iter().max().unwrap();
    let avg = sizes.iter().sum::<usize>() as f64 / sizes.len() as f64;
    println!(
        "typical 40-entity delta snapshot (+self +5 Shot events): avg {avg:.0} B, max {max} B"
    );
    assert!(max <= 600);
    // keyframe-only worst case for the same 40 entities still fits the 1150 B budget
    let mut e2 = SnapshotEncoder::new();
    let mut b = e2.begin(1, 1, DEFAULT_BUDGET);
    for i in 0..40 {
        assert!(b.add_entity(&walker(i, 100.0, 0.0), true).unwrap());
    }
    let k = b.finish().len();
    println!("40 entities, all keyframes: {k} B");
    assert!(k <= DEFAULT_BUDGET);
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Golden bytes: regenerate deliberately with `UPDATE_GOLDEN=1` (breaking change: bump
/// `PROTOCOL_VERSION`).
#[test]
fn golden_bytes() {
    let mut enc = SnapshotEncoder::new();
    let mk = |i: usize, dx: f32| walker(i, 100.0 + dx, 1.0);
    let mut b = enc.begin(1, 90_000, DEFAULT_BUDGET);
    b.set_ack(777, 3).unwrap();
    b.add_self(&SelfState {
        pos_corr: Some(QPos {
            x: 3200,
            y: 384,
            z: 9600,
        }),
        ammo_in_mag: 30,
        ammo_reserve: 210,
        health: 100,
        stamina: 50,
        suppression: 2,
        stance: 1,
        deploy_state: 0,
        cooldown_bits: 0b101,
        vehicle_slot: 255,
        seat: 2,
    })
    .unwrap();
    b.add_events(&[
        EventEntry {
            id: 10,
            event: Event::Shot {
                shooter: 3,
                weapon: 1,
                muzzle: [100, 200, 300],
                dir_yaw: 512,
                dir_pitch: 256,
            },
        },
        EventEntry {
            id: 11,
            event: Event::HitConfirm {
                target: 6,
                zone: 1,
                damage_q: 40,
            },
        },
    ])
    .unwrap();
    for i in 0..3 {
        b.add_entity(&mk(i, 0.0), true).unwrap();
    }
    let key = b.finish();
    enc.on_ack(1, 90_000);
    let mut b = enc.begin(1, 90_002, DEFAULT_BUDGET);
    b.set_ack(779, 3).unwrap();
    b.add_self(&SelfState::default()).unwrap();
    b.add_entity(&mk(0, 0.4), true).unwrap();
    b.add_entity(&mk(1, 0.4), false).unwrap();
    b.add_removed(6).unwrap();
    let delta = b.finish();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    for (name, bytes) in [("snapshot_key", &key), ("snapshot_delta", &delta)] {
        let path = dir.join(format!("{name}.hex"));
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::write(&path, format!("{}\n", hex(bytes))).unwrap();
        }
        let want = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing {name}"));
        assert_eq!(hex(bytes), want.trim(), "golden mismatch for {name}");
    }
    let mut dec = SnapshotDecoder::new();
    dec.decode(&key).unwrap();
    let d = dec.decode(&delta).unwrap();
    assert_eq!(d.removed_ids, vec![6]);
}

/// Timing is only recorded. Run: `cargo test -p frontline-protocol --release --test
/// snapshot_size -- --ignored --nocapture bench_snapshot`.
#[test]
#[ignore]
fn bench_snapshot() {
    let mut enc = SnapshotEncoder::new();
    let mut dec = SnapshotDecoder::new();
    let n = 60;
    let mut tick = 1000;
    // warm baseline
    let mk = |t: u32| -> Vec<EntityState> {
        (0..n)
            .map(|i| walker(i % 80, 100.0 + t as f32 * 0.13, t as f32 * 0.01))
            .collect()
    };
    let mut b = enc.begin(1, tick, DEFAULT_BUDGET);
    for s in &mk(tick) {
        b.add_entity(s, true).unwrap();
    }
    let first = b.finish();
    dec.decode(&first).unwrap();
    enc.on_ack(1, tick);
    let iters = 200_000u32;
    let mut total = 0usize;
    let mut last = Vec::new();
    let t0 = std::time::Instant::now();
    for _ in 0..iters {
        tick += 1;
        let states = mk(tick); // building input is part of the measured loop's cost
        let mut b = enc.begin(1, tick, DEFAULT_BUDGET);
        b.set_ack(1, 1).unwrap();
        b.add_self(&SelfState::default()).unwrap();
        for (i, s) in states.iter().enumerate() {
            b.add_entity(s, i < 30).unwrap();
        }
        last = b.finish();
        total += last.len();
        enc.on_ack(1, tick);
    }
    let enc_us = t0.elapsed().as_secs_f64() * 1e6 / iters as f64;
    // build-inputs-only cost to subtract
    let t1 = std::time::Instant::now();
    let mut sink = 0u32;
    for k in 0..iters {
        sink = sink.wrapping_add(mk(k).len() as u32);
    }
    let in_us = t1.elapsed().as_secs_f64() * 1e6 / iters as f64;
    // decode the same delta packet repeatedly against a decoder holding its baseline
    let mut dec = SnapshotDecoder::new();
    let mut e2 = SnapshotEncoder::new();
    let mut b = e2.begin(1, 10, DEFAULT_BUDGET);
    for s in &mk(10) {
        b.add_entity(s, true).unwrap();
    }
    dec.decode(&b.finish()).unwrap();
    e2.on_ack(1, 10);
    let mut b = e2.begin(1, 11, DEFAULT_BUDGET);
    for s in &mk(11) {
        b.add_entity(s, true).unwrap();
    }
    let pkt = b.finish();
    let t2 = std::time::Instant::now();
    for _ in 0..iters {
        total += dec.decode(&pkt).unwrap().entities.len();
    }
    let dec_us = t2.elapsed().as_secs_f64() * 1e6 / iters as f64;
    println!(
        "snapshot_encode_client (60 ents, 30 near + self): {:.1} us incl. input build ({in_us:.1} us) -> {:.1} us; \
         snapshot_decode: {dec_us:.1} us; last packet {} B (checksum {total} {sink})",
        enc_us,
        enc_us - in_us,
        last.len()
    );
}
