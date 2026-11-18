//! Armour resolution (GAMEDESIGN 14.4.1): pure functions, hand-checkable.
//!
//! Steps: incidence angle -> normalisation (AP) -> overmatch / ricochet -> effective
//! thickness -> penetration at range -> penetrate or not. The luck factor `U(0.95, 1.05)` is
//! an argument so tests can pin it.

use glam::Vec3;

/// AP / APCBC ricochet angle after normalisation, degrees.
pub const AP_RICOCHET_DEG: f32 = 68.0;
/// HEAT ricochet angle, degrees.
pub const HEAT_RICOCHET_DEG: f32 = 75.0;
/// AP normalisation, degrees (subtracted from the incidence angle).
pub const AP_NORMALIZATION_DEG: f32 = 5.0;
/// Overmatch: shell calibre at least this many plate thicknesses.
pub const OVERMATCH_RATIO: f32 = 3.0;
/// Overmatch caps the effective incidence angle at this, degrees.
pub const OVERMATCH_ANGLE_CAP_DEG: f32 = 45.0;
/// Effective thickness never exceeds this many plate thicknesses.
pub const T_EFF_CAP: f32 = 3.0;
/// Penetration never drops below this fraction of `P100`.
pub const P_MIN_FRACTION: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    /// AP / APCBC kinetic shell (normalises, loses penetration with range).
    Ap,
    /// Shaped charge: no normalisation, no range drop.
    Heat,
}

/// What `armor_hit` needs from a shell (data: `vehicles/*.json` `weapons[].shells[]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellParams {
    pub kind: ShellKind,
    pub caliber_mm: f32,
    /// Penetration at 100 m, millimetres of plate at 0 degrees.
    pub p100_mm: f32,
    /// Penetration lost per 100 m beyond 100 m, percent of `P100` (0 for HEAT).
    pub drop_pct_per_100m: f32,
    pub structure_damage_hp: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArmorResult {
    /// Penetrated: `p_rem` mm of penetration left after the plate.
    Pen {
        p_rem: f32,
        t_eff: f32,
    },
    NoPen {
        t_eff: f32,
    },
    Ricochet,
}

/// Every intermediate value (used by the hand-calculation fixtures).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmorSteps {
    pub phi_deg: f32,
    pub phi_eff_deg: f32,
    pub overmatch: bool,
    pub t_eff: f32,
    pub p_mm: f32,
    pub result: ArmorResult,
}

/// Penetration at `dist_m`: `P100 * (1 - drop * (d - 100) / 100)`, flat inside 100 m, never
/// below 30% of `P100`.
pub fn penetration_at(shell: &ShellParams, dist_m: f32) -> f32 {
    let beyond = (dist_m - 100.0).max(0.0);
    let p = shell.p100_mm * (1.0 - shell.drop_pct_per_100m / 100.0 * beyond / 100.0);
    p.max(P_MIN_FRACTION * shell.p100_mm)
}

/// Maps `u` in `[0, 1)` to the luck factor `U(0.95, 1.05)`.
pub fn luck_factor(u: f32) -> f32 {
    0.95 + 0.1 * u
}

/// Resolves one hit. `n` is the outward surface normal, `v_hat` the shell's travel direction
/// (both unit); `factor` multiplies the penetration (see [`luck_factor`]).
pub fn armor_hit_steps(
    shell: &ShellParams,
    dist_m: f32,
    n: Vec3,
    v_hat: Vec3,
    thickness_mm: f32,
    factor: f32,
) -> ArmorSteps {
    let cos_phi = (-v_hat.dot(n)).clamp(-1.0, 1.0);
    let phi = libm::acosf(cos_phi).to_degrees();
    let mut phi_eff = match shell.kind {
        ShellKind::Ap => (phi - AP_NORMALIZATION_DEG).max(0.0),
        ShellKind::Heat => phi,
    };
    let p = penetration_at(shell, dist_m);
    let overmatch = shell.caliber_mm >= OVERMATCH_RATIO * thickness_mm;
    let limit = match shell.kind {
        ShellKind::Ap => AP_RICOCHET_DEG,
        ShellKind::Heat => HEAT_RICOCHET_DEG,
    };
    if overmatch {
        phi_eff = phi_eff.min(OVERMATCH_ANGLE_CAP_DEG);
    } else if phi_eff >= limit {
        return ArmorSteps {
            phi_deg: phi,
            phi_eff_deg: phi_eff,
            overmatch,
            t_eff: f32::INFINITY,
            p_mm: p,
            result: ArmorResult::Ricochet,
        };
    }
    let cos_eff = libm::cosf(phi_eff.to_radians()).max(1e-3);
    let t_eff = (thickness_mm / cos_eff).min(T_EFF_CAP * thickness_mm);
    let result = if p * factor >= t_eff {
        ArmorResult::Pen {
            p_rem: p - t_eff,
            t_eff,
        }
    } else {
        ArmorResult::NoPen { t_eff }
    };
    ArmorSteps {
        phi_deg: phi,
        phi_eff_deg: phi_eff,
        overmatch,
        t_eff,
        p_mm: p,
        result,
    }
}

pub fn armor_hit(
    shell: &ShellParams,
    dist_m: f32,
    n: Vec3,
    v_hat: Vec3,
    thickness_mm: f32,
    factor: f32,
) -> ArmorResult {
    armor_hit_steps(shell, dist_m, n, v_hat, thickness_mm, factor).result
}
