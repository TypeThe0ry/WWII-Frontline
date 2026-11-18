//! M1-020a: Supplies: Depot / point stores, carrying, crate limits, the SupplyProvider.
mod common;

use common::*;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::spawn::garrison::PlaceError;
use frontline_sim::supply::CrateDamage;
use frontline_sim::{ObjectiveSpec, SimEvent, Team, World};
use glam::Vec3;

const HZ: u32 = 30;

fn idle() -> InputCmd {
    cmd(0, 0, 49152, 0, 0)
}

fn use_btn() -> InputCmd {
    cmd(0, 0, 49152, buttons::USE, 0)
}

/// Player 0: US Support / Officer at (300, 60); 1: German far away; 2: US at (300, 70).
fn world() -> World {
    let players = [
        (0u8, Team::Us, 300.0, 60.0, 16384u16),
        (1, Team::Ger, 1800.0, 60.0, 16384),
        (2, Team::Us, 300.0, 70.0, 16384),
    ];
    let mut w = duel_world(
        data_with(|_| {}),
        &synth_data(30, 6, &|_, _| 5.0),
        5,
        &players,
    );
    w.set_hq(Team::Us, Vec3::new(60.0, 5.0, 60.0));
    w.set_hq(Team::Ger, Vec3::new(1860.0, 5.0, 60.0));
    w.set_support(0, true);
    w.set_officer(0, true);
    w
}

fn step_with(w: &mut World, special: &[(u8, InputCmd)]) -> Vec<SimEvent> {
    let ins: Vec<_> = w
        .view()
        .players()
        .iter()
        .map(|p| {
            (
                p.player,
                special
                    .iter()
                    .find(|(i, _)| *i == p.player)
                    .map_or_else(idle, |(_, c)| *c),
            )
        })
        .collect();
    w.step(&ins).events
}

fn step(w: &mut World) {
    step_with(w, &[]);
}

fn stock(w: &World) -> u32 {
    w.supply_world().depots[0].stock
}

#[test]
fn depot_makes_a_crate_every_45_s_until_eight_then_resumes() {
    let mut w = world();
    w.add_depot(Team::Us, Vec3::new(300.0, 5.0, 60.0));
    let mut first = None;
    for t in 1..=(8 * 45 * HZ + 100) {
        step(&mut w);
        if first.is_none() && stock(&w) == 1 {
            first = Some(t);
        }
        if t == 8 * 45 * HZ + 50 {
            break;
        }
    }
    let f = first.unwrap();
    assert!((f as i32 - 1350).abs() <= 1, "first crate at {f}");
    assert_eq!(stock(&w), 8, "capped at 8");
    for _ in 0..(60 * HZ) {
        step(&mut w);
    }
    assert_eq!(stock(&w), 8, "does not overflow");
    // Take one: generation resumes and refills after 45 s.
    for _ in 0..(1 + 45 * HZ) {
        step_with(&mut w, &[(0, use_btn())]);
        if w.carrying(0).is_some() {
            break;
        }
    }
    assert_eq!(stock(&w), 7);
    for _ in 0..(46 * HZ) {
        step(&mut w);
    }
    assert_eq!(stock(&w), 8);
}

#[test]
fn point_store_90_s_cap_4_and_emptied_on_capture() {
    let mut w = world();
    w.set_objectives(&[ObjectiveSpec {
        id: 3,
        pos: Vec3::new(800.0, 5.0, 60.0),
        radius_m: 45.0,
        owner: Some(Team::Us),
    }]);
    w.add_supply_point(3, Vec3::new(790.0, 5.0, 60.0));
    let st = |w: &World| w.supply_world().points[0].stock;
    let mut first = None;
    for t in 1..=(4 * 90 * HZ + 10) {
        step(&mut w);
        if first.is_none() && st(&w) == 1 {
            first = Some(t);
        }
    }
    assert!((first.unwrap() as i32 - 2700).abs() <= 1);
    assert_eq!(st(&w), 4);
    w.debug_set_objective(3, -100.0, Some(Team::Ger), Some(Team::Us));
    step(&mut w);
    assert_eq!(st(&w), 0, "emptied when the point changes hands");
}

