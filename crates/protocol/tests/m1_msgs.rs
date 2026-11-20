//! M1 message families: proptest roundtrips (C0 additions, C4, new events) and error cases.
//! Fast run: `PROPTEST_CASES=10000`; default is 100000 per family (acceptance >= 1e5).

use frontline_protocol::msg::*;
use frontline_protocol::snapshot::events::{Event, EventEntry};
use frontline_protocol::*;
use proptest::prelude::*;

fn cfg() -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100_000);
    ProptestConfig::with_cases(cases)
}

fn arb_str(max: usize) -> impl Strategy<Value = String> {
    // Mix ASCII and multi-byte chars; truncate on a char boundary to <= max bytes.
    proptest::collection::vec(any::<char>(), 0..40).prop_map(move |cs| {
        let mut s = String::new();
        for c in cs {
            if s.len() + c.len_utf8() > max {
                break;
            }
            s.push(c);
        }
        s
    })
}

fn arb_c0_client() -> impl Strategy<Value = ClientMsgC0> {
    prop_oneof![
        (any::<u16>(), any::<u8>(), any::<u8>(), any::<u8>()).prop_map(|(a, b, c, d)| {
            ClientMsgC0::DeployRequest(DeployRequest {
                spawn_point: a,
                role: b,
                loadout: c,
                squad: d,
            })
        }),
        (1u8..=9, any::<u8>(), any::<u8>(), arb_str(MAX_NAME_KEY_LEN)).prop_map(
            |(op, squad, target, name_key)| ClientMsgC0::SquadOp(SquadOp {
                op: SquadOpKind::from_u8(op).unwrap(),
                squad,
                target,
                name_key
            })
        ),
        (0u8..=3, arb_str(MAX_CHAT_LEN), any::<u8>()).prop_map(|(c, text, sender)| {
            ClientMsgC0::Chat(Chat {
                channel: ChatChannel::from_u8(c).unwrap(),
                text,
                sender,
            })
        }),
        (any::<u8>(), any::<u16>(), any::<u16>(), any::<u8>()).prop_map(|(kind, x, z, ttl)| {
            ClientMsgC0::MarkerPlace(MarkerPlace {
                kind,
                pos: [x, z],
                ttl,
            })
        }),
        (any::<u16>(), any::<u8>()).prop_map(|(v, f)| ClientMsgC0::Settings(Settings {
            view_dist_m: v,
            flags: f
        })),
        (any::<u8>(), any::<u8>(), arb_str(MAX_REPORT_NOTE_LEN)).prop_map(|(t, r, note)| {
            ClientMsgC0::Report(Report {
                target: t,
                reason: r,
                note,
            })
        }),
    ]
}

fn arb_c0_server() -> impl Strategy<Value = ServerMsgC0> {
    prop_oneof![
        (any::<bool>(), any::<u32>(), any::<u8>()).prop_map(|(ok, t, r)| {
            ServerMsgC0::DeployResult(DeployResult {
                ok,
                spawn_tick: t,
                reason: r,
            })
        }),
        (0u8..=3, arb_str(MAX_CHAT_LEN), any::<u8>()).prop_map(|(c, text, sender)| {
            ServerMsgC0::Chat(Chat {
                channel: ChatChannel::from_u8(c).unwrap(),
                text,
                sender,
            })
        }),
    ]
}

fn arb_c4() -> impl Strategy<Value = ServerEventC4> {
    prop_oneof![
        (
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<bool>(),
            0u16..4096,
            any::<u8>(),
            any::<u8>()
        )
            .prop_map(
                |(victim, killer, weapon, headshot, dist_q, assist1, assist2)| {
                    ServerEventC4::Kill(Kill {
                        victim,
                        killer,
                        weapon,
                        headshot,
                        dist_q,
                        assist1,
                        assist2,
                    })
                }
            ),
        (any::<u8>(), any::<u8>(), any::<bool>()).prop_map(|(v, r, c)| {
            ServerEventC4::Death(Death {
                victim: v,
                respawn_after: r,
                can_be_revived: c,
            })
        }),
        (any::<u8>(), any::<u8>()).prop_map(|(m, t)| ServerEventC4::Revive(Revive {
            medic: m,
            target: t
        })),
        (any::<u8>(), any::<u8>(), any::<i16>()).prop_map(|(p, k, v)| {
            ServerEventC4::ScoreDelta(ScoreDelta {
                player: p,
                kind: k,
                value: v,
            })
        }),
        (any::<u8>(), any::<u8>(), any::<u8>()).prop_map(|(o, w, r)| {
            ServerEventC4::ObjectiveChange(ObjectiveChange {
                objective: o,
                owner: w,
                reason: r,
            })
        }),
        (
            any::<u16>(),
            0u8..=5,
            any::<u8>(),
            any::<u16>(),
            any::<u16>(),
            any::<u8>()
        )
            .prop_map(|(id, k, team, x, z, state)| {
                ServerEventC4::SpawnPointState(SpawnPointState {
                    id,
                    kind: SpawnKind::from_u8(k).unwrap(),
                    team,
                    pos: [x, z],
                    state,
                })
            }),
        (
            any::<u8>(),
            arb_str(MAX_NICK_LEN),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>()
        )
            .prop_map(|(player, nick, team, squad, role, flags)| {
                ServerEventC4::PlayerRoster(PlayerRoster {
                    player,
                    nick,
                    team,
                    squad,
                    role,
                    flags,
                })
            }),
        (any::<u16>(), any::<[i32; 3]>())
            .prop_map(|(code, args)| ServerEventC4::Notice(Notice { code, args })),
        (0u8..=3, any::<u32>()).prop_map(|(p, e)| {
            ServerEventC4::MatchPhase(MatchPhase {
                phase: Phase::from_u8(p).unwrap(),
                ends_at: e,
            })
        }),
    ]
}

