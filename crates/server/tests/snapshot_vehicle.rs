//! M1-041c: vehicles travel as wire entities 128..=255 (kind Vehicle) in snapshots.

mod common;

use common::sim_harness::{gen_cmd, terrain};
use common::synth::*;
use frontline_protocol::snapshot::EntityKind;
use frontline_server::snapshot::LinkInfo;
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
