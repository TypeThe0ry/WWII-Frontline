//! Vault and climb (GAMEDESIGN 9.4, M1-011b): detection and the scripted motion.
//!
//! Detection looks along the view direction at a *surface height function* (today the terrain
//! height field, tomorrow terrain plus static props, hence the closure): the obstacle face has
//! to start within `vault_reach_m`, the top has to be a plateau at least `vault_top_width_m`
//! wide, and the rise above the feet decides the move: up to `vault_max_m` a vault, up to
//! `climb_max_m` (with `climb_headroom_m` free above the top) a climb, anything higher is
//! refused. Lower rises are the character controller's autostep business.
//!
//! The motion is a pure function of the tick counter: horizontal linear interpolation from
//! `start` to `target` and a parabolic lift whose apex clears the top by [`CLEARANCE_M`].
//! Weapons cannot fire while it runs (`PlayerAuthState::vault.is_some()`).

use glam::Vec3;

use super::params::MoveParams;

/// How far the feet pass above the obstacle top at the apex of the motion.
pub const CLEARANCE_M: f32 = 0.1;
/// Sampling step of the forward probe, metres.
const STEP_M: f32 = 0.05;
/// Surface noise tolerated on a "flat" top, metres.
const FLAT_TOL_M: f32 = 0.05;
/// Resting hover above the surface at the landing point (matches the KCC rest height).
const LAND_HOVER_M: f32 = 0.03;

/// A vault / climb in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vault {
    /// Ticks still to run (`1..=total`); the move ends when it reaches 0.
    pub ticks_left: u8,
    pub total: u8,
    pub climb: bool,
    pub start: Vec3,
    pub target: Vec3,
    /// Height of the obstacle top.
    pub top: f32,
}

/// What [`detect`] found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Probe {
    /// Nothing worth vaulting (flat, or only an autostep-able rise).
    None,
    /// An obstacle that cannot be passed (too high, too thin, no room to land).
    Blocked,
    /// Go: the plan of the move.
    Go(Vault),
}

/// Looks for a vaultable obstacle in front of `pos` (facing `yaw`). `surface(x, z)` is the walkable
/// surface height (`None` where unknown). `walkable(x, z)` says whether one may stand there.
pub fn detect(
    pos: Vec3,
    yaw: f32,
    p: &MoveParams,
    surface: &dyn Fn(f32, f32) -> Option<f32>,
    walkable: &dyn Fn(f32, f32) -> bool,
) -> Probe {
    let (fx, fz) = (libm::sinf(yaw), libm::cosf(yaw));
    let at = |d: f32| surface(pos.x + fx * d, pos.z + fz * d);
    // Heights are measured from the ground under the feet (the capsule hovers a few cm above it).
    let base = surface(pos.x, pos.z).unwrap_or(pos.y);
    let rise = |d: f32| at(d).map(|h| h - base);

    // 1. The face: first sample within reach that rises above autostep height.
    let n_reach = libm::ceilf(p.vault_reach_m / STEP_M) as u32;
    let mut face = None;
    for i in 1..=n_reach {
        let d = i as f32 * STEP_M;
        match rise(d) {
            Some(r) if r > p.autostep_max_m => {
                face = Some(d);
                break;
            }
            Some(_) => {}
            None => return Probe::Blocked,
        }
    }
    let Some(d_face) = face else {
        return Probe::None;
    };

    // 2. The top: follow the rise until it stops (at most 1.5 m further).
    let n_top = libm::ceilf(1.5 / STEP_M) as u32;
    let mut d_top = d_face;
    let mut top = at(d_face).unwrap_or(pos.y);
    let mut found = false;
    for i in 1..=n_top {
        let d = d_face + i as f32 * STEP_M;
        let Some(h) = at(d) else {
            return Probe::Blocked;
        };
        if h > top + 1e-3 {
            top = h;
            d_top = d;
        } else {
            found = true;
            break;
        }
    }
    if !found {
        return Probe::Blocked;
    }

    // 3. Width of the flat top.
    let mut d_end = d_top;
    for i in 1..=n_top {
        let d = d_top + i as f32 * STEP_M;
        match at(d) {
            Some(h) if (h - top).abs() <= FLAT_TOL_M => d_end = d,
            _ => break,
        }
    }
    if d_end - d_top + 1e-4 < p.vault_top_width_m {
        return Probe::Blocked;
    }

    // 4. Classify by the rise of the top above the feet.
    let h = top - base;
    let climb = h > p.vault_max_m;
    if h > p.climb_max_m {
        return Probe::Blocked;
    }

    // 5. Landing: on the top, or beyond the obstacle, on walkable ground not above the top.
    let mut target = None;
    for k in 0..6 {
        let d = d_top + 0.5 + 0.4 * k as f32;
        let (x, z) = (pos.x + fx * d, pos.z + fz * d);
        let Some(g) = surface(x, z) else {
            return Probe::Blocked;
        };
        if walkable(x, z) && g <= top + FLAT_TOL_M {
            target = Some(Vec3::new(x, g + LAND_HOVER_M, z));
            break;
        }
    }
    let Some(target) = target else {
        return Probe::Blocked;
    };
    // Headroom above the top is guaranteed by the (terrain-only) world; props will test it
    // through `surface` of the ceiling once they exist (climb_headroom_m is already data).
    let total = if climb { p.climb_ticks } else { p.vault_ticks };
    Probe::Go(Vault {
        ticks_left: total,
        total,
        climb,
        start: pos,
        target,
        top,
    })
}

/// Foot position after `elapsed` of `total` ticks.
pub fn path(v: &Vault, elapsed: u8) -> Vec3 {
    let t = f32::from(elapsed) / f32::from(v.total.max(1));
    let t = t.clamp(0.0, 1.0);
    let base = v.start + (v.target - v.start) * t;
    let apex = (v.top + CLEARANCE_M) - 0.5 * (v.start.y + v.target.y);
    let lift = 4.0 * t * (1.0 - t) * apex.max(0.0);
    Vec3::new(base.x, base.y + lift, base.z)
}

/// Advances the vault by one tick: the new position and whether it is finished.
pub fn advance(v: &mut Vault) -> (Vec3, bool) {
    v.ticks_left = v.ticks_left.saturating_sub(1);
    let elapsed = v.total - v.ticks_left;
    (path(v, elapsed), v.ticks_left == 0)
}
