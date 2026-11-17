//! Suppression (M1-017, GAMEDESIGN 8): a pure player-experience effect plus bot input. It never
//! changes damage. Each player carries `S in [0, 100]` ([`state::SuppressionState`]); enemy
//! bullets that miss them and explosions near them raise it, time lowers it.
//!
//! All coefficients are 【待调参】 constants here (not yet in `/data`, PI-057).

pub mod anti_farm;
pub mod detect;
pub mod effects;
pub mod state;
pub mod system;

pub use effects::{effects, SuppressionEffects};
pub use state::SuppressionState;
pub use system::{suppression, suppression_assists, SuppressSeen};

/// Largest miss distance (m) that still suppresses.
pub const D_MAX_M: f32 = 5.0;
pub const EXPONENT: f32 = 1.2;
/// Bullet speed (m/s) at and above which the speed factor is 1.
pub const SPEED_REF_MPS: f32 = 500.0;
/// Grace after a hit during which `S` does not decay (s).
pub const WINDOW_S: f32 = 0.5;
pub const DECAY_BASE: f32 = 6.0;
pub const DECAY_PER_S: f32 = 0.12;
pub const STANCE_PRONE: f32 = 1.3;
pub const STANCE_CROUCH: f32 = 1.15;
pub const COVER_MULT: f32 = 1.4;
/// `S >= PINNED_S` pins the player (ADS ignored for `PINNED_HOLD_S`).
pub const PINNED_S: f32 = 85.0;
pub const PINNED_HOLD_S: f32 = 0.5;
/// Explosion suppression `40 (1 - r/R)^2`, `R` 15 m (shells 25 m), x0.5 when walled off.
pub const EXPLOSION_S: f32 = 40.0;
pub const EXPLOSION_R_M: f32 = 15.0;
pub const WALL_MULT: f32 = 0.5;
/// Bullets processed per player per tick.
pub const MAX_BULLETS_PER_TICK: u8 = 12;
/// How long a suppressor stays on record for the anti-farm assist (s).
pub const ASSIST_WINDOW_S: f32 = 5.0;
/// Capsule axis extends this far above the head and below the feet (m).
pub const AXIS_MARGIN_M: f32 = 0.2;
/// `Suppress` events per player at most at this rate (Hz).
pub const EVENT_MAX_HZ: u16 = 10;

/// Weight of a bullet by weapon class (GAMEDESIGN 8.1).
pub fn caliber_weight(class: &str) -> f32 {
    match class {
        "pistol" | "smg" => 0.8,
        "carbine" => 1.5,
        "rifle_bolt" | "rifle_semi" => 2.2,
        "mg" | "lmg" => 2.8,
        "hmg" | "amr" => 6.0,
        _ => 1.5,
    }
}
