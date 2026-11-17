//! Per-player suppression state and its accumulate / decay rules (GAMEDESIGN 8.2).

use bevy_ecs::prelude::*;
use glam::Vec3;

use super::anti_farm::Source;
use super::{
    COVER_MULT, DECAY_BASE, DECAY_PER_S, MAX_BULLETS_PER_TICK, STANCE_CROUCH, STANCE_PRONE,
};

#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct SuppressionState {
    /// `S in [0, 100]`.
    pub s: f32,
    /// `S` does not decay up to and including this tick.
    pub window_until: u32,
    /// Bullets processed in `hit_tick` (capped at 12 per tick).
    pub tick_hits: u8,
    pub hit_tick: u32,
    /// World position of the latest suppressor (cover test, source direction).
    pub src_pos: Vec3,
    /// Tick of the last `Suppress` event sent (10 Hz throttle).
    pub last_event: u32,
    /// Tick at which `S` last reached the pin threshold (`pinned_until` = that + 0.5 s).
    pub pinned_until: u32,
    pub sources: Vec<Source>,
}

impl SuppressionState {
    /// Adds `amount` at `tick`; ignored beyond 12 bullets in that tick. Returns whether it counted.
    pub fn add(&mut self, amount: f32, tick: u32, window_ticks: u32) -> bool {
        if self.hit_tick != tick {
            self.hit_tick = tick;
            self.tick_hits = 0;
        }
        if self.tick_hits >= MAX_BULLETS_PER_TICK {
            return false;
        }
        self.tick_hits += 1;
        self.s = (self.s + amount).clamp(0.0, 100.0);
        self.window_until = tick + window_ticks;
        true
    }

    pub fn is_active(&self) -> bool {
        self.s > 0.0 || !self.sources.is_empty()
    }
}

/// Stance coordinate `0 stand .. 1 crouch .. 2 prone` to the decay multiplier.
pub fn stance_mult(blend: f32) -> f32 {
    if blend >= 1.5 {
        STANCE_PRONE
    } else if blend >= 0.5 {
        STANCE_CROUCH
    } else {
        1.0
    }
}

/// One tick of decay: `dS/dt = -(6 + 0.12 S) x mult` once the window has passed.
pub fn decay(st: &mut SuppressionState, now: u32, hz: u8, blend: f32, in_cover: bool) {
    if st.s <= 0.0 || now <= st.window_until {
        return;
    }
    let mut m = stance_mult(blend);
    if in_cover {
        m = m.max(COVER_MULT);
    }
    st.s = (st.s - (DECAY_BASE + DECAY_PER_S * st.s) * m / f32::from(hz)).max(0.0);
}

/// `self_state.suppression` (4 bit): `round(S / 100 x 15)`.
pub fn quantise4(s: f32) -> u8 {
    (s.clamp(0.0, 100.0) / 100.0 * 15.0 + 0.5) as u8
}
