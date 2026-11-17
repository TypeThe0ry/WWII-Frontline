//! `self_state` of the receiving player (ARCHITECTURE §6.5.3), filled from the sim.

use frontline_protocol::snapshot::{QPos, SelfState};
use frontline_sim::{PlayerId, World};

/// Bits of `cooldown_bits` (M0 convention): 0..=2 weapon FSM code (0 ready, 1 cooldown,
/// 2 bolt cycle, 3 reload (tactical), 4 reload (empty), 5 empty), bit 3 trigger held,
/// bit 4 aiming down sights.
pub fn cooldown_bits(w: &frontline_sim::WeaponState) -> u8 {
    let fsm = w.to_bytes()[6] & 7;
    fsm | (w.trigger_held as u8) << 3 | ((w.ads_units > 0) as u8) << 4
}

/// Builds the player's own state. `health_q` is the quantised health from the entity view.
pub fn build(world: &World, player: PlayerId, health_q: u8, pos_corr: Option<QPos>) -> SelfState {
    let mut s = SelfState {
        pos_corr,
        health: health_q,
        ..SelfState::default()
    };
    if let Some(w) = world.weapon_state(player) {
        s.ammo_in_mag = w.mag.min(255) as u8;
        s.ammo_reserve = w.reserve.min(4095);
        s.cooldown_bits = cooldown_bits(&w);
    }
    if let Some(m) = world.player_state(player) {
        s.stance = m.stance as u8;
    }
    s
}
