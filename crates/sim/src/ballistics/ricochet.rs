//! Minimal ricochet (M1-012; thresholds are 【待调参】, PI-050): a hit on a hard material at less
//! than [`RICOCHET_MAX_ANGLE_DEG`] to the surface reflects the shot and keeps
//! [`RICOCHET_ENERGY_KEEP`] of its kinetic energy (speed x sqrt of that).

use glam::Vec3;

/// Largest angle between the shot and the surface that still ricochets, degrees.
pub const RICOCHET_MAX_ANGLE_DEG: f32 = 15.0;
/// Kinetic energy kept by the reflected shot.
pub const RICOCHET_ENERGY_KEEP: f32 = 0.4;

/// Angle between the unit direction `dir` and the surface with unit `normal`, degrees (0..=90).
pub fn incidence_deg(dir: Vec3, normal: Vec3) -> f32 {
    libm::asinf(libm::fabsf(dir.dot(normal)).min(1.0)).to_degrees()
}

/// Reflected direction (unit) off a surface with unit `normal`.
pub fn reflect(dir: Vec3, normal: Vec3) -> Vec3 {
    let r = dir - 2.0 * dir.dot(normal) * normal;
    r.normalize_or_zero()
}

/// Speed factor after the bounce: `sqrt(RICOCHET_ENERGY_KEEP)`.
pub fn speed_factor() -> f32 {
    libm::sqrtf(RICOCHET_ENERGY_KEEP)
}

/// Whether a hit with this incidence on a ricochet-capable material bounces.
pub fn bounces(dir: Vec3, normal: Vec3) -> bool {
    incidence_deg(dir, normal) < RICOCHET_MAX_ANGLE_DEG
}
