#![no_main]
//! Feeds arbitrary bytes to the snapshot decoder, both with an empty decoder and with one that
//! already holds a baseline (so delta records are reachable). Must never panic.
use frontline_protocol::snapshot::*;
use libfuzzer_sys::fuzz_target;

fn seeded() -> SnapshotDecoder {
    let mut enc = SnapshotEncoder::new();
    let mut dec = SnapshotDecoder::new();
    let mut b = enc.begin(1, 100, DEFAULT_BUDGET);
    for i in 0..8u8 {
        let s = EntityState {
            id: i * 5,
            pos: QPos { x: 1000 + i as u16, y: 50, z: 2000 },
            ..Default::default()
        };
        let _ = b.add_entity(&s, true);
    }
    let _ = dec.decode(&b.finish());
    dec
}

fuzz_target!(|data: &[u8]| {
    let _ = SnapshotDecoder::new().decode(data);
    let mut d = seeded();
    // header tick 100 + small deltas reference the seeded baseline
    let _ = d.decode(data);
    let _ = d.decode(data);
});
