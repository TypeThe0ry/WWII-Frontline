//! Snapshot codec properties: round trips (keyframe and delta paths), baseline ring and
//! convergence under packet loss. Iteration count: 10^5 in release, 10^4 in debug
//! (`cargo test -p frontline-protocol --release snapshot_prop`).

use frontline_protocol::snapshot::*;
use frontline_protocol::CodecError;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn chance(&mut self, pct: u64) -> bool {
        self.below(100) < pct
    }
}

const ITER: usize = if cfg!(debug_assertions) {
    10_000
} else {
    100_000
};

fn rand_state(rng: &mut Rng, id: u8) -> EntityState {
    EntityState {
        id,
        kind: EntityKind::Player,
        pos: QPos {
            x: rng.below(65536) as u16,
            y: rng.below(8192) as u16,
            z: rng.below(65536) as u16,
        },
        yaw: rng.below(1024) as u16,
        pitch: rng.below(64) as u8,
        vel: None,
        anim: rng.below(16) as u8,
        stance: rng.below(8) as u8,
        weapon: rng.below(64) as u8,
        flags: rng.below(256) as u8,
        health_q: rng.below(256) as u8,
        vehicle: None,
    }
}

fn step(rng: &mut Rng, v: u16, max: u16) -> u16 {
    let span = match rng.below(5) {
        0 => 8,
        1 => 40,
        2 => 400,
        3 => 3000,
        _ => 70000,
    } as i64;
    let d = rng.below(2 * span as u64 + 1) as i64 - span;
    (v as i64 + d).clamp(0, max as i64) as u16
}

fn mutate(rng: &mut Rng, s: &EntityState) -> EntityState {
    let mut o = *s;
    if rng.chance(70) {
        o.pos.x = step(rng, o.pos.x, 65535);
        o.pos.z = step(rng, o.pos.z, 65535);
    }
    if rng.chance(20) {
        o.pos.y = step(rng, o.pos.y, 8191);
    }
    if rng.chance(50) {
        o.yaw = rng.below(1024) as u16;
    }
    if rng.chance(30) {
        o.pitch = rng.below(64) as u8;
    }
    if rng.chance(15) {
        o.anim = rng.below(16) as u8;
        o.stance = rng.below(8) as u8;
    }
    if rng.chance(10) {
        o.weapon = rng.below(64) as u8;
    }
    if rng.chance(10) {
        o.flags = rng.below(256) as u8;
    }
    if rng.chance(20) {
        o.health_q = rng.below(256) as u8;
    }
    o
}

/// What the decoder must reconstruct for `s` sent against baseline `base`.
fn expected(base: Option<&EntityState>, s: &EntityState, near: bool) -> EntityState {
    match (base, near) {
        (Some(b), false) => {
            let mut e = *s;
            e.yaw = ((((s.yaw + 2) >> 2) & 0xFF) << 2) as u16;
            e.pitch = b.pitch;
            e
        }
        _ => *s,
    }
}

fn sorted(mut v: Vec<EntityState>) -> Vec<EntityState> {
    v.sort_by_key(|s| s.id);
    v
}

fn pick_ids(rng: &mut Rng, n: usize) -> Vec<u8> {
    let mut ids: Vec<u8> = (0..=255).collect();
    for i in 0..n {
        let j = i + rng.below((256 - i) as u64) as usize;
        ids.swap(i, j);
    }
    ids.truncate(n);
    ids
}

#[test]
fn keyframe_path_roundtrip() {
    let mut rng = Rng(0xA11C_E000_0000_0001);
    for it in 0..ITER {
        let n = 1 + rng.below(60) as usize;
        let ids = pick_ids(&mut rng, n);
        let set: Vec<EntityState> = ids.iter().map(|&i| rand_state(&mut rng, i)).collect();
        let mut enc = SnapshotEncoder::new();
        let mut b = enc.begin(1, 1000 + it as u32, DEFAULT_BUDGET);
        b.set_ack(it as u16, (it % 16) as u8).unwrap();
        for s in &set {
            assert!(b.add_entity(s, rng.chance(50)).unwrap());
        }
        let bytes = b.finish();
        assert!(bytes.len() <= MAX_PACKET);
        let mut dec = SnapshotDecoder::new();
        let d = dec.decode(&bytes).unwrap();
        assert_eq!(d.tick, 1000 + it as u32);
        assert_eq!(d.ack_input_seq, it as u16);
        assert!(d.entities.keyframe.iter().all(|&k| k));
        assert_eq!(sorted(d.entities.to_states()), sorted(set));
    }
}

