use frontline_protocol::msg::*;
use frontline_protocol::CodecError;
use proptest::prelude::*;

fn member() -> impl Strategy<Value = TeamMember> {
    (0u16..2048, 0u16..2048, 0u8..16, 0u8..8, 0u8..8).prop_map(|(x, z, role, state, facing)| {
        TeamMember {
            x,
            z,
            role,
            state,
            facing,
        }
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100_000))]
    #[test]
    fn team_roundtrip(tick in any::<u16>(), ms in proptest::collection::vec(member(), 0..=49)) {
        let t = TeamState { tick, members: ms };
        let b = encode_team_state(&t).unwrap();
        prop_assert_eq!(decode_team_state(&b).unwrap(), t);
    }

    #[test]
    fn world_roundtrip(
        timer in any::<u16>(), o in proptest::array::uniform5((0u8..4, 0u8..128)),
        a in any::<u16>(), b in any::<u16>(), c in any::<u16>(), f in any::<u16>(),
    ) {
        let w = WorldState {
            timer,
            objectives: o.map(|(owner, capture)| ObjectiveState { owner, capture }),
            manpower: a, munitions: b, fuel: c, front_line_pos: f,
        };
        prop_assert_eq!(decode_world_state(&encode_world_state(&w).unwrap()).unwrap(), w);
    }
}

fn full(n: usize) -> Vec<TeamMember> {
    (0..n)
        .map(|i| TeamMember {
            x: 2047 - i as u16,
            z: i as u16,
            role: 15,
            state: 7,
            facing: 7,
        })
        .collect()
}

#[test]
fn team_state_49_players_size() {
    let (t, dropped) = TeamState::from_members(9, &full(49));
    assert_eq!(dropped, 0);
    let b = encode_team_state(&t).unwrap();
    println!("TeamState 49 players: {} B", b.len());
    assert!(b.len() <= 200);
}

#[test]
fn team_state_truncates_over_49() {
    let (t, dropped) = TeamState::from_members(1, &full(60));
    assert_eq!((t.members.len(), dropped), (49, 11));
    assert_eq!(t.members[48], full(60)[48]);
}

#[test]
fn team_state_errors() {
    let over = TeamState {
        tick: 0,
        members: full(50),
    };
    assert_eq!(encode_team_state(&over), Err(CodecError::TooLong));
    // n = 50 on the wire
    let mut b = vec![0, 0, 50 << 2];
    b.extend(std::iter::repeat_n(0, 200));
    assert_eq!(decode_team_state(&b), Err(CodecError::TooLong));
    // n = 3 but truncated
    assert!(decode_team_state(&[0, 0, 3 << 2, 0, 0]).is_err());
    let big = TeamMember {
        x: 2048,
        z: 0,
        role: 0,
        state: 0,
        facing: 0,
    };
    assert!(encode_team_state(&TeamState {
        tick: 0,
        members: vec![big]
    })
    .is_err());
    let bad = WorldState {
        timer: 0,
        objectives: [ObjectiveState {
            owner: 0,
            capture: 128,
        }; 5],
        manpower: 0,
        munitions: 0,
        fuel: 0,
        front_line_pos: 0,
    };
    assert!(encode_world_state(&bad).is_err());
    assert!(decode_world_state(&[0; 5]).is_err());
}

#[test]
fn world_state_size() {
    let w = WorldState {
        timer: 1,
        objectives: [ObjectiveState {
            owner: 1,
            capture: 50,
        }; 5],
        manpower: 1,
        munitions: 2,
        fuel: 3,
        front_line_pos: 4,
    };
    assert_eq!(encode_world_state(&w).unwrap().len(), 16);
}
