#![no_main]
//! Snapshot decoder with a baseline that contains vehicles, so vehicle delta records are
//! reachable. Must never panic.
use frontline_protocol::snapshot::*;
use libfuzzer_sys::fuzz_target;

fn seeded() -> SnapshotDecoder {
    let mut enc = SnapshotEncoder::new();
    let mut dec = SnapshotDecoder::new();
    let mut b = enc.begin(1, 100, DEFAULT_BUDGET);
    for i in 0..4u8 {
        let v = EntityState {
            id: 70 + i,
            kind: EntityKind::Vehicle,
            pos: QPos { x: 1000 + i as u16, y: 50, z: 2000 },
            vel: Some(QVel { x: 10, y: 0, z: -10 }),
            vehicle: Some(VehicleExtra { seats: 1, ..Default::default() }),
            ..Default::default()
        };
        let _ = b.add_entity(&v, true);
        let p = EntityState { id: i, pos: QPos { x: 5, y: 5, z: 5 }, ..Default::default() };
        let _ = b.add_entity(&p, true);
    }
    let _ = dec.decode(&b.finish());
    dec
}

fuzz_target!(|data: &[u8]| {
    let mut d = seeded();
    let _ = d.decode(data);
    let _ = d.decode(data);
});