#[test]
fn carrying_slows_to_75_percent_and_putting_down_takes_1_5_s() {
    let mut w = world();
    w.add_depot(Team::Us, Vec3::new(300.0, 5.0, 60.0));
    w.debug_add_depot_stock(0, 1);
    // Pick up.
    let mut picked = None;
    for t in 1..=(2 * HZ) {
        step_with(&mut w, &[(0, use_btn())]);
        if w.carrying(0).is_some() {
            picked = Some(t);
            break;
        }
    }
    assert_eq!(picked, Some(45), "1.5 s to pick up");
    // Walk 60 ticks with and without the crate.
    let walk = |w: &mut World| {
        let a = w.player_state(0).unwrap().pos;
        for _ in 0..60 {
            step_with(w, &[(0, cmd(0, 100, 16384, 0, 0))]);
        }
        let b = w.player_state(0).unwrap().pos;
        ((b.x - a.x).powi(2) + (b.z - a.z).powi(2)).sqrt()
    };
    let loaded = walk(&mut w);
    // Put down 1.5 s later.
    let mut placed = None;
    for t in 1..=(2 * HZ) {
        let ev = step_with(&mut w, &[(0, use_btn())]);
        if ev.iter().any(|e| matches!(e, SimEvent::CratePlaced { .. })) {
            placed = Some(t);
            break;
        }
    }
    assert_eq!(placed, Some(45));
    assert!(w.carrying(0).is_none());
    assert_eq!(w.supply_crates().len(), 1);
    let free = walk(&mut w);
    let ratio = loaded / free;
    assert!((ratio - 0.75).abs() < 0.01, "ratio {ratio}");
}

#[test]
fn no_crates_within_80_m_of_an_enemy_point() {
    for (d, ok) in [(79.0f32, false), (81.0, true)] {
        let mut w = world();
        w.set_objectives(&[ObjectiveSpec {
            id: 1,
            pos: Vec3::new(600.0, 5.0, 60.0),
            radius_m: 45.0,
            owner: Some(Team::Ger),
        }]);
        w.add_depot(Team::Us, Vec3::new(300.0, 5.0, 60.0));
        w.debug_add_depot_stock(0, 1);
        for _ in 0..45 {
            step_with(&mut w, &[(0, use_btn())]);
        }
        assert!(w.carrying(0).is_some());
        step(&mut w); // release USE
        w.debug_teleport(0, Vec3::new(600.0 - d, 5.02, 60.0));
        let mut placed = false;
        let mut noticed = false;
        for _ in 0..50 {
            for e in step_with(&mut w, &[(0, use_btn())]) {
                placed |= matches!(e, SimEvent::CratePlaced { .. });
                noticed |= matches!(e, SimEvent::SupplyNotice { .. });
            }
        }
        assert_eq!(placed, ok, "{d} m");
        assert_eq!(noticed, !ok, "{d} m");
    }
}

#[test]
fn forty_first_crate_removes_the_earliest() {
    let mut w = world();
    let ids: Vec<u32> = (0..41)
        .map(|i| w.debug_add_crate(Team::Us, Vec3::new(400.0 + i as f32, 5.0, 60.0), 100))
        .collect();
    step(&mut w);
    let left: Vec<u32> = w.supply_crates().iter().map(|c| c.id).collect();
    assert_eq!(left.len(), 40);
    assert!(!left.contains(&ids[0]));
    assert!(left.contains(&ids[40]));
}

