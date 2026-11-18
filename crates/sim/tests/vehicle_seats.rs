//! M1-022b: seats, driver input, exit damage, crushing, hull impacts, snapshot size.
mod common;

use common::*;
use frontline_sim::squad::{SquadKind, SquadOp};
use frontline_sim::vehicle::seats::SeatError;
use frontline_sim::vehicle::snapshot::{VehicleExtra, EXTRA_BYTES};
use frontline_sim::{GameData, LifeStateView, SimEvent, Team, World};
use glam::{Quat, Vec3};

const HZ: u32 = 30;
const ID: &str = "us_m4a1_75";

/// Bodies: US crewmen 0, 1; US rifleman 2; German 5. The tank starts at (500, 100) facing +Z.
fn fx(f: &dyn Fn(f32, f32) -> f32) -> (World, u16) {
    let col = synth_data(30, 6, f);
    let players = [
        (0u8, Team::Us, 500.0, 104.0, 16384u16),
        (1, Team::Us, 502.0, 104.0, 16384),
        (2, Team::Us, 504.0, 104.0, 16384),
        (5, Team::Ger, 700.0, 100.0, 16384),
    ];
    let mut w = duel_world(GameData::embedded(), &col, 9, &players);
    for p in [0u8, 1, 2] {
        w.roster_add(p, Team::Us).unwrap();
        w.roster_add(100 + p, Team::Ger).unwrap();
    }
    w.roster_add(5, Team::Ger).unwrap();
    w.roster_add(105, Team::Us).unwrap();
    w.squad_op(
        0,
        SquadOp::Create {
            kind: SquadKind::ArmorCrew,
            suffix: String::new(),
        },
    )
    .unwrap();
    w.squad_op(1, SquadOp::Join { squad: 1 }).unwrap();
    w.set_role(0, Some("crewman")).unwrap();
    w.set_role(1, Some("crewman")).unwrap();
    let id = w
        .spawn_vehicle(ID, Team::Us, Vec3::new(500.0, 5.9, 100.0), 0.0)
        .unwrap();
    for _ in 0..120 {
        w.step(&[]);
    }
    (w, id)
}

fn flat() -> (World, u16) {
    fx(&|_, _| 5.0)
}

/// Inputs holding the given players still, with `driver` pushing forward by `z`.
fn inputs(
    w: &World,
    driver: Option<(u8, i8)>,
) -> Vec<(u8, frontline_protocol::msg::input::InputCmd)> {
    w.view()
        .players()
        .iter()
        .map(|p| {
            let c = match driver {
                Some((d, z)) if d == p.player => cmd(0, z, 16384, 0, 0),
                _ => cmd(0, 0, 16384, 0, 0),
            };
            (p.player, c)
        })
        .collect()
}

fn step_drive(w: &mut World, driver: Option<(u8, i8)>, ticks: u32) -> Vec<SimEvent> {
    let mut ev = Vec::new();
    for _ in 0..ticks {
        let i = inputs(w, driver);
        ev.extend(w.step(&i).events);
    }
    ev
}

#[test]
fn boarding_rules() {
    let (mut w, v) = flat();
    assert_eq!(w.seat_enter(2, v, 0), Err(SeatError::NotCrewman));
    assert_eq!(w.seat_enter(5, v, 0), Err(SeatError::NotCrewman));
    assert_eq!(w.seat_enter(0, 999, 0), Err(SeatError::NoSuchVehicle));
    assert_eq!(w.seat_enter(0, v, 9), Err(SeatError::NoSuchSeat));
    w.debug_teleport(1, Vec3::new(560.0, 5.02, 100.0));
    assert_eq!(w.seat_enter(1, v, 0), Err(SeatError::TooFar));
    w.seat_enter(0, v, 0).unwrap();
    assert_eq!(w.seat_of(0), Some((v, 0)));
    assert_eq!(w.seat_enter(0, v, 1), Err(SeatError::AlreadyAboard));
    w.debug_teleport(1, Vec3::new(502.0, 5.02, 104.0));
    assert_eq!(w.seat_enter(1, v, 0), Err(SeatError::Occupied));
    w.seat_enter(1, v, 1).unwrap();
}