#[test]
fn delta_path_roundtrip() {
    let mut rng = Rng(0xDE17_A000_0000_0002);
    let (mut deltas, mut keys, mut removed_total) = (0usize, 0usize, 0usize);
    for it in 0..ITER {
        let n = 1 + rng.below(60) as usize;
        let ids = pick_ids(&mut rng, n + 10);
        let a: Vec<EntityState> = ids[..n].iter().map(|&i| rand_state(&mut rng, i)).collect();
        let t0 = 5000 + it as u32 * 7;
        let mut enc = SnapshotEncoder::new();
        let mut dec = SnapshotDecoder::new();
        let crc = rng.chance(80);
        let mut b = enc.begin(9, t0, DEFAULT_BUDGET).with_crc(crc);
        for s in &a {
            assert!(b.add_entity(s, true).unwrap());
        }
        let d0 = dec.decode(&b.finish()).unwrap();
        assert_eq!(sorted(d0.entities.to_states()), sorted(a.clone()));
        enc.on_ack(9, t0);

        // B: mutate / remove some of A, add new ones
        let t1 = t0 + 1 + rng.below(32) as u32;
        let mut b = enc.begin(9, t1, DEFAULT_BUDGET).with_crc(crc);
        let mut want = Vec::new();
        let mut want_removed = Vec::new();
        let mut want_keys = Vec::new();
        for s in &a {
            if rng.chance(15) {
                assert!(b.add_removed(s.id).unwrap());
                want_removed.push(s.id);
                continue;
            }
            let m = mutate(&mut rng, s);
            let near = rng.chance(50);
            assert!(b.add_entity(&m, near).unwrap());
            want.push(expected(Some(s), &m, near));
            want_keys.push((s.id, false));
        }
        for &id in &ids[n..] {
            if rng.chance(50) {
                let s = rand_state(&mut rng, id);
                assert!(b.add_entity(&s, rng.chance(50)).unwrap());
                want.push(s);
                want_keys.push((id, true));
            }
        }
        let (k, dl) = b.record_counts();
        let bytes = b.finish();
        assert!(bytes.len() <= MAX_PACKET);
        let d1 = dec.decode(&bytes).unwrap();
        assert_eq!(sorted(d1.entities.to_states()), sorted(want));
        let mut got_removed = d1.removed_ids.clone();
        got_removed.sort();
        want_removed.sort();
        assert_eq!(got_removed, want_removed);
        for (i, id) in d1.entities.ids.iter().enumerate() {
            let w = want_keys.iter().find(|(x, _)| x == id).unwrap().1;
            assert_eq!(d1.entities.keyframe[i], w, "record type of {id}");
        }
        deltas += dl;
        keys += k;
        removed_total += want_removed.len();
    }
    println!("{ITER} iterations: {deltas} delta, {keys} keyframe records, {removed_total} removes");
}

fn walkers(n: usize, rng: &mut Rng) -> Vec<EntityState> {
    (0..n).map(|i| rand_state(rng, (i * 2) as u8)).collect()
}

#[test]
fn baseline_ring_lag_gives_keyframes() {
    let mut rng = Rng(7);
    let mut enc = SnapshotEncoder::new();
    let mut dec = SnapshotDecoder::new();
    let set = walkers(20, &mut rng);
    let send =
        |enc: &mut SnapshotEncoder, dec: &mut SnapshotDecoder, tick: u32, set: &[EntityState]| {
            let mut b = enc.begin(1, tick, DEFAULT_BUDGET);
            for s in set {
                assert!(b.add_entity(s, true).unwrap());
            }
            dec.decode(&b.finish()).unwrap()
        };
    let d = send(&mut enc, &mut dec, 100, &set);
    assert!(d.entities.keyframe.iter().all(|&k| k));
    enc.on_ack(1, 100);
    // lag 32: still deltas
    let d = send(&mut enc, &mut dec, 132, &set);
    assert!(
        d.entities.keyframe.iter().all(|&k| !k),
        "lag 32 must be delta"
    );
    // lag 33: all keyframes
    let d = send(&mut enc, &mut dec, 133, &set);
    assert_eq!(d.entities.len(), 20);
    assert!(
        d.entities.keyframe.iter().all(|&k| k),
        "lag 33 must be keyframes"
    );
    // acknowledging the keyframe packet restores deltas
    enc.on_ack(1, 133);
    let d = send(&mut enc, &mut dec, 134, &set);
    assert!(d.entities.keyframe.iter().all(|&k| !k));
}

