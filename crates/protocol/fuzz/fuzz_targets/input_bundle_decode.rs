#![no_main]
use frontline_protocol::msg::{decode_input_bundle, encode_input_bundle};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(b) = decode_input_bundle(data) {
        // anything that decodes must re-encode to something that decodes to the same value
        if let Ok(bytes) = encode_input_bundle(&b) {
            assert_eq!(decode_input_bundle(&bytes).as_ref(), Ok(&b));
        }
    }
});
