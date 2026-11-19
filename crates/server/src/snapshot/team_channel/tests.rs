use super::*;
use frontline_protocol::msg::team::{
    decode_team_state, decode_team_state_with_markers, TeamMarker, TeamMarkerKind as Kind,
    MAX_TEAM_MARKERS,
};
use frontline_sim::squad::{SquadId, SquadKind, SquadOp};
use frontline_sim::{CollisionData, GameMode, MapId, SimConfig};

const HZ: u32 = 30;
const MAP: MapFrame = MapFrame {
    origin_x: -100.0,
    origin_z: 200.0,
    extent_m: 2048.0,
};

fn world() -> World {
    World::new(
        SimConfig {
            tick_hz: HZ as u8,
            match_seed: 7,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        &CollisionData::empty(),
    )
}

fn channel(map: MapFrame) -> TeamChannel {
    // A one-tick emission period lets tests inspect the exact expiration boundary.
    TeamChannel::new(1.0 / HZ as f32, HZ as f32, map)
}

fn create(world: &mut World, player: PlayerId, members: &[PlayerId]) -> SquadId {
    world
        .squad_op(
            player,
            SquadOp::Create {
                kind: SquadKind::Infantry,
                suffix: String::new(),
            },
        )
        .unwrap();
    let id = world.squads().squad_of(player).unwrap().id;
    for &member in members {
        world.squad_op(member, SquadOp::Join { squad: id }).unwrap();
    }
    id
}

fn request(kind: Kind) -> MarkerPlace {
    MarkerPlace {
        kind: kind as u8,
        pos: [32_768, u16::MAX],
        ttl: 255,
    }
}

fn visible(
    channel: &mut TeamChannel,
    world: &World,
    player: PlayerId,
    tick: u32,
) -> Vec<TeamMarker> {
    let team = world.squads().entry(player).unwrap().team;
    let payloads =
        channel.tick_with_markers(world, GameData::embedded(), &[(player, team, true)], tick);
    decode_team_state_with_markers(&payloads[0].1)
        .unwrap()
        .markers
}

#[test]
fn placement_checks_roster_squad_permission_and_kind() {
    let mut w = world();
    let mut ch = channel(MAP);
    let order = request(Kind::OrderAttack);
    assert_eq!(
        ch.place_marker(&w, 9, order, 0),
        Err(MarkerReject::NotOnRoster)
    );
    w.roster_add(1, Team::Us).unwrap();
    assert_eq!(
        ch.place_marker(&w, 1, order, 0),
        Err(MarkerReject::NeedSquad)
    );
    assert_eq!(
        ch.place_marker(&w, 1, request(Kind::Attention), 0),
        Err(MarkerReject::NeedSquad)
    );
    ch.place_marker(&w, 1, request(Kind::NeedMedic), 0).unwrap();
    w.roster_add(2, Team::Us).unwrap();
    create(&mut w, 1, &[2]);
    for kind in [1, 2, 255] {
        assert_eq!(
            ch.place_marker(&w, 1, MarkerPlace { kind, ..order }, 0),
            Err(MarkerReject::UnsupportedKind)
        );
    }
    assert_eq!(
        ch.place_marker(&w, 2, order, 0),
        Err(MarkerReject::NotSquadLeader)
    );
    ch.place_marker(&w, 1, order, 0).unwrap();
    w.set_roster_online(1, false);
    assert_eq!(
        ch.place_marker(&w, 1, order, 0),
        Err(MarkerReject::NotOnRoster)
    );
    assert!(visible(&mut ch, &w, 2, 0).is_empty());
}

#[test]
fn routes_to_current_squad_and_role_without_cross_team_fallback() {
    let mut w = world();
    for (p, team) in [
        (1, Team::Us),
        (10, Team::Ger),
        (2, Team::Us),
        (11, Team::Ger),
        (3, Team::Us),
        (12, Team::Ger),
        (4, Team::Us),
        (5, Team::Us),
    ] {
        w.roster_add(p, team).unwrap();
    }
    create(&mut w, 1, &[2]);
    create(&mut w, 3, &[4, 5]);
    create(&mut w, 10, &[11]);
    w.set_role(4, Some("medic")).unwrap();
    w.set_role(5, Some("support")).unwrap();
    w.set_role(11, Some("medic")).unwrap();
    let mut ch = channel(MAP);
    for (p, kind) in [
        (2, Kind::Attention),
        (2, Kind::NeedMedic),
        (2, Kind::NeedAmmo),
        (1, Kind::OrderDefend),
    ] {
        ch.place_marker(&w, p, request(kind), 0).unwrap();
    }
    let clients = [
        (1, Team::Us, true),
        (2, Team::Us, true),
        (3, Team::Us, true),
        (4, Team::Us, true),
        (5, Team::Us, true),
        (11, Team::Ger, true),
        (10, Team::Us, true),
    ];
    let data = GameData::embedded();
    let payloads = ch.tick_with_markers(&w, data, &clients, 0);
    let contents: Vec<_> = payloads
        .iter()
        .map(|(p, bytes)| {
            let decoded = decode_team_state_with_markers(bytes).unwrap();
            (
                *p,
                decoded.markers.iter().map(|m| m.kind).collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(
        contents,
        [
            (
                1,
                vec![
                    Kind::Attention,
                    Kind::NeedMedic,
                    Kind::NeedAmmo,
                    Kind::OrderDefend
                ]
            ),
            (2, vec![Kind::Attention, Kind::OrderDefend]),
            (3, vec![]),
            (4, vec![Kind::NeedMedic]),
            (5, vec![Kind::NeedAmmo]),
            (11, vec![]),
        ]
    );
    let mixed = ch.tick_with_markers(&w, data, &[(1, Team::Us, false), (2, Team::Us, true)], 0);
    assert_eq!(decode_team_state(&mixed[0].1).unwrap().members.len(), 5);
    assert_eq!(&mixed[..1], ch.tick(&w, data, &[(1, Team::Us)], 0));
    assert_eq!(&payloads[0].1[..mixed[0].1.len()], mixed[0].1);
    assert_eq!(mixed[1], payloads[1]);

    // Outgoing privileges are recomputed, not cached across role/squad changes.
    w.set_role(4, None).unwrap();
    assert!(visible(&mut ch, &w, 4, 1).is_empty());
    w.squad_op(2, SquadOp::Leave).unwrap();
    let marks = visible(&mut ch, &w, 1, 1);
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].kind, Kind::OrderDefend);
    w.squad_op(5, SquadOp::Leave).unwrap();
    let id = w.squads().squad_of(1).unwrap().id;
    w.squad_op(5, SquadOp::Join { squad: id }).unwrap();
    w.squad_op(1, SquadOp::Promote { target: 5 }).unwrap();
    w.squad_op(5, SquadOp::AcceptLead).unwrap();
    assert!(visible(&mut ch, &w, 5, 2).is_empty());
}

#[test]
fn enforces_quantization_server_expiry_rate_and_per_scope_caps() {
    let mut w = world();
    w.roster_add(1, Team::Us).unwrap();
    create(&mut w, 1, &[]);
    let mut ch = channel(MAP);
    let ping = request(Kind::Attention);
    ch.place_marker(&w, 1, ping, 0).unwrap();
    assert_eq!(
        visible(&mut ch, &w, 1, 0),
        [TeamMarker {
            kind: Kind::Attention,
            x: 204,
            z: 409,
            ttl_s: 8
        }]
    );
    assert_eq!(
        ch.place_marker(&w, 1, ping, 2 * HZ - 1),
        Err(MarkerReject::Cooldown)
    );
    for secs in [2, 4] {
        ch.place_marker(&w, 1, ping, secs * HZ).unwrap();
    }
    assert_eq!(
        ch.place_marker(&w, 1, ping, 6 * HZ),
        Err(MarkerReject::PlayerCap)
    );
    assert_eq!(visible(&mut ch, &w, 1, 8 * HZ - 1)[0].ttl_s, 1);
    assert_eq!(visible(&mut ch, &w, 1, 8 * HZ).len(), 2);
    assert!(visible(&mut ch, &w, 1, 12 * HZ).is_empty());

    let t = 12 * HZ;
    for (kind, ttl_s) in [(Kind::NeedMedic, 20), (Kind::OrderMove, 180)] {
        ch.place_marker(&w, 1, request(kind), t).unwrap();
        ch.place_marker(
            &w,
            1,
            MarkerPlace {
                pos: [0, 0],
                ttl: 0,
                ..request(kind)
            },
            t + 1,
        )
        .unwrap();
        let marks = visible(&mut ch, &w, 1, t + 1);
        let m = marks
            .iter()
            .filter(|m| m.kind == kind)
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(
            m,
            [TeamMarker {
                kind,
                x: 0,
                z: 0,
                ttl_s
            }]
        );
    }
    assert_eq!(visible(&mut ch, &w, 1, t + 20 * HZ + 1).len(), 1);
    assert!(visible(&mut ch, &w, 1, t + 180 * HZ + 1).is_empty());
}

#[test]
fn roster_slot_forget_and_tick_wrap_bound_marker_lifetime() {
    let mut w = world();
    w.roster_add(1, Team::Us).unwrap();
    w.roster_add(2, Team::Us).unwrap();
    let id = create(&mut w, 2, &[1]);
    let mut ch = channel(MAP);
    let ping = request(Kind::Attention);
    let start = u32::MAX - HZ;
    ch.place_marker(&w, 1, ping, start).unwrap();
    ch.place_marker(&w, 2, ping, start).unwrap();
    ch.place_marker(&w, 1, request(Kind::NeedMedic), start)
        .unwrap();
    assert_eq!(visible(&mut ch, &w, 2, 0).len(), 3);
    ch.forget(1);
    w.roster_remove(1);
    w.roster_add(1, Team::Us).unwrap();
    w.squad_op(1, SquadOp::Join { squad: id }).unwrap();
    assert_eq!(visible(&mut ch, &w, 2, 0).len(), 1);
    ch.place_marker(&w, 1, ping, 0).unwrap();
    assert_eq!(visible(&mut ch, &w, 2, start.wrapping_add(8 * HZ)).len(), 1);
    assert!(visible(&mut ch, &w, 2, 8 * HZ).is_empty());
}

#[test]
fn rejects_invalid_map_and_bounds_storage_per_team() {
    let mut w = world();
    for p in 0..=32 {
        w.roster_add(p, Team::Us).unwrap();
        w.roster_add(p + 100, Team::Ger).unwrap();
    }
    for extent_m in [0.0, -5.0, f32::NAN, 20_480.0] {
        let mut ch = channel(MapFrame { extent_m, ..MAP });
        assert_eq!(
            ch.place_marker(&w, 0, request(Kind::NeedAmmo), 0),
            Err(MarkerReject::InvalidMap)
        );
    }
    let mut ch = channel(MAP);
    for p in 0..(MAX_TEAM_MARKERS as u8 / 2) {
        ch.place_marker(&w, p, request(Kind::NeedAmmo), 0).unwrap();
        ch.place_marker(&w, p, request(Kind::NeedMedic), 0).unwrap();
    }
    assert_eq!(
        ch.place_marker(&w, 32, request(Kind::NeedAmmo), 0),
        Err(MarkerReject::TeamCap)
    );
    ch.place_marker(&w, 0, request(Kind::NeedMedic), 1).unwrap(); // Existing slot is replaceable.
    ch.place_marker(&w, 100, request(Kind::NeedMedic), 1)
        .unwrap(); // Other team's cap is independent.
    ch.place_marker(&w, 32, request(Kind::NeedAmmo), 20 * HZ)
        .unwrap(); // Expirations free slots.
}