#[test]
fn missing_baseline_and_bad_input() {
    let mut rng = Rng(8);
    let mut enc = SnapshotEncoder::new();
    let set = walkers(5, &mut rng);
    let mut b = enc.begin(1, 10, DEFAULT_BUDGET);
    for s in &set {
        b.add_entity(s, true).unwrap();
    }
    let p0 = b.finish();
    enc.on_ack(1, 10);
    let mut b = enc.begin(1, 11, DEFAULT_BUDGET);
    for s in &set {
        b.add_entity(s, true).unwrap();
    }
    let p1 = b.finish();
    // a decoder that never saw p0 cannot decode the 16-bit-tick delta packet
    assert_eq!(
        SnapshotDecoder::new().decode(&p1),
        Err(CodecError::MissingBaseline)
    );
    // ... nor can one that saw a different packet
    let mut other = SnapshotDecoder::new();
    let mut e2 = SnapshotEncoder::new();
    let b = e2.begin(1, 500, DEFAULT_BUDGET);
    other.decode(&b.finish()).unwrap();
    assert_eq!(other.decode(&p1), Err(CodecError::MissingBaseline));
    // failed decode leaves the decoder usable
    let mut dec = SnapshotDecoder::new();
    dec.decode(&p0).unwrap();
    assert!(dec.decode(&p1).is_ok());
    // corrupted CRC, oversize, empty
    let mut bad = p0.clone();
    bad[5] ^= 1;
    assert_eq!(SnapshotDecoder::new().decode(&bad), Err(CodecError::BadCrc));
    assert_eq!(
        SnapshotDecoder::new().decode(&vec![0u8; 1201]),
        Err(CodecError::TooLong)
    );
    assert!(SnapshotDecoder::new().decode(&[]).is_err());
}

#[test]
fn convergence_under_30_percent_loss() {
    let mut rng = Rng(0x1055_1055);
    let mut enc = SnapshotEncoder::new();
    let mut dec = SnapshotDecoder::new();
    let mut world = walkers(40, &mut rng);
    let mut tick = 1000u32;
    let (mut missing, mut decoded, mut lost) = (0, 0, 0);
    let mut last_sent: Vec<EntityState> = Vec::new();
    let mut last_decoded_tick = 0;
    // 60 s at 30 Hz sim, a snapshot every 1.5 ticks (20 Hz): alternate +1 / +2
    let mut snaps = 0;
    while snaps < 1200 {
        tick += if snaps % 2 == 0 { 1 } else { 2 };
        snaps += 1;
        for s in world.iter_mut() {
            s.pos.x = s.pos.x.saturating_add(6).min(65000);
            if rng.chance(10) {
                s.yaw = rng.below(1024) as u16;
            }
            if rng.chance(2) {
                s.health_q = rng.below(256) as u8;
            }
        }
        let mut b = enc.begin(1, tick, DEFAULT_BUDGET);
        for s in &world {
            assert!(b.add_entity(s, true).unwrap());
        }
        let pkt = b.finish();
        last_sent = world.clone();
        if rng.chance(30) {
            lost += 1;
            continue;
        }
        match dec.decode(&pkt) {
            Ok(d) => {
                decoded += 1;
                last_decoded_tick = d.tick;
                assert_eq!(sorted(d.entities.to_states()), sorted(world.clone()));
                // ack travels back with 30% loss too
                if !rng.chance(30) {
                    enc.on_ack(1, d.tick);
                }
            }
            Err(CodecError::MissingBaseline) => missing += 1,
            Err(e) => panic!("{e:?}"),
        }
    }
    println!("60 s, 30% loss: {decoded} decoded, {lost} lost, {missing} MissingBaseline");
    assert_eq!(
        missing, 0,
        "decoder must never lack the baseline the server uses"
    );
    assert!(decoded > 700);
    assert!(last_decoded_tick + 60 > tick);
    let _ = last_sent;
}

