//! Quantisation error bounds, size budgets, decoder robustness and golden bytes.

use arrayvec::ArrayVec;
use frontline_protocol::msg::*;
use frontline_protocol::quant::*;
use frontline_protocol::*;
use std::f32::consts::{FRAC_PI_2, TAU};

/// xorshift64* - deterministic, no dependencies.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

#[test]
fn quant_error_bounds_1m() {
    let mut rng = Rng(0x1234_5678_9ABC_DEF1);
    let (mut ex, mut ez, mut ey) = (0f32, 0f32, 0f32);
    let (mut eyaw10, mut eyaw8, mut epitch8, mut epitch9, mut epitch16, mut ei16) =
        (0f32, 0f32, 0f32, 0f32, 0f32, 0f32);
    let ang = |a: f32, b: f32| {
        let d = (a - b).rem_euclid(TAU);
        d.min(TAU - d).to_degrees()
    };
    for _ in 0..1_000_000 {
        let x = rng.unit() * 2047.96;
        let y = rng.unit() * 255.96;
        let yaw = rng.unit() * TAU;
        let pitch = (rng.unit() * 2.0 - 1.0) * FRAC_PI_2;
        ex = ex.max((dequant_pos_xz(quant_pos_xz(x).code) - x).abs());
        ey = ey.max((dequant_pos_y(quant_pos_y(y).code) - y).abs());
        eyaw10 = eyaw10.max(ang(dequant_yaw(quant_yaw(yaw, 10).code, 10), yaw));
        eyaw8 = eyaw8.max(ang(dequant_yaw(quant_yaw(yaw, 8).code, 8), yaw));
        epitch8 = epitch8.max((dequant_pitch(quant_pitch(pitch, 8).code, 8) - pitch).abs());
        epitch9 = epitch9.max((dequant_pitch(quant_pitch(pitch, 9).code, 9) - pitch).abs());
        epitch16 = epitch16.max((dequant_pitch(quant_pitch(pitch, 16).code, 16) - pitch).abs());
        ei16 = ei16.max((pitch_i16_to_rad(rad_to_pitch_i16(pitch).code) - pitch).abs());
        let z = rng.unit() * 2047.96;
        ez = ez.max((dequant_pos_xz(quant_pos_xz(z).code) - z).abs());
    }
    let deg = |r: f32| r.to_degrees();
    println!(
        "max err: x={ex} z={ez} y={ey} m | yaw10={eyaw10} yaw8={eyaw8} deg | pitch8={} pitch9={} pitch16={} pitch_i16={} deg",
        deg(epitch8), deg(epitch9), deg(epitch16), deg(ei16)
    );
    // f32 arithmetic adds ~1e-4 m of slack on top of the exact 1/64 m half step
    assert!(ex <= 1.0 / 64.0 + 2e-4, "x {ex}");
    assert!(ez <= 1.0 / 64.0 + 2e-4, "z {ez}");
    assert!(ey <= 1.0 / 64.0 + 2e-4, "y {ey}");
    assert!(eyaw10 <= 0.35, "yaw10 {eyaw10}");
    assert!(eyaw8 <= 1.41 / 2.0 + 0.01, "yaw8 {eyaw8}");
    // 8-bit pitch half step is 0.3529 deg; 9 bit is 0.176 deg
    assert!(deg(epitch8) <= 0.3529 + 0.005, "pitch8 {}", deg(epitch8));
    assert!(deg(epitch9) <= 0.35, "pitch9 {}", deg(epitch9));
    assert!(deg(epitch16) <= 0.35, "pitch16 {}", deg(epitch16));
    assert!(deg(ei16) <= 0.35, "pitch_i16 {}", deg(ei16));
}

fn walking_cmd(i: u32) -> InputCmd {
    InputCmd {
        seq: 100 + i as u16,
        tick: 5000 + i,
        move_x: 0,
        move_z: 127,
        yaw: 12000u16.wrapping_add(i as u16 * 37),
        pitch: -300 + i as i16 * 5,
        buttons: buttons::SPRINT,
        weapon_slot: 1,
        aux: 0,
        view_tick: 3,
    }
}

fn typical_bundle() -> InputBundle {
    InputBundle {
        ack_snapshot_tick: 2,
        pred_hash: 0xA5,
        cmds: (0..3).map(walking_cmd).collect(),
        fire: ArrayVec::new(),
    }
}

