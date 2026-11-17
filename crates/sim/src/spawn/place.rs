//! Spawn coordinates: a random non-overlapping point within [`SPAWN_JITTER_M`] of the spawn
//! point, preferring one with no living enemy within [`SPAWN_KILL_RADIUS_M`] that has a line of
//! sight (anti spawn-kill, GAMEDESIGN 10.6).

use glam::Vec3;

use crate::collision::CollisionData;

use super::{SPAWN_CLEARANCE_M, SPAWN_JITTER_M, SPAWN_KILL_RADIUS_M, SPAWN_TRIES};

/// Result of [`pick_spawn`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpawnPick {
    pub pos: Vec3,
    /// Even the best candidate had an enemy with a line of sight within the kill radius.
    pub threatened: bool,
}

fn ground(col: &CollisionData, base: Vec3, x: f32, z: f32) -> Vec3 {
    Vec3::new(x, col.height_at(x, z).map_or(base.y, |h| h + 0.02), z)
}

/// Nearest living enemy with a line of sight within the kill radius, or `f32::MAX`.
fn threat_dist(col: &CollisionData, p: Vec3, enemies: &[Vec3]) -> f32 {
    let eye = p + Vec3::Y * 1.6;
    let mut best = f32::MAX;
    for &e in enemies {
        let d = ((e.x - p.x).powi(2) + (e.z - p.z).powi(2)).sqrt();
        if d <= SPAWN_KILL_RADIUS_M && d < best && col.los(eye, e + Vec3::Y * 1.6) {
            best = d;
        }
    }
    best
}

/// `unit` yields uniform draws in `[0, 1)`; `all` are the positions of every living player,
/// `enemies` those of the living enemies.
pub fn pick_spawn(
    col: &CollisionData,
    base: Vec3,
    all: &[Vec3],
    enemies: &[Vec3],
    mut unit: impl FnMut() -> f32,
) -> SpawnPick {
    let mut best: Option<(f32, Vec3)> = None;
    for _ in 0..SPAWN_TRIES {
        let ang = unit() * core::f32::consts::TAU;
        let rad = SPAWN_JITTER_M * libm::sqrtf(unit());
        let p = ground(
            col,
            base,
            base.x + rad * libm::cosf(ang),
            base.z + rad * libm::sinf(ang),
        );
        let clear = all.iter().all(|a| {
            let d2 = (a.x - p.x).powi(2) + (a.z - p.z).powi(2);
            d2 >= SPAWN_CLEARANCE_M * SPAWN_CLEARANCE_M
        });
        if !clear {
            continue;
        }
        let t = threat_dist(col, p, enemies);
        if t == f32::MAX {
            return SpawnPick {
                pos: p,
                threatened: false,
            };
        }
        if best.is_none_or(|(bt, _)| t > bt) {
            best = Some((t, p));
        }
    }
    match best {
        Some((_, p)) => SpawnPick {
            pos: p,
            threatened: true,
        },
        // Every candidate overlapped somebody: stand on the spawn point itself.
        None => SpawnPick {
            pos: ground(col, base, base.x, base.z),
            threatened: false,
        },
    }
}
