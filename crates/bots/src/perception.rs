//! What a bot can see: view cone, range, line of sight, distance-dependent recognition.
//!
//! The only way to learn about an enemy is [`can_see`] / [`scan`]: both require the enemy to be
//! inside the horizontal view cone, within the difficulty's view distance and in clear line of
//! sight of the bot's eye (GAMEDESIGN 20.3 B-05, ARCHITECTURE 16.3). Nothing here reads health,
//! weapons or any other state of an enemy, and enemies that fail the test are not reported.

use frontline_sim::rng::{unit_f32, Pcg64Mcg};
use frontline_sim::view::PlayerView;
use frontline_sim::{PlayerId, Vec3, WorldView};

use crate::difficulty::{Difficulty, Tuning};

/// An audible location, not a player track. Never confers permission to shoot.
#[derive(Debug, Clone, Copy)]
pub struct Sound {
    pub pos: Vec3,
    pub tick: u32,
    pub range_m: f32,
}

/// Height of the aimed-at point above an enemy's feet (centre of the torso capsule).
pub const TORSO_Y: f32 = 1.23;
/// Height of the head-sphere centre above the feet (standing).
pub const HEAD_Y: f32 = 1.66;
/// Eye height above the feet when standing / crouching (role data: 1.65 / 1.05).
pub const EYE_STAND_M: f32 = 1.65;
pub const EYE_CROUCH_M: f32 = 1.05;

/// An enemy the bot currently perceives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    pub player: PlayerId,
    /// Feet position at the time of sight.
    pub pos: Vec3,
    pub dist: f32,
}

/// Effective distance an enemy can be seen from.
pub fn view_distance(t: &Tuning, d: &Difficulty) -> f32 {
    d.view_distance_m.min(t.view_distance_cap_m)
}

/// Recognition probability per scan: `clamp(1 - dist/viewdist, min, 1)`.
pub fn recognition_probability(dist: f32, view_dist: f32, min_p: f32) -> f32 {
    if view_dist <= 0.0 {
        return 0.0;
    }
    (1.0 - dist / view_dist).clamp(min_p, 1.0)
}

/// Whether `target` (feet position) lies in the horizontal view cone of an observer at `eye`
/// looking along `yaw` (0 = +z, growing toward +x).
pub fn in_view_cone(eye: Vec3, yaw: f32, target: Vec3, fov_deg: f32) -> bool {
    let (dx, dz) = (target.x - eye.x, target.z - eye.z);
    let h2 = dx * dx + dz * dz;
    if h2 < 1.0 {
        return true; // practically on top of the observer
    }
    let (fx, fz) = (libm::sinf(yaw), libm::cosf(yaw));
    let cos_angle = (dx * fx + dz * fz) / libm::sqrtf(h2);
    cos_angle >= libm::cosf(0.5 * fov_deg.to_radians())
}

/// Range + cone + line of sight (to the torso, else the head). No random roll.
pub fn can_see(
    view: &dyn WorldView,
    eye: Vec3,
    yaw: f32,
    target_feet: Vec3,
    t: &Tuning,
    d: &Difficulty,
) -> bool {
    let dist = (target_feet + Vec3::Y * TORSO_Y - eye).length();
    if dist > view_distance(t, d) {
        return false;
    }
    if !in_view_cone(eye, yaw, target_feet, t.fov_deg) {
        return false;
    }
    view.los(eye, target_feet + Vec3::Y * TORSO_Y) || view.los(eye, target_feet + Vec3::Y * HEAD_Y)
}

/// Enemies recognised by this scan, nearest first. Every enemy that passes the physical test
/// rolls once (in ascending `EntityId` order, so the draw sequence is deterministic).
pub fn scan(
    view: &dyn WorldView,
    me: &PlayerView,
    eye: Vec3,
    t: &Tuning,
    d: &Difficulty,
    rng: &mut Pcg64Mcg,
    out: &mut Vec<Contact>,
) {
    out.clear();
    let vd = view_distance(t, d);
    for p in view.players() {
        if p.team == me.team || p.player == me.player {
            continue;
        }
        let to = p.pos + Vec3::Y * TORSO_Y - eye;
        let dist2 = to.length_squared();
        if dist2 > vd * vd {
            continue;
        }
        if !can_see(view, eye, me.yaw, p.pos, t, d) {
            continue;
        }
        let dist = libm::sqrtf(dist2);
        let prob = recognition_probability(dist, vd, t.min_recognition);
        if unit_f32(rng) < prob {
            out.push(Contact {
                player: p.player,
                pos: p.pos,
                dist,
            });
        }
    }
    out.sort_by(|a, b| a.dist.total_cmp(&b.dist).then(a.player.cmp(&b.player)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probability_curve() {
        assert_eq!(recognition_probability(0.0, 200.0, 0.1), 1.0);
        assert!((recognition_probability(100.0, 200.0, 0.1) - 0.5).abs() < 1e-6);
        assert_eq!(recognition_probability(199.0, 200.0, 0.1), 0.1);
        assert_eq!(recognition_probability(500.0, 200.0, 0.1), 0.1);
    }

    #[test]
    fn cone_is_120_degrees() {
        let eye = Vec3::new(0.0, 1.65, 0.0);
        // Looking along +z: targets within +-60 deg count, beyond do not.
        let at = |deg: f32| {
            Vec3::new(
                libm::sinf(deg.to_radians()) * 50.0,
                0.0,
                libm::cosf(deg.to_radians()) * 50.0,
            )
        };
        assert!(in_view_cone(eye, 0.0, at(0.0), 120.0));
        assert!(in_view_cone(eye, 0.0, at(59.0), 120.0));
        assert!(in_view_cone(eye, 0.0, at(-59.0), 120.0));
        assert!(!in_view_cone(eye, 0.0, at(61.0), 120.0));
        assert!(!in_view_cone(eye, 0.0, at(-61.0), 120.0));
        assert!(!in_view_cone(eye, 0.0, at(180.0), 120.0));
        // Rotated view: looking along +x (yaw = 90 deg).
        assert!(in_view_cone(
            eye,
            core::f32::consts::FRAC_PI_2,
            at(90.0),
            120.0
        ));
        assert!(!in_view_cone(
            eye,
            core::f32::consts::FRAC_PI_2,
            at(0.0 + 180.0),
            120.0
        ));
    }
}
