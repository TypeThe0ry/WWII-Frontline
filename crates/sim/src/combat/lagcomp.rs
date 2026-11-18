//! Lag-compensation time selection (M1-025, ARCHITECTURE 6.7). The server shooter fires from its
//! authoritative pose along the reported aim; only the *targets* are looked up in the hitbox
//! history, `rewind_ticks` behind the current tick. Rules (game rules live here, not in `server`):
//! rewind is clamped to [`MAX_REWIND_TICKS`], a `view_tick` (a delta in ticks) of 0 means "no view
//! time reported" (scripted fire, prediction) and rewinds nothing, and an origin hint further than
//! [`ORIGIN_TOLERANCE_M`] from the authoritative eye position is ignored.

use bevy_ecs::prelude::Resource;
use glam::Vec3;

use super::lagcomp_history::LagCompHistory;

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

/// Time selection and origin check for one fire command.
pub fn resolve_fire(view_delta: u16, origin_hint: Vec3, auth_origin: Vec3) -> FireResolution {
    let (r, clamped) = rewind_ticks(view_delta);
    let corrected = (origin_hint - auth_origin).length() > ORIGIN_TOLERANCE_M;
    FireResolution {
        rewind_ticks: r,
        origin: if corrected { auth_origin } else { origin_hint },
        clamped,
        origin_corrected: corrected,
    }
}

/// World resource: the hitbox history plus the on/off switch (off = no-compensation control).
#[derive(Resource, Debug, Clone)]
pub struct LagComp {
    pub history: LagCompHistory,
    pub enabled: bool,
}

impl Default for LagComp {
    fn default() -> Self {
        Self {
            history: LagCompHistory::new(),
            enabled: true,
        }
    }
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
        let near = resolve_fire(5, a + Vec3::X * 0.49, a);
        assert!(!near.origin_corrected && near.origin.x > a.x);
        let far = resolve_fire(5, a + Vec3::X * 0.51, a);
        assert!(far.origin_corrected && far.origin == a);
    }
}
