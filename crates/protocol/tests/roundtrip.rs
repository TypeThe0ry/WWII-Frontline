//! Property tests: encode -> decode is the identity for every M0 message.
//! Fast run: `PROPTEST_CASES=10000 cargo test -p frontline-protocol --release`.
//! Full run: `cargo test -p frontline-protocol --release -- --ignored roundtrip_1m`.

use arrayvec::ArrayVec;
use frontline_protocol::msg::*;
use frontline_protocol::*;
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};

fn arb_input_cmd() -> impl Strategy<Value = InputCmd> {
    (
        any::<u16>(),
        any::<u32>(),
        -127i8..=127,
        -127i8..=127,
        any::<u16>(),
        any::<i16>(),
        any::<u16>(),
        any::<u8>(),
        any::<u8>(),
        any::<u16>(),
    )
        .prop_map(
            |(seq, tick, move_x, move_z, yaw, pitch, buttons, weapon_slot, aux, view_tick)| {
                InputCmd {
                    seq,
                    tick,
                    move_x,
                    move_z,
                    yaw,
                    pitch,
                    buttons,
                    weapon_slot,
                    aux,
                    view_tick,
                }
            },
        )
}

fn arb_fire() -> impl Strategy<Value = FireCmd> {
    (
        any::<u16>(),
        any::<u16>(),
        any::<u8>(),
        any::<u16>(),
        any::<u16>(),
        proptest::array::uniform3(0u16..0x4000),
        any::<u16>(),
    )
        .prop_map(
            |(seq, view_tick, weapon, dir_yaw, dir_pitch, origin_hint, spread_seed)| FireCmd {
                seq,
                view_tick,
                weapon,
                dir_yaw,
                dir_pitch,
                origin_hint,
                spread_seed,
            },
        )
}

/// A bundle: consecutive ticks; `seq` mostly consecutive but sometimes arbitrary; later
/// commands mostly near the previous one so every delta form is exercised.
fn arb_bundle() -> impl Strategy<Value = InputBundle> {
    (
        any::<u16>(),
        any::<u8>(),
        arb_input_cmd(),
        proptest::collection::vec((arb_input_cmd(), any::<bool>(), 0u8..8), 0..=2),
        proptest::collection::vec(arb_fire(), 0..=2),
    )
        .prop_map(|(ack, pred_hash, first, rest, fires)| {
            let mut cmds = ArrayVec::<InputCmd, 3>::new();
            cmds.push(first);
            for (mut c, keep_seq, mutate) in rest {
                let prev = *cmds.last().unwrap();
                c.tick = prev.tick.wrapping_add(1);
                if keep_seq {
                    c.seq = prev.seq.wrapping_add(1);
                }
                // make some fields equal / close to the previous to hit the compact forms
                if mutate & 1 != 0 {
                    c.move_x = prev.move_x;
                    c.move_z = prev.move_z;
                }
                if mutate & 2 != 0 {
                    c.yaw = prev.yaw.wrapping_add((mutate as u16).wrapping_sub(4));
                }
                if mutate & 4 != 0 {
                    c.buttons = prev.buttons;
                    c.weapon_slot = prev.weapon_slot;
                    c.aux = prev.aux;
                    c.view_tick = prev.view_tick;
                    c.pitch = prev.pitch.wrapping_add(mutate as i16 - 4);
                }
                cmds.push(c);
            }
            InputBundle {
                ack_snapshot_tick: ack,
                pred_hash,
                cmds,
                fire: fires.into_iter().collect(),
            }
        })
}

fn arb_hello() -> impl Strategy<Value = Hello> {
    (
        any::<u16>(),
        any::<u64>(),
        proptest::collection::vec(any::<u8>(), 0..=256),
        any::<u32>(),
        any::<u8>(),
    )
        .prop_map(
            |(protocol_version, data_hash, ticket, client_build, caps)| Hello {
                protocol_version,
                data_hash,
                ticket,
                client_build,
                caps,
            },
        )
}

fn arb_welcome() -> impl Strategy<Value = Welcome> {
    (
        any::<u8>(),
        any::<u32>(),
        any::<u8>(),
        any::<u64>(),
        any::<u64>(),
        any::<[u8; 3]>(),
        any::<i16>(),
    )
        .prop_map(
            |(player_id, server_tick, tick_hz, match_seed, data_hash, m, off)| Welcome {
                player_id,
                server_tick,
                tick_hz,
                match_seed,
                data_hash,
                map_id: m[0],
                mode: m[1],
                team: m[2],
                client_tick_offset: off,
            },
        )
}

