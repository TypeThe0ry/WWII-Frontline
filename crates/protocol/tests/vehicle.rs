//! Vehicle records: quaternion precision, snapshot roundtrip with mixed kinds, delta size.
//! `cargo test -p frontline-protocol --release vehicle_delta_size -- --nocapture`

use frontline_protocol::quant::*;
use frontline_protocol::snapshot::*;
use proptest::prelude::*;

fn vehicle(id: u8, pos: QPos, quat: u32, turret: u16, gun: u16) -> EntityState {
    EntityState {
        id,
        kind: EntityKind::Vehicle,
        pos,
        vel: Some(QVel {
            x: 100,
            y: -3,
            z: -200,
        }),
        health_q: 200,
        flags: 1,
        vehicle: Some(VehicleExtra {
            quat,
            turret_yaw: turret,
            gun_pitch: gun,
            track_flags: 0b0101,
            seats: 0b101,
            damage_flags: 0b10,
        }),
        ..Default::default()
    }
}

fn player(id: u8, x: u16) -> EntityState {
    EntityState {
        id,
        pos: QPos { x, y: 400, z: 9000 },
        yaw: 300,
        pitch: 30,
        anim: 2,
        weapon: 5,
        health_q: 255,
        ..Default::default()
    }
}

fn angle_deg(a: [f32; 4], b: [f32; 4]) -> f32 {
    let d: f32 = a
        .iter()
        .zip(b)
        .map(|(x, y)| x * y)
        .sum::<f32>()
        .abs()
        .min(1.0);
    (2.0 * d.acos()).to_degrees()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100_000))]

    #[test]
    fn quat_error_under_035_deg(x in -1f32..1.0, y in -1f32..1.0, z in -1f32..1.0, w in -1f32..1.0) {
        let n = (x * x + y * y + z * z + w * w).sqrt();
        prop_assume!(n > 0.1);
        let q = [x / n, y / n, z / n, w / n];
        let u = quat_unpack(quat_pack(q));
        let len: f32 = u.iter().map(|v| v * v).sum::<f32>().sqrt();
        prop_assert!((len - 1.0).abs() < 0.01, "len {len}");
        prop_assert!(angle_deg(q, u) <= 0.35, "err {}", angle_deg(q, u));
    }

    #[test]
    fn turret_and_gun_error(yaw in 0f32..core::f32::consts::TAU, pitch in -core::f32::consts::FRAC_PI_2..core::f32::consts::FRAC_PI_2) {
        let y = dequant_yaw(quant_yaw(yaw, 12).code, 12);
        let d = (y - yaw + core::f32::consts::PI).rem_euclid(core::f32::consts::TAU) - core::f32::consts::PI;
        prop_assert!(d.abs().to_degrees() <= 0.35);
        let p = dequant_pitch(quant_pitch(pitch, 12).code, 12);
        prop_assert!((p - pitch).abs().to_degrees() <= 0.35);
    }

    #[test]
    fn vehicle_state_roundtrip(
        x in any::<u16>(), y in 0u16..8192, z in any::<u16>(), quat in any::<u32>(),
        ty in 0u16..4096, gp in 0u16..4096, vx in -1024i16..1024, vy in -1024i16..1024,
        vz in -1024i16..1024, tf in any::<u8>(), seats in 0u8..8, dmg in any::<u8>(),
        fl in any::<u8>(), hp in any::<u8>(), dx in -3000i32..3000,
    ) {
        let mut a = vehicle(70, QPos { x, y, z }, quat, ty, gp);
        a.vel = Some(QVel { x: vx, y: vy, z: vz });
        {
            let v = a.vehicle.as_mut().unwrap();
            v.track_flags = tf;
            v.seats = seats;
            v.damage_flags = dmg;
        }
        a.flags = fl;
        a.health_q = hp;
        let mut enc = SnapshotEncoder::new();
        let mut dec = SnapshotDecoder::new();
        let mut b = enc.begin(1, 100, DEFAULT_BUDGET);
        prop_assert!(b.add_entity(&a, true).unwrap());
        prop_assert!(b.add_entity(&player(3, x), true).unwrap());
        let d = dec.decode(&b.finish()).unwrap();
        prop_assert_eq!(d.entities.to_states(), vec![a, player(3, x)]);
        enc.on_ack(1, 100);
        // delta against that baseline
        let mut c = a;
        c.pos.x = (x as i32 + dx).clamp(0, 65535) as u16;
        c.vehicle.as_mut().unwrap().quat = quat.wrapping_add(1234);
        c.health_q = hp.wrapping_add(1);
        let mut b = enc.begin(1, 103, DEFAULT_BUDGET);
        prop_assert!(b.add_entity(&c, true).unwrap());
        let (k, dl) = b.record_counts();
        prop_assert_eq!((k, dl), (0, 1));
        let d = dec.decode(&b.finish()).unwrap();
        prop_assert_eq!(d.entities.state(0).unwrap(), c);
    }
}

