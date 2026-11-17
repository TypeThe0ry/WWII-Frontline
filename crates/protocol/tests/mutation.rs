//! Deterministic stand-in for the cargo-fuzz targets `c0,c4,events,snapshot_vehicle`
//! (same properties, run in CI without a nightly toolchain): random bytes and bit-flipped /
//! truncated valid inputs never panic, and anything that decodes re-encodes and decodes equal.

use frontline_protocol::msg::*;
use frontline_protocol::snapshot::events::{Event, EventEntry};
use frontline_protocol::snapshot::*;
use frontline_protocol::{BitReader, BitWriter};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn mutate(rng: &mut Rng, seed: &[u8]) -> Vec<u8> {
    let mut b = seed.to_vec();
    match rng.below(4) {
        0 => {
            for _ in 0..1 + rng.below(4) {
                if !b.is_empty() {
                    let i = rng.below(b.len());
                    b[i] ^= 1 << rng.below(8);
                }
            }
        }
        1 => b.truncate(rng.below(b.len() + 1)),
        2 => b.extend((0..rng.below(8)).map(|_| rng.next() as u8)),
        _ => b = (0..rng.below(300)).map(|_| rng.next() as u8).collect(),
    }
    b
}

const N: usize = 200_000;

#[test]
fn mutate_c0_c4_team() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let seeds: Vec<Vec<u8>> = vec![
        encode_c0_client(&ClientMsgC0::Chat(Chat {
            channel: ChatChannel::Team,
            text: "hello 世界".into(),
            sender: 3,
        }))
        .unwrap(),
        encode_c0_client(&ClientMsgC0::SquadOp(SquadOp {
            op: SquadOpKind::Invite,
            squad: 1,
            target: 2,
            name_key: "squad.alpha".into(),
        }))
        .unwrap(),
        encode_c0_client(&ClientMsgC0::Report(Report {
            target: 1,
            reason: 2,
            note: "x".into(),
        }))
        .unwrap(),
        encode_c4(&ServerEventC4::PlayerRoster(PlayerRoster {
            player: 1,
            nick: "abc".into(),
            team: 1,
            squad: 2,
            role: 3,
            flags: 4,
        }))
        .unwrap(),
        encode_c4(&ServerEventC4::Kill(Kill {
            victim: 1,
            killer: 2,
            weapon: 3,
            headshot: true,
            dist_q: 100,
            assist1: 0,
            assist2: 255,
        }))
        .unwrap(),
        encode_team_state(&TeamState {
            tick: 5,
            members: vec![
                TeamMember {
                    x: 1,
                    z: 2,
                    role: 3,
                    state: 4,
                    facing: 5
                };
                5
            ],
        })
        .unwrap(),
    ];
    for _ in 0..N {
        let k = rng.below(seeds.len());
        let b = mutate(&mut rng, &seeds[k]);
        if let Ok(m) = decode_c0_client(&b) {
            assert_eq!(decode_c0_client(&encode_c0_client(&m).unwrap()).unwrap(), m);
        }
        if let Ok(m) = decode_c0_server(&b) {
            assert_eq!(decode_c0_server(&encode_c0_server(&m).unwrap()).unwrap(), m);
        }
        if let Ok(m) = decode_c4(&b) {
            assert_eq!(decode_c4(&encode_c4(&m).unwrap()).unwrap(), m);
        }
        if let Ok(t) = decode_team_state(&b) {
            assert_eq!(
                decode_team_state(&encode_team_state(&t).unwrap()).unwrap(),
                t
            );
        }
        let _ = decode_world_state(&b);
    }
}

#[test]
fn mutate_events() {
    let mut rng = Rng(12345);
    let evs = [
        Event::Explosion {
            pos: [1, 2, 3],
            kind: 4,
            radius_q: 5,
        },
        Event::Suppress {
            source_dir: 1,
            intensity: 2,
        },
        Event::Marker {
            kind: 1,
            pos: [100, 200],
            ttl: 9,
        },
        Event::Shot {
            shooter: 1,
            weapon: 2,
            muzzle: [3, 4, 5],
            dir_yaw: 6,
            dir_pitch: 7,
        },
    ];
    let mut w = BitWriter::new();
    for (i, e) in evs.iter().enumerate() {
        EventEntry {
            id: i as u16,
            event: *e,
        }
        .encode(&mut w)
        .unwrap();
    }
    let seed = w.finish();
    for _ in 0..N {
        let b = mutate(&mut rng, &seed);
        let mut r = BitReader::new(&b);
        let mut got = Vec::new();
        while let Ok(e) = EventEntry::decode(&mut r) {
            got.push(e);
            if got.len() >= 63 {
                break;
            }
        }
        let mut w = BitWriter::new();
        for e in &got {
            e.encode(&mut w).unwrap();
        }
        let b2 = w.finish();
        let mut r = BitReader::new(&b2);
        for e in &got {
            assert_eq!(&EventEntry::decode(&mut r).unwrap(), e);
        }
    }
}

#[test]
fn mutate_snapshot_vehicle() {
    let mut rng = Rng(777);
    let mut enc = SnapshotEncoder::new();
    let mut b = enc.begin(1, 100, DEFAULT_BUDGET);
    let mk = |i: u8| EntityState {
        id: 70 + i,
        kind: EntityKind::Vehicle,
        pos: QPos {
            x: 1000,
            y: 50,
            z: 2000,
        },
        vel: Some(QVel {
            x: 10,
            y: 0,
            z: -10,
        }),
        vehicle: Some(VehicleExtra {
            seats: 1,
            ..Default::default()
        }),
        ..Default::default()
    };
    for i in 0..3 {
        b.add_entity(&mk(i), true).unwrap();
        b.add_entity(
            &EntityState {
                id: i,
                ..Default::default()
            },
            true,
        )
        .unwrap();
    }
    let first = b.finish();
    let seeded = || {
        let mut d = SnapshotDecoder::new();
        d.decode(&first).unwrap();
        d
    };
    enc.on_ack(1, 100);
    let mut b = enc.begin(1, 103, DEFAULT_BUDGET);
    for i in 0..3 {
        let mut s = mk(i);
        s.pos.x += 40;
        s.vehicle.as_mut().unwrap().turret_yaw = 77;
        b.add_entity(&s, true).unwrap();
        b.add_entity(
            &EntityState {
                id: i,
                ..Default::default()
            },
            true,
        )
        .unwrap();
    }
    let seed = b.finish();
    for _ in 0..N / 4 {
        let data = mutate(&mut rng, &seed);
        let mut d = seeded();
        let _ = d.decode(&data);
    }
}