#[derive(Debug, Clone)]
enum Sample {
    Cmd(InputCmd),
    Bundle(InputBundle),
    Hello(Hello),
    Welcome(Welcome),
    Ping(u32),
    Pong(u32, u32, u32),
}

fn arb_sample() -> impl Strategy<Value = Sample> {
    prop_oneof![
        arb_input_cmd().prop_map(Sample::Cmd),
        arb_bundle().prop_map(Sample::Bundle),
        arb_hello().prop_map(Sample::Hello),
        arb_welcome().prop_map(Sample::Welcome),
        any::<u32>().prop_map(Sample::Ping),
        (any::<u32>(), any::<u32>(), any::<u32>()).prop_map(|(a, b, c)| Sample::Pong(a, b, c)),
    ]
}

fn check(s: &Sample) -> Result<(), TestCaseError> {
    match s {
        Sample::Cmd(c) => {
            let mut w = BitWriter::new();
            c.encode(&mut w).unwrap();
            let bytes = w.finish();
            let mut r = BitReader::new(&bytes);
            prop_assert_eq!(&InputCmd::decode(&mut r).unwrap(), c);
        }
        Sample::Bundle(b) => {
            let bytes = encode_input_bundle(b).unwrap();
            prop_assert_eq!(&decode_input_bundle(&bytes).unwrap(), b);
        }
        Sample::Hello(h) => {
            let m = ClientMsgC0::Hello(h.clone());
            let bytes = encode_c0_client(&m).unwrap();
            prop_assert_eq!(decode_c0_client(&bytes).unwrap(), m);
        }
        Sample::Welcome(x) => {
            let m = ServerMsgC0::Welcome(*x);
            let bytes = encode_c0_server(&m).unwrap();
            prop_assert_eq!(decode_c0_server(&bytes).unwrap(), m);
        }
        Sample::Ping(t) => {
            let m = ClientMsgC0::Ping(Ping { client_time: *t });
            let bytes = encode_c0_client(&m).unwrap();
            prop_assert_eq!(decode_c0_client(&bytes).unwrap(), m);
        }
        Sample::Pong(a, b, c) => {
            let m = ServerMsgC0::Pong(Pong {
                client_time: *a,
                server_tick: *b,
                server_time_ms: *c,
            });
            let bytes = encode_c0_server(&m).unwrap();
            prop_assert_eq!(decode_c0_server(&bytes).unwrap(), m);
        }
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn roundtrip_input_cmd(c in arb_input_cmd()) { check(&Sample::Cmd(c))?; }

    #[test]
    fn roundtrip_input_bundle(b in arb_bundle()) { check(&Sample::Bundle(b))?; }

    #[test]
    fn roundtrip_hello(h in arb_hello()) { check(&Sample::Hello(h))?; }

    #[test]
    fn roundtrip_welcome(x in arb_welcome()) { check(&Sample::Welcome(x))?; }

    #[test]
    fn roundtrip_ping_pong(a in any::<u32>(), b in any::<u32>(), c in any::<u32>()) {
        check(&Sample::Ping(a))?;
        check(&Sample::Pong(a, b, c))?;
    }

    #[test]
    fn roundtrip_reject_disconnect(r in 1u8..=5, d in any::<u8>()) {
        let m = ServerMsgC0::Reject(Reject { reason: RejectReason::from_u8(r).unwrap() });
        prop_assert_eq!(decode_c0_server(&encode_c0_server(&m).unwrap()).unwrap(), m);
        let m = ServerMsgC0::Disconnect(Disconnect { reason: d });
        prop_assert_eq!(decode_c0_server(&encode_c0_server(&m).unwrap()).unwrap(), m);
        let m = ClientMsgC0::Disconnect(Disconnect { reason: d });
        prop_assert_eq!(decode_c0_client(&encode_c0_client(&m).unwrap()).unwrap(), m);
    }
}

/// 10^6 random samples across InputCmd / InputBundle / Hello / Welcome / Ping / Pong.
#[test]
#[ignore]
fn roundtrip_1m() {
    let mut runner = TestRunner::new(Config {
        cases: 1_000_000,
        max_shrink_iters: 0,
        failure_persistence: None,
        ..Config::default()
    });
    runner
        .run(&arb_sample(), |s| check(&s))
        .expect("roundtrip failed");
}
