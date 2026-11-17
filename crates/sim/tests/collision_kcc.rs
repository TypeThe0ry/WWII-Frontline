//! `slope_limit_35deg`: Rapier's KinematicCharacterController on our terrain colliders.
mod common;

use common::*;
use frontline_sim::CollisionData;
use rapier3d::control::{CharacterLength, KinematicCharacterController};
use rapier3d::prelude::{Pose, SharedShape, Vector};

const R: f32 = 0.35;
const H: f32 = 1.8;
const SPEED: f32 = 4.4;
const DT: f32 = 1.0 / 30.0;

/// Pushes a 0.35 m x 1.8 m capsule along `dir` (xz) for 5 s with gravity. Returns the
/// final foot position (x, y, z).
fn push(data: &CollisionData, start_xz: (f32, f32), dir: (f32, f32)) -> (f32, f32, f32) {
    let shape = SharedShape::capsule_y(H * 0.5 - R, R);
    let kcc = KinematicCharacterController {
        max_slope_climb_angle: 35.0f32.to_radians(),
        min_slope_slide_angle: 35.0f32.to_radians(),
        snap_to_ground: Some(CharacterLength::Absolute(0.3)),
        ..Default::default()
    };
    let ground = data.height_at(start_xz.0, start_xz.1).unwrap();
    let mut pos = Vector::new(start_xz.0, ground + H * 0.5 + 0.02, start_xz.1);
    let mut vy = 0.0f32;
    let world = data.physics().world();
    for _ in 0..150 {
        let want = Vector::new(dir.0 * SPEED * DT, vy * DT, dir.1 * SPEED * DT);
        let qp = world.query_pipeline();
        let m = kcc.move_shape(
            DT,
            &qp,
            shape.as_ref(),
            &Pose::from_translation(pos),
            want,
            |_| {},
        );
        pos += m.translation;
        vy = if m.grounded { 0.0 } else { vy - 9.81 * DT };
    }
    (pos.x, pos.y - H * 0.5, pos.z)
}

/// Flat 2 m plain for z < 40, then a ramp of `rise` metres at `deg`, then a plateau.
fn ramp(deg: f32, rise: f32) -> CollisionData {
    let k = deg.to_radians().tan();
    synth_data(2, 2, &move |_, z| 2.0 + (k * (z - 40.0)).clamp(0.0, rise))
}

#[test]
fn slope_limit_35deg() {
    // Steep ramps (> 35 degrees) cannot be climbed: at most 0.5 m above the foot of the slope.
    for deg in [36.0f32, 40.0, 45.0, 55.0] {
        let d = ramp(deg, 3.0);
        let (_, y, z) = push(&d, (64.0, 30.0), (0.0, 1.0));
        let gain = y - 2.0;
        println!("{deg} deg ramp: final z {z:.2}, height gain {gain:.3} m");
        assert!(gain <= 0.5, "{deg} deg ramp climbed: gain {gain}");
    }
    // Gentle ramps (<= 30 degrees) are climbed in full (>= 90 % of the 3 m rise).
    for deg in [10.0f32, 20.0, 25.0, 30.0] {
        // Ramp at most ~8 m long so that the 22 m walk (10 m flat first) reaches the plateau.
        let rise = (8.0 * deg.to_radians().tan()).min(3.0);
        let d = ramp(deg, rise);
        let (_, y, z) = push(&d, (64.0, 30.0), (0.0, 1.0));
        let gain = y - 2.0;
        println!("{deg} deg ramp (rise {rise:.2} m): final z {z:.2}, height gain {gain:.3} m");
        assert!(
            gain >= 0.9 * rise,
            "{deg} deg ramp not climbed: gain {gain} of {rise}"
        );
    }
}

#[test]
fn slope_limit_35deg_on_m0_terrain() {
    let Some(d) = real_terrain("slope_limit_35deg_on_m0_terrain") else {
        return;
    };
    // The M0 embankment is a 38.7 degree, 8 m high ramp over z = 123..133.
    let (_, y, z) = push(&d, (128.0, 108.0), (0.0, 1.0));
    let base = d.height_at(128.0, 118.0).unwrap();
    println!(
        "M0 embankment (38.7 deg): final z {z:.2}, height above foot of slope {:.3} m",
        y - base
    );
    assert!(y - base <= 0.5, "climbed the embankment: {}", y - base);
    // Its gentle side (rising along +x, <= 27 degrees) is climbable.
    let (x, y, z) = push(&d, (10.0, 128.0), (1.0, 0.0));
    let start = d.height_at(10.0, 128.0).unwrap();
    let end = d.height_at(x, z).unwrap();
    println!(
        "M0 gentle side: travelled {:.2} m in x, climbed {:.3} m, foot {y:.3} vs terrain {end:.3}",
        x - 10.0,
        y - start
    );
    assert!(end - start > 2.0, "test must exercise a real climb");
    assert!(x - 10.0 > 0.9 * SPEED * 5.0, "stalled on a gentle slope");
    assert!((y - end).abs() < 0.2, "foot does not follow the terrain");
}