fn arb_entry() -> impl Strategy<Value = EventEntry> {
    let c14 = 0u16..(1 << 14);
    (
        any::<u16>(),
        prop_oneof![
            (
                any::<u8>(),
                any::<u8>(),
                [c14.clone(), c14.clone(), c14.clone()],
                0u16..1024,
                0u16..512
            )
                .prop_map(|(shooter, weapon, muzzle, dir_yaw, dir_pitch)| {
                    Event::Shot {
                        shooter,
                        weapon,
                        muzzle,
                        dir_yaw,
                        dir_pitch,
                    }
                }),
            (
                [c14.clone(), c14.clone(), c14.clone()],
                0u16..4096,
                any::<u8>()
            )
                .prop_map(|(pos, normal, material)| Event::Impact {
                    pos,
                    normal,
                    material
                }),
            (any::<u8>(), any::<u8>(), any::<u8>()).prop_map(|(target, zone, damage_q)| {
                Event::HitConfirm {
                    target,
                    zone,
                    damage_q,
                }
            }),
            (any::<u8>(), any::<u8>()).prop_map(|(angle, dmg_q)| Event::DamageDir { angle, dmg_q }),
            (
                [c14.clone(), c14.clone(), c14.clone()],
                any::<u8>(),
                any::<u8>()
            )
                .prop_map(|(pos, kind, radius_q)| Event::Explosion {
                    pos,
                    kind,
                    radius_q
                }),
            (any::<u8>(), any::<u8>()).prop_map(|(source_dir, intensity)| Event::Suppress {
                source_dir,
                intensity
            }),
            (any::<u8>(), [c14.clone(), c14], any::<u8>())
                .prop_map(|(kind, pos, ttl)| Event::Marker { kind, pos, ttl }),
        ],
    )
        .prop_map(|(id, event)| EventEntry { id, event })
}

proptest! {
    #![proptest_config(cfg())]

    #[test]
    fn rt_c0_client(m in arb_c0_client()) {
        let b = encode_c0_client(&m).unwrap();
        prop_assert_eq!(decode_c0_client(&b).unwrap(), m);
    }

    #[test]
    fn rt_c0_server(m in arb_c0_server()) {
        let b = encode_c0_server(&m).unwrap();
        prop_assert_eq!(decode_c0_server(&b).unwrap(), m);
    }

    #[test]
    fn rt_c4(m in arb_c4()) {
        let b = encode_c4(&m).unwrap();
        prop_assert_eq!(decode_c4(&b).unwrap(), m);
    }

    #[test]
    fn rt_events(e in arb_entry()) {
        let mut w = BitWriter::new();
        e.encode(&mut w).unwrap();
        let b = w.finish();
        let mut r = BitReader::new(&b);
        prop_assert_eq!(EventEntry::decode(&mut r).unwrap(), e);
        r.finish().unwrap();
    }

    /// Arbitrary bytes never panic any decoder.
    #[test]
    fn garbage_never_panics(b in proptest::collection::vec(any::<u8>(), 0..300)) {
        let _ = decode_c0_client(&b);
        let _ = decode_c0_server(&b);
        let _ = decode_c4(&b);
        let mut r = BitReader::new(&b);
        let _ = EventEntry::decode(&mut r);
    }
}

// ---- error cases -------------------------------------------------------------------------

