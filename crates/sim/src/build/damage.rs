//! Deterministic damage handed from projectile flight to the construction rules stage.

use bevy_ecs::prelude::Resource;
use glam::Vec3;

use super::{BuildSystem, StructureDamage};
use crate::ids::EntityId;
use crate::spawn::outpost::Outposts;

#[derive(Resource, Default)]
pub(crate) struct FacilityHits(pub Vec<(EntityId, StructureDamage, f32)>);

#[derive(Clone, Copy)]
pub(crate) enum BulletTarget {
    Outpost(u16),
    Structure(EntityId),
    Shield,
}

pub(crate) fn nearest_hit(
    facilities: &BuildSystem,
    outposts: &Outposts,
    guns: &crate::emplacement::at_gun::AtGuns,
    origin: Vec3,
    segment: Vec3,
    length: f32,
) -> Option<(f32, BulletTarget)> {
    if length <= 1e-6 {
        return None;
    }
    let capsule = |pos, low, high, radius| {
        crate::weapon::hitbox::seg_capsule(origin - pos, segment, low, high, radius)
    };
    let shields = guns
        .raycast_shield(origin, segment / length, length)
        .map(|t| (t / length, BulletTarget::Shield));
    shields
        .into_iter()
        .chain(outposts.list.values().filter_map(|p| {
            capsule(p.pos, 0.25, 1.2, 0.25).map(|t| (t, BulletTarget::Outpost(p.id)))
        }))
        .chain(facilities.structures().filter_map(|s| {
            capsule(s.pos, 0.45, 0.65, 0.45).map(|t| (t, BulletTarget::Structure(s.id)))
        }))
        .min_by(|a, b| a.0.total_cmp(&b.0))
}

/// Queue explosion damage with the same cover and falloff as infantry blasts.
pub(crate) fn blast_targets<'a>(
    targets: impl Iterator<Item = (EntityId, Vec3)> + 'a,
    collision: &'a crate::CollisionData,
    center: Vec3,
    damage: f32,
    radius: f32,
) -> impl Iterator<Item = (EntityId, f32)> + 'a {
    targets.filter_map(move |(id, pos)| {
        let distance = center.distance(pos);
        if distance >= radius {
            return None;
        }
        let lift = Vec3::Y * crate::throwables::explosion::RAY_LIFT_M;
        let cover = collision.blast_cover(center + lift, pos + lift);
        Some((
            id,
            crate::throwables::explosion_damage(damage, distance, radius, cover),
        ))
    })
}
