//! `sim + protocol` as WebAssembly for the TS client (ARCHITECTURE §4.3.3, §8.3; task M0-014).
//!
//! No game logic lives here: this crate only marshals bytes and POD across the JS boundary.
//! Every fallible entry point returns a JS exception (`Result<_, JsError>`), never aborts on bad
//! input. Hot-path accessors (`Predictor.step/state`, `SnapshotDecoder` columns) hand out views
//! into wasm memory (zero copy): a view is valid until the next call into this module that
//! may allocate (or until the next `step`/`decode`), so consume it in the same frame.
//!
//! The pure-Rust cores (`*_core` / `*_inner` functions) are what `cargo test` exercises on the
//! host; the `#[wasm_bindgen]` methods are thin adapters around them.

mod data;
mod messages;
mod predictor;
mod snapshot;

use wasm_bindgen::prelude::*;

pub use predictor::{layout_table, LAYOUT_STATE_LEN};
pub use snapshot::pack_event as pack_event_for_golden;

/// Installs the panic hook (panics become `console.error` messages with the Rust message and
/// location). Idempotent; call once after instantiating the module.
#[wasm_bindgen]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Wire protocol version this build speaks (`Hello.protocol_version`).
#[wasm_bindgen(js_name = protocolVersion)]
pub fn protocol_version() -> u32 {
    u32::from(frontline_protocol::PROTOCOL_VERSION)
}

/// `DATA_HASH` of the embedded game data (`Hello.data_hash`).
#[wasm_bindgen(js_name = dataHash)]
pub fn data_hash() -> u64 {
    frontline_sim::data::DATA_HASH
}

/// All `LAYOUT_*` constants as JSON (`{"STATE_LEN":22,"X":0,...}`); the TS constant table in
/// `client/src/net/wasm/layout.ts` is checked against this by a test.
#[wasm_bindgen(js_name = layoutJson)]
pub fn layout_json() -> String {
    layout_table_json()
}

pub(crate) fn layout_table_json() -> String {
    let mut s = String::from("{");
    let all = predictor::layout_table()
        .iter()
        .chain(snapshot::layout_table().iter())
        .chain(messages::layout_table().iter());
    for (i, (k, v)) in all.enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("\"{k}\":{v}"));
    }
    s.push('}');
    s
}

pub(crate) fn js_err(e: impl core::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn layout_snapshot() {
        // Golden: changing the layout must be mirrored in client/src/net/wasm/layout.ts (M0-022).
        assert_eq!(
            super::layout_table_json(),
            include_str!("../tests/golden/layout.json").trim()
        );
    }
}
