//! One tick of projectile flight: [`SUBSTEPS`] sub-steps (GAMEDESIGN 6.4), parameterised by the
//! ammunition's drag `k` (the M0 per-weapon constant moves to `data/ammo`).

use glam::Vec3;

use crate::weapon::ballistics::{substep, MIN_SPEED_MPS, SUBSTEPS};

/// What a projectile did in one tick: the swept segment (for hit detection and tracers).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectileStep {
    pub from: Vec3,
    pub to: Vec3,
    /// Velocity at the end of the tick.
    pub v_end: Vec3,
    /// Below the minimum lethal speed: remove it.
    pub expired: bool,
}

/// Advances `pos` / `vel` by one tick of `dt` seconds with drag `k_drag_per_m` and gravity `g`.
pub fn step_tick(pos: Vec3, vel: Vec3, k_drag_per_m: f32, g: f32, dt: f32) -> ProjectileStep {
    let h = dt / SUBSTEPS as f32;
    let (mut p, mut v) = (pos, vel);
    for _ in 0..SUBSTEPS {
        substep(&mut p, &mut v, k_drag_per_m, g, h);
    }
    ProjectileStep {
        from: pos,
        to: p,
        v_end: v,
        expired: v.length() < MIN_SPEED_MPS,
    }
}
