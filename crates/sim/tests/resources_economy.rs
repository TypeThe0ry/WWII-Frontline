use frontline_sim::rules::resource_node::{
    NodeDamageKind, NodeDestroyReason, NodeSpawnError, ResourceSite,
};
use frontline_sim::rules::resources::{
    Insufficient, ResourceKind, ResourceReason, Resources, UpkeepClass,
};
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, ObjectiveSpec, SimConfig, SimEvent,
    SpawnReq, StepOutput, Team, Vec3, World,
};
use proptest::prelude::*;

const MINUTE: u32 = 1800;
const US: Team = Team::Us;
const GER: Team = Team::Ger;
const M: ResourceKind = ResourceKind::Manpower;
const U: ResourceKind = ResourceKind::Munitions;
const F: ResourceKind = ResourceKind::Fuel;

fn world(mode: GameMode, owners: &[Option<Team>]) -> World {
    let collision = CollisionData::from_chunks(vec![CollisionChunk::new(
        0,
        0,
        vec![0.0; 65 * 65],
        vec![0; 64 * 64],
    )]);
    let mut world = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 9,
            map: MapId(0),
            mode,
            data: GameData::embedded(),
        },
        &collision,
    );
    world.set_objectives(
        &owners
            .iter()
            .enumerate()
            .map(|(i, &owner)| ObjectiveSpec {
                id: i as u16,
                pos: Vec3::new(10.0 + i as f32 * 8.0, 0.0, 50.0),
                radius_m: 4.0,
                owner,
            })
            .collect::<Vec<_>>(),
    );
    world
}

fn frontline() -> World {
    world(
        GameMode::Frontline,
        &[Some(US), Some(US), None, Some(GER), Some(GER)],
    )
}

fn steps(w: &mut World, n: u32) -> StepOutput {
    for _ in 1..n {
        w.step(&[]);
    }
    w.step(&[])
}

fn site(w: &mut World, id: u16, sector: u16, kind: ResourceKind) {
    assert!(w.install_resource_site(ResourceSite {
        id,
        sector,
        kind,
        pos: Vec3::new(20.0, 0.0, 20.0)
    }));
}

fn pool(w: &World, team: Team) -> Resources {
    w.view().team_resources(team)
}

fn spawn(w: &mut World, player: u8, team: Team, pos: Vec3) {
    w.spawn_player(SpawnReq {
        player,
        team,
        pos,
        yaw: 0.0,
    })
    .unwrap();
}

#[test]
fn resources_first_minute() {
    let mut w = frontline();
    assert_eq!(pool(&w, US), Resources::new(200, 80, 40));
    assert_eq!(steps(&mut w, MINUTE - 1).resource_history, None);
    assert_eq!(pool(&w, US), Resources::new(200, 80, 40));
    let out = w.step(&[]);
    assert_eq!(out.resource_history, Some([Resources::new(242, 98, 50); 2]));
    assert_eq!(
        out.events
            .iter()
            .filter(|e| matches!(
                e,
                SimEvent::ResourceDelta {
                    reason: ResourceReason::Income,
                    ..
                }
            ))
            .count(),
        6
    );
    assert_eq!(w.step(&[]).resource_history, None);
}

#[test]
fn both_teams_node_income() {
    let mut w = frontline();
    for (team, first) in [(US, 0), (GER, 3)] {
        let entries = [(first, M), (first + 1, M), (first, U), (first, F)];
        for (i, (sector, kind)) in entries.into_iter().enumerate() {
            let id = first * 4 + i as u16;
            site(&mut w, id, sector, kind);
            w.spawn_node(id, kind, team, &[2, 1, 2]).unwrap();
        }
        assert_eq!(w.income_per_min(team), Resources::new(92, 28, 16));
    }
    assert_eq!(w.resource_nodes().next().unwrap().builders, [1, 2]);
    assert_eq!(
        steps(&mut w, MINUTE).resource_history,
        Some([Resources::new(292, 108, 56); 2])
    );
}

