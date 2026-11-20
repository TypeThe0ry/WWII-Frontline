use frontline_protocol::msg::{ClientMsgC0 as C, ServerEventC4 as S, *};
use frontline_protocol::CodecError;
use proptest::prelude::*;

fn check_c0(m: C) -> Vec<u8> {
    let b = encode_c0_client(&m).unwrap();
    assert_eq!(decode_c0_client(&b), Ok(m));
    b
}

fn check_c4(m: S) -> Vec<u8> {
    let b = encode_c4(&m).unwrap();
    assert_eq!(decode_c4(&b), Ok(m));
    b
}

#[test]
fn append_only_wire_and_strict_framing() {
    let vote_id = 0x8901_2345;
    let c0 = [
        check_c0(C::VoteStart(VoteStart {
            kind: VoteKind::Mute,
            target: 37,
        })),
        check_c0(C::VoteCast(VoteCast { vote_id, yes: true })),
        check_c0(C::TeamSwitchRequest(TeamSwitchRequest {
            team: 1,
            accept_offer: true,
        })),
        check_c0(C::ForgivePunish(ForgivePunish {
            victim_event: 0xbeef,
            punish: true,
        })),
        check_c0(C::MapVoteCast(MapVoteCast { vote_id, choice: 2 })),
    ];
    let c4 = [
        check_c4(S::VoteState(VoteState {
            vote_id,
            kind: VoteKind::RemoveCommander,
            target: 37,
            yes: 5,
            no: 2,
            ends_in_s: 45,
        })),
        check_c4(S::VoteResult(VoteResult {
            vote_id,
            passed: false,
        })),
        check_c4(S::TkPrompt(TkPrompt {
            killer: 37,
            expires_s: 15,
            victim_event: 0xbeef,
        })),
        check_c4(S::TeamBalanceNotice(TeamBalanceNotice {
            code: TeamBalanceCode::Moved,
            args: [1, team_move_flags::VOLUNTEER_BONUS],
        })),
        check_c4(S::MapVoteState(MapVoteState {
            vote_id,
            candidates: [0, 2, 255],
            votes: [16, 0, 8],
            ends_in_s: 30,
        })),
        check_c4(S::MapVoteResult(MapVoteResult {
            vote_id,
            winner: 255,
        })),
    ];
    let mut hex = include_str!("golden/server_rules.hex").lines();
    for (is_c0, mut b) in c0
        .into_iter()
        .map(|b| (true, b))
        .chain(c4.into_iter().map(|b| (false, b)))
    {
        assert_eq!(
            b.iter().map(|v| format!("{v:02x}")).collect::<String>(),
            hex.next().unwrap()
        );
        let decode_ok = |buf: &[u8]| {
            if is_c0 {
                decode_c0_client(buf).is_ok()
            } else {
                decode_c4(buf).is_ok()
            }
        };
        for end in 0..b.len() {
            assert!(!decode_ok(&b[..end]));
        }
        b.push(0);
        assert!(!decode_ok(&b));
    }
    assert!(hex.next().is_none());
}

#[test]
fn invalid_tags_choices_and_padding() {
    assert_eq!(
        decode_c0_client(&[0x19, 3, 0]),
        Err(CodecError::InvalidValue)
    );
    assert_eq!(
        decode_c4(&[0x0a, 0, 0, 0, 0, 3, 0, 0, 0, 0]),
        Err(CodecError::InvalidValue)
    );
    assert_eq!(
        decode_c4(&[0x0d, 14, 0, 0, 0, 0, 0, 0, 0, 0]),
        Err(CodecError::InvalidValue)
    );
    assert_eq!(
        decode_c0_client(&[0x1d, 0, 0, 0, 0, 3]),
        Err(CodecError::InvalidValue)
    );
    assert_eq!(
        encode_c0_client(&C::MapVoteCast(MapVoteCast {
            vote_id: 0,
            choice: 3
        })),
        Err(CodecError::InvalidValue)
    );
    assert_eq!(
        decode_c0_client(&[0x1a, 0, 0, 0, 0, 0x81]),
        Err(CodecError::Trailing)
    );
}

fn all_variants(id: u32, event: u16, bytes: [u8; 8], args: [i32; 2], flag: bool) {
    let [team, player, yes, no, t, a, b, c] = bytes;
    let kind = VoteKind::from_u8(a % 3).unwrap();
    check_c0(C::VoteStart(VoteStart {
        kind,
        target: player,
    }));
    check_c0(C::VoteCast(VoteCast {
        vote_id: id,
        yes: flag,
    }));
    check_c0(C::TeamSwitchRequest(TeamSwitchRequest {
        team,
        accept_offer: flag,
    }));
    check_c0(C::ForgivePunish(ForgivePunish {
        victim_event: event,
        punish: flag,
    }));
    check_c0(C::MapVoteCast(MapVoteCast {
        vote_id: id,
        choice: b.min(2),
    }));
    check_c4(S::VoteState(VoteState {
        vote_id: id,
        kind,
        target: player,
        yes,
        no,
        ends_in_s: t,
    }));
    check_c4(S::VoteResult(VoteResult {
        vote_id: id,
        passed: flag,
    }));
    check_c4(S::TkPrompt(TkPrompt {
        killer: player,
        expires_s: t,
        victim_event: event,
    }));
    check_c4(S::TeamBalanceNotice(TeamBalanceNotice {
        code: TeamBalanceCode::from_u8(1 + c % 13).unwrap(),
        args,
    }));
    check_c4(S::MapVoteState(MapVoteState {
        vote_id: id,
        candidates: [a, b, c],
        votes: [yes, no, team],
        ends_in_s: t,
    }));
    check_c4(S::MapVoteResult(MapVoteResult {
        vote_id: id,
        winner: player,
    }));
}

#[test]
fn field_boundaries() {
    all_variants(0, 0, [0; 8], [i32::MIN; 2], false);
    all_variants(u32::MAX, u16::MAX, [255; 8], [i32::MAX; 2], true);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]
    #[test]
    fn every_server_rule_message_round_trips(
        id: u32, event: u16, bytes: [u8; 8], args: [i32; 2], flag: bool,
    ) {
        all_variants(id, event, bytes, args, flag);
    }
}
