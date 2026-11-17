//! Client-side prediction subset (feature `client-predict`, ARCHITECTURE 8.1).
//!
//! Wraps [`step_player`] with the same [`MoveParams`] and collision the server uses, so a
//! predicted step is bit-identical to the authoritative one for equal inputs.

use crate::collision::CollisionData;
use crate::ids::{PlayerId, Tick};
use crate::movement::{step_player, MoveParams, PlayerAuthState};
use crate::world::{SimConfig, DEFAULT_ROLE};
use frontline_protocol::msg::input::InputCmd;
use glam::Vec3;

pub struct PredictSim {
    params: MoveParams,
    collision: CollisionData,
    me: PlayerId,
    tick: Tick,
    state: PlayerAuthState,
}

impl PredictSim {
    /// `collision` is cloned (the client keeps loading chunks into its own copy via
    /// [`collision_mut`](Self::collision_mut)). Panics if the embedded data lacks a usable
    /// default role (a build error in practice: covered by tests).
    pub fn new(cfg: SimConfig, collision: &CollisionData, me: PlayerId) -> Self {
        let role = cfg
            .data
            .role(DEFAULT_ROLE)
            .expect("embedded data has the default role");
        let params = MoveParams::from_role(role, cfg.tick_hz).expect("default role is valid");
        Self {
            params,
            collision: collision.clone(),
            me,
            tick: Tick(0),
            state: PlayerAuthState::spawn(Vec3::ZERO, 0.0),
        }
    }

    pub fn player(&self) -> PlayerId {
        self.me
    }

    pub fn tick(&self) -> Tick {
        self.tick
    }

    pub fn collision_mut(&mut self) -> &mut CollisionData {
        &mut self.collision
    }

    /// Resets to an authoritative state (reconciliation).
    pub fn reset_to(&mut self, auth: &PlayerAuthState, tick: Tick) {
        self.state = *auth;
        self.tick = tick;
    }

    /// Predicts one tick.
    pub fn step_player(&mut self, cmd: &InputCmd) -> PlayerAuthState {
        self.state = step_player(&self.state, cmd, &self.params, &self.collision);
        self.tick = self.tick.next();
        self.state
    }

    pub fn state(&self) -> PlayerAuthState {
        self.state
    }
}