#[test]
fn quat_identity_and_axes() {
    for q in [
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, -1.0],
        [0.5, 0.5, 0.5, 0.5],
    ] {
        assert!(angle_deg(q, quat_unpack(quat_pack(q))) < 0.2, "{q:?}");
    }
    assert_eq!(
        quat_pack([f32::NAN, 0.0, 0.0, 1.0]),
        quat_pack([0.0, 0.0, 0.0, 1.0])
    );
}

#[test]
fn vehicle_validate_rules() {
    let mut p = player(1, 5);
    p.vel = Some(QVel::default());
    assert!(p.validate().is_err());
    let mut v = vehicle(70, QPos::default(), 0, 0, 0);
    assert!(v.validate().is_ok());
    v.vehicle.as_mut().unwrap().seats = 8;
    assert!(v.validate().is_err());
    let mut v = vehicle(70, QPos::default(), 0, 4096, 0);
    assert!(v.validate().is_err());
    v.vehicle = None;
    assert!(v.validate().is_err());
    let mut v = vehicle(70, QPos::default(), 0, 0, 0);
    v.vel = Some(QVel {
        x: 1024,
        y: 0,
        z: 0,
    });
    assert!(v.validate().is_err());
    let mut v = vehicle(70, QPos::default(), 0, 0, 0);
    v.yaw = 5;
    assert!(v.validate().is_err());
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn unit(&mut self) -> f32 {
        (self.next() % 10_000) as f32 / 10_000.0
    }
}

/// A tank driving 10^4 snapshots (20 Hz) with turning hull, a turret that slews now and then,
/// crew/damage changes, every snapshot acked. Reports delta record size.
#[test]
fn vehicle_delta_size() {
    let mut rng = Lcg(7);
    let mut enc = SnapshotEncoder::new();
    let mut dec = SnapshotDecoder::new();
    let (mut x, mut z, mut heading) = (400.0f32, 400.0f32, 0.0f32);
    let (mut turret, mut gun) = (0.0f32, 0.0f32);
    let (mut turret_rate, mut gun_rate) = (0.0f32, 0.0f32);
    let mut speed = 5.0f32;
    let (mut total_bits, mut n, mut max_bits) = (0usize, 0usize, 0usize);
    let mut tick = 1000u32;
    let mut hp = 255u8;
    let mut tracks = 0u8;
    let mut damage = 0u8;
    for step in 0..10_000usize {
        tick += 1 + (step as u32 % 2);
        let dt = 0.05f32;
        if step % 40 == 0 {
            speed = 1.0 + rng.unit() * 6.5;
        }
        if step % 25 == 0 {
            turret_rate = (rng.unit() - 0.5) * 1.6;
            gun_rate = (rng.unit() - 0.5) * 0.3;
        }
        heading += (rng.unit() - 0.5) * 0.12;
        x += heading.cos() * speed * dt;
        z += heading.sin() * speed * dt;
        turret = (turret + turret_rate * dt).rem_euclid(core::f32::consts::TAU);
        gun = (gun + gun_rate * dt).clamp(-0.2, 0.5);
        if rng.next().is_multiple_of(200) {
            hp = hp.saturating_sub(10);
            damage |= 1 << (rng.next() % 8);
        }
        if rng.next().is_multiple_of(8) {
            tracks = (rng.next() % 16) as u8;
        }
        let h = heading * 0.5;
        let quat = quat_pack([0.0, h.sin(), 0.0, h.cos()]);
        let s = EntityState {
            id: 70,
            kind: EntityKind::Vehicle,
            pos: QPos::from_meters(x, 12.0 + (rng.unit() - 0.5) * 0.1, z),
            vel: Some(QVel::from_mps(
                heading.cos() * speed,
                0.0,
                heading.sin() * speed,
            )),
            health_q: hp,
            vehicle: Some(VehicleExtra {
                quat,
                turret_yaw: quant_yaw(turret, 12).code,
                gun_pitch: quant_pitch(gun, 12).code,
                track_flags: tracks,
                seats: 0b101,
                damage_flags: damage,
            }),
            ..Default::default()
        };
        let mut b = enc.begin(1, tick, DEFAULT_BUDGET);
        assert!(b.add_entity(&s, true).unwrap());
        let (k, d) = b.record_counts();
        let bits = b.entity_bits();
        if step > 0 && k == 0 && d == 1 {
            let bytes = bits.div_ceil(8);
            total_bits += bits;
            n += 1;
            max_bits = max_bits.max(bytes);
        }
        let d = dec.decode(&b.finish()).unwrap();
        assert_eq!(d.entities.state(0).unwrap(), s);
        enc.on_ack(1, tick);
    }
    let avg = total_bits as f64 / n as f64 / 8.0;
    println!("vehicle delta: n={n} avg={avg:.2} B max={max_bits} B");
    assert!(avg <= 24.0, "avg {avg}");
    assert!(max_bits <= 28, "max {max_bits}");
}

