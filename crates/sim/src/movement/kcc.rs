//! Thin wrapper over Rapier's `KinematicCharacterController` (ARCHITECTURE 5.3).
//!
//! The capsule is built on the stack for every call (its height changes with the stance), so
//! there is no allocation. Terrain colliders are the per-chunk height fields of
//! [`CollisionData::physics`].
//!
//! **Ground snapping is ours, not Rapier's.** Rapier's `snap_to_ground` is one more shape cast
//! against the height fields and doubled the cost of a move (measured ~41 us -> ~19 us per
//! call on the dev machine, PLAN_ISSUES PI-014). The terrain is analytic, so after the move we
//! put the capsule back onto the surface with [`CollisionData::ground_probe`] instead: for
//! a capsule resting on a plane with unit normal `n` the foot is
//! `surface + r (1 / n.y - 1) + hover / n.y` above the surface point under the axis.

use glam::Vec3;
use rapier3d::control::{CharacterAutostep, CharacterLength, KinematicCharacterController};
use rapier3d::parry::shape::Capsule;
use rapier3d::prelude::{Pose, Vector};

use super::params::MoveParams;
use crate::collision::CollisionData;

/// Skin width kept between capsule and ground (numerical, not a gameplay value).
const SKIN_M: f32 = 0.01;
/// Resting hover of the snapped capsule above the contact surface. Resting exactly at the skin
/// width makes Rapier's shape casts start in contact on every move (extra slide iterations,
/// ~+50 % cost); 3 cm keeps it out of contact and is invisible on screen.
const REST_HOVER_M: f32 = 0.03;
/// Snap only when at least this far above the resting height (hysteresis).
const SNAP_EPS_M: f32 = 0.005;

/// Result of one KCC move.
#[derive(Debug, Clone, Copy)]
pub struct KccMove {
    /// Applied translation (may differ from the desired one: sliding, snapping).
    pub translation: Vec3,
    pub grounded: bool,
}

/// Moves a capsule whose bottom is at `foot` and whose total height is `height` by
/// `desired`. Gravity/jump are the caller's business. `was_grounded` enables the snap: when
/// the character stood on the ground and `desired.y <= 0`, it is put back down onto the surface
/// (resting hover = [`REST_HOVER_M`]) if the ground is within `snap_to_ground_m` below it.
pub fn move_capsule(
    col: &CollisionData,
    p: &MoveParams,
    foot: Vec3,
    height: f32,
    desired: Vec3,
    was_grounded: bool,
) -> KccMove {
    let half_cyl = (height * 0.5 - p.radius_m).max(0.0);
    let shape = Capsule::new_y(half_cyl, p.radius_m);
    let kcc = KinematicCharacterController {
        offset: CharacterLength::Absolute(SKIN_M),
        max_slope_climb_angle: p.max_slope_rad,
        min_slope_slide_angle: p.max_slope_rad,
        snap_to_ground: None,
        autostep: if p.autostep_max_m > 0.0 {
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
    let mut translation = Vec3::new(m.translation.x, m.translation.y, m.translation.z);
    let mut grounded = m.grounded;
    if was_grounded && desired.y <= 0.0 && translation.y <= 0.0 {
        let end = foot + translation;
        let range = p.snap_to_ground_m + height;
        let terrain = col.ground_probe(end, range).map(|g| (g.point.y, g.normal));
        let building = col
            .vertical_building_hit(end, false, range)
            .map(|(t, normal)| (end.y - t, normal));
        // Never snap to terrain through a floor or ramp. The analytic height field alone
        // cannot distinguish the stacked surfaces of a building.
        let support = match building {
            Some(b) if terrain.is_none_or(|(y, _)| b.0 >= y) => Some(b),
            _ => terrain,
        };
        if let Some((y, normal)) = support {
            let ny = normal.y.max(1e-3);
            let rest = y + p.radius_m * (1.0 / ny - 1.0) + REST_HOVER_M / ny;
            let drop = end.y - rest;
            if drop > SNAP_EPS_M && drop <= p.snap_to_ground_m {
                translation.y -= drop;
                grounded = true;
            }
        }
    }
    KccMove {
        translation,
        grounded,
    }
}
