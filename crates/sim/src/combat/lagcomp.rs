//! Lag-compensation time selection (M1-025, ARCHITECTURE 6.7). The server shooter fires from its
//! authoritative pose along the reported aim; only the *targets* are looked up in the hitbox
//! history, `rewind_ticks` behind the current tick. Rules (game rules live here, not in `server`):
//! rewind is clamped to [`MAX_REWIND_TICKS`], a `view_tick` (a delta in ticks) of 0 means "no view
//! time reported" (scripted fire, prediction) and rewinds nothing, and an origin hint further than
//! [`ORIGIN_TOLERANCE_M`] from the authoritative muzzle is counted. Hints never move the muzzle.

use bevy_ecs::prelude::{Resource, World};
use glam::Vec3;

use serde::Deserialize;

use super::lagcomp_history::{LagCompHistory, PlayerHitboxes, VehicleHitboxes};
use crate::collision::CollisionData;
use crate::components::{CurrentTick, EntityTable, Motion, MoveParamsRes, Transform};
use crate::health::Health;
use crate::ids::Tick;
use crate::vehicle::Vehicles;
use crate::weapon::ballistics::SUBSTEPS;
use crate::weapon::hitbox;

/// Longest rewind (about 266 ms at 30 Hz; T2 caps at 250 ms).
pub const MAX_REWIND_TICKS: u8 = 8;
/// Largest accepted distance between the client's `origin_hint` and the authoritative origin.
pub const ORIGIN_TOLERANCE_M: f32 = 0.5;

/// Outcome of [`resolve_fire`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FireResolution {
    pub rewind_ticks: u8,
    pub origin: Vec3,
    pub clamped: bool,
    pub origin_corrected: bool,
}

/// Ticks to rewind for a shot whose client rendered the world `view_delta` ticks behind its own
/// tick (`InputCmd.view_tick` is that difference, ARCHITECTURE §6.7.1). 0 means "no view time
/// reported". Returns `(used, clamped)`.
pub fn rewind_ticks(view_delta: u16) -> (u8, bool) {
    if view_delta > u16::from(MAX_REWIND_TICKS) {
        (MAX_REWIND_TICKS, true)
    } else {
        (view_delta as u8, false)
    }
}

/// Whether an untrusted hint is invalid or more than 0.5 m from the authoritative muzzle.
pub fn origin_needs_correction(origin_hint: Vec3, auth_origin: Vec3) -> bool {
    !origin_hint.is_finite()
        || origin_hint.distance_squared(auth_origin) > ORIGIN_TOLERANCE_M * ORIGIN_TOLERANCE_M
}

/// Time selection and origin check for one fire command. The origin is always authoritative.
pub fn resolve_fire(view_delta: u16, origin_hint: Vec3, auth_origin: Vec3) -> FireResolution {
    let (r, clamped) = rewind_ticks(view_delta);
    FireResolution {
        rewind_ticks: r,
        origin: auth_origin,
        clamped,
        origin_corrected: origin_needs_correction(origin_hint, auth_origin),
    }
}

/// A shot starts at the reported view tick and advances one tick per simulation step.
/// Once that clock reaches the server tick when the shot was accepted, use live collision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileClock {
    ViewTime(Tick),
    Realtime,
}

