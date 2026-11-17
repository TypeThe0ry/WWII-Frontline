#![no_main]
//! Event-segment records: decode a run of records from arbitrary bytes; success must roundtrip.
use frontline_protocol::snapshot::events::EventEntry;
use frontline_protocol::{BitReader, BitWriter};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut r = BitReader::new(data);
    let mut got = Vec::new();
    while let Ok(e) = EventEntry::decode(&mut r) {
        got.push(e);
        if got.len() >= 63 {
            break;
        }
    }
    let mut w = BitWriter::new();
    for e in &got {
        e.encode(&mut w).expect("decoded event re-encodes");
    }
    let b = w.finish();
    let mut r = BitReader::new(&b);
    for e in &got {
        assert_eq!(&EventEntry::decode(&mut r).unwrap(), e);
    }
});