#[test]
fn caps_and_overflow() {
    let mut w = frontline();
    let caps = Resources::new(1500, 800, 400);
    for team in [US, GER] {
        w.refund(team, Resources::new(1300, 720, 360), 1.0);
    }
    assert_eq!(steps(&mut w, MINUTE).resource_overflow, [84, 36, 20]);
    assert_eq!(pool(&w, US), caps);
    assert_eq!(pool(&w, GER), caps);
    assert_eq!(w.step(&[]).resource_overflow, [0; 3]);
    w.refund(US, Resources::new(i32::MAX, 0, 0), 1.0);
    assert_eq!(w.step(&[]).resource_overflow, [i32::MAX as u64, 0, 0]);
}

#[test]
fn node_limits_and_team_ownership() {
    let mut w = world(GameMode::Frontline, &[Some(US); 4]);
    for sector in 0..4 {
        for kind in ResourceKind::ALL {
            site(&mut w, 3 * sector + kind as u16, sector, kind);
        }
    }
    site(&mut w, 20, 0, M);
    assert_eq!(
        w.spawn_node(0, M, GER, &[]),
        Err(NodeSpawnError::WrongSectorOwner)
    );
    assert_eq!(w.spawn_node(0, U, US, &[]), Err(NodeSpawnError::WrongKind));
    w.spawn_node(0, M, US, &[1]).unwrap();
    assert_eq!(w.spawn_node(0, M, US, &[]), Err(NodeSpawnError::Occupied));
    assert_eq!(
        w.spawn_node(20, M, US, &[]),
        Err(NodeSpawnError::SectorKindLimit)
    );
    for sector in 1..3 {
        w.spawn_node(3 * sector, M, US, &[]).unwrap();
    }
    assert_eq!(w.spawn_node(9, M, US, &[]), Err(NodeSpawnError::KindLimit));
    for sector in 0..3 {
        for kind in [U, F] {
            w.spawn_node(3 * sector + kind as u16, kind, US, &[])
                .unwrap();
        }
    }
    assert_eq!(w.resource_nodes().count(), 9);
    assert_eq!(w.spawn_node(9, M, US, &[]), Err(NodeSpawnError::TeamLimit));
}

#[cfg(feature = "test-util")]
#[test]
fn sector_capture_removes_all_nodes_before_payout_and_awards_only_manpower() {
    let mut w = frontline();
    for kind in ResourceKind::ALL {
        site(&mut w, kind as u16, 0, kind);
        w.spawn_node(kind as u16, kind, US, &[1]).unwrap();
    }
    steps(&mut w, MINUTE - 1);
    w.debug_set_objective(0, -99.999, Some(US), Some(US));
    spawn(&mut w, 1, GER, Vec3::new(10.0, 0.01, 50.0));
    let out = w.step(&[]);
    assert!(out.rule_changes.iter().any(|ev| matches!(
        ev,
        frontline_sim::RuleChange::ObjectiveOwner {
            objective: 0,
            owner: GER,
            ..
        }
    )));
    assert_eq!(w.resource_nodes().count(), 0);
    assert_eq!(
        out.events
            .iter()
            .filter(|e| matches!(
                e,
                SimEvent::ResourceNodeDestroyed {
                    reason: NodeDestroyReason::SectorFlip(GER),
                    ..
                }
            ))
            .count(),
        3
    );
    assert_eq!(
        out.events
            .iter()
            .filter(|e| matches!(
                e,
                SimEvent::ResourceDelta {
                    team: GER,
                    kind: M,
                    value: 50,
                    reason: ResourceReason::Spoils
                }
            ))
            .count(),
        1
    );
    assert_eq!(
        out.resource_history,
        Some([Resources::new(236, 95, 48), Resources::new(298, 101, 52)])
    );
    assert!(w.spawn_node(0, M, GER, &[1]).is_ok());
}

