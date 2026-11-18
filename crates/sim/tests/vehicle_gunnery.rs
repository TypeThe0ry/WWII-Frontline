//! M1-023c: a gunner seated in a Sherman fires at an enemy Sherman: shell flight, armour
//! hit events, reload timing, knock-out and the burning wreck.
mod common;

use common::*;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::squad::{SquadKind, SquadOp};
use frontline_sim::{GameData, SimEvent, Team, World};
use glam::Vec3;

const HZ: u32 = 30;
const ID: &str = "us_m4a1_75";

/// US tank at (500, 100) facing +Z with crewmen 0 (driver) and 1 (gunner); German tank 200 m
/// ahead, also facing +Z (so the shot meets its rear).
fn fx() -> (World, u16, u16) {
    let col = synth_data(30, 6, &|_, _| 5.0);
    let players = [
        (0u8, Team::Us, 500.0, 104.0, 0u16),
        (1, Team::Us, 502.0, 104.0, 0),
        (5, Team::Ger, 900.0, 100.0, 0),
    ];
    let mut w = duel_world(GameData::embedded(), &col, 9, &players);
    for p in [0u8, 1] {
        w.roster_add(p, Team::Us).unwrap();
    }
    w.roster_add(5, Team::Ger).unwrap();
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
    let us = w
        .spawn_vehicle(ID, Team::Us, Vec3::new(500.0, 5.9, 100.0), 0.0)
        .unwrap();
    let ger = w
        .spawn_vehicle(ID, Team::Ger, Vec3::new(500.0, 5.9, 300.0), 0.0)
        .unwrap();
    for _ in 0..120 {
        w.step(&[]);
    }
    w.seat_enter(0, us, 0).unwrap();
    w.seat_enter(1, us, 1).unwrap();
    for _ in 0..HZ * 3 {
        w.step(&[]);
    }
    (w, us, ger)
}

fn gunner_cmd(fire: bool) -> InputCmd {
    cmd(0, 0, 0, if fire { buttons::FIRE } else { 0 }, 0)
}

fn step(w: &mut World, fire: bool, ticks: u32) -> Vec<SimEvent> {
    let mut ev = Vec::new();
    for _ in 0..ticks {
        let inputs: Vec<_> = w
            .view()
            .players()
            .iter()
            .map(|p| {
                (
                    p.player,
                    if p.player == 1 {
                        gunner_cmd(fire)
                    } else {
                        cmd(0, 0, 0, 0, 0)
                    },
                )
            })
            .collect();
        ev.extend(w.step(&inputs).events);
    }
    ev
}

#[test]
fn shot_hits_the_rear_and_reload_is_3_9_s_without_commander() {
    let (mut w, _us, ger) = fx();
    let mut ev = step(&mut w, true, 1);
    assert!(ev
        .iter()
        .any(|e| matches!(e, SimEvent::VehicleFired { shell: 0, .. })));
    ev.extend(step(&mut w, false, HZ));
    let hits: Vec<_> = ev
        .iter()
        .filter_map(|e| match e {
            SimEvent::ArmorHit {
                vehicle, result, ..
            } if *vehicle == ger => Some(*result),
            _ => None,
        })
        .collect();
    assert_eq!(hits.len(), 1, "{ev:?}");
    assert!(w.vehicle(ger).unwrap().hull_hp < 1600.0);
    // Holding fire: no second shot until 3.9 s (117 ticks at 30 Hz) after the first.
    let early = step(&mut w, true, 117 - HZ - 3);
    assert!(!early
        .iter()
        .any(|e| matches!(e, SimEvent::VehicleFired { .. })));
    let late = step(&mut w, true, 8);
    assert!(late
        .iter()
        .any(|e| matches!(e, SimEvent::VehicleFired { .. })));
}

#[test]
fn repeated_hits_knock_the_tank_out_and_the_wreck_burns_out() {
    let (mut w, _us, ger) = fx();
    let mut all = Vec::new();
    for _ in 0..40 {
        all.extend(step(&mut w, true, 120));
        if w.vehicle(ger).unwrap().knocked_out {
            break;
        }
    }
    assert!(w.vehicle(ger).unwrap().knocked_out, "never knocked out");
    assert!(all
        .iter()
        .any(|e| matches!(e, SimEvent::VehicleKnockedOut { vehicle } if *vehicle == ger)));
    all.extend(step(&mut w, false, 10 * HZ));
    assert!(
        all.iter()
            .any(|e| matches!(e, SimEvent::Explosion { kind, .. } if *kind == 2 || *kind == 3)),
        "no wreck explosion"
    );
}

#[test]
fn deterministic_over_a_firefight() {
    let run = || {
        let (mut w, ..) = fx();
        for _ in 0..6 {
            step(&mut w, true, 120);
        }
        w.state_hash()
    };
    assert_eq!(run(), run());
}
