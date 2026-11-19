//! M1-041c/d: vehicle wire entities and the receiving player's seat in snapshots.

mod common;

use common::sim_harness::{gen_cmd, terrain};
use common::synth::*;
use frontline_protocol::msg::input::InputCmd;
use frontline_protocol::snapshot::{EntityKind, QPos};
use frontline_server::snapshot::LinkInfo;
use frontline_sim::squad::{SquadKind, SquadOp};
use frontline_sim::{Team, Vec3};

#[test]
fn a_nearby_tank_reaches_the_client_as_a_vehicle_entity() {
    let Some(map) = terrain() else { return };
    let mut s = Synth::new(new_world(Some(&map)));
    s.add(
        0,
        Team::Us,
        Vec3::new(120.0, 0.0, 120.0),
        0.0,
        Some(LinkInfo::DATAGRAM),
    );
    let id = s
        .world
        .spawn_vehicle("us_m4a1_75", Team::Us, Vec3::new(130.0, 0.0, 130.0), 0.0)
        .expect("spawn tank");
    for _ in 0..90 {
        s.step(gen_cmd);
    }
    let known = &s.client(0).known;
    let tanks: Vec<_> = known
        .values()
        .filter(|e| e.kind == EntityKind::Vehicle)
        .collect();
    assert_eq!(tanks.len(), 1, "one vehicle entity, got {known:?}");
    let t = tanks[0];
    assert!((128..=255).contains(&t.id), "vehicle wire id {}", t.id);
    let x = frontline_protocol::quant::dequant_pos_xz(t.pos.x);
    assert!((x - 130.0).abs() < 1.0, "x {x}");
    assert!(t.vehicle.is_some() && t.vel.is_some());
    assert!(t.health_q > 200, "full hull health, got {}", t.health_q);
    let _ = id;
}

#[test]
fn self_state_identifies_the_occupied_vehicle_and_clears_after_exit() {
    let mut s = Synth::new(new_world(None));
    s.add(
        0,
        Team::Us,
        Vec3::new(32.0, 0.0, 32.0),
        0.0,
        Some(LinkInfo::DATAGRAM),
    );
    s.world.roster_add(0, Team::Us).unwrap();
    s.world.roster_add(100, Team::Ger).unwrap();
    s.world
        .squad_op(
            0,
            SquadOp::Create {
                kind: SquadKind::ArmorCrew,
                suffix: String::new(),
            },
        )
        .unwrap();
    s.world.set_role(0, Some("crewman")).unwrap();

    s.world
        .spawn_vehicle("us_m4a1_75", Team::Us, Vec3::new(52.0, 0.0, 32.0), 0.0)
        .unwrap();
    let vehicle = s
        .world
        .spawn_vehicle("us_m4a1_75", Team::Us, Vec3::new(36.0, 0.0, 32.0), 0.0)
        .unwrap();
    s.world.seat_enter(0, vehicle, 2).unwrap();

    let idle = |player, tick| InputCmd {
        move_x: 0,
        move_z: 0,
        buttons: 0,
        ..gen_cmd(player, tick)
    };
    s.step(idle);
    let packet = s.client(0).last.as_ref().expect("first snapshot");
    let entities = packet.entities.to_states();
    let mut tanks = entities.iter().filter(|e| e.kind == EntityKind::Vehicle);
    assert_eq!(tanks.clone().count(), 2, "both tanks in the first snapshot");
    let occupied = tanks
        .find(|e| e.pos.x == QPos::from_meters(36.0, 0.0, 32.0).x)
        .expect("boarded tank in the first snapshot");
    assert!((128..=255).contains(&occupied.id));
    let self_state = packet.self_state.expect("boarded self_state");
    assert_eq!((self_state.vehicle_slot, self_state.seat), (occupied.id, 2));

    s.world.seat_exit_begin(0).unwrap();
    let exit_tick = s.tick + 24; // 0.8 s at 30 Hz.
    for _ in 0..26 {
        s.step(idle);
    }
    assert_eq!(s.world.seat_of(0), None);
    let packet = s.client(0).last.as_ref().expect("snapshot after exit");
    assert!(packet.tick >= exit_tick, "expected a post-exit snapshot");
    let self_state = packet.self_state.expect("on-foot self_state");
    assert_eq!((self_state.vehicle_slot, self_state.seat), (0, 0));
}
