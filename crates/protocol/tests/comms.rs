use frontline_protocol::msg::{
    decode_c0_client, decode_c0_server, decode_team_state_with_markers, encode_c0_client,
    encode_c0_server, encode_team_state_with_markers, ClientMsgC0, NoticeC0, PingKind, PingPlace,
    ServerMsgC0, TeamMarker, TeamMarkerKind, TeamState, TeamStateWithMarkers, MAX_MARKER_CELL,
    MAX_NOTICE_KEY_LEN,
};
use frontline_protocol::snapshot::{Event, EventEntry, MarkerKind};
use frontline_protocol::{BitReader, BitWriter, CodecError};
use proptest::prelude::*;

#[test]
fn ping_is_eight_bytes_and_validates_kind_direction_and_size() {
    let ping = ClientMsgC0::PingPlace(PingPlace {
        kind: PingKind::Enemy,
        pos: [0x1234, 0x5678],
        target: 0x9abc,
    });
    let golden = [0x1e, 1, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc];
    assert_eq!(encode_c0_client(&ping).unwrap(), golden);
    assert_eq!(decode_c0_client(&golden).unwrap(), ping);
    assert_eq!(decode_c0_server(&golden), Err(CodecError::InvalidTag));
    for len in 0..golden.len() {
        assert!(decode_c0_client(&golden[..len]).is_err());
    }
    let mut bad = golden;
    bad[1] = 8;
    assert_eq!(decode_c0_client(&bad), Err(CodecError::InvalidValue));
}

#[test]
fn notice_uses_bounded_utf8_keys() {
    let notice = |key| ServerMsgC0::Notice(NoticeC0 { key });
    for key in [
        "x".into(),
        "comms.ping.cooldown".into(),
        "é".repeat(MAX_NOTICE_KEY_LEN / 2),
    ] {
        let message = notice(key);
        let bytes = encode_c0_server(&message).unwrap();
        assert_eq!(bytes[0], 0x1f);
        assert_eq!(decode_c0_server(&bytes).unwrap(), message);
        assert_eq!(decode_c0_client(&bytes), Err(CodecError::InvalidTag));
        assert!(decode_c0_server(&bytes[..bytes.len() - 1]).is_err());
    }
    assert_eq!(
        encode_c0_server(&notice(String::new())),
        Err(CodecError::InvalidValue)
    );
    assert_eq!(decode_c0_server(&[0x1f, 0]), Err(CodecError::InvalidValue));
    assert_eq!(
        encode_c0_server(&notice("a".repeat(MAX_NOTICE_KEY_LEN + 1))),
        Err(CodecError::TooLong)
    );
    assert_eq!(
        decode_c0_server(&[0x1f, (MAX_NOTICE_KEY_LEN + 1) as u8]),
        Err(CodecError::TooLong)
    );
    assert_eq!(
        decode_c0_server(&[0x1f, 1, 0xff]),
        Err(CodecError::InvalidValue)
    );
}

fn encode_event(event: EventEntry) -> Result<Vec<u8>, CodecError> {
    let mut writer = BitWriter::new();
    event.encode(&mut writer)?;
    Ok(writer.finish())
}

fn decode_event(bytes: &[u8]) -> Result<EventEntry, CodecError> {
    let mut reader = BitReader::new(bytes);
    let event = EventEntry::decode(&mut reader)?;
    reader.finish()?;
    Ok(event)
}

