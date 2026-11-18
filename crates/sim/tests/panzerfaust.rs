//! M1-023d: Panzerfaust 60 flight, HEAT penetration, range limit and kill.
mod common;

use common::*;
use frontline_protocol::msg::input::buttons;
use frontline_sim::{GameData, SimEvent, Team, World};
use glam::Vec3;

const HZ: u32 = 30;
const V: f32 = 45.0;
const G: f32 = 9.81;

/// Pitch (rad) whose arc is `drop` metres below the launch height after `x` metres.
fn pitch_for(x: f32, drop: f32) -> f32 {
    let h = |th: f32| x * th.tan() - G * x * x / (2.0 * V * V * th.cos() * th.cos()) + drop;
    let (mut lo, mut hi) = (0.0f32, 0.6f32);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if h(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// US tank at (500, 5.9, 100) facing +Z; German riflemen placed by the caller.
fn world(shooters: &[(u8, f32, f32, u16)]) -> (World, u16) {
    let col = synth_data(30, 6, &|_, _| 5.0);
    let mut players = vec![(0u8, Team::Us, 400.0, 100.0, 0u16)];
    for &(id, x, z, yaw) in shooters {
        players.push((id, Team::Ger, x, z, yaw));
    }
    let mut w = duel_world(GameData::embedded(), &col, 4, &players);
    let tank = w
        .spawn_vehicle("us_m4a1_75", Team::Us, Vec3::new(500.0, 5.9, 100.0), 0.0)
        .unwrap();
    for _ in 0..90 {
        w.step(&[]);
    }
    for &(id, ..) in shooters {
        w.debug_set_rockets(id, 1);
    }
    (w, tank)
}

fn pitch_code(rad: f32) -> i16 {
    (rad / core::f32::consts::FRAC_PI_2 * 32768.0) as i16
}

fn fire_all(
    w: &mut World,
    shooters: &[(u8, f32, f32, u16)],
    pitch: f32,
    ticks: u32,
) -> Vec<SimEvent> {
    let mut ev = Vec::new();
    for t in 0..ticks {
        let inputs: Vec<_> = w
            .view()
            .players()
            .iter()
            .map(|p| {
                let mut c = cmd(0, 0, 0, 0, 0);
                if let Some(s) = shooters.iter().find(|s| s.0 == p.player) {
                    c.yaw = s.3;
                    c.pitch = pitch_code(pitch);
                    if t == 1 {
                        c.buttons = buttons::FIRE;
                    }
                }
                (p.player, c)
            })
            .collect();
        ev.extend(w.step(&inputs).events);
    }
    ev
}

fn first_armor_result(ev: &[SimEvent]) -> Option<u8> {
    ev.iter().find_map(|e| match e {
        SimEvent::ArmorHit { result, .. } => Some(*result),
        _ => None,
    })
}

#[test]
fn side_hit_at_60m_penetrates() {
    // West of the tank, firing +X (yaw code 16384): the 38 mm side plate at 0 degrees.
    let sh = [(5u8, 440.0, 100.0, 16384u16)];
    let (mut w, tank) = world(&sh);
    let ev = fire_all(&mut w, &sh, pitch_for(60.0, 0.75), 3 * HZ);
    assert!(ev.iter().any(|e| matches!(e, SimEvent::RocketFired { .. })));
    assert_eq!(first_armor_result(&ev), Some(0), "{ev:?}");
    assert!(w.vehicle(tank).unwrap().hull_hp < 1600.0 - 400.0);
}

#[test]
fn upper_glacis_at_60m_penetrates() {
    // North of the tank, firing -Z (yaw code 32768): the 51 mm plate at 56 degrees.
    let sh = [(5u8, 500.0, 163.0, 32768u16)];
    let (mut w, _) = world(&sh);
    let ev = fire_all(&mut w, &sh, pitch_for(60.0, 0.75), 3 * HZ);
    assert_eq!(first_armor_result(&ev), Some(0), "{ev:?}");
}

#[test]
fn the_rocket_drops_hard_past_80m_and_is_removed() {
    // The shooter stands on a plateau (x < 410) above lower ground, so the arc is not cut off.
    let col = synth_data(30, 6, &|x, _| if x < 410.0 { 5.0 } else { 0.0 });
    let mut w = duel_world(
        GameData::embedded(),
        &col,
        4,
        &[(5u8, Team::Ger, 400.0, 160.0, 16384u16)],
    );
    w.debug_set_rockets(5, 1);
    let pitch = pitch_for(60.0, 0.0);
    let mut track: Vec<Vec3> = Vec::new();
    for t in 0..(4 * HZ) {
        let mut c = cmd(0, 0, 16384, 0, 0);
        c.pitch = pitch_code(pitch);
        if t == 1 {
            c.buttons = buttons::FIRE;
        }
        w.step(&[(5, c)]);
        track.extend(w.shell_positions().into_iter().map(|(_, p)| p));
    }
    let y0 = track[0].y;
    let drop_at = |d: f32| {
        track
            .iter()
            .find(|p| p.x - 400.0 >= d)
            .map(|p| y0 - p.y)
            .unwrap()
    };
    // Zeroed for 60 m: level there, about 4 m low by 80 m (analytic 3.97 m), then removed.
    assert!(drop_at(60.0).abs() < 0.7, "{}", drop_at(60.0));
    assert!(drop_at(78.0) > 1.5, "{}", drop_at(78.0));
    assert!(
        track.last().unwrap().x - 400.0 <= 82.0,
        "flew past max range"
    );
    assert!(w.shell_positions().is_empty(), "still flying");
}

#[test]
fn six_rockets_knock_out_a_sherman() {
    let sh = [
        (5u8, 440.0, 100.0, 16384u16),
        (6, 560.0, 100.0, 49152),
        (7, 440.0, 98.0, 16384),
        (8, 560.0, 102.0, 49152),
        (9, 440.0, 102.0, 16384),
        (10, 560.0, 98.0, 49152),
    ];
    let (mut w, tank) = world(&sh);
    let ev = fire_all(&mut w, &sh, pitch_for(60.0, 0.75), 3 * HZ);
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::VehicleKnockedOut { vehicle } if *vehicle == tank)),
        "{:?}",
        w.vehicle(tank).map(|v| (v.hull_hp, v.modules.hp.to_vec()))
    );
}