#[test]
fn budget_never_writes_half_a_record() {
    let mut rng = Rng(99);
    let set = walkers(100, &mut rng);
    for budget in [60usize, 100, 333, 700, 1150] {
        let mut enc = SnapshotEncoder::new();
        let mut b = enc.begin(1, 50, budget);
        b.add_self(&SelfState::default()).unwrap();
        let mut ok = 0;
        for s in &set {
            if b.add_entity(s, true).unwrap() {
                ok += 1;
            } else {
                break;
            }
        }
        let bytes = b.finish();
        assert!(bytes.len() <= budget, "{} > {budget}", bytes.len());
        let d = SnapshotDecoder::new().decode(&bytes).unwrap();
        assert_eq!(d.entities.len(), ok);
    }
    // record limit 127 even with a huge budget
    let many: Vec<EntityState> = (0..=200u8).map(|i| rand_state(&mut rng, i)).collect();
    let mut enc = SnapshotEncoder::new();
    let mut b = enc.begin(1, 50, 1200);
    let ok = many
        .iter()
        .filter(|s| b.add_entity(s, true).unwrap())
        .count();
    assert!(ok <= 127);
    let bytes = b.finish();
    assert!(bytes.len() <= 1200);
    assert_eq!(
        SnapshotDecoder::new()
            .decode(&bytes)
            .unwrap()
            .entities
            .len(),
        ok
    );
}

#[test]
fn self_state_events_and_validation() {
    let mut enc = SnapshotEncoder::new();
    let mut b = enc.begin(1, 77, DEFAULT_BUDGET).with_crc(false);
    let s = SelfState {
        pos_corr: Some(QPos {
            x: 65535,
            y: 8191,
            z: 3,
        }),
        ammo_in_mag: 30,
        ammo_reserve: 4095,
        health: 200,
        stamina: 63,
        suppression: 15,
        stance: 7,
        deploy_state: 5,
        cooldown_bits: 0xA5,
    };
    assert!(b.add_self(&s).unwrap());
    assert_eq!(b.add_self(&s), Err(CodecError::InvalidValue));
    let evs = [
        EventEntry {
            id: 1,
            event: Event::Shot {
                shooter: 3,
                weapon: 4,
                muzzle: [1, 16383, 77],
                dir_yaw: 1023,
                dir_pitch: 511,
            },
        },
        EventEntry {
            id: 2,
            event: Event::Impact {
                pos: [5, 6, 7],
                normal: 4095,
                material: 9,
            },
        },
        EventEntry {
            id: 3,
            event: Event::HitConfirm {
                target: 1,
                zone: 2,
                damage_q: 3,
            },
        },
        EventEntry {
            id: 65535,
            event: Event::DamageDir {
                angle: 250,
                dmg_q: 9,
            },
        },
    ];
    assert_eq!(b.add_events(&evs).unwrap(), 4);
    assert_eq!(
        b.add_events(&[EventEntry {
            id: 9,
            event: Event::Impact {
                pos: [16384, 0, 0],
                normal: 0,
                material: 0
            }
        }]),
        Err(CodecError::Overflow)
    );
    // invalid entity values are errors, duplicates too
    let mut bad = rand_state(&mut Rng(1), 1);
    bad.anim = 16;
    assert_eq!(b.add_entity(&bad, true), Err(CodecError::Overflow));
    bad.anim = 0;
    bad.vel = Some(QVel::default());
    assert_eq!(b.add_entity(&bad, true), Err(CodecError::InvalidValue));
    bad.vel = None;
    assert!(b.add_entity(&bad, true).unwrap());
    assert_eq!(b.add_entity(&bad, true), Err(CodecError::InvalidValue));
    assert!(b.add_removed(200).unwrap());
    b.set_ack(4321, 9).unwrap();
    assert_eq!(b.set_ack(0, 16), Err(CodecError::Overflow));
    let bytes = b.finish();
    let d = SnapshotDecoder::new().decode(&bytes).unwrap();
    assert_eq!(d.self_state, Some(s));
    assert_eq!(d.events.len(), 4);
    assert_eq!(d.events, evs);
    assert_eq!((d.ack_input_seq, d.input_slack, d.tick), (4321, 9, 77));
    assert_eq!(d.removed_ids, vec![200]);
}