#[test]
fn the_driver_drives_and_occupants_ride_along() {
    let (mut w, v) = flat();
    w.seat_enter(0, v, 0).unwrap();
    w.seat_enter(1, v, 1).unwrap();
    let z0 = w.vehicle(v).unwrap().pos.z;
    step_drive(&mut w, Some((0, 127)), 10 * HZ);
    let veh = w.vehicle(v).unwrap();
    assert!(veh.pos.z - z0 > 30.0, "moved {}", veh.pos.z - z0);
    let view = w.view();
    let p1 = view.players().iter().find(|p| p.player == 1).unwrap().pos;
    assert!(
        (p1 - veh.pos).length() < 3.0,
        "gunner rides: {p1:?} vs {:?}",
        veh.pos
    );
    // Unmanned (the driver leaves) the tank parks itself.
    w.seat_exit_begin(0).unwrap();
    step_drive(&mut w, Some((0, 127)), HZ);
    step_drive(&mut w, None, 6 * HZ);
    assert!(w.vehicle(v).unwrap().vel.length() < 0.2);
}

#[test]
fn seat_switch_is_instant_when_slow_and_two_seconds_when_moving() {
    let (mut w, v) = flat();
    w.seat_enter(0, v, 0).unwrap();
    w.seat_enter(1, v, 1).unwrap();
    // Stopped: instant.
    w.seat_switch(1, 2).unwrap();
    assert_eq!(w.seat_of(1), Some((v, 2)));
    assert_eq!(w.seat_switch(1, 0), Err(SeatError::Occupied));
    // 10 km/h: hold the driver at 0.35 throttle until steady.
    step_drive(&mut w, Some((0, 45)), 8 * HZ);
    let kph = w.vehicle(v).unwrap().vel.length() * 3.6;
    assert!((9.0..11.0).contains(&kph), "{kph} km/h");
    w.seat_switch(1, 1).unwrap();
    step_drive(&mut w, Some((0, 45)), 2 * HZ - 2);
    assert_eq!(w.seat_of(1), Some((v, 2)), "not yet at 1.93 s");
    step_drive(&mut w, Some((0, 45)), 3);
    assert_eq!(w.seat_of(1), Some((v, 1)), "after 2 s");
}

#[test]
fn leaving_takes_point_eight_seconds_and_hurts_above_20_kph() {
    for hurt in [true, false] {
        let (mut w, v) = flat();
        w.seat_enter(0, v, 0).unwrap();
        w.seat_enter(1, v, 1).unwrap();
        let throttle = if hurt { 93 } else { 79 }; // ~20.9 and ~17.7 km/h
        step_drive(&mut w, Some((0, throttle)), 10 * HZ);
        let kph = w.vehicle(v).unwrap().vel.length() * 3.6;
        assert_eq!(kph > 20.0, hurt, "{kph}");
        w.seat_exit_begin(1).unwrap();
        step_drive(&mut w, Some((0, throttle)), 23);
        assert!(w.seat_of(1).is_some(), "still aboard after 0.77 s");
        step_drive(&mut w, Some((0, throttle)), 2);
        assert!(w.seat_of(1).is_none(), "out after 0.8 s");
        let life = w.player_life(1).unwrap();
        assert!(matches!(life, LifeStateView::Alive));
        let hp = w.player_hp(1).unwrap();
        let want = if hurt { 70.0 } else { 100.0 };
        assert!((hp - want).abs() < 1.0, "hp {hp}");
    }
    // Releasing the key cancels the exit.
    let (mut w, v) = flat();
    w.seat_enter(1, v, 1).unwrap();
    w.seat_exit_begin(1).unwrap();
    step_drive(&mut w, None, 10);
    w.seat_exit_cancel(1);
    step_drive(&mut w, None, 30);
    assert!(w.seat_of(1).is_some());
}

