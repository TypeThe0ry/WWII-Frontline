//! M1-022a: suspension, track drive, top speed, turning, slopes, determinism.
mod common;

use common::*;
use frontline_sim::vehicle::VehicleCtrl;
use frontline_sim::{GameData, Team, World};
use glam::{Quat, Vec3};

const HZ: u32 = 30;
const ID: &str = "us_m4a1_75";
const KPH: f32 = 1.0 / 3.6;

fn world(f: &dyn Fn(f32, f32) -> f32) -> World {
    World::new(sim_cfg(GameData::embedded(), 3), &synth_data(30, 6, f))
}

fn flat_world() -> World {
    world(&|_, _| 5.0)
}

fn ctrl(throttle: f32, steer: f32) -> VehicleCtrl {
    VehicleCtrl {
        throttle,
        steer,
        boost: false,
        brake: false,
    }
}

fn run(w: &mut World, secs: f32) {
    for _ in 0..(secs * HZ as f32) as u32 {
        w.step(&[]);
    }
}

#[test]
fn spawn_errors() {
    let mut w = flat_world();
    assert!(w
        .spawn_vehicle("nope", Team::Us, Vec3::new(100.0, 6.0, 100.0), 0.0)
        .is_err());
}

#[test]
fn suspension_settles_and_recovers_from_a_drop() {
    let mut w = flat_world();
    let id = w
        .spawn_vehicle(ID, Team::Us, Vec3::new(500.0, 5.9, 100.0), 0.0)
        .unwrap();
    run(&mut w, 6.0);
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for _ in 0..(6 * HZ) {
        w.step(&[]);
        let y = w.vehicle(id).unwrap().pos.y;
        lo = lo.min(y);
        hi = hi.max(y);
    }
    assert!(hi - lo < 0.02, "resting amplitude {} m", hi - lo);
    let rest = w.vehicle(id).unwrap().pos.y;
    // Drop from 0.5 m above the resting height.
    w.debug_place_vehicle(id, Vec3::new(500.0, rest + 0.5, 100.0), Quat::IDENTITY);
    run(&mut w, 2.0);
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for _ in 0..HZ {
        w.step(&[]);
        let y = w.vehicle(id).unwrap().pos.y;
        lo = lo.min(y);
        hi = hi.max(y);
    }
    assert!(hi - lo < 0.02, "after the drop {} m", hi - lo);
    assert!((w.vehicle(id).unwrap().pos.y - rest).abs() < 0.03);
    assert_eq!(w.vehicle(id).unwrap().grounded_rays, 12);
}

#[test]
fn top_speed_matches_data_within_three_percent() {
    let mut w = flat_world();
    let id = w
        .spawn_vehicle(ID, Team::Us, Vec3::new(100.0, 5.9, 100.0), 0.0)
        .unwrap();
    // Faces +Z; give it a long run.
    let want = GameData::embedded()
        .vehicle(ID)
        .unwrap()
        .gameplay
        .max_speed_mps
        .unwrap() as f32;
    assert!((want / KPH - 28.5).abs() < 0.1, "data is 28.5 km/h");
    w.set_vehicle_ctrl(id, ctrl(1.0, 0.0));
    run(&mut w, 8.0);
    let mut sum = 0.0;
    for _ in 0..(2 * HZ) {
        w.step(&[]);
        sum += w.vehicle(id).unwrap().forward_speed();
    }
    let avg = sum / (2 * HZ) as f32;
    assert!(
        (avg - want).abs() / want < 0.03,
        "avg {} km/h vs {}",
        avg / KPH,
        want / KPH
    );
}

#[test]
fn acceleration_to_ninety_percent_takes_six_to_twelve_seconds() {
    let mut w = flat_world();
    let id = w
        .spawn_vehicle(ID, Team::Us, Vec3::new(100.0, 5.9, 100.0), 0.0)
        .unwrap();
    let vmax = GameData::embedded()
        .vehicle(ID)
        .unwrap()
        .gameplay
        .max_speed_mps
        .unwrap() as f32;
    run(&mut w, 2.0);
    w.set_vehicle_ctrl(id, ctrl(1.0, 0.0));
    let mut t = None;
    for k in 0..(20 * HZ) {
        w.step(&[]);
        if w.vehicle(id).unwrap().forward_speed() >= 0.9 * vmax {
            t = Some(k as f32 / HZ as f32);
            break;
        }
    }
    let t = t.expect("never reached 90%");
    assert!((6.0..=12.0).contains(&t), "0->90% in {t} s");
}

