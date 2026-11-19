//! What the host tells a bot besides the world.

use frontline_sim::{PlayerId, Team, Vec3, WeaponState};

use crate::difficulty::Difficulty;
use crate::perception::Sound;
use crate::squad_leader::SquadOrder;

/// Own HUD state and friendly information available to the soldier.
#[derive(Debug, Clone, Copy, Default)]
pub struct SoldierStatus {
    pub downed_since: Option<u32>,
    pub downed_ally: Option<(PlayerId, Vec3)>,
    pub ammo_crate: Option<Vec3>,
    pub frag: u8,
    pub smoke: u8,
}

/// Per-call context of [`BotBrain::think`](crate::BotBrain::think).
#[derive(Debug, Clone, Copy)]
pub struct BotCtx<'a> {
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
    pub squad_order: Option<SquadOrder>,
    pub squad_leader: Option<PlayerId>,
    pub hearing: &'a [Sound],
    pub survival: SoldierStatus,
}