#[test]
fn chat_counts_scalars_in_both_directions() {
    for scalar in ['a', 'é', '界', '😀'] {
        let mut chat = Chat {
            channel: ChatChannel::All,
            text: scalar.to_string().repeat(MAX_CHAT_LEN),
            sender: 7,
        };
        let client = ClientMsgC0::Chat(chat.clone());
        let server = ServerMsgC0::Chat(chat.clone());
        let bytes = encode_c0_client(&client).unwrap();
        assert_eq!(bytes, encode_c0_server(&server).unwrap());
        assert_eq!(bytes.len(), 5 + MAX_CHAT_LEN * scalar.len_utf8());
        assert_eq!(decode_c0_client(&bytes), Ok(client));
        assert_eq!(decode_c0_server(&bytes), Ok(server));

        chat.text.push(scalar);
        assert_eq!(
            encode_c0_client(&ClientMsgC0::Chat(chat.clone())),
            Err(CodecError::TooLong)
        );
        assert_eq!(
            encode_c0_server(&ServerMsgC0::Chat(chat)),
            Err(CodecError::TooLong)
        );
    }
}

#[test]
fn nick_limit_24_25() {
    let mk = |n: usize| {
        ServerEventC4::PlayerRoster(PlayerRoster {
            player: 1,
            nick: "n".repeat(n),
            team: 0,
            squad: 0,
            role: 0,
            flags: 0,
        })
    };
    assert!(encode_c4(&mk(24)).is_ok());
    assert_eq!(encode_c4(&mk(25)), Err(CodecError::TooLong));
}

#[test]
fn report_and_name_key_limits() {
    let rep = |n| {
        ClientMsgC0::Report(Report {
            target: 1,
            reason: 1,
            note: "x".repeat(n),
        })
    };
    assert!(encode_c0_client(&rep(120)).is_ok());
    assert_eq!(encode_c0_client(&rep(121)), Err(CodecError::TooLong));
    let sq = |n| {
        ClientMsgC0::SquadOp(SquadOp {
            op: SquadOpKind::Rename,
            squad: 1,
            target: 0,
            name_key: "k".repeat(n),
        })
    };
    assert!(encode_c0_client(&sq(32)).is_ok());
    assert_eq!(encode_c0_client(&sq(33)), Err(CodecError::TooLong));
}

fn raw_chat(text: &[u8], declared_len: usize) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.write_bits(0x13, 8).unwrap();
    w.write_bits(0, 8).unwrap();
    w.write_bits(declared_len as u64, 10).unwrap();
    for &byte in text {
        w.write_bits(byte.into(), 8).unwrap();
    }
    w.write_bits(0, 8).unwrap(); // sender
    w.finish()
}

#[test]
fn wire_length_over_limit_rejected_on_decode() {
    for text in [vec![b'a'; MAX_CHAT_LEN + 1], vec![b'a'; MAX_CHAT_BYTES + 1]] {
        let bytes = raw_chat(&text, text.len());
        assert_eq!(decode_c0_client(&bytes), Err(CodecError::TooLong));
        assert_eq!(decode_c0_server(&bytes), Err(CodecError::TooLong));
    }
}

#[test]
fn invalid_utf8_rejected() {
    let bytes = raw_chat(&[0xff, 0xfe], 2);
    assert_eq!(decode_c0_client(&bytes), Err(CodecError::InvalidValue));
    assert_eq!(decode_c0_server(&bytes), Err(CodecError::InvalidValue));
}

#[test]
fn length_prefix_beyond_input_rejected() {
    let bytes = raw_chat(b"ab", 50);
    assert!(decode_c0_client(&bytes).is_err());
    assert!(decode_c0_server(&bytes).is_err());
}

#[test]
fn invalid_enums() {
    assert_eq!(
        decode_c0_client(&[0x12, 0, 0, 0, 0]),
        Err(CodecError::InvalidTag)
    );
    assert_eq!(
        decode_c0_client(&[0x12, 10, 0, 0, 0]),
        Err(CodecError::InvalidTag)
    );
    assert_eq!(
        decode_c0_client(&[0x13, 4, 0, 0]),
        Err(CodecError::InvalidTag)
    );
    // SpawnPointState with kind 6
    assert_eq!(
        decode_c4(&[0x06, 0, 1, 6, 0, 0, 0, 0, 0, 0, 0]),
        Err(CodecError::InvalidTag)
    );
    // MatchPhase with phase 4
    assert_eq!(
        decode_c4(&[0x09, 4, 0, 0, 0, 0]),
        Err(CodecError::InvalidTag)
    );
    assert_eq!(decode_c4(&[0x00]), Err(CodecError::InvalidTag));
    assert_eq!(decode_c4(&[0xFF]), Err(CodecError::InvalidTag));
}