#[test]
fn enemy_events_have_no_target_field() {
    for kind in [
        MarkerKind::Ping(PingKind::Enemy),
        MarkerKind::Manual(TeamMarkerKind::MarkVehicle),
    ] {
        assert!(kind.is_enemy());
        let marker = |target, ttl, pos| EventEntry {
            id: u16::MAX,
            event: Event::MarkerUpsert {
                id: 0,
                kind,
                pos,
                ttl,
                owner: 0,
                target,
            },
        };
        let valid = marker(None, 1, [0, MAX_MARKER_CELL]);
        let bytes = encode_event(valid).unwrap();
        // No target-presence bit or ID, just 85 bits including the event id and tag.
        assert_eq!(bytes.len(), 11);
        let mut reader = BitReader::new(&bytes);
        assert_eq!(EventEntry::decode(&mut reader).unwrap(), valid);
        assert_eq!(reader.remaining_bits(), 3);
        for len in 0..bytes.len() {
            assert!(decode_event(&bytes[..len]).is_err());
        }
        assert_eq!(
            encode_event(marker(Some(0), 1, [0, 0])),
            Err(CodecError::InvalidValue)
        );
        assert_eq!(
            encode_event(marker(None, 255, [0, 0])),
            Err(CodecError::InvalidValue)
        );
        assert_eq!(
            encode_event(marker(None, 0, [0, 0])),
            Err(CodecError::InvalidValue)
        );
        assert_eq!(
            encode_event(marker(None, 1, [MAX_MARKER_CELL + 1, 0])),
            Err(CodecError::Overflow)
        );
    }
    // Unknown kind is rejected even though the remaining fields would decode.
    let mut writer = BitWriter::new();
    writer.write_bits(0, 16).unwrap();
    writer.write_bits(10, 5).unwrap();
    writer.write_bits(0, 16).unwrap();
    writer.write_bits(0xff, 8).unwrap();
    writer.write_bits(1, 40).unwrap();
    assert_eq!(
        decode_event(&writer.finish()),
        Err(CodecError::InvalidValue)
    );
}

#[test]
fn only_command_orders_can_use_the_until_revoked_ttl() {
    for kind in [
        MarkerKind::Manual(TeamMarkerKind::CommandAttack),
        MarkerKind::Manual(TeamMarkerKind::CommandDefend),
    ] {
        let entry = EventEntry {
            id: 0,
            event: Event::MarkerUpsert {
                id: u16::MAX,
                kind,
                pos: [MAX_MARKER_CELL; 2],
                ttl: u8::MAX,
                owner: u8::MAX,
                target: None,
            },
        };
        let mut bytes = encode_event(entry).unwrap();
        assert_eq!(decode_event(&bytes).unwrap(), entry);
        assert!(!kind.is_enemy());
        // Replace the 8-bit kind (starting at bit 37) with Enemy while keeping the TTL.
        bytes[4] &= 0xf8;
        bytes[5] = (bytes[5] & 0x07) | (1 << 3);
        assert_eq!(decode_event(&bytes), Err(CodecError::InvalidValue));
    }
}

#[test]
fn legacy_manual_ids_and_projection_keep_their_wire_shape() {
    let tags = [
        0, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
    ];
    assert!(TeamMarkerKind::from_u8(1).is_none());
    assert!(TeamMarkerKind::from_u8(2).is_none());
    for tag in tags {
        let frame = TeamStateWithMarkers {
            team: TeamState {
                tick: 2,
                members: vec![],
            },
            markers: vec![TeamMarker {
                kind: TeamMarkerKind::from_u8(tag).unwrap(),
                x: 4,
                z: 3,
                ttl_s: 20,
            }],
        };
        let golden = [0, 2, 0, 1, tag, 0, 0x40, 3, 20];
        assert_eq!(encode_team_state_with_markers(&frame).unwrap(), golden);
        assert_eq!(decode_team_state_with_markers(&golden).unwrap(), frame);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]
    #[test]
    fn ping_and_lifecycle_roundtrip(
        ping_tag in 0u8..8, x in any::<u16>(), z in any::<u16>(), hint in any::<u16>(),
        id in any::<u16>(), seq in any::<u16>(), ttl in 1u8..=254, owner in any::<u8>(),
        target in proptest::option::of(any::<u16>()), manual_tag in 3u8..20,
    ) {
        let ping = PingKind::from_u8(ping_tag).unwrap();
        let message = ClientMsgC0::PingPlace(PingPlace { kind: ping, pos: [x, z], target: hint });
        prop_assert_eq!(decode_c0_client(&encode_c0_client(&message).unwrap()), Ok(message));
        let pos = [x & MAX_MARKER_CELL, z & MAX_MARKER_CELL];
        for event in [
            Event::MarkerUpsert { id, kind: MarkerKind::Ping(ping), pos, ttl, owner,
                target: if ping == PingKind::Thanks { target } else { None } },
            Event::MarkerUpsert { id, kind: MarkerKind::Manual(TeamMarkerKind::from_u8(manual_tag).unwrap()),
                pos, ttl, owner, target: None },
            Event::MarkerExpired { id },
        ] {
            let entry = EventEntry { id: seq, event };
            prop_assert_eq!(decode_event(&encode_event(entry).unwrap()), Ok(entry));
        }
    }
}
