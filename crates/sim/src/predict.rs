//! Client-side prediction subset (feature `client-predict`, ARCHITECTURE 8.1).
//!
//! Wraps [`step_player`] with the same [`MoveParams`] and collision the server uses, so a
//! predicted step is bit-identical to the authoritative one for equal inputs. The weapon state
//! (ammo, fire state machine, recoil, bloom, ADS) is predicted with the same [`step_weapon`];
//! projectiles, hits and damage are server-only (the client shows no predicted damage).

use crate::collision::CollisionData;
use crate::ids::{PlayerId, Tick};
use crate::movement::{step_player_ex, MoveCtx, MoveParams, PlayerAuthState};
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
    /// The vehicle this player drives, when seated in the driver seat (M1-041b).
    vehicle: Option<crate::vehicle::Vehicle>,
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
            vehicle: None,
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

    /// Sets the authoritative state of the driven vehicle (reconciliation / entering the driver seat);
    /// `None` leaves vehicle prediction off.
    pub fn reset_vehicle(&mut self, v: Option<crate::vehicle::Vehicle>) {
        self.vehicle = v;
    }

    /// Sets the driven vehicle from an authoritative kinematic state: a fresh vehicle of `def_id` with the
    /// given pose, velocity and yaw rate; the slewed speed target starts at the current forward speed.
    pub fn reset_vehicle_kinematics(
        &mut self,
        def_id: &str,
        pos: crate::Vec3,
        vel: crate::Vec3,
        rot: glam::Quat,
        yaw_rate: f32,
    ) -> Result<(), crate::vehicle::VehicleSpawnError> {
        let mut v = crate::vehicle::Vehicle::from_def(
            self.data,
            def_id,
            0,
            crate::components::Team::Us,
            pos,
            rot,
        )?;
        v.vel = vel;
        v.ang_vel = crate::Vec3::new(0.0, yaw_rate, 0.0);
        v.speed_target = v.forward_speed();
        self.vehicle = Some(v);
        Ok(())
    }

    pub fn vehicle(&self) -> Option<&crate::vehicle::Vehicle> {
        self.vehicle.as_ref()
    }

    /// Predicts one tick of the driven vehicle with the driver's command: the same `ctrl_from_cmd`, `step_one` and
    /// hull-vs-terrain response as `step_vehicles` (hull damage, crush and gunnery stay server-only). `None` when
    /// no vehicle is set.
    pub fn step_vehicle_driver(&mut self, cmd: &InputCmd) -> Option<&crate::vehicle::Vehicle> {
        use crate::vehicle::{collision, ctrl_from_cmd, step_one, VehicleCtrl};
        let v = self.vehicle.as_mut()?;
        v.ctrl = if v.knocked_out {
            VehicleCtrl {
                brake: true,
                ..Default::default()
            }
        } else {
            ctrl_from_cmd(cmd)
        };
        step_one(v, &self.collision, 1.0 / f32::from(self.hz));
        match collision::probe(v, &self.collision) {
            Some(hit) => {
                let vn = v.vel.dot(hit.normal);
                if vn < 0.0 {
                    v.vel -= hit.normal * vn;
                }
                v.in_contact = true;
            }
            None => v.in_contact = false,
        }
        self.tick = self.tick.next();
        self.vehicle.as_ref()
    }

    /// Predicts one tick.
    pub fn step_player(&mut self, cmd: &InputCmd) -> PlayerAuthState {
        let mctx = MoveCtx {
            reloading: self
                .weapon
                .as_ref()
                .is_some_and(|(w, _)| w.fsm.is_reloading()),
            load_kg: None,
        };
        self.state = step_player_ex(&self.state, cmd, &self.params, &self.collision, &mctx);
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