fn crush_case(kph: f32) -> (f32, bool, Vec<SimEvent>) {
    let (mut w, v) = flat();
    w.seat_enter(0, v, 0).unwrap();
    let throttle = (kph / 28.5 * 127.0).round() as i8;
    // Run up to speed, then stand the German on the line ahead.
    step_drive(&mut w, Some((0, throttle)), 6 * HZ);
    let z = w.vehicle(v).unwrap().pos.z;
    w.debug_teleport(5, Vec3::new(500.0, 5.02, z + 14.0));
    let ev = step_drive(&mut w, Some((0, throttle)), 5 * HZ);
    let dead = matches!(w.player_life(5).unwrap(), LifeStateView::Dead { .. });
    (w.player_hp(5).unwrap(), dead, ev)
}

#[test]
fn crushing_infantry() {
    let (hp, dead, ev) = crush_case(16.0);
    assert!(dead, "16 km/h kills ({hp} hp left)");
    assert!(ev
        .iter()
        .any(|e| matches!(e, SimEvent::Crush { victim: 5, .. })));
    let (hp, dead, _) = crush_case(10.0);
    assert!(!dead);
    assert!((hp - 30.0).abs() < 3.0, "10 km/h takes 70 HP ({hp} left)");
    let (hp, dead, ev) = crush_case(4.0);
    assert!(!dead && hp == 100.0, "4 km/h is harmless");
    assert!(!ev.iter().any(|e| matches!(e, SimEvent::Crush { .. })));
}

fn wall_world(z_wall: f32) -> impl Fn(f32, f32) -> f32 {
    move |_, z| if z < z_wall { 5.0 } else { 14.0 }
}

fn wall_hit(kph: f32) -> (f32, Vec<SimEvent>) {
    let (mut w, v) = fx(&wall_world(160.0));
    w.seat_enter(0, v, 0).unwrap();
    let throttle = (kph / 28.5 * 127.0).round() as i8;
    let ev = step_drive(&mut w, Some((0, throttle)), 14 * HZ);
    let hp = w.vehicle(v).unwrap().hull_hp;
    (1600.0 - hp, ev)
}

#[test]
fn wall_impact_damage_is_half_v_squared_above_25_kph() {
    let (dmg, ev) = wall_hit(28.5);
    let v = 28.5 / 3.6;
    let want = 0.5 * v * v;
    assert!((dmg - want).abs() / want < 0.05, "{dmg} vs {want}");
    let hits = ev
        .iter()
        .filter(|e| matches!(e, SimEvent::VehicleCollision { .. }))
        .count();
    assert_eq!(hits, 1, "one impact per contact");
    let (dmg, _) = wall_hit(24.0);
    assert_eq!(dmg, 0.0, "24 km/h is below the threshold");
}

#[test]
fn extra_is_small_and_roundtrips() {
    let (mut w, v) = flat();
    w.debug_place_vehicle(
        v,
        Vec3::new(500.0, 5.6, 100.0),
        Quat::from_euler(glam::EulerRot::YXZ, 0.7, -0.2, 0.1),
    );
    let veh = w.vehicle(v).unwrap();
    let e = VehicleExtra::from_vehicle(veh);
    let bytes = e.encode();
    assert!(bytes.len() <= 24 && bytes.len() == EXTRA_BYTES);
    let back = VehicleExtra::decode(&bytes);
    assert_eq!(back, e);
    let angle = back.rotation().angle_between(veh.rot).to_degrees();
    assert!(angle < 0.2, "{angle} deg");
    assert_eq!(w.vehicle_views().len(), 1);
}

#[test]
fn overturn_is_reported_after_three_seconds() {
    let (mut w, v) = flat();
    w.debug_place_vehicle(
        v,
        Vec3::new(500.0, 8.0, 100.0),
        Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
    );
    let early = step_drive(&mut w, None, 3 * HZ - 2);
    assert!(!early
        .iter()
        .any(|e| matches!(e, SimEvent::VehicleOverturned { .. })));
    let late = step_drive(&mut w, None, 4);
    assert_eq!(
        late.iter()
            .filter(|e| matches!(e, SimEvent::VehicleOverturned { .. }))
            .count(),
        1
    );
}
