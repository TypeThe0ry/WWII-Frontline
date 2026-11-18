//! Lag-compensation time selection (M1-025, ARCHITECTURE 6.7). The server shooter fires from its
//! authoritative pose along the reported aim; only the *targets* are looked up in the hitbox
//! history, `rewind_ticks` behind the current tick. Rules (game rules live here, not in `server`):
//! rewind is clamped to [`MAX_REWIND_TICKS`], a `view_tick` of 0 means "no view time reported"
//! (scripted fire, prediction) and rewinds nothing, and an origin hint further than
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

/// Ticks to rewind for a shot simulated at `now` that the client saw at `view_tick`
/// (16-bit wrapping). Returns `(used, clamped)`.
pub fn rewind_ticks(now: u32, view_tick: u16) -> (u8, bool) {
    if view_tick == 0 {
        return (0, false);
    }
    let diff = (now as u16).wrapping_sub(view_tick) as i16;
    if diff <= 0 {
        return (0, false);
    }
    if diff > i16::from(MAX_REWIND_TICKS) {
        (MAX_REWIND_TICKS, true)
    } else {
        (diff as u8, false)
    }
}

/// Time selection and origin check for one fire command.
pub fn resolve_fire(
    now: u32,
    view_tick: u16,
    origin_hint: Vec3,
    auth_origin: Vec3,
) -> FireResolution {
    let (r, clamped) = rewind_ticks(now, view_tick);
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
    fn rewind_clamps_and_ignores_future() {
        assert_eq!(rewind_ticks(100, 0), (0, false));
        assert_eq!(rewind_ticks(100, 97), (3, false));
        assert_eq!(rewind_ticks(100, 92), (8, false));
        assert_eq!(rewind_ticks(100, 88), (8, true));
        assert_eq!(rewind_ticks(100, 103), (0, false));
        // 16-bit wrap
        assert_eq!(rewind_ticks(65_538, 65_535), (3, false));
    }

    #[test]
    fn origin_tolerance_edges() {
        let a = Vec3::new(10.0, 1.6, 10.0);
        let near = resolve_fire(10, 5, a + Vec3::X * 0.49, a);
        assert!(!near.origin_corrected && near.origin.x > a.x);
        let far = resolve_fire(10, 5, a + Vec3::X * 0.51, a);
        assert!(far.origin_corrected && far.origin == a);
    }
}
