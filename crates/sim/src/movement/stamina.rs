//! Stamina (GAMEDESIGN 9.3). Pure per-tick update; the state lives in `PlayerAuthState`.
//!
//! Spending (sprint, fast prone crawl, jump) resets the regeneration delay. Regeneration is
//! `stamina_regen_per_s * stance multiplier` after the delay. Reaching 0 locks sprinting
//! (and jumping / vaulting) until stamina is back to `stamina_unlock`.

use super::params::MoveParams;
use super::stance::Stance;
use super::state::PlayerAuthState;

/// What the player spent this tick.
#[derive(Debug, Clone, Copy, Default)]
pub struct Spend {
    pub sprint: bool,
    pub prone_fast: bool,
    pub jump: bool,
    /// One-off cost (vault / climb).
    pub extra: f32,
}

/// Sprint and fast-crawl are possible: not locked and stamina left.
pub fn can_exert(s: &PlayerAuthState) -> bool {
    !s.sprint_locked && s.stamina > 0.0
}

/// Jump / vault allowed for a one-off cost of `cost`.
pub fn can_afford(s: &PlayerAuthState, cost: f32) -> bool {
    !s.sprint_locked && s.stamina >= cost
}

pub(crate) fn update(s: &mut PlayerAuthState, spend: Spend, stance: Stance, p: &MoveParams) {
    let mut cost = spend.extra;
    if spend.sprint {
        cost += p.sprint_cost_per_s * p.dt;
    }
    if spend.prone_fast {
        cost += p.prone_fast_cost_per_s * p.dt;
    }
    if spend.jump {
        cost += p.jump_cost;
    }
    if cost > 0.0 {
        s.stamina_delay = p.stamina_regen_delay_ticks;
        s.stamina -= cost;
    } else if s.stamina_delay > 0 {
        s.stamina_delay -= 1;
    } else {
        s.stamina += p.stamina_regen_per_s * p.stamina_regen_mult[stance as usize] * p.dt;
    }
    // Exhaustion tolerance: 150 sprint ticks of 20/s at 30 Hz must end at exactly 0.
    if s.stamina <= 1e-3 {
        s.stamina = 0.0;
        s.sprint_locked = true;
    }
    s.stamina = s.stamina.min(p.stamina_max);
    if s.sprint_locked && s.stamina >= p.stamina_unlock {
        s.sprint_locked = false;
    }
}

/// Stamina quantised for `self_state` (6 bit, 0..=63).
pub fn quantised(s: &PlayerAuthState, p: &MoveParams) -> u8 {
    let q = libm::roundf(s.stamina / p.stamina_max * 63.0);
    q.clamp(0.0, 63.0) as u8
}