#[test]
fn overrun_resets_and_destroys_on_the_900th_consecutive_tick() {
    let mut w = frontline();
    site(&mut w, 0, 0, M);
    spawn(&mut w, 1, GER, Vec3::new(20.0, 0.01, 21.0));
    assert_eq!(
        w.check_resource_node_placement(0, M, US),
        Err(NodeSpawnError::HostileNearby)
    );
    let id = w.spawn_node(0, M, US, &[2]).unwrap();
    steps(&mut w, 899);
    assert_eq!(w.resource_nodes().next().unwrap().overrun_ticks, 899);
    spawn(&mut w, 2, US, Vec3::new(20.0, 0.01, 22.0));
    w.step(&[]);
    assert_eq!(w.resource_nodes().next().unwrap().overrun_ticks, 0);
    w.remove_player(2);
    steps(&mut w, 899);
    assert_eq!(w.resource_nodes().next().unwrap().overrun_ticks, 899);
    let out = w.step(&[]);
    assert_eq!(w.resource_nodes().count(), 0);
    assert!(out.events.contains(&SimEvent::ResourceNodeDestroyed {
        id,
        reason: NodeDestroyReason::Overrun
    }));
    assert_eq!(pool(&w, GER).manpower, 242);
    assert!(w.spawn_node(0, M, US, &[]).is_ok());
}

#[test]
fn demolition_refunds_only_supplies_and_damage_never_refunds() {
    let mut w = frontline();
    site(&mut w, 0, 0, M);
    let id = w.spawn_node(0, M, US, &[]).unwrap();
    assert!(w.destroy_node(id, NodeDestroyReason::Demolished));
    assert!(!w.destroy_node(id, NodeDestroyReason::Demolished));
    let crates = w.supply_crates();
    assert_eq!(crates.len(), 1);
    assert_eq!((crates[0].team, crates[0].supplies), (US, 125));
    assert_eq!(pool(&w, US), Resources::new(200, 80, 40));
    let id2 = w.spawn_node(0, M, US, &[]).unwrap();
    assert_ne!(id, id2);
    assert_eq!(
        w.damage_resource_node(id2, 100.0, NodeDamageKind::Bullet),
        Some(1498.0)
    );
    assert_eq!(
        w.damage_resource_node(id2, 2500.0, NodeDamageKind::Explosive),
        Some(0.0)
    );
    assert_eq!(w.resource_nodes().count(), 0);
    assert_eq!(w.supply_crates().len(), 1);
}

#[test]
fn spend_is_atomic_and_refund_floors_in_input_order() {
    let mut w = frontline();
    let before = pool(&w, US);
    assert_eq!(
        w.try_spend(US, Resources::new(1, 1, 41)),
        Err(Insufficient::Resource(F))
    );
    assert_eq!(
        w.try_spend(US, Resources::new(-1, 0, 0)),
        Err(Insufficient::InvalidCost)
    );
    w.refund(US, Resources::new(9, 0, -1), 0.5);
    w.refund(US, Resources::new(9, 0, 0), f64::NAN);
    assert_eq!(pool(&w, US), before);
    let cost = Resources::new(151, 11, 31);
    w.try_spend(US, cost).unwrap();
    assert_eq!(
        w.try_spend(US, Resources::new(50, 0, 0)),
        Err(Insufficient::Resource(M))
    );
    w.refund(US, cost, 0.5);
    assert_eq!(pool(&w, US), Resources::new(124, 74, 24));
    let out = w.step(&[]);
    let deltas: Vec<_> = out
        .events
        .iter()
        .filter_map(|e| match e {
            SimEvent::ResourceDelta {
                kind,
                value,
                reason,
                ..
            } => Some((*kind, *value, *reason)),
            _ => None,
        })
        .collect();
    assert_eq!(
        deltas,
        [
            (M, -151, ResourceReason::Spend),
            (U, -11, ResourceReason::Spend),
            (F, -31, ResourceReason::Spend),
            (M, 75, ResourceReason::Refund),
            (U, 5, ResourceReason::Refund),
            (F, 15, ResourceReason::Refund)
        ]
    );
}