#[test]
fn direction_mismatch_rejected() {
    // server-only DeployResult must not decode as a client message and vice versa
    assert_eq!(
        decode_c0_client(&[0x11, 0, 0, 0, 0, 0, 0]),
        Err(CodecError::InvalidTag)
    );
    assert_eq!(
        decode_c0_server(&[0x10, 0, 0, 0, 0, 0]),
        Err(CodecError::InvalidTag)
    );
}

#[test]
fn truncated_messages_rejected() {
    let full = encode_c4(&ServerEventC4::Kill(Kill {
        victim: 1,
        killer: 2,
        weapon: 3,
        headshot: true,
        dist_q: 4095,
        assist1: 5,
        assist2: 6,
    }))
    .unwrap();
    for n in 0..full.len() {
        assert!(decode_c4(&full[..n]).is_err(), "prefix {n}");
    }
    assert!(decode_c4(&full).is_ok());
    let mut extra = full.clone();
    extra.push(0xFF);
    assert!(decode_c4(&extra).is_err());
}

#[test]
fn kill_dist_overflow_rejected() {
    let k = ServerEventC4::Kill(Kill {
        victim: 0,
        killer: 0xFF,
        weapon: 0,
        headshot: false,
        dist_q: 4096,
        assist1: 0,
        assist2: 0,
    });
    assert!(encode_c4(&k).is_err());
}

#[test]
fn unknown_event_type_rejected() {
    let mut w = BitWriter::new();
    w.write_bits(7, 16).unwrap();
    w.write_bits(8, 5).unwrap();
    w.write_bits(0, 40).unwrap();
    let b = w.finish();
    assert_eq!(
        EventEntry::decode(&mut BitReader::new(&b)),
        Err(CodecError::InvalidTag)
    );
}

#[test]
fn event_wire_sizes() {
    let size = |e: Event| {
        let mut w = BitWriter::new();
        EventEntry { id: 1, event: e }.encode(&mut w).unwrap();
        w.bit_len()
    };
    assert_eq!(
        size(Event::Suppress {
            source_dir: 0,
            intensity: 0
        }),
        16 + 5 + 16
    );
    assert_eq!(
        size(Event::Explosion {
            pos: [0; 3],
            kind: 0,
            radius_q: 0
        }),
        16 + 5 + 58
    );
    assert_eq!(
        size(Event::Marker {
            kind: 0,
            pos: [0; 2],
            ttl: 0
        }),
        16 + 5 + 44
    );
}

#[test]
fn score_kind_preserves_all_bytes_and_decodes_category_and_reason() {
    let categories = [
        ScoreCategory::Combat,
        ScoreCategory::Offense,
        ScoreCategory::Defense,
        ScoreCategory::Support,
    ];
    for kind in 0u8..=255 {
        let score = ScoreDelta {
            player: 42,
            kind,
            value: -30,
        };
        let event = ServerEventC4::ScoreDelta(score);
        let bytes = encode_c4(&event).unwrap();
        assert_eq!(bytes.as_slice(), &[0x04, 42, kind, 0xff, 0xe2]);
        assert_eq!(decode_c4(&bytes).unwrap(), event);
        assert_eq!(score.category(), categories[usize::from(kind / 64)]);
        assert_eq!(score.reason(), kind % 64);
    }
}

#[test]
fn boundary_values() {
    let m = ServerEventC4::Notice(Notice {
        code: u16::MAX,
        args: [i32::MIN, i32::MAX, 0],
    });
    assert_eq!(decode_c4(&encode_c4(&m).unwrap()).unwrap(), m);
    let s = ServerEventC4::ScoreDelta(ScoreDelta {
        player: 255,
        kind: 255,
        value: i16::MIN,
    });
    assert_eq!(decode_c4(&encode_c4(&s).unwrap()).unwrap(), s);
}

#[test]
fn m2_facilities_wire() {
    let request = ClientMsgC0::PlacementRequest(PlacementRequest {
        id: 1,
        action: PlacementAction::Facility {
            op: FacilityOp::Reorient,
            structure: 0x1234,
            arg: 0x4321,
        },
    });
    let bytes = [0x17, 0, 1, 7, 2, 0x12, 0x34, 0x43, 0x21];
    assert_eq!(encode_c0_client(&request).unwrap(), bytes);
    assert_eq!(decode_c0_client(&bytes), Ok(request));
    let structure = ServerEventC4::StructureState(StructureState {
        id: 0x1234,
        kind: 1,
        pos: [0x1020, 0x3040],
        state: 2,
    });
    let bytes = [0x10, 0x12, 0x34, 1, 0x10, 0x20, 0x30, 0x40, 2];
    assert_eq!(encode_c4(&structure).unwrap(), bytes);
    assert_eq!(decode_c4(&bytes), Ok(structure));
}