#[test]
fn input_bundle_size_budget() {
    let bytes = encode_input_bundle(&typical_bundle()).unwrap();
    println!("typical 3-cmd InputBundle = {} bytes", bytes.len());
    assert!(bytes.len() <= 30, "{} bytes", bytes.len());
    assert_eq!(decode_input_bundle(&bytes).unwrap(), typical_bundle());

    // worst case: every field changes arbitrarily (still decodes, just larger)
    let mut b = typical_bundle();
    for (i, c) in b.cmds.iter_mut().enumerate() {
        c.seq = (i as u16).wrapping_mul(7919);
        c.move_x = (i as i8) * 40 - 60;
        c.yaw = (i as u16).wrapping_mul(30011);
        c.pitch = (i as i16).wrapping_mul(-20011);
        c.buttons = 0x3FFF ^ i as u16;
        c.weapon_slot = i as u8 + 9;
        c.aux = 0x55;
        c.view_tick = 100 * i as u16;
    }
    let bytes = encode_input_bundle(&b).unwrap();
    println!("worst-case 3-cmd InputBundle = {} bytes", bytes.len());
    assert_eq!(decode_input_bundle(&bytes).unwrap(), b);
}

#[test]
fn input_bundle_rules() {
    // tick must be previous + 1
    let mut b = typical_bundle();
    b.cmds[1].tick += 1;
    assert_eq!(encode_input_bundle(&b), Err(CodecError::InvalidValue));
    // empty bundle rejected
    let mut b = typical_bundle();
    b.cmds.clear();
    assert_eq!(encode_input_bundle(&b), Err(CodecError::InvalidValue));
    // move_x == -128 rejected
    let mut b = typical_bundle();
    b.cmds[0].move_x = -128;
    assert_eq!(encode_input_bundle(&b), Err(CodecError::InvalidValue));
    // origin hint over 14 bits rejected
    let mut b = typical_bundle();
    b.fire.push(FireCmd {
        seq: 1,
        view_tick: 1,
        weapon: 1,
        dir_yaw: 0,
        dir_pitch: 0,
        origin_hint: [0x4000, 0, 0],
        spread_seed: 0,
    });
    assert_eq!(encode_input_bundle(&b), Err(CodecError::Overflow));
    // n == 0 and fire_n == 3 on the wire are invalid tags
    assert_eq!(
        decode_input_bundle(&[
            0,
            0,
            0,
            0b0000_0000,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0
        ]),
        Err(CodecError::InvalidTag)
    );
    // fire_n == 3 (bits 26..27 of the header) is an invalid tag
    let mut bad = encode_input_bundle(&typical_bundle()).unwrap();
    bad[3] |= 0b0011_0000;
    assert_eq!(decode_input_bundle(&bad), Err(CodecError::InvalidTag));
}

fn sample_messages() -> Vec<Vec<u8>> {
    let mut v = vec![encode_input_bundle(&typical_bundle()).unwrap()];
    let mut b = typical_bundle();
    b.fire.push(FireCmd {
        seq: 9,
        view_tick: 2,
        weapon: 1,
        dir_yaw: 40000,
        dir_pitch: 31000,
        origin_hint: [1, 0x3FFF, 77],
        spread_seed: 4242,
    });
    v.push(encode_input_bundle(&b).unwrap());
    v.push(
        encode_c0_client(&ClientMsgC0::Hello(Hello {
            protocol_version: PROTOCOL_VERSION,
            data_hash: 0x0123_4567_89AB_CDEF,
            ticket: vec![7; 40],
            client_build: 42,
            caps: 3,
        }))
        .unwrap(),
    );
    v.push(encode_c0_client(&ClientMsgC0::Ping(Ping { client_time: 1234 })).unwrap());
    v.push(
        encode_c0_client(&ClientMsgC0::Disconnect(Disconnect {
            reason: disconnect_reason::NORMAL,
        }))
        .unwrap(),
    );
    v.push(
        encode_c0_server(&ServerMsgC0::Welcome(Welcome {
            player_id: 3,
            server_tick: 90_000,
            tick_hz: 30,
            match_seed: 0xDEAD_BEEF_0000_0001,
            data_hash: 0x0123_4567_89AB_CDEF,
            map_id: 1,
            mode: 0,
            team: 1,
            client_tick_offset: -4,
        }))
        .unwrap(),
    );
    v.push(
        encode_c0_server(&ServerMsgC0::Reject(Reject {
            reason: RejectReason::DataHashMismatch,
        }))
        .unwrap(),
    );
    v.push(
        encode_c0_server(&ServerMsgC0::Pong(Pong {
            client_time: 1234,
            server_tick: 90_001,
            server_time_ms: 3_000_033,
        }))
        .unwrap(),
    );
    v
}