#[test]
fn provider_takes_from_the_nearest_crate_first() {
    // Friendly area: cost 50 from the nearest of 100 (5 m) and 60 (10 m).
    let mut w = world();
    let near = w.debug_add_crate(Team::Us, Vec3::new(305.0, 5.0, 60.0), 100);
    let far = w.debug_add_crate(Team::Us, Vec3::new(310.0, 5.0, 60.0), 60);
    w.place_garrison(0, Vec3::new(500.0, 5.0, 60.0)).unwrap();
    let c = w.supply_crates();
    assert_eq!(c.iter().find(|c| c.id == near).unwrap().supplies, 50);
    assert_eq!(c.iter().find(|c| c.id == far).unwrap().supplies, 60);
    // Enemy area: cost 100 empties the nearest crate only.
    let mut w = world();
    w.set_objectives(&[ObjectiveSpec {
        id: 1,
        pos: Vec3::new(800.0, 5.0, 60.0),
        radius_m: 45.0,
        owner: Some(Team::Ger),
    }]);
    let near = w.debug_add_crate(Team::Us, Vec3::new(305.0, 5.0, 60.0), 100);
    let far = w.debug_add_crate(Team::Us, Vec3::new(310.0, 5.0, 60.0), 60);
    w.place_garrison(0, Vec3::new(600.0, 5.0, 60.0)).unwrap();
    let c = w.supply_crates();
    assert!(c.iter().all(|c| c.id != near), "emptied crate is removed");
    assert_eq!(c.iter().find(|c| c.id == far).unwrap().supplies, 60);
    // Not enough: nothing is taken.
    let mut w = world();
    w.set_objectives(&[ObjectiveSpec {
        id: 1,
        pos: Vec3::new(800.0, 5.0, 60.0),
        radius_m: 45.0,
        owner: Some(Team::Ger),
    }]);
    w.debug_add_crate(Team::Us, Vec3::new(305.0, 5.0, 60.0), 30);
    w.debug_add_crate(Team::Us, Vec3::new(310.0, 5.0, 60.0), 40);
    assert_eq!(
        w.place_garrison(0, Vec3::new(600.0, 5.0, 60.0)),
        Err(PlaceError::NoSupplies)
    );
    let total: u32 = w.supply_crates().iter().map(|c| c.supplies).sum();
    assert_eq!(total, 70);
    // A crate 16 m away is out of reach.
    let mut w = world();
    w.debug_add_crate(Team::Us, Vec3::new(316.0, 5.0, 60.0), 100);
    assert_eq!(
        w.place_garrison(0, Vec3::new(500.0, 5.0, 60.0)),
        Err(PlaceError::NoSupplies)
    );
}

#[test]
fn crate_damage_and_dropping_on_death() {
    let mut w = world();
    let c = w.debug_add_crate(Team::Us, Vec3::new(350.0, 5.0, 60.0), 100);
    assert!(!w.damage_supply_crate(c, 1400.0, CrateDamage::Bullet));
    assert!(
        w.damage_supply_crate(c, 100.0, CrateDamage::Bullet),
        "1500 nominal"
    );
    let c = w.debug_add_crate(Team::Us, Vec3::new(350.0, 5.0, 60.0), 100);
    assert!(w.damage_supply_crate(c, 150.0, CrateDamage::Explosion));
    assert!(w.supply_crates().is_empty());
    // Carrier dies: the crate drops where it fell.
    w.add_depot(Team::Us, Vec3::new(300.0, 5.0, 60.0));
    w.debug_add_depot_stock(0, 1);
    for _ in 0..45 {
        step_with(&mut w, &[(0, use_btn())]);
    }
    assert!(w.carrying(0).is_some());
    w.debug_kill(0);
    step(&mut w);
    step(&mut w);
    assert!(w.carrying(0).is_none());
    let cr = w.supply_crates();
    assert_eq!(cr.len(), 1);
    assert!(cr[0].carried_by.is_none());
}

#[test]
fn deterministic() {
    let run = || {
        let mut w = world();
        w.add_depot(Team::Us, Vec3::new(300.0, 5.0, 60.0));
        w.debug_add_depot_stock(0, 2);
        for _ in 0..100 {
            step_with(&mut w, &[(0, use_btn())]);
        }
        for _ in 0..1500 {
            step(&mut w);
        }
        w.state_hash()
    };
    assert_eq!(run(), run());
}
