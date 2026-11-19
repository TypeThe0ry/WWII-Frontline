mod common;

use common::{cmd, duel_world, synth_data};
use frontline_protocol::msg::input::buttons;
use frontline_sim::spawn::garrison::PlaceError;
use frontline_sim::supply::{
    ammo::AmmoPlaceError, system::NOTICE_IN_NO_BUILD_ZONE, SupplyPlaceError,
};
use frontline_sim::{GameData, ObjectiveSpec, SimEvent, Team, Vec3, World};

const INSIDE: Vec3 = Vec3::new(24.0, 5.0, 24.0);
const EDGE: Vec3 = Vec3::new(32.0, 5.0, 24.0);
const OUTSIDE: Vec3 = Vec3::new(48.0, 5.0, 24.0);

fn setup() -> World {
    let mut w = duel_world(
        GameData::embedded(),
        &synth_data(1, 1, &|_, _| 5.0),
        5,
        &[(0, Team::Us, OUTSIDE.x, OUTSIDE.z, 0)],
    );
    w.set_no_build_zones(vec![vec![
        [16.0, 16.0],
        [32.0, 16.0],
        [32.0, 32.0],
        [16.0, 32.0],
    ]])
    .unwrap();
    w
}

#[test]
fn garrison_zone_rejection_does_not_consume_supplies() {
    let mut w = setup();
    w.set_officer(0, true);
    w.debug_add_crate(Team::Us, OUTSIDE, 50);
    assert_eq!(w.place_garrison(0, INSIDE), Err(PlaceError::InNoBuildZone));
    assert_eq!(w.place_garrison(0, EDGE), Err(PlaceError::InNoBuildZone));
    assert_eq!(w.supply_crates()[0].supplies, 50);
    assert!(w.garrison_list().is_empty());
    assert!(w.place_garrison(0, OUTSIDE).is_ok());
}

#[test]
fn ammo_crate_zone_rejection_does_not_start_cooldown_or_consume_a_use() {
    let mut w = setup();
    w.set_support(0, true);
    w.debug_teleport(0, INSIDE);
    assert_eq!(w.place_ammo_crate(0), Err(AmmoPlaceError::InNoBuildZone));
    assert!(w.ammo_crates().is_empty());
    assert!(w.supply_world().ammo_count.is_empty());
    assert!(w.supply_world().ammo_last.is_empty());
    w.debug_teleport(0, OUTSIDE);
    assert!(w.place_ammo_crate(0).is_ok());
    assert_eq!(w.ammo_crates()[0].pos, OUTSIDE);
}

#[test]
fn direct_and_hold_supply_drops_share_map_restrictions_and_keep_carried_crate_on_rejection() {
    let mut w = setup();
    assert_eq!(w.drop_supply_crate(1), Err(SupplyPlaceError::NotAlive));
    assert_eq!(w.drop_supply_crate(0), Err(SupplyPlaceError::NotSupport));
    w.set_support(0, true);
    assert_eq!(w.drop_supply_crate(0), Err(SupplyPlaceError::NotCarrying));
    w.add_depot(Team::Us, OUTSIDE);
    w.debug_add_depot_stock(0, 1);
    let use_btn = cmd(0, 0, 0, buttons::USE, 0);
    for _ in 0..45 {
        w.step(&[(0, use_btn)]);
    }
    let id = w.carrying(0).expect("picked up at the depot");
    w.step(&[(0, cmd(0, 0, 0, 0, 0))]);

    w.debug_teleport(0, EDGE);
    assert_eq!(w.drop_supply_crate(0), Err(SupplyPlaceError::InNoBuildZone));
    let mut rejected = false;
    for _ in 0..45 {
        let events = w.step(&[(0, use_btn)]).events;
        assert!(!events
            .iter()
            .any(|e| matches!(e, SimEvent::CratePlaced { .. })));
        rejected |= events.iter().any(|e| {
            matches!(
                e,
                SimEvent::SupplyNotice {
                    player: 0,
                    code: NOTICE_IN_NO_BUILD_ZONE
                }
            )
        });
    }
    assert!(rejected);
    assert_eq!(w.carrying(0), Some(id));
    assert_eq!(w.supply_crates()[0].carried_by, Some(0));

    w.debug_teleport(0, OUTSIDE);
    w.set_objectives(&[ObjectiveSpec {
        id: 1,
        pos: OUTSIDE,
        radius_m: 4.0,
        owner: Some(Team::Ger),
    }]);
    assert_eq!(
        w.drop_supply_crate(0),
        Err(SupplyPlaceError::TooCloseObjective)
    );
    w.set_objectives(&[]);
    assert_eq!(w.drop_supply_crate(0), Ok(id));
    assert_eq!(w.carrying(0), None);
    let crate_on_ground = &w.supply_crates()[0];
    assert_eq!(crate_on_ground.carried_by, None);
    assert_eq!(crate_on_ground.pos, OUTSIDE);
    let events = w.step(&[]).events;
    assert_eq!(events.iter().filter(|e| matches!(e, SimEvent::CratePlaced { id: placed, team: Team::Us, pos } if *placed == id && *pos == OUTSIDE)).count(), 1);
}