fn try_all(bytes: &[u8]) {
    let _ = decode_input_bundle(bytes);
    let _ = decode_c0_client(bytes);
    let _ = decode_c0_server(bytes);
}

#[test]
fn fuzz_smoke() {
    let mut rng = Rng(0xFEED_FACE_CAFE_BEEF);
    // random byte strings
    for _ in 0..100_000 {
        let len = (rng.next() % 80) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
        try_all(&bytes);
    }
    // random strings starting with valid message-type bytes (reach deeper decode paths)
    for _ in 0..100_000 {
        let len = (rng.next() % 400) as usize;
        let mut bytes: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
        if let Some(b0) = bytes.first_mut() {
            *b0 = (rng.next() % 8) as u8;
        }
        try_all(&bytes);
    }
    // every prefix of every valid message, and every single-bit flip
    for msg in sample_messages() {
        for n in 0..msg.len() {
            try_all(&msg[..n]);
        }
        for i in 0..msg.len() * 8 {
            let mut m = msg.clone();
            m[i / 8] ^= 0x80 >> (i % 8);
            try_all(&m);
        }
    }
    // truncated valid messages must fail to decode (never silently succeed)
    let m = encode_c0_server(&ServerMsgC0::Pong(Pong {
        client_time: 1,
        server_tick: 2,
        server_time_ms: 3,
    }))
    .unwrap();
    for n in 0..m.len() {
        assert!(decode_c0_server(&m[..n]).is_err());
    }
}

#[test]
fn hello_ticket_257_rejected() {
    let mk = |n| {
        ClientMsgC0::Hello(Hello {
            protocol_version: 1,
            data_hash: 1,
            ticket: vec![1; n],
            client_build: 1,
            caps: 0,
        })
    };
    assert!(encode_c0_client(&mk(256)).is_ok());
    assert_eq!(encode_c0_client(&mk(257)), Err(CodecError::TooLong));
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Golden bytes guard the wire format. Regenerate deliberately with `UPDATE_GOLDEN=1`
/// (and bump `PROTOCOL_VERSION` if the change is breaking).
#[test]
fn golden_bytes() {
    let names = [
        "input_bundle_walk",
        "input_bundle_fire",
        "hello",
        "ping",
        "disconnect",
        "welcome",
        "reject",
        "pong",
    ];
    let msgs = sample_messages();
    assert_eq!(names.len(), msgs.len());
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    for (name, bytes) in names.iter().zip(&msgs) {
        let path = dir.join(format!("{name}.hex"));
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::write(&path, format!("{}\n", hex(bytes))).unwrap();
        }
        let want = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("missing golden {}", path.display()));
        assert_eq!(hex(bytes), want.trim(), "golden mismatch for {name}");
    }
}

/// Timing is only recorded, never asserted. Run: `cargo test -p frontline-protocol --release
/// -- --ignored --nocapture bench_codec`.
#[test]
#[ignore]
fn bench_codec() {
    let b = typical_bundle();
    let n = 1_000_000u32;
    let t = std::time::Instant::now();
    let mut total = 0usize;
    for _ in 0..n {
        total += encode_input_bundle(std::hint::black_box(&b)).unwrap().len();
    }
    let enc = t.elapsed().as_nanos() as f64 / n as f64;
    let bytes = encode_input_bundle(&b).unwrap();
    let t = std::time::Instant::now();
    for _ in 0..n {
        total += decode_input_bundle(std::hint::black_box(&bytes))
            .unwrap()
            .cmds
            .len();
    }
    let dec = t.elapsed().as_nanos() as f64 / n as f64;
    println!("InputBundle encode {enc:.0} ns, decode {dec:.0} ns (checksum {total})");
}
