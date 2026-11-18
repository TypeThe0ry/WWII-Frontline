//! M1-041b: client driver prediction (`PredictSim::step_vehicle_driver`) vs the server step of the same vehicle.
mod common;

use common::*;
use frontline_sim::predict::PredictSim;
use frontline_sim::vehicle::ctrl_from_cmd;
use frontline_sim::{GameData, Team, World};
use glam::Vec3;

const ID: &str = "us_m4a1_75";

fn world(f: &dyn Fn(f32, f32) -> f32) -> World {
    World::new(sim_cfg(GameData::embedded(), 3), &synth_data(30, 6, f))
}

/// Script of driver commands: accelerate, turn, hold, reverse, handbrake.
fn script(t: u32) -> frontline_protocol::msg::input::InputCmd {
    match t {
        0..=89 => cmd(0, 127, 0, 0, 0),
        90..=179 => cmd(80, 100, 0, 0, 0),
        180..=239 => cmd(-100, 127, 0, 0, 0),
        240..=299 => cmd(0, -90, 0, 0, 0),
        _ => cmd(0, 0, 0, frontline_protocol::msg::input::buttons::JUMP, 0),
    }
}

fn run(f: &dyn Fn(f32, f32) -> f32) -> (u32, f32) {
    let mut w = world(f);
    let id = w
        .spawn_vehicle(ID, Team::Us, Vec3::new(300.0, 6.2, 100.0), 0.0)
        .unwrap();
    let mut p = PredictSim::new(sim_cfg(GameData::embedded(), 3), &synth_data(30, 6, f), 1);
    p.reset_vehicle(w.vehicle(id).cloned());
    let start = w.vehicle(id).unwrap().pos;
    let (mut exact, mut worst) = (0u32, 0f32);
    for t in 0..360 {
        let c = script(t);
        w.set_vehicle_ctrl(id, ctrl_from_cmd(&c));
        w.step(&[]);
        let pv = p.step_vehicle_driver(&c).expect("vehicle set");
        let sv = w.vehicle(id).unwrap();
        let err = (pv.pos - sv.pos).length();
        worst = worst.max(err);
        if pv.pos == sv.pos && pv.vel == sv.vel && pv.rot == sv.rot {
            exact += 1;
        }
    }
    // The script really drives the tank (not a trivially identical standstill).
    assert!((w.vehicle(id).unwrap().pos - start).length() > 15.0);
    (exact, worst)
}

#[test]
fn flat_ground_prediction_is_bit_identical_to_the_server_step() {
    let (exact, worst) = run(&|_, _| 5.0);
    assert_eq!((exact, worst), (360, 0.0));
}

#[test]
fn rolling_terrain_prediction_matches_too() {
    let (exact, worst) = run(&|x, z| 5.0 + 0.6 * ((x * 0.05).sin() + (z * 0.04).cos()));
    assert_eq!((exact, worst), (360, 0.0));
}

#[test]
fn no_vehicle_means_no_prediction() {
    let mut p = PredictSim::new(
        sim_cfg(GameData::embedded(), 3),
        &synth_data(30, 6, &|_, _| 5.0),
        1,
    );
    assert!(p.step_vehicle_driver(&cmd(0, 127, 0, 0, 0)).is_none());
}
