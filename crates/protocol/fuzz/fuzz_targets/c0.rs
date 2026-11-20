#![no_main]
//! Arbitrary bytes into both C0 decoders; a successful decode must re-encode and decode equal.
use frontline_protocol::msg::*;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(m) = decode_c0_client(data) {
        let b = encode_c0_client(&m).expect("decoded message re-encodes");
        assert_eq!(decode_c0_client(&b).unwrap(), m);
    }
    if let Ok(m) = decode_c0_server(data) {
        let b = encode_c0_server(&m).expect("decoded message re-encodes");
        assert_eq!(decode_c0_server(&b).unwrap(), m);
    }
    let _ = decode_team_state(data);
    if let Ok(m) = decode_team_state_with_markers(data) {
        let b = encode_team_state_with_markers(&m).expect("decoded team state re-encodes");
        assert_eq!(decode_team_state_with_markers(&b).unwrap(), m);
    }
    let _ = decode_world_state(data);
});
