//! Thin wrapper over Rapier's `KinematicCharacterController` (ARCHITECTURE 5.3).
//!
//! The capsule is built on the stack for every call (its height changes with the stance), so
//! there is no allocation. Terrain colliders are the per-chunk height fields of
//! [`CollisionData::physics`].

use glam::Vec3;
use rapier3d::control::{CharacterAutostep, CharacterLength, KinematicCharacterController};
use rapier3d::parry::shape::Capsule;
use rapier3d::prelude::{Pose, Vector};

use super::params::MoveParams;
use crate::collision::CollisionData;

/// Skin width kept between capsule and ground (numerical, not a gameplay value).
const SKIN_M: f32 = 0.01;

/// Result of one KCC move.
#[derive(Debug, Clone, Copy)]
pub struct KccMove {
    /// Applied translation (may differ from the desired one: sliding, snapping).
    pub translation: Vec3,
    pub grounded: bool,
}

/// Moves a capsule whose bottom is at `foot` and whose total height is `height` by
/// `desired`. Gravity/jump are the caller's business; when the character was grounded and
/// `desired.y <= 0`, the controller snaps it down to the ground (`snap_to_ground_m`).
pub fn move_capsule(
    col: &CollisionData,
    p: &MoveParams,
    foot: Vec3,
    height: f32,
    desired: Vec3,
) -> KccMove {
    let half_cyl = (height * 0.5 - p.radius_m).max(0.0);
    let shape = Capsule::new_y(half_cyl, p.radius_m);
    let kcc = KinematicCharacterController {
        offset: CharacterLength::Absolute(SKIN_M),
        max_slope_climb_angle: p.max_slope_rad,
        min_slope_slide_angle: p.max_slope_rad,
        snap_to_ground: Some(CharacterLength::Absolute(p.snap_to_ground_m)),
        autostep: if false {
            Some(CharacterAutostep {
                max_height: CharacterLength::Absolute(p.autostep_max_m),
                min_width: CharacterLength::Absolute(p.autostep_min_width_m),
                include_dynamic_bodies: false,
            })
        } else {
            None
        },
        ..Default::default()
    };
    let centre = Vector::new(foot.x, foot.y + height * 0.5, foot.z);
    let qp = col.physics().world().query_pipeline();
    let m = kcc.move_shape(
        p.dt,
        &qp,
        &shape,
        &Pose::from_translation(centre),
        Vector::new(desired.x, desired.y, desired.z),
        |_| {},
    );
    KccMove {
        translation: Vec3::new(m.translation.x, m.translation.y, m.translation.z),
        grounded: m.grounded,
    }
}
