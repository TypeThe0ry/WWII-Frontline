//! Regenerates `tests/golden/snapshot_{0,1}.bin` and `snapshot_expected.json` from the
//! protocol crate's golden snapshot hex (key frame, then delta against it), decoded natively.
//! Run: `cargo run -p frontline-sim-wasm --example gen_golden`.
use std::fmt::Write as _;
use std::path::PathBuf;

use frontline_protocol::snapshot::{DecodedSnapshot, SnapshotDecoder};

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

fn arr<T: std::fmt::Display>(v: &[T]) -> String {
    let mut s = String::from("[");
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(s, "{x}");
    }
    s.push(']');
    s
}

fn json(d: &DecodedSnapshot, events: &[u8]) -> String {
    let e = &d.entities;
    let (selfp, corr) = d.self_state.as_ref().map_or((false, None), |s| (true, s.pos_corr));
    let st = d.self_state.unwrap_or_default();
    let c = corr.map(|c| c.to_meters());
    format!(
        "{{\"tick\":{},\"ackInputSeq\":{},\"inputSlack\":{},\"selfPresent\":{},\"corr\":{},\"ammoInMag\":{},\"ammoReserve\":{},\"health\":{},\"stamina\":{},\"suppression\":{},\"stance\":{},\"deployState\":{},\"cooldownBits\":{},\"ids\":{},\"pos\":{},\"yaw\":{},\"pitch\":{},\"anim\":{},\"entStance\":{},\"weapon\":{},\"flags\":{},\"entHealth\":{},\"removed\":{},\"events\":{}}}",
        d.tick, d.ack_input_seq, d.input_slack, selfp,
        c.map_or("null".to_string(), |c| arr(&c)),
        st.ammo_in_mag, st.ammo_reserve, st.health, st.stamina, st.suppression, st.stance,
        st.deploy_state, st.cooldown_bits,
        arr(&e.ids), arr(&e.pos), arr(&e.yaw), arr(&e.pitch), arr(&e.anim), arr(&e.stance),
        arr(&e.weapon), arr(&e.flags), arr(&e.health_q), arr(&d.removed_ids), arr(events)
    )
}

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let gold = dir.join("../protocol/tests/golden");
    let out = dir.join("tests/golden");
    let mut dec = SnapshotDecoder::new();
    let mut docs = Vec::new();
    for (i, name) in ["snapshot_key", "snapshot_delta"].iter().enumerate() {
        let hex = std::fs::read_to_string(gold.join(format!("{name}.hex"))).unwrap();
        let bytes = unhex(hex.trim());
        std::fs::write(out.join(format!("snapshot_{i}.bin")), &bytes).unwrap();
        let d = dec.decode(&bytes).unwrap();
        // The wasm event record packing is defined in src/snapshot.rs (pack_event).
        let mut ev = Vec::new();
        for e in &d.events {
            frontline_sim_wasm::pack_event_for_golden(e, &mut ev);
        }
        docs.push(json(&d, &ev));
    }
    std::fs::write(
        out.join("snapshot_expected.json"),
        format!("[{}]\n", docs.join(",")),
    )
    .unwrap();
}
