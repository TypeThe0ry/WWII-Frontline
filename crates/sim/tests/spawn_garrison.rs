//! M1-019b: Garrison placement rules, building, blocking, Overrun, damage, slots, spawning.
mod common;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use common::*;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::spawn::garrison::{
    GarrisonDamage, GarrisonState, PlaceError, SpawnRequestError,
};
use frontline_sim::supply_iface::SupplyProvider;
use frontline_sim::{LifeStateView, ObjectiveSpec, SimEvent, Team, World};
use glam::Vec3;

const HZ: u32 = 30;

#[derive(Clone)]
struct Stub(Arc<AtomicU32>);

impl SupplyProvider for Stub {
    fn available(&self, _: Vec3, _: f32) -> u32 {
        self.0.load(Ordering::Relaxed)
    }
    fn consume(&mut self, _: Vec3, _: f32, amount: u32) -> bool {
        let cur = self.0.load(Ordering::Relaxed);
        if cur >= amount {
            self.0.store(cur - amount, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
}

fn idle() -> InputCmd {
    cmd(0, 0, 49152, 0, 0)
}

fn use_btn() -> InputCmd {
    cmd(0, 0, 49152, buttons::USE, 0)
}

struct Fx {
    w: World,
    stock: Arc<AtomicU32>,
}

/// 30 x 6 chunks (1920 x 384 m), HQs at x=60 / 1860. Player 0 is a US officer, 1 a German,
/// `extra_us` more US players follow (ids 2..).
fn fx_with(col: frontline_sim::CollisionData, extra_us: u8) -> Fx {
    let mut players = vec![
        (0u8, Team::Us, 300.0, 60.0, 16384u16),
        (1, Team::Ger, 1800.0, 60.0, 16384),
    ];
    for i in 0..extra_us {
        players.push((2 + i, Team::Us, 300.0, 70.0 + f32::from(i) * 0.1, 16384));
    }
    let mut w = duel_world(data_with(|_| {}), &col, 5, &players);
    w.set_hq(Team::Us, Vec3::new(60.0, 5.0, 60.0));
    w.set_hq(Team::Ger, Vec3::new(1860.0, 5.0, 60.0));
    w.set_officer(0, true);
    let stock = Arc::new(AtomicU32::new(1000));
    w.set_supply_provider(Box::new(Stub(stock.clone())));
    Fx { w, stock }
}

fn fx(extra_us: u8) -> Fx {
    fx_with(synth_data(30, 6, &|_, _| 5.0), extra_us)
}

fn at(x: f32, z: f32) -> Vec3 {
    Vec3::new(x, 5.0, z)
}

fn stub(n: u32) -> Stub {
    Stub(Arc::new(AtomicU32::new(n)))
}

fn step_all(w: &mut World, special: &[(u8, InputCmd)]) {
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
    w.step(&ins);
}

fn step(w: &mut World) {
    step_all(w, &[]);
}

#[test]
fn placement_error_matrix() {
    let f = fx(0);
    let w = &f.w;
    let ok = |x: f32, z: f32| w.validate_garrison_place(0, at(x, z), &stub(1000));
    // HQ distance (US HQ x=60, so 199/201 m east of it).
    assert_eq!(ok(259.0, 60.0), Err(PlaceError::TooCloseHq));
    assert!(ok(261.0, 60.0).is_ok());
    // Role.
    let mut f2 = fx(0);
    f2.w.set_officer(0, false);
    assert_eq!(
        f2.w.validate_garrison_place(0, at(400.0, 60.0), &stub(1000)),
        Err(PlaceError::WrongRole)
    );
    // Supplies 49 / 50.
    assert_eq!(
        w.validate_garrison_place(0, at(400.0, 60.0), &stub(49)),
        Err(PlaceError::NoSupplies)
    );
    assert_eq!(
        w.validate_garrison_place(0, at(400.0, 60.0), &stub(50)),
        Ok(frontline_sim::spawn::garrison::GarrisonCost { supplies: 50 })
    );
}

#[test]
fn own_and_enemy_garrison_spacing() {
    let mut f = fx(0);
    f.w.place_garrison(0, at(400.0, 60.0)).unwrap();
    let ok = |w: &World, x: f32| w.validate_garrison_place(0, at(x, 60.0), &stub(1000));
    assert_eq!(
        ok(&f.w, 400.0 + 149.0),
        Err(PlaceError::TooCloseOwnGarrison)
    );
    assert!(ok(&f.w, 400.0 + 151.0).is_ok());
    // Enemy Garrison 250 m: a German officer at x=1500.
    let mut g = fx(0);
    g.w.set_officer(1, true);
    g.w.place_garrison(1, at(1500.0, 60.0)).unwrap();
    assert_eq!(
        ok(&g.w, 1500.0 - 249.0),
        Err(PlaceError::TooCloseEnemyGarrison)
    );
    assert!(ok(&g.w, 1500.0 - 251.0).is_ok());
}

#[test]
fn objective_exclusion_and_enemy_sector_cost() {
    let mut f = fx(0);
    let spec = |owner| ObjectiveSpec {
        id: 7,
        pos: Vec3::new(800.0, 5.0, 60.0),
        radius_m: 45.0,
        owner,
    };
    f.w.set_objectives(&[spec(None)]);
    let ok = |w: &World, x: f32, s: u32| w.validate_garrison_place(0, at(x, 60.0), &stub(s));
    // R_sp + 100 = 145 m.
    assert_eq!(
        ok(&f.w, 800.0 - 144.0, 1000),
        Err(PlaceError::TooCloseObjective)
    );
    assert!(ok(&f.w, 800.0 - 146.0, 1000).is_ok());
    // A German-owned sector costs 100 for the US.
    f.w.set_objectives(&[spec(Some(Team::Ger))]);
    assert_eq!(ok(&f.w, 800.0 - 200.0, 99), Err(PlaceError::NoSupplies));
    assert_eq!(
        ok(&f.w, 800.0 - 200.0, 100),
        Ok(frontline_sim::spawn::garrison::GarrisonCost { supplies: 100 })
    );
}

#[test]
fn slope_and_water() {
    let tan = |deg: f32| deg.to_radians().tan();
    let ramp = |deg: f32| {
        let t = tan(deg);
        move |x: f32, _z: f32| 5.0 + t * (100.0 - (x - 500.0).abs()).max(0.0)
    };
    for (deg, expect_ok) in [(26.0f32, false), (24.0, true)] {
        let r = ramp(deg);
        let f = fx_with(synth_data(30, 6, &r), 0);
        let res = f.w.validate_garrison_place(0, at(450.0, 60.0), &stub(1000));
        if expect_ok {
            assert!(res.is_ok(), "{deg}: {res:?}");
        } else {
            assert_eq!(res, Err(PlaceError::Slope), "{deg}");
        }
    }
    // Water: chunk column 7 (x 448..512) is water.
    let water = frontline_sim::GameData::embedded()
        .materials
        .iter()
        .position(|m| m.id == "water")
        .unwrap() as u8;
    let mut chunks = Vec::new();
    for bz in 0..6 {
        for bx in 0..30 {
            let h = vec![5.0f32; 65 * 65];
            let m = if bx == 7 { water } else { 0 };
            chunks.push(frontline_sim::CollisionChunk::new(
                bx,
                bz,
                h,
                vec![m; 64 * 64],
            ));
        }
    }
    let f = fx_with(frontline_sim::CollisionData::from_chunks(chunks), 0);
    assert_eq!(
        f.w.validate_garrison_place(0, at(480.0, 60.0), &stub(1000)),
        Err(PlaceError::Water)
    );
    assert!(f
        .w
        .validate_garrison_place(0, at(600.0, 60.0), &stub(1000))
        .is_ok());
}

#[test]
fn enemy_near_needs_line_of_sight() {
    // A ridge at x = 520..524 hides x >= 524 from x = 500.
    let ridge = |x: f32, _z: f32| {
        if (520.0..524.0).contains(&x) {
            60.0
        } else {
            5.0
        }
    };
    let mut f = fx_with(synth_data(30, 6, &ridge), 0);
    let p = at(500.0, 60.0);
    f.w.debug_teleport(1, Vec3::new(540.0, 5.02, 60.0));
    assert!(
        f.w.validate_garrison_place(0, p, &stub(1000)).is_ok(),
        "hidden behind the ridge"
    );
    f.w.debug_teleport(1, Vec3::new(510.0, 5.02, 60.0));
    assert_eq!(
        f.w.validate_garrison_place(0, p, &stub(1000)),
        Err(PlaceError::EnemyNear)
    );
    // 51 m away in the open is not "near".
    f.w.debug_teleport(1, Vec3::new(451.0 - 100.0, 5.02, 60.0));
    f.w.debug_teleport(1, Vec3::new(551.0, 5.02, 60.0));
    assert!(f.w.validate_garrison_place(0, p, &stub(1000)).is_ok());
}

#[test]
fn team_and_sector_caps() {
    let mut f = fx_with(synth_data(30, 6, &|_, _| 5.0), 0);
    let obj = |id, x| ObjectiveSpec {
        id,
        pos: Vec3::new(x, 5.0, 60.0),
        radius_m: 10.0,
        owner: None,
    };
    f.w.set_objectives(&[obj(1, 380.0), obj(2, 860.0), obj(3, 1340.0)]);
    // Two per sector: 110 m either side of each objective (R + 100 = 110).
    for x in [270.0, 490.0, 750.0, 970.0, 1230.0, 1450.0] {
        f.w.place_garrison(0, at(x, 60.0)).unwrap();
    }
    assert_eq!(
        f.w.validate_garrison_place(0, at(1640.0, 60.0), &stub(1000)),
        Err(PlaceError::TeamCap)
    );
    // Sector cap: a third near objective 1 with only two placed.
    let mut g = fx_with(synth_data(30, 6, &|_, _| 5.0), 0);
    g.w.set_objectives(&[obj(1, 380.0), obj(2, 860.0), obj(3, 1340.0)]);
    for x in [270.0, 490.0] {
        g.w.place_garrison(0, at(x, 60.0)).unwrap();
    }
    assert_eq!(
        g.w.validate_garrison_place(0, at(380.0, 300.0), &stub(1000)),
        Err(PlaceError::SectorCap)
    );
    assert!(g
        .w
        .validate_garrison_place(0, at(750.0, 60.0), &stub(1000))
        .is_ok());
}

/// Builds a Garrison at (500, 60) with `n` US builders (ids 0, 2, 3); returns the tick it
/// finished and the world.
fn build(n: u8) -> (Fx, u32, u16) {
    let mut f = fx(2);
    let id = f.w.place_garrison(0, at(500.0, 60.0)).unwrap();
    assert_eq!(f.stock.load(Ordering::Relaxed), 950, "cost paid");
    let ids = [0u8, 2, 3];
    for &b in ids.iter().take(n as usize) {
        f.w.debug_teleport(b, Vec3::new(502.0, 5.02, 60.0));
    }
    let start = f.w.tick().0;
    let special: Vec<_> = ids
        .iter()
        .take(n as usize)
        .map(|&b| (b, use_btn()))
        .collect();
    let mut done = None;
    for _ in 0..(40 * HZ) {
        step_all(&mut f.w, &special);
        if f.w.garrison_list()[0].state == GarrisonState::Active {
            done = Some(f.w.tick().0 - start);
            break;
        }
    }
    (f, done.expect("never finished"), id)
}

#[test]
fn build_times_and_health_ramp() {
    let (_, one, _) = build(1);
    assert!((one as i32 - 600).abs() <= 1, "1 builder {one}");
    let (_, three, _) = build(3);
    let want = 20.0 / 2.2 * HZ as f32;
    assert!(
        (three as f32 - want).abs() <= want * 0.02 + 1.0,
        "3 builders {three} vs {want}"
    );
    // Health: 20 % at the start, ~60 % at half progress, full at the end.
    let mut f = fx(0);
    let pos = at(500.0, 60.0);
    let id = f.w.place_garrison(0, pos).unwrap();
    assert_eq!(f.w.garrison_list()[0].hp, 240.0);
    let placed = f.w.step(&[]);
    assert!(placed.events.iter().any(|ev| matches!(ev,
        SimEvent::SpawnPointState { id: event_id, team: Team::Us, pos: event_pos, state: GarrisonState::Building }
            if *event_id == id && *event_pos == pos
    )));
    f.w.debug_teleport(0, Vec3::new(502.0, 5.02, 60.0));
    for _ in 0..300 {
        step_all(&mut f.w, &[(0, use_btn())]);
    }
    let hp = f.w.garrison_list()[0].hp;
    assert!((hp - 720.0).abs() < 6.0, "{hp}");
}

#[test]
fn damage_classes() {
    let (mut f, _, id) = build(1);
    assert!(!f.w.damage_garrison(id, 23_000.0, GarrisonDamage::Bullet));
    assert!(
        f.w.damage_garrison(id, 1_000.0, GarrisonDamage::Bullet),
        "24000 nominal"
    );
    let (mut g, _, id) = build(1);
    assert!(
        g.w.damage_garrison(id, 2500.0, GarrisonDamage::Explosion),
        "satchel"
    );
    assert_eq!(g.w.garrison_list()[0].state, GarrisonState::Destroyed);
    let _ = &mut f;
}

#[test]
fn blocking_and_overrun() {
    let (mut f, _, id) = build(1);
    // Move the builder away from the point; park the enemy.
    f.w.debug_teleport(0, Vec3::new(300.0, 5.02, 60.0));
    let now = f.w.tick().0;
    f.w.debug_teleport(1, Vec3::new(500.0 + 30.1, 5.02, 60.0));
    step(&mut f.w);
    assert!(!f.w.garrison_list()[0].blocked(f.w.tick().0, 30), "30.1 m");
    f.w.debug_teleport(1, Vec3::new(500.0 + 29.9, 5.02, 60.0));
    step(&mut f.w);
    assert!(f.w.garrison_list()[0].blocked(f.w.tick().0, 30), "29.9 m");
    // The enemy leaves: blocked for 3 s more.
    f.w.debug_teleport(1, Vec3::new(1800.0, 5.02, 60.0));
    for _ in 0..80 {
        step(&mut f.w);
    }
    assert!(f.w.garrison_list()[0].blocked(f.w.tick().0, 30));
    for _ in 0..20 {
        step(&mut f.w);
    }
    assert!(!f.w.garrison_list()[0].blocked(f.w.tick().0, 30));
    let _ = (now, id);
    // Overrun: enemy 15 m away, nobody friendly within 20 m, for 25 s.
    f.w.debug_teleport(1, Vec3::new(515.0, 5.02, 60.0));
    for _ in 0..(25 * HZ - 2) {
        step(&mut f.w);
    }
    assert_eq!(
        f.w.garrison_list()[0].state,
        GarrisonState::Active,
        "24.9 s"
    );
    for _ in 0..4 {
        step(&mut f.w);
    }
    assert_eq!(
        f.w.garrison_list()[0].state,
        GarrisonState::Destroyed,
        "25 s"
    );
}

#[test]
fn friend_nearby_prevents_overrun() {
    let (mut f, _, _) = build(1);
    f.w.debug_teleport(1, Vec3::new(515.0, 5.02, 60.0));
    // The builder stays within 20 m.
    for _ in 0..(30 * HZ) {
        step(&mut f.w);
    }
    assert_eq!(f.w.garrison_list()[0].state, GarrisonState::Active);
}

#[test]
fn spawning_at_a_garrison_waits_eight_seconds_and_gives_no_protection() {
    let (mut f, _, id) = build(1);
    f.w.debug_teleport(0, Vec3::new(300.0, 5.02, 60.0));
    f.w.debug_kill(2);
    step(&mut f.w);
    let k = f.w.tick().0;
    let ready = f.w.request_spawn(2, id).unwrap();
    assert_eq!(ready, 8 * HZ, "estimate");
    while !matches!(f.w.player_life(2).unwrap(), LifeStateView::Alive) {
        step(&mut f.w);
        assert!(f.w.tick().0 < k + 20 * HZ);
    }
    let t = f.w.tick().0;
    assert!(
        (t as i32 - (k + 8 * HZ) as i32).abs() <= 1,
        "{t} vs {}",
        k + 8 * HZ
    );
    let p =
        f.w.view()
            .players()
            .iter()
            .find(|p| p.player == 2)
            .unwrap()
            .pos;
    assert!(((p.x - 500.0).hypot(p.z - 60.0)) <= 8.0 + 1e-3);
    assert_eq!(
        f.w.spawn_protection_left(2),
        0,
        "no protection at a Garrison"
    );
    let g = &f.w.garrison_list()[0];
    assert!(g.slots < 24.0);
}

#[test]
fn a_destroyed_garrison_sends_the_spawn_to_the_hq() {
    let (mut f, _, id) = build(1);
    f.w.debug_teleport(0, Vec3::new(300.0, 5.02, 60.0));
    f.w.debug_kill(2);
    step(&mut f.w);
    f.w.request_spawn(2, id).unwrap();
    f.w.damage_garrison(id, 5000.0, GarrisonDamage::Explosion);
    for _ in 0..(40 * HZ) {
        step(&mut f.w);
        if matches!(f.w.player_life(2).unwrap(), LifeStateView::Alive) {
            break;
        }
    }
    let p =
        f.w.view()
            .players()
            .iter()
            .find(|p| p.player == 2)
            .unwrap()
            .pos;
    assert!(((p.x - 60.0).hypot(p.z - 60.0)) <= 8.0 + 1e-3, "{p:?}");
    assert_eq!(f.w.request_spawn(2, id), Err(SpawnRequestError::NotDead),);
}

#[test]
fn slots_and_throughput() {
    // 30 US players besides the officer and the German.
    let (mut f, _, id) = {
        let mut f = fx(30);
        let id = f.w.place_garrison(0, at(500.0, 60.0)).unwrap();
        f.w.debug_teleport(0, Vec3::new(502.0, 5.02, 60.0));
        while f.w.garrison_list()[0].state != GarrisonState::Active {
            step_all(&mut f.w, &[(0, use_btn())]);
        }
        f.w.debug_teleport(0, Vec3::new(300.0, 5.02, 60.0));
        (f, 0, id)
    };
    assert_eq!(f.w.garrison_list()[0].slots, 24.0);
    for p in 2..32u8 {
        f.w.debug_kill(p);
    }
    step(&mut f.w);
    for p in 2..32u8 {
        f.w.request_spawn(p, id).unwrap();
    }
    let mut at_tick: Vec<u32> = Vec::new();
    let mut seen = [false; 32];
    for _ in 0..(120 * HZ) {
        step(&mut f.w);
        for p in 2..32u8 {
            if !seen[p as usize] && matches!(f.w.player_life(p).unwrap(), LifeStateView::Alive) {
                seen[p as usize] = true;
                at_tick.push(f.w.tick().0);
            }
        }
        if at_tick.len() == 30 {
            break;
        }
    }
    assert_eq!(at_tick.len(), 30, "everybody got out eventually");
    at_tick.sort_unstable();
    // At most 2 per second.
    for w in at_tick.windows(3) {
        assert!(w[2] - w[0] >= HZ, "3 spawns within a second: {w:?}");
    }
    // Slots: 24 plus one regenerated per 5 s.
    let t0 = at_tick[0];
    for (i, &t) in at_tick.iter().enumerate() {
        let allowed = 24 + ((t - t0) / (5 * HZ)) as usize + 1;
        assert!(i < allowed, "spawn #{i} at +{} ticks", t - t0);
    }
}

#[test]
fn deterministic() {
    let run = || {
        let (mut f, _, id) = build(1);
        f.w.debug_kill(2);
        step(&mut f.w);
        f.w.request_spawn(2, id).unwrap();
        for _ in 0..400 {
            step(&mut f.w);
        }
        f.w.state_hash()
    };
    assert_eq!(run(), run());
}