#[test]
fn pivot_turn_is_thirty_degrees_per_second() {
    let mut w = flat_world();
    let id = w
        .spawn_vehicle(ID, Team::Us, Vec3::new(500.0, 5.9, 100.0), 0.0)
        .unwrap();
    run(&mut w, 3.0);
    w.set_vehicle_ctrl(id, ctrl(0.0, 1.0));
    run(&mut w, 3.0);
    let rate = w.vehicle(id).unwrap().yaw_rate().to_degrees();
    // Steer right = clockwise from above = negative rate about +Y.
    assert!((rate + 30.0).abs() < 3.0, "yaw rate {rate} deg/s");
    let p = w.vehicle(id).unwrap().pos;
    assert!(((p.x - 500.0).hypot(p.z - 100.0)) < 1.0, "pivots in place");
    w.set_vehicle_ctrl(id, ctrl(0.0, -1.0));
    run(&mut w, 4.0);
    let rate = w.vehicle(id).unwrap().yaw_rate().to_degrees();
    assert!((rate - 30.0).abs() < 3.0, "left {rate}");
}

fn slope_world(deg: f32) -> (World, u16) {
    let t = deg.to_radians().tan();
    let mut w = world(&move |x, _| 5.0 + (x - 100.0).max(0.0) * t);
    let rot = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
        * Quat::from_rotation_x(-deg.to_radians());
    let id = w
        .spawn_vehicle(ID, Team::Us, Vec3::new(120.0, 7.0, 200.0), 0.0)
        .unwrap();
    let y = 5.0 + 20.0 * t + 0.9;
    w.debug_place_vehicle(id, Vec3::new(120.0, y, 200.0), rot);
    (w, id)
}

#[test]
fn climbs_25_degrees_but_not_32() {
    let (mut w, id) = slope_world(25.0);
    run(&mut w, 2.0);
    let x0 = w.vehicle(id).unwrap().pos.x;
    w.set_vehicle_ctrl(id, ctrl(1.0, 0.0));
    run(&mut w, 10.0);
    let v = w.vehicle(id).unwrap();
    assert!(v.pos.x - x0 > 15.0, "25 deg: moved {} m", v.pos.x - x0);

    let (mut w, id) = slope_world(32.0);
    run(&mut w, 2.0);
    let x0 = w.vehicle(id).unwrap().pos.x;
    w.set_vehicle_ctrl(id, ctrl(1.0, 0.0));
    run(&mut w, 10.0);
    let v = w.vehicle(id).unwrap();
    let dx = v.pos.x - x0;
    assert!(dx < 1.0 && dx > -3.0, "32 deg: moved {dx} m");
    assert!(v.vel.length() < 0.6, "holds, speed {}", v.vel.length());
}

#[test]
fn drives_straight_and_reverses_at_forty_percent() {
    let mut w = flat_world();
    let id = w
        .spawn_vehicle(ID, Team::Us, Vec3::new(500.0, 5.9, 100.0), 0.0)
        .unwrap();
    w.set_vehicle_ctrl(id, ctrl(1.0, 0.0));
    run(&mut w, 20.0);
    let v = w.vehicle(id).unwrap();
    assert!((v.pos.x - 500.0).abs() < 0.2, "drift {}", v.pos.x - 500.0);
    w.set_vehicle_ctrl(id, ctrl(-1.0, 0.0));
    run(&mut w, 12.0);
    let v = w.vehicle(id).unwrap();
    let vmax = GameData::embedded()
        .vehicle(ID)
        .unwrap()
        .gameplay
        .max_speed_mps
        .unwrap() as f32;
    assert!(
        (v.forward_speed() + 0.4 * vmax).abs() < 0.1 * vmax,
        "{}",
        v.forward_speed()
    );
    // Brake stops it.
    w.set_vehicle_ctrl(
        id,
        VehicleCtrl {
            brake: true,
            ..Default::default()
        },
    );
    run(&mut w, 4.0);
    assert!(w.vehicle(id).unwrap().vel.length() < 0.2);
}

#[test]
fn deterministic_with_ten_vehicles() {
    let run_once = || {
        let mut w = flat_world();
        let mut ids = Vec::new();
        for i in 0..10 {
            ids.push(
                w.spawn_vehicle(
                    ID,
                    Team::Us,
                    Vec3::new(100.0 + 30.0 * i as f32, 5.9, 100.0),
                    0.0,
                )
                .unwrap(),
            );
        }
        let mut rng = Rng(7);
        for tick in 0..1500 {
            if tick % 20 == 0 {
                for &id in &ids {
                    w.set_vehicle_ctrl(id, ctrl(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0)));
                }
            }
            w.step(&[]);
        }
        w.state_hash()
    };
    assert_eq!(run_once(), run_once());
}
