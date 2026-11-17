#![no_main]
use frontline_protocol::msg::*;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(m) = decode_c4(data) {
        let b = encode_c4(&m).expect("decoded event re-encodes");
        assert_eq!(decode_c4(&b).unwrap(), m);
    }
});
