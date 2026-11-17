//! Client-side prediction subset (feature `client-predict`, ARCHITECTURE 8.1).
//!
//! Wraps [`step_player`] with the same [`MoveParams`] and collision the server uses, so a
//! predicted step is bit-identical to the authoritative one for equal inputs. The weapon state
//! (ammo, fire state machine, recoil, bloom, ADS) is predicted with the same [`step_weapon`];
//! projectiles, hits and damage are server-only (the client shows no predicted damage).

use crate::collision::CollisionData;
use crate::ids::{PlayerId, Tick};
use crate::movement::{step_player, MoveParams, PlayerAuthState};
use crate::weapon::{step_weapon, ShotCtx, WeaponOut, WeaponParams, WeaponState};
use crate::world::{SimConfig, DEFAULT_ROLE};
use crate::{data::GameData, ids::EntityId};
use frontline_protocol::msg::input::InputCmd;
use glam::Vec3;

pub struct PredictSim {
    params: MoveParams,
    collision: CollisionData,
    me: PlayerId,
    tick: Tick,
    state: PlayerAuthState,
    data: &'static GameData,
    hz: u8,
    match_seed: u64,
    entity: EntityId,
    weapon: Option<(WeaponState, WeaponParams)>,
    last_weapon_out: WeaponOut,
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
            data: cfg.data,
            hz: cfg.tick_hz,
            match_seed: cfg.match_seed,
            entity: 0,
            weapon: None,
            last_weapon_out: WeaponOut::default(),
        }
    }

    /// The server `EntityId` of the predicted player (part of the shot RNG stream key).
    pub fn set_entity(&mut self, entity: EntityId) {
        self.entity = entity;
    }

    /// Sets the authoritative weapon state (from `self_state`); `None` disables weapon prediction.
    /// Ignores a state whose weapon index or data is unusable.
    pub fn reset_weapon(&mut self, w: Option<WeaponState>) {
        self.weapon = w.and_then(|w| {
            let def = self.data.weapons.get(usize::from(w.def_id))?;
            Some((w, WeaponParams::from_def(def, self.hz).ok()?))
        });
    }

    /// Predicted weapon state.
    pub fn weapon(&self) -> Option<WeaponState> {
        self.weapon.map(|(w, _)| w)
    }

    /// What the last predicted step did with the weapon (shot for local muzzle flash / audio,
    /// dry fire, reload start / end).
    pub fn last_weapon_out(&self) -> WeaponOut {
        self.last_weapon_out
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
        self.last_weapon_out = WeaponOut::default();
        if let Some((w, wp)) = self.weapon.as_mut() {
            let ctx = ShotCtx {
                match_seed: self.match_seed,
                tick: self.tick,
                entity: self.entity,
            };
            self.last_weapon_out = step_weapon(w, wp, cmd, &self.state, &self.params, ctx);
        }
        self.state
    }

    pub fn state(&self) -> PlayerAuthState {
        self.state
    }
}
