//! M1-020b: Ammo Crates, resupply prices and the Garrison + supply loop.
mod common;

use common::*;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::spawn::garrison::GarrisonState;
use frontline_sim::supply::ammo::{
    resupply, AmmoCosts, AmmoPlaceError, Inventory, Item, LoadoutTarget,
};
use frontline_sim::throwables::system::Gadgets;
use frontline_sim::{LifeStateView, SimEvent, Team, WeaponState, World};
use glam::Vec3;

const HZ: u32 = 30;

fn idle() -> InputCmd {
    cmd(0, 0, 49152, 0, 0)
}

fn use_btn() -> InputCmd {
    cmd(0, 0, 49152, buttons::USE, 0)
}

/// 0: US officer, 1: German, 2: US support, 3: US rifleman, 4/5: more US supports.
fn world() -> World {
    let players = [
        (0u8, Team::Us, 300.0, 60.0, 16384u16),
        (1, Team::Ger, 1800.0, 60.0, 16384),
        (2, Team::Us, 300.0, 60.5, 16384),
        (3, Team::Us, 302.0, 60.0, 16384),
        (4, Team::Us, 300.0, 60.2, 16384),
        (5, Team::Us, 300.0, 60.3, 16384),
    ];
    let mut w = duel_world(
        data_with(|_| {}),
        &synth_data(30, 6, &|_, _| 5.0),
        5,
        &players,
    );
    w.set_hq(Team::Us, Vec3::new(60.0, 5.0, 60.0));
    w.set_hq(Team::Ger, Vec3::new(1860.0, 5.0, 60.0));
    for s in [2u8, 4, 5] {
        w.set_support(s, true);
    }
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

fn costs() -> AmmoCosts {
    AmmoCosts(
        frontline_sim::GameData::embedded()
            .rules
            .supplies
            .ammo_crate
            .price_points
            .clone(),
    )
}

fn inv<'a>(w: Option<&'a mut WeaponState>, g: &'a mut Gadgets, b: &'a mut u8) -> Inventory<'a> {
    Inventory {
        weapon: w,
        gadgets: g,
        bandages: b,
    }
}

fn target(cap: u16, reserve: u16) -> LoadoutTarget {
    LoadoutTarget {
        mag_capacity: cap,
        reserve,
        frag: 2,
        smoke: 1,
        bandages: 2,
    }
}

#[test]
fn prices_rifle_smg_grenade() {
    let c = costs();
    // Rifle: 6 of 9 spare clips spent (8 rounds each): 6 x 4 = 24.
    let mut w = WeaponState::new(0, 8, 24);
    let mut g = Gadgets::default();
    let mut b = 2u8;
    let mut pts = 600;
    let r = resupply(
        &mut inv(Some(&mut w), &mut g, &mut b),
        &mut pts,
        &target(8, 72),
        "rifle_semi",
        &c,
    );
    assert_eq!(r.points_spent, 24);
    assert_eq!(w.reserve, 72);
    assert_eq!(pts, 576);
    assert_eq!(r.items, vec![(Item::Mag, 6)]);
    // SMG: 3 mags x 3 = 9.
    let mut w = WeaponState::new(0, 30, 120 - 90);
    let mut pts = 600;
    let r = resupply(
        &mut inv(Some(&mut w), &mut g, &mut b),
        &mut pts,
        &target(30, 120),
        "smg",
        &c,
    );
    assert_eq!(r.points_spent, 9);
    assert_eq!(w.reserve, 120);
    // Grenades: 2 x 6 = 12.
    let mut g = Gadgets {
        frag: 0,
        ..Gadgets::default()
    };
    let mut pts = 600;
    let r = resupply(
        &mut inv(None, &mut g, &mut b),
        &mut pts,
        &target(8, 72),
        "rifle_semi",
        &c,
    );
    assert_eq!(r.points_spent, 12);
    assert_eq!(g.frag, 2);
    // Bandages 2 each.
    let mut b = 0u8;
    let mut pts = 600;
    let mut g = Gadgets::default();
    let r = resupply(
        &mut inv(None, &mut g, &mut b),
        &mut pts,
        &target(8, 72),
        "rifle_semi",
        &c,
    );
    assert_eq!((r.points_spent, b), (4, 2));
}