impl ProjectileClock {
    pub fn at(now: Tick, rewind: u8, substeps_flown: u32) -> Self {
        if substeps_flown / SUBSTEPS >= u32::from(rewind) {
            Self::Realtime
        } else {
            Self::ViewTime(Tick(now.0.wrapping_sub(u32::from(rewind))))
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LagCompStats {
    pub history_clamped: u64,
    pub wall_rejected: u64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LagCompRules {
    pub cover_duration_s: f32,
    pub cover_min_thickness_m: f32,
}

impl Default for LagCompRules {
    fn default() -> Self {
        static RULES: std::sync::OnceLock<LagCompRules> = std::sync::OnceLock::new();
        *RULES.get_or_init(|| {
            let rules: Self =
                serde_json::from_str(include_str!("../../../../data/rules/lagcomp.json"))
                    .expect("valid lag compensation rule data");
            assert!(rules.cover_duration_s.is_finite() && rules.cover_duration_s >= 0.0);
            assert!(rules.cover_min_thickness_m.is_finite() && rules.cover_min_thickness_m > 0.0);
            rules
        })
    }
}

/// World resource: the hitbox history plus the on/off switch (off = no-compensation control).
#[derive(Resource, Debug, Clone)]
pub struct LagComp {
    pub history: LagCompHistory,
    pub enabled: bool,
    pub rules: LagCompRules,
    pub stats: LagCompStats,
}

impl Default for LagComp {
    fn default() -> Self {
        Self {
            history: LagCompHistory::new(),
            enabled: true,
            rules: LagCompRules::default(),
            stats: LagCompStats::default(),
        }
    }
}

impl LagComp {
    /// Record the current simulation pose after the tick. Reuse storage in the ring.
    pub fn record_world(world: &mut World) {
        world.resource_scope(|world, mut lag: bevy_ecs::prelude::Mut<Self>| {
            let params = world.resource::<MoveParamsRes>().0;
            let players = world
                .resource::<EntityTable>()
                .sorted()
                .filter_map(|(id, e)| {
                    let entity = world.get_entity(e).ok()?;
                    let transform = entity.get::<Transform>()?;
                    let motion = entity.get::<Motion>()?;
                    let health = entity.get::<Health>()?;
                    if health.hp <= 0.0 {
                        return None;
                    }
                    let head = params.map_or(Vec3::ZERO, |p| {
                        crate::movement::lean::head_offset(&motion.to_auth(transform), &p)
                    });
                    Some(PlayerHitboxes {
                        id,
                        foot: transform.pos,
                        blend: motion.stance_blend,
                        head_dx: head.x,
                        head_dz: head.z,
                    })
                });
            let vehicles = world
                .resource::<Vehicles>()
                .list
                .values()
                .filter(|v| !v.gone)
                .map(VehicleHitboxes::from);
            lag.history
                .record_frame(world.resource::<CurrentTick>().0, players, vehicles);
        });
    }
}

/// Reject a rewind hit only if every sampled target pose, through one tick older than the
/// duration threshold, was behind thick cover from the shooter's current eye. Missing exact
/// frames do not establish cover. Hold the vantage fixed so shooter movement alone does not
/// establish that the target entered cover.
pub(crate) fn protected_by_cover(
    collision: &CollisionData,
    history: &LagCompHistory,
    rules: LagCompRules,
    now: Tick,
    hz: u8,
    shooter_eye: Vec3,
    target: PlayerHitboxes,
) -> bool {
    let ticks = (rules.cover_duration_s * f32::from(hz)) as u32;
    let oldest_age = ticks.saturating_add(1);
    if (oldest_age as usize) > history.len() {
        return false;
    }
    let covered = |pose: PlayerHitboxes| {
        let torso = hitbox::volumes(pose.blend)[1];
        let chest = pose.foot + Vec3::Y * ((torso.y0 + torso.y1) * 0.5);
        collision.solid_cover_between(shooter_eye, chest, rules.cover_min_thickness_m)
            || thick_terrain_between(collision, shooter_eye, chest, rules.cover_min_thickness_m)
    };
    covered(target)
        && (1..=oldest_age).all(|age| {
            history
                .frame(Tick(now.0.wrapping_sub(age)))
                .and_then(|frame| frame.player(target.id))
                .is_some_and(covered)
        })
}

// The sim collision world exposes a triangulated heightfield. Require a contiguous sampled
// span inside it after the first intersection. Missing terrain is never evidence of a wall.
fn thick_terrain_between(collision: &CollisionData, from: Vec3, to: Vec3, thickness: f32) -> bool {
    let delta = to - from;
    let len = delta.length();
    let Some(hit) = collision.raycast(from, delta, len) else {
        return false;
    };
    if hit.t <= 0.0 || len - hit.t <= thickness {
        return false;
    }
    let dir = delta / len;
    let samples = (thickness / 0.05).ceil().clamp(1.0, 128.0) as u32;
    (1..=samples).all(|n| {
        let point = hit.point + dir * (thickness * n as f32 / samples as f32);
        collision.ground_probe(point, 0.0).is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewind_clamps() {
        assert_eq!(rewind_ticks(0), (0, false));
        assert_eq!(rewind_ticks(3), (3, false));
        assert_eq!(rewind_ticks(8), (8, false));
        assert_eq!(rewind_ticks(12), (8, true));
    }

    #[test]
    fn origin_tolerance_edges() {
        let a = Vec3::new(10.0, 1.6, 10.0);
        for (offset, corrected) in [
            (Vec3::X * 0.49, false),
            (Vec3::X * 0.50, false),
            (Vec3::X * 0.51, true),
            (Vec3::new(0.375, 0.0, 0.375), true),
        ] {
            let result = resolve_fire(5, a + offset, a);
            assert_eq!(result.origin_corrected, corrected);
            assert_eq!(result.origin, a);
        }
    }

    #[test]
    fn nonfinite_origin_hint_is_corrected() {
        let a = Vec3::new(10.0, 1.6, 10.0);
        for x in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let result = resolve_fire(0, Vec3::new(x, a.y, a.z), a);
            assert!(result.origin_corrected);
            assert_eq!(result.origin, a);
        }
    }

    #[test]
    fn projectile_clock_crosses_tick_wrap_and_switches_to_realtime() {
        assert_eq!(
            ProjectileClock::at(Tick(1), 0, 0),
            ProjectileClock::Realtime
        );
        assert_eq!(
            ProjectileClock::at(Tick(1), 2, 0),
            ProjectileClock::ViewTime(Tick(u32::MAX))
        );
        assert_eq!(
            ProjectileClock::at(Tick(2), 2, SUBSTEPS),
            ProjectileClock::ViewTime(Tick(0))
        );
        assert_eq!(
            ProjectileClock::at(Tick(3), 2, 2 * SUBSTEPS),
            ProjectileClock::Realtime
        );
    }

    #[test]
    fn terrain_thickness_check_distinguishes_thin_and_missing_cover() {
        use crate::collision::CollisionChunk;

        let ridge = |height: f32| {
            let heights = (0..65 * 65)
                .map(|i| if i % 65 == 20 { height } else { 0.0 })
                .collect();
            CollisionData::from_chunks([CollisionChunk::new(0, 0, heights, vec![0; 64 * 64])])
        };
        let from = Vec3::new(10.0, 1.0, 10.0);
        let to = Vec3::new(30.0, 1.0, 10.0);
        let thin = ridge(1.1);
        assert!(!thin.los(from, to));
        assert!(!thick_terrain_between(&thin, from, to, 0.5));
        assert!(thick_terrain_between(&ridge(2.0), from, to, 0.5));
        assert!(!thick_terrain_between(
            &CollisionData::default(),
            from,
            to,
            0.5
        ));
    }
}
