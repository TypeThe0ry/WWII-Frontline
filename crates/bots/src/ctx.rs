//! What the host tells a bot besides the world.

use frontline_sim::{PlayerId, Team, WeaponState};

use crate::difficulty::Difficulty;

/// Per-call context of [`BotBrain::think`](crate::BotBrain::think).
#[derive(Debug, Clone, Copy)]
pub struct BotCtx {
    /// The bot's `PlayerId`.
    pub bot_id: PlayerId,
    pub team: Team,
    /// The tick the returned command is for (`world.tick() + 1`).
    pub tick: u32,
    pub difficulty: Difficulty,
    /// Match seed; all randomness is `Rng::for_(rng_seed, tick, bot_id, Purpose::Bot)`.
    pub rng_seed: u64,
    /// The bot's own weapon state (ammo, firing state machine): a client knows its own HUD
    /// state, so this is not extra knowledge. `None` while dead or unarmed.
    pub self_weapon: Option<WeaponState>,
}