#[test]
fn not_enough_points_fills_as_much_as_possible() {
    let c = costs();
    let mut w = WeaponState::new(0, 8, 72 - 24); // 3 clips short
    let mut g = Gadgets::default();
    let mut b = 2u8;
    let mut pts = 10;
    let r = resupply(
        &mut inv(Some(&mut w), &mut g, &mut b),
        &mut pts,
        &target(8, 72),
        "rifle_semi",
        &c,
    );
    assert_eq!(r.points_spent, 8, "2 clips");
    assert_eq!(w.reserve, 72 - 24 + 16);
    assert_eq!(pts, 2);
    // Never above the loadout.
    let mut w = WeaponState::new(0, 8, 72);
    let mut pts = 600;
    let r = resupply(
        &mut inv(Some(&mut w), &mut g, &mut b),
        &mut pts,
        &target(8, 72),
        "rifle_semi",
        &c,
    );
    assert_eq!((r.points_spent, w.reserve), (0, 72));
}

#[test]
fn crate_resupplies_a_rifleman_in_1_5_s_and_runs_dry_after_600_points() {
    let mut w = world();
    w.place_ammo_crate(2).unwrap();
    let full = w.ammo_state(3).unwrap().0;
    w.debug_set_ammo(3, full - 48, 2, 2);
    let mut got = None;
    for t in 1..=60 {
        for e in step_with(&mut w, &[(3, use_btn())]) {
            if let SimEvent::Resupplied { points, .. } = e {
                got = Some((t, points));
            }
        }
    }
    assert_eq!(got, Some((45, 24)), "after 1.5 s, 6 clips x 4");
    assert_eq!(w.ammo_state(3).unwrap().0, full);
    assert_eq!(w.ammo_crates()[0].points, 576);
    // Drain the rest: each refill is a different loss pattern.
    let mut spent = 24u32;
    for _ in 0..40 {
        w.debug_set_ammo(3, 0, 0, 0);
        step(&mut w);
        for _ in 0..50 {
            for e in step_with(&mut w, &[(3, use_btn())]) {
                if let SimEvent::Resupplied { points, .. } = e {
                    spent += points;
                }
            }
        }
        if w.ammo_crates().is_empty() {
            break;
        }
    }
    assert!(w.ammo_crates().is_empty(), "removed once empty");
    assert_eq!(spent, 600);
}

#[test]
fn crate_lives_150_s() {
    let mut w = world();
    w.place_ammo_crate(2).unwrap();
    let t0 = w.tick().0;
    while !w.ammo_crates().is_empty() {
        step(&mut w);
        assert!(w.tick().0 < t0 + 5000);
    }
    let life = w.tick().0 - t0;
    assert!((life as i32 - 4500).abs() <= 1, "{life}");
}

#[test]
fn placement_limits() {
    let mut w = world();
    assert_eq!(w.place_ammo_crate(3), Err(AmmoPlaceError::NotSupport));
    w.place_ammo_crate(2).unwrap();
    assert_eq!(w.place_ammo_crate(2), Err(AmmoPlaceError::Cooldown));
    for _ in 0..(30 * HZ - 2) {
        step(&mut w);
    }
    assert_eq!(
        w.place_ammo_crate(2),
        Err(AmmoPlaceError::Cooldown),
        "29.9 s"
    );
    for _ in 0..3 {
        step(&mut w);
    }
    w.place_ammo_crate(2).unwrap();
    for _ in 0..(31 * HZ) {
        step(&mut w);
    }
    assert_eq!(w.place_ammo_crate(2), Err(AmmoPlaceError::LimitPerLife));
    // A new life resets the counter.
    w.debug_kill(2);
    for _ in 0..(40 * HZ) {
        step(&mut w);
        if matches!(w.player_life(2).unwrap(), LifeStateView::Alive) {
            break;
        }
    }
    step(&mut w);
    w.place_ammo_crate(2).unwrap();
}