#[test]
fn vehicle_keyframe_size() {
    let mut enc = SnapshotEncoder::new();
    let mut b = enc.begin(1, 10, DEFAULT_BUDGET);
    b.add_entity(&vehicle(70, QPos::default(), 1, 2, 3), true)
        .unwrap();
    println!("vehicle keyframe bits {}", b.entity_bits());
    assert!(b.entity_bits() <= 23 * 8);
}

/// Timing (reference machine only): `cargo test -p frontline-protocol --release -- --ignored vehicle_timing --nocapture`
#[test]
#[ignore]
fn vehicle_timing() {
    use std::time::Instant;
    let mut enc = SnapshotEncoder::new();
    let mut dec = SnapshotDecoder::new();
    let mk = |i: u8, k: u16| {
        if i < 8 {
            let mut v = vehicle(
                100 + i,
                QPos {
                    x: 1000 + k,
                    y: 50,
                    z: 2000,
                },
                77 + k as u32,
                k % 4096,
                5,
            );
            v.vel = Some(QVel {
                x: k as i16 % 500,
                y: 0,
                z: 3,
            });
            v
        } else {
            player(i, 1000 + k)
        }
    };
    let mut b = enc.begin(1, 10, DEFAULT_BUDGET);
    for i in 0..60u8 {
        b.add_entity(&mk(i, 0), true).unwrap();
    }
    dec.decode(&b.finish()).unwrap();
    enc.on_ack(1, 10);
    let iters = 20_000u32;
    let t = Instant::now();
    let mut tick = 10;
    for k in 0..iters {
        tick += 1;
        let mut b = enc.begin(1, tick, DEFAULT_BUDGET);
        for i in 0..60u8 {
            b.add_entity(&mk(i, (k % 30) as u16 + 1), true).unwrap();
        }
        std::hint::black_box(b.finish());
        enc.on_ack(1, 10); // keep the baseline fixed so every packet is a delta
    }
    let per_packet = t.elapsed().as_secs_f64() * 1e6 / iters as f64;
    let per_vehicle = {
        let t = Instant::now();
        let mut n = 0u32;
        for k in 0..iters {
            let mut b = enc.begin(1, 10 + k, DEFAULT_BUDGET);
            b.add_entity(&mk(0, (k % 30) as u16 + 1), true).unwrap();
            n += 1;
            std::hint::black_box(&b);
        }
        t.elapsed().as_secs_f64() * 1e6 / n as f64
    };
    println!("60-entity packet (8 vehicles): {per_packet:.2} us; single vehicle delta incl. begin: {per_vehicle:.2} us");
    assert!(per_packet <= 25.0);
}
