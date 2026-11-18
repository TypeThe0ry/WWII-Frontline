//! Vehicle physics (M1-022; GAMEDESIGN 14, ARCHITECTURE 5.3): a rigid hull on raycast
//! spring suspension with a track drive model. No general vehicle controller and no Rapier
//! body (PI-066): one small deterministic integrator, bodies visited in ascending `EntityId`.

pub mod armor;
pub mod collision;
pub mod gun;
pub mod internal;
pub mod modules;
pub mod seats;
pub mod shell;
pub mod shells;
pub mod snapshot;
pub mod suspension;
pub mod tracks;
pub mod turret;

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use glam::{Mat3, Quat, Vec3};
use serde::Deserialize;

use crate::data::defs::VehicleDef;
use crate::hash::HashWriter;
use crate::ids::EntityId;

pub use shells::shell_flight;

pub const GRAVITY: f32 = 9.81;
/// Integration sub-steps per tick (spring stiffness needs `dt <= ~1/90 s`).
pub const SUBSTEPS: u32 = 3;

/// Driver input mapped to the hull (GD 14.7). `throttle`/`steer` in -1..=1; steer > 0 turns
/// right (clockwise seen from above).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VehicleCtrl {
    pub throttle: f32,
    pub steer: f32,
    pub boost: bool,
    pub brake: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct SuspensionData {
    rays_per_side: u32,
    rest_length_m: f32,
    stiffness_n_per_m: f32,
    damping_ns_per_m: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct TracksData {
    max_torque_nm: f32,
    friction_forward: f32,
    friction_lateral: f32,
}

/// Sprocket radius turning track torque into force (not in data yet; PI-066).
const SPROCKET_RADIUS_M: f32 = 0.3;
/// Reverse top speed as a fraction of forward (GD 14.7).
pub(crate) const REVERSE_FRACTION: f32 = 0.4;
/// Time constant of the track speed controller.
pub(crate) const TRACK_TAU_S: f32 = 0.05;

/// Constants derived from a `VehicleDef` once per vehicle.
#[derive(Debug, Clone)]
pub struct VehicleParams {
    pub def_id: String,
    pub mass: f32,
    pub inertia: Vec3,
    pub half_extents: Vec3,
    pub max_speed: f32,
    /// Forward speed target slew, m/s^2 (top speed over `accel_time_s[0] + 25% of the range`).
    pub accel: f32,
    pub turn_rate: f32,
    pub max_slope_deg: f32,
    pub rays_per_side: u32,
    pub rest_length: f32,
    pub stiffness: f32,
    pub damping: f32,
    pub track_force_max: f32,
    pub mu_fwd: f32,
    pub mu_lat: f32,
    /// Half the distance between the tracks.
    pub half_gauge: f32,
    pub hull_hp: f32,
    pub seat_count: usize,
    pub zones: [shell::Zone; shell::ZONE_COUNT],
    pub module_specs: modules::ModuleSpecs,
    pub turret: turret::TurretParams,
    pub gun: gun::GunParams,
    pub shells: Vec<shell::ShellDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleSpawnError {
    UnknownVehicle,
    Data(VehicleDataError),
    NoFreeSlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleDataError {
    Missing(&'static str),
    Bad(&'static str),
}

fn parse_zones(
    g: &crate::data::defs::VehicleGameplay,
) -> Result<[shell::Zone; shell::ZONE_COUNT], VehicleDataError> {
    let mut out = [shell::Zone {
        thickness_mm: 0.0,
        slope_deg: 0.0,
    }; shell::ZONE_COUNT];
    for (i, id) in shell::ZONE_IDS.iter().enumerate() {
        let z = g
            .armor_zones
            .iter()
            .find(|z| z.id == *id)
            .ok_or(VehicleDataError::Missing("armor_zones"))?;
        out[i] = shell::Zone {
            thickness_mm: z.thickness_mm as f32,
            slope_deg: z.slope_deg as f32,
        };
    }
    Ok(out)
}

fn main_gun(
    g: &crate::data::defs::VehicleGameplay,
) -> Result<&serde_json::Value, VehicleDataError> {
    g.weapons
        .iter()
        .find(|w| w["kind"] == "cannon")
        .ok_or(VehicleDataError::Missing("weapons"))
}

fn parse_gun(g: &crate::data::defs::VehicleGameplay) -> Result<gun::GunParams, VehicleDataError> {
    let w = main_gun(g)?;
    let f = |k: &str| w[k].as_f64().ok_or(VehicleDataError::Bad("weapons"));
    let n = |k: &str| w["ammo_rounds"][k].as_u64().unwrap_or(0) as u16;
    Ok(gun::GunParams {
        reload_s: f("reload_s")? as f32,
        switch_reload_s: f("switch_reload_s")? as f32,
        ammo: [n("ap"), n("he"), n("wp")],
    })
}

fn parse_shells(
    g: &crate::data::defs::VehicleGameplay,
) -> Result<Vec<shell::ShellDef>, VehicleDataError> {
    let w = main_gun(g)?;
    serde_json::from_value(w["shells"].clone()).map_err(|_| VehicleDataError::Bad("shells"))
}

impl VehicleParams {
    pub fn from_def(def: &VehicleDef) -> Result<Self, VehicleDataError> {
        let g = &def.gameplay;
        let h = &def.historical;
        let dims = h
            .dimensions_m
            .as_ref()
            .ok_or(VehicleDataError::Missing("dimensions_m"))?;
        let susp: SuspensionData = serde_json::from_value(
            g.suspension
                .clone()
                .ok_or(VehicleDataError::Missing("suspension"))?,
        )
        .map_err(|_| VehicleDataError::Bad("suspension"))?;
        let tr: TracksData = serde_json::from_value(
            g.tracks
                .clone()
                .ok_or(VehicleDataError::Missing("tracks"))?,
        )
        .map_err(|_| VehicleDataError::Bad("tracks"))?;
        let accel_t = g
            .accel_time_s
            .ok_or(VehicleDataError::Missing("accel_time_s"))?;
        let max_speed = g
            .max_speed_mps
            .ok_or(VehicleDataError::Missing("max_speed_mps"))? as f32;
        let mass = match &h.mass_kg {
            crate::data::defs::Hist::Known(m) => *m as f32,
            _ => return Err(VehicleDataError::Missing("mass_kg")),
        };
        let (l, w, hh) = (dims.length as f32, dims.width as f32, dims.height as f32);
        Ok(VehicleParams {
            def_id: def.id.clone(),
            mass,
            inertia: Vec3::new(
                mass * (hh * hh + l * l) / 12.0,
                mass * (w * w + l * l) / 12.0,
                mass * (w * w + hh * hh) / 12.0,
            ),
            half_extents: Vec3::new(w / 2.0, hh / 2.0, l / 2.0),
            max_speed,
            // Top speed at a quarter of the way through `accel_time_s`: 0->90% lands inside it.
            accel: max_speed / (accel_t[0] + 0.25 * (accel_t[1] - accel_t[0])) as f32,
            turn_rate: (g
                .hull_turn_deg_s
                .ok_or(VehicleDataError::Missing("hull_turn_deg_s"))?
                as f32)
                .to_radians(),
            max_slope_deg: g
                .max_slope_deg
                .ok_or(VehicleDataError::Missing("max_slope_deg"))?
                as f32,
            rays_per_side: susp.rays_per_side,
            rest_length: susp.rest_length_m,
            stiffness: susp.stiffness_n_per_m,
            damping: susp.damping_ns_per_m,
            track_force_max: 2.0 * tr.max_torque_nm / SPROCKET_RADIUS_M,
            mu_fwd: tr.friction_forward,
            mu_lat: tr.friction_lateral,
            half_gauge: (w / 2.0 - 0.3).max(0.5),
            hull_hp: g.hull_health_hp as f32,
            seat_count: g.seats.len(),
            zones: parse_zones(g)?,
            module_specs: modules::ModuleSpecs::from_def(def),
            turret: turret::TurretParams {
                traverse_deg_s: g
                    .turret_traverse_deg_s
                    .ok_or(VehicleDataError::Missing("turret_traverse_deg_s"))?
                    as f32,
                elev_min_deg: g
                    .gun_elevation_deg
                    .ok_or(VehicleDataError::Missing("gun_elevation_deg"))?[0]
                    as f32,
                elev_max_deg: g.gun_elevation_deg.unwrap()[1] as f32,
            },
            gun: parse_gun(g)?,
            shells: parse_shells(g)?,
        })
    }
}

/// One vehicle body. `rot` maps hull-local (X right-hand side, Y up, Z forward) to world.
#[derive(Debug, Clone)]
pub struct Vehicle {
    pub id: EntityId,
    pub team: crate::components::Team,
    pub params: VehicleParams,
    pub pos: Vec3,
    pub vel: Vec3,
    pub rot: Quat,
    /// Angular velocity, world frame.
    pub ang_vel: Vec3,
    pub ctrl: VehicleCtrl,
    /// Slewed forward speed target, m/s (signed).
    pub speed_target: f32,
    /// Suspension rays in contact after the last step (diagnostics / grounded checks).
    pub grounded_rays: u32,
    pub hull_hp: f32,
    /// One slot per seat of the definition (index = seat id; 0 is the driver).
    pub seats: Vec<Option<seats::Occupant>>,
    /// Hull touched an obstacle last tick (impacts count once per contact).
    /// Driven by `World::set_vehicle_ctrl` (tests, later bots) instead of a seated driver.
    pub scripted: bool,
    pub in_contact: bool,
    /// Victim -> tick of the last crush (repeat guard).
    pub crushed: BTreeMap<crate::ids::PlayerId, u32>,
    /// Ticks spent past the overturn tilt; `overturned` latches the event.
    pub flip_ticks: u32,
    pub overturned: bool,
    pub modules: modules::Modules,
    pub turret: turret::Turret,
    pub gun: gun::Gun,
    /// Destroyed (hull HP 0 or cook-off); no further combat processing.
    pub knocked_out: bool,
    /// Ticks until the burning wreck explodes (0 = not burning).
    pub burn_left: u32,
    /// Ticks until the wreck is removed (0 = not yet a wreck).
    pub wreck_left: u32,
    /// Wreck expired; the world removes it after the tick.
    pub gone: bool,
    /// SWAP was held last tick (shell cycling acts on the rising edge).
    pub swap_held: bool,
}

impl Vehicle {
    pub fn forward(&self) -> Vec3 {
        self.rot * Vec3::Z
    }
    pub fn up(&self) -> Vec3 {
        self.rot * Vec3::Y
    }
    /// Signed speed along the hull's forward axis, m/s.
    pub fn forward_speed(&self) -> f32 {
        self.vel.dot(self.forward())
    }
    /// Hull yaw rate (about world up), rad/s.
    pub fn yaw_rate(&self) -> f32 {
        self.ang_vel.y
    }
}

#[derive(Resource, Debug, Default)]
pub struct Vehicles {
    pub list: BTreeMap<EntityId, Vehicle>,
}

impl Vehicles {
    pub fn hash_into(&self, h: &mut HashWriter) {
        h.write_u8(0xF1);
        for v in self.list.values() {
            h.write_u16(v.id);
            for f in [v.pos.x, v.pos.y, v.pos.z, v.vel.x, v.vel.y, v.vel.z] {
                h.write_f32(f);
            }
            for f in [
                v.rot.x,
                v.rot.y,
                v.rot.z,
                v.rot.w,
                v.ang_vel.x,
                v.ang_vel.y,
                v.ang_vel.z,
            ] {
                h.write_f32(f);
            }
            h.write_f32(v.speed_target);
            h.write_f32(v.hull_hp);
        }
    }
}

pub fn step_one(v: &mut Vehicle, col: &crate::CollisionData, dt: f32) {
    tracks::update_speed_target(v, dt);
    let h = dt / SUBSTEPS as f32;
    for _ in 0..SUBSTEPS {
        let (force, torque, grounded) = tracks::forces(v, col, h);
        v.grounded_rays = grounded;
        let inv_m = 1.0 / v.params.mass;
        v.vel += (force * inv_m + Vec3::new(0.0, -GRAVITY, 0.0)) * h;
        // Torque in the body frame so the inertia stays diagonal.
        let rot_inv = v.rot.inverse();
        let w_b = rot_inv * v.ang_vel;
        let t_b = rot_inv * torque;
        let i = v.params.inertia;
        let gyro = w_b.cross(Vec3::new(i.x * w_b.x, i.y * w_b.y, i.z * w_b.z));
        let a_b = (t_b - gyro) / i;
        v.ang_vel = v.rot * (w_b + a_b * h);
        v.pos += v.vel * h;
        let dq = Quat::from_scaled_axis(v.ang_vel * h);
        v.rot = (dq * v.rot).normalize();
    }
}

/// Orientation for a hull facing `yaw` radians (0 = +Z, positive turns toward +X).
pub fn yaw_quat(yaw: f32) -> Quat {
    Quat::from_rotation_y(yaw)
}

/// Rotation matrix helper for tests and views.
pub fn basis(rot: Quat) -> Mat3 {
    Mat3::from_quat(rot)
}

/// Per-tick driver of every vehicle: input mapping, physics, collisions, seat timers, crushing.
pub use system::step_vehicles;
mod system;