#[test]
fn at_most_three_crates_on_one_spot() {
    let mut w = world();
    w.place_ammo_crate(2).unwrap();
    w.place_ammo_crate(4).unwrap();
    w.place_ammo_crate(5).unwrap();
    w.set_support(3, true);
    w.debug_teleport(3, Vec3::new(300.0, 5.02, 60.4));
    assert_eq!(w.place_ammo_crate(3), Err(AmmoPlaceError::Stacked));
}

/// Support fetches a crate from the Depot, the Officer places a Garrison paid from it, a
/// teammate spawns there after 8 s, and the Support's Ammo Crate refills that teammate.
fn e2e() -> World {
    let mut w = world();
    w.add_depot(Team::Us, Vec3::new(300.0, 5.0, 60.0));
    w.debug_add_depot_stock(0, 1);
    // 1. Support takes a crate from the Depot (1.5 s) and carries it to the Officer.
    for _ in 0..45 {
        step_with(&mut w, &[(2, use_btn())]);
    }
    assert!(w.carrying(2).is_some());
    step(&mut w);
    w.debug_teleport(2, Vec3::new(500.0, 5.02, 60.0));
    w.debug_teleport(0, Vec3::new(502.0, 5.02, 60.0));
    for _ in 0..45 {
        step_with(&mut w, &[(2, use_btn())]);
    }
    assert!(w.carrying(2).is_none());
    assert_eq!(w.supply_crates().len(), 1);
    step(&mut w);
    // 2. The Officer places a Garrison (50 Supplies from the crate) and builds it.
    let g = w.place_garrison(0, Vec3::new(520.0, 5.0, 60.0)).unwrap();
    assert_eq!(w.supply_crates()[0].supplies, 50);
    w.debug_teleport(0, Vec3::new(521.0, 5.02, 60.0));
    for _ in 0..620 {
        step_with(&mut w, &[(0, use_btn())]);
    }
    assert_eq!(w.garrison_list()[0].state, GarrisonState::Active);
    // 3. A teammate dies and spawns at the Garrison 8 s later.
    w.debug_teleport(0, Vec3::new(300.0, 5.02, 60.0));
    w.debug_kill(3);
    step(&mut w);
    let k = w.tick().0;
    w.request_spawn(3, g).unwrap();
    while !matches!(w.player_life(3).unwrap(), LifeStateView::Alive) {
        step(&mut w);
        assert!(w.tick().0 < k + 20 * HZ);
    }
    assert!((w.tick().0 as i32 - (k + 8 * HZ) as i32).abs() <= 1);
    // 4. The Support puts down an Ammo Crate beside the teammate, who refills from it.
    let p = w
        .view()
        .players()
        .iter()
        .find(|p| p.player == 3)
        .unwrap()
        .pos;
    w.debug_teleport(2, Vec3::new(p.x + 1.0, 5.02, p.z));
    w.place_ammo_crate(2).unwrap();
    w.debug_set_ammo(3, 10, 2, 2);
    let mut spent = 0;
    for _ in 0..60 {
        for e in step_with(&mut w, &[(3, use_btn())]) {
            if let SimEvent::Resupplied { points, .. } = e {
                spent = points;
            }
        }
    }
    assert!(spent > 0, "refilled");
    assert_eq!(w.ammo_state(3).unwrap().0, 72);
    w
}

#[test]
fn e2e_garrison_supply_loop() {
    let w = e2e();
    let _ = w;
}

#[test]
fn e2e_is_deterministic() {
    assert_eq!(e2e().state_hash(), e2e().state_hash());
}
