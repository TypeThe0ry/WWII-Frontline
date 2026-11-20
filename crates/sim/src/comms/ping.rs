//! Authoritative aim resolution for comms. `PingPlace.target` is never trusted.

use crate::movement::{eye_height, MoveParams};
use crate::spawn::garrison::GarrisonState;
use crate::weapon::spread::view_dir;
use crate::{GameData, PlayerId, Team, Vec3, World};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TargetKind {
    Infantry,
    Vehicle,
    Artillery,
    AntiTankGun,
    Building,
}

/// Garrison ids are a different namespace from entity ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TargetId {
    pub kind: TargetKind,
    pub id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    pub key: TargetId,
    pub team: Team,
    pub pos: Vec3,
    pub downed: bool,
    pub half_extent: Vec3,
}

fn eye(world: &World, author: PlayerId) -> Option<(Vec3, Vec3)> {
    if !world.player_life(author)?.is_alive() || world.seat_of(author).is_some() {
        return None;
    }
    let actor = world.player_state(author)?;
    let role = world.squads().entry(author)?.role.unwrap_or("rifleman");
    let data = GameData::embedded();
    let role = data.role(role).or_else(|| data.role("rifleman"))?;
    let params = MoveParams::from_role(role, world.config().tick_hz).ok()?;
    Some((
        actor.pos + Vec3::Y * eye_height(actor.stance_blend, &params),
        view_dir(actor.yaw, actor.pitch),
    ))
}

/// Nearest aim hit among authoritative targets, including friendlies as possible occluders.
/// A target must pass terrain/static geometry and smoke LOS, not just an angular check.
pub fn identify(world: &World, author: PlayerId, range_m: f32) -> Option<Target> {
    let (origin, dir) = eye(world, author)?;
    // Cap the ray at the first terrain hit before checking candidates. LOS also handles smoke.
    let range_m = world
        .collision()
        .raycast(origin, dir, range_m)
        .map_or(range_m, |hit| hit.t);
    let mut nearest: Option<(f32, Target)> = None;
    for_each_target(world, author, |target| {
        let Some(t) = ray_box(origin, dir, target.pos, target.half_extent, range_m) else {
            return;
        };
        if nearest
            .as_ref()
            .is_none_or(|(best, hit)| (t, target.key) < (*best, hit.key))
            && world.view().los(origin, origin + dir * t)
        {
            nearest = Some((t, target));
        }
    });
    nearest.map(|(_, target)| target)
}

/// A tracked enemy keeps its last observation when the author loses LOS, only for the TTL.
/// The target id remains wholly server-side; refreshing cannot replace it with a reused slot.
pub fn observed_position(
    world: &World,
    author: PlayerId,
    key: TargetId,
    range_m: f32,
) -> Option<Vec3> {
    let (origin, _) = eye(world, author)?;
    let team = world.squads().entry(author)?.team;
    if key.kind != TargetKind::Vehicle {
        return None;
    }
    let vehicle = world.vehicle(key.id)?;
    let pos = vehicle.pos;
    (vehicle.team != team
        && vehicle.hull_hp > 0.0
        && pos.distance_squared(origin) <= range_m * range_m
        && world.view().los(origin, pos))
    .then_some(pos)
}

fn for_each_target(world: &World, author: PlayerId, mut visit: impl FnMut(Target)) {
    for player in world.view().players() {
        if player.player == author || world.seat_of(player.player).is_some() {
            continue;
        }
        let Some(life) = world
            .player_life(player.player)
            .filter(|life| !life.is_dead())
        else {
            continue;
        };
        let blend = world
            .player_state(player.player)
            .map_or(0.0, |state| state.stance_blend);
        let height = if life.is_downed() {
            0.5
        } else if blend <= 1.0 {
            1.8 - 0.6 * blend
        } else {
            1.2 - 0.7 * (blend - 1.0)
        };
        visit(Target {
            key: TargetId {
                kind: TargetKind::Infantry,
                id: player.entity,
            },
            team: player.team,
            pos: player.pos + Vec3::Y * (height * 0.5),
            downed: life.is_downed(),
            half_extent: Vec3::new(0.4, height * 0.5, 0.4),
        });
    }
    for view in world
        .vehicle_views()
        .into_iter()
        .filter(|v| v.hull_hp > 0.0)
    {
        let Some(vehicle) = world.vehicle(view.id) else {
            continue;
        };
        visit(Target {
            key: TargetId {
                kind: TargetKind::Vehicle,
                id: vehicle.id,
            },
            team: vehicle.team,
            pos: vehicle.pos,
            downed: false,
            half_extent: glam::Mat3::from_quat(vehicle.rot).abs() * vehicle.params.half_extents,
        });
    }
    for garrison in world
        .garrison_list()
        .into_iter()
        .filter(|g| g.state != GarrisonState::Destroyed)
    {
        visit(Target {
            key: TargetId {
                kind: TargetKind::Building,
                id: garrison.id,
            },
            team: garrison.team,
            pos: garrison.pos + Vec3::Y,
            downed: false,
            half_extent: Vec3::new(1.5, 1.0, 1.5),
        });
    }
}

fn ray_box(origin: Vec3, dir: Vec3, center: Vec3, half: Vec3, range_m: f32) -> Option<f32> {
    let low = center - half - origin;
    let high = center + half - origin;
    let (mut near, mut far) = (0.0_f32, range_m);
    for a in 0..3 {
        if dir[a].abs() <= f32::EPSILON {
            if low[a] > 0.0 || high[a] < 0.0 {
                return None;
            }
        } else {
            let t0 = low[a] / dir[a];
            let t1 = high[a] / dir[a];
            near = near.max(t0.min(t1));
            far = far.min(t0.max(t1));
            if near > far {
                return None;
            }
        }
    }
    (near > 0.0).then_some(near)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_rejects_off_axis_behind_and_out_of_range_targets() {
        let half = Vec3::splat(0.5);
        let ray = |x, z| ray_box(Vec3::ZERO, Vec3::Z, Vec3::new(x, 0.0, z), half, 50.0);
        assert_eq!(ray(0.0, 5.0), Some(4.5));
        assert_eq!(ray(1.0, 5.0), None);
        assert_eq!(ray(0.0, -5.0), None);
        assert_eq!(ray(0.0, 51.0), None);
    }
}