#[test]
fn upkeep_consumes_six_and_low_fuel_can_recover() {
    let mut w = frontline();
    for (id, class) in [
        (1, UpkeepClass::Light),
        (2, UpkeepClass::Medium),
        (3, UpkeepClass::Heavy),
    ] {
        w.register_upkeep(id, US, class);
    }
    let out = steps(&mut w, MINUTE);
    assert!(out.events.contains(&SimEvent::ResourceDelta {
        team: US,
        kind: F,
        value: -6,
        reason: ResourceReason::Upkeep
    }));
    assert_eq!(pool(&w, US).fuel, 44);
    assert_eq!(pool(&w, GER).fuel, 50);
    for id in 4..=6 {
        w.register_upkeep(id, US, UpkeepClass::Heavy);
    }
    w.try_spend(US, Resources::new(0, 0, 43)).unwrap();
    assert!(!w.low_fuel(US));
    steps(&mut w, MINUTE);
    assert_eq!(pool(&w, US).fuel, 0);
    assert!(w.low_fuel(US));
    for id in 4..=6 {
        w.unregister_upkeep(id);
    }
    steps(&mut w, MINUTE);
    assert_eq!(pool(&w, US).fuel, 4);
    assert!(!w.low_fuel(US));
}

#[test]
fn disabled_modes_keep_economy_inert() {
    for mode in [GameMode::Sandbox, GameMode::Skirmish] {
        let mut w = world(mode, &[Some(US)]);
        assert_eq!(
            w.try_spend(US, Resources::default()),
            Err(Insufficient::Disabled)
        );
        w.refund(US, Resources::new(1, 1, 1), 1.0);
        w.register_upkeep(1, US, UpkeepClass::Heavy);
        assert_eq!(w.spawn_node(0, M, US, &[]), Err(NodeSpawnError::Disabled));
        assert_eq!(steps(&mut w, MINUTE).resource_history, None);
        assert_eq!(pool(&w, US), Resources::default());
        assert_eq!(w.income_per_min(US), Resources::default());
        assert!(!w.low_fuel(US));
    }
    assert_eq!(
        pool(&world(GameMode::Assault, &[]), US),
        Resources::new(200, 80, 40)
    );
}

#[test]
fn resource_pools_change_state_hash() {
    let mut a = frontline();
    let mut b = frontline();
    assert_eq!(a.state_hash(), b.state_hash());
    let cost = Resources::new(1, 2, 3);
    a.try_spend(US, cost).unwrap();
    assert_ne!(a.state_hash(), b.state_hash());
    b.try_spend(US, cost).unwrap();
    assert_eq!(a.state_hash(), b.state_hash());
    steps(&mut a, MINUTE);
    steps(&mut b, MINUTE);
    assert_eq!(a.state_hash(), b.state_hash());
    // Snapshot for the enabled economy at minute one (no players).
    assert_eq!(a.state_hash(), 0x9fe5_8dd0_8d95_5880);
}

proptest! {
    #[test]
    fn arbitrary_spending_and_refunds_stay_bounded(actions in prop::collection::vec((any::<bool>(), -50i32..1600, -50i32..900, -50i32..450, 0u8..=100), 0..100)) {
        let mut w = frontline();
        for (refund, m, u, f, frac) in actions {
            let before = pool(&w, US);
            let cost = Resources::new(m, u, f);
            if refund { w.refund(US, cost, f64::from(frac) / 100.0); }
            else if w.try_spend(US, cost).is_err() { prop_assert_eq!(pool(&w, US), before); }
            let next = pool(&w, US);
            prop_assert!((0..=1500).contains(&next.manpower));
            prop_assert!((0..=800).contains(&next.munitions));
            prop_assert!((0..=400).contains(&next.fuel));
        }
    }
}
