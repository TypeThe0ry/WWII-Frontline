//! Data-driven AT emplacement state. World integration supplies stable IDs, actor eligibility,
//! construction transactions, and collision ordering against other world targets.

use std::collections::BTreeMap;

use bevy_ecs::prelude::Resource;
use glam::Vec3;
use serde::Deserialize;

use crate::components::Team;
use crate::data::defs::{Faction, Meta};
use crate::hash::HashWriter;
use crate::ids::{EntityId, PlayerId};
use crate::vehicle::armor::{armor_hit, ArmorResult};
use crate::vehicle::shell::{ray_obb, ShellDef, ShellType};
use crate::vehicle::shells::{shell_from_def, Shell, Shells, Warhead};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtGunDef {
    pub id: String,
    pub team: Faction,
    pub build: BuildRules,
    hp: f32,
    ammo: AmmoRules,
    reload: ReloadRules,
    aim: AimRules,
    shield: ShieldRules,
    chassis: Chassis,
    muzzle: Muzzle,
    ap: ShellDef,
    he: ShellDef,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildRules {
    pub supplies: u16,
    pub time_s: f32,
    pub team_limit: u8,
    pub sector_limit: u8,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct AmmoRules {
    ap: u8,
    he: u8,
    resupply_ap: u8,
    resupply_supplies: u16,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReloadRules {
    with_loader_s: f32,
    solo_s: f32,
    untrained_mult: f32,
    switch_s: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct AimRules {
    traverse_deg: f32,
    min_pitch_deg: f32,
    max_pitch_deg: f32,
    reorient_s: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShieldRules {
    thickness_mm: f32,
    slope_deg: f32,
    arc_deg: f32,
    offset_m: [f32; 3],
    half_width_m: f32,
    half_height_m: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct Chassis {
    offset_m: [f32; 3],
    half_extents_m: [f32; 3],
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct Muzzle {
    height_m: f32,
    forward_m: f32,
    visual_s: f32,
    exposure_s: f32,
    audience_radius_m: f32,
    error_radius_m: f32,
}

impl AtGunDef {
    pub fn load(json: &str) -> Result<Self, String> {
        let def: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let valid = !def.id.is_empty()
            && def.build.supplies > 0
            && def.build.sector_limit > 0
            && def.build.sector_limit <= def.build.team_limit
            && def.ammo.ap > 0
            && def.ammo.he > 0
            && (1..=def.ammo.ap).contains(&def.ammo.resupply_ap)
            && def.ammo.resupply_supplies > 0
            && def.ap.kind == ShellType::Ap
            && def.he.kind == ShellType::He
            && (0.0..=100.0).contains(&def.ap.drop_pct_per_100m)
            && (0.0..=180.0).contains(&def.aim.traverse_deg)
            && (-90.0..=0.0).contains(&def.aim.min_pitch_deg)
            && (0.0..=90.0).contains(&def.aim.max_pitch_deg)
            && (0.0..=90.0).contains(&def.shield.arc_deg)
            && (-89.0..=89.0).contains(&def.shield.slope_deg)
            && def.reload.untrained_mult >= 1.0
            && def.reload.with_loader_s <= def.reload.solo_s
            && def.muzzle.visual_s <= def.muzzle.exposure_s
            && [
                def.build.time_s,
                def.hp,
                def.reload.with_loader_s,
                def.reload.solo_s,
                def.reload.untrained_mult,
                def.reload.switch_s,
                def.aim.reorient_s,
                def.shield.thickness_mm,
                def.shield.half_width_m,
                def.shield.half_height_m,
                def.muzzle.height_m,
                def.muzzle.forward_m,
                def.muzzle.visual_s,
                def.muzzle.exposure_s,
                def.muzzle.audience_radius_m,
                def.muzzle.error_radius_m,
                def.ap.caliber_mm,
                def.ap.muzzle_velocity_mps,
                def.ap.p100_mm,
                def.ap.structure_damage_hp,
                def.he.caliber_mm,
                def.he.muzzle_velocity_mps,
                def.he.he_pen_mm,
                def.he.explosion_damage_max_hp,
                def.he.explosion_radius_m,
            ]
            .into_iter()
            .chain(def.chassis.half_extents_m)
            .all(|f| f.is_finite() && f > 0.0)
            && def
                .shield
                .offset_m
                .into_iter()
                .chain(def.chassis.offset_m)
                .all(f32::is_finite);
        if !valid {
            return Err(format!("invalid AT gun definition: {}", def.id));
        }
        Ok(def)
    }

    pub fn max_hp(&self) -> f32 {
        self.hp
    }

    pub fn team(&self) -> Team {
        match self.team {
            Faction::Us => Team::Us,
            Faction::Ger => Team::Ger,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildError {
    NotAntiTank,
    WrongTeam,
    TeamCap,
    SectorCap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Ammo {
    Ap = 0,
    He = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seat {
    Gunner,
    Loader,
}

#[derive(Debug, Clone, Copy)]
pub struct Actor<'a> {
    pub player: PlayerId,
    pub team: Team,
    pub role_id: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MuzzleFlash {
    pub gun: EntityId,
    pub team: Team,
    pub pos: Vec3,
    pub shell_id: u32,
    pub ammo: Ammo,
    pub visual_ticks: u32,
    pub exposure_ticks: u32,
    pub audience_radius_m: f32,
    pub error_radius_m: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellHit {
    pub t: f32,
    /// The outward shield normal. `None` means the projectile hit the unarmored chassis.
    pub shield_normal: Option<Vec3>,
}

#[derive(Debug, Clone, Copy)]
struct Reorient {
    engineer: PlayerId,
    target_yaw: f32,
    left: u32,
}

#[derive(Debug)]
pub struct AtGunState {
    pub id: EntityId,
    pub team: Team,
    pub sector: u16,
    pub pos: Vec3,
    pub hp: f32,
    /// Total remaining inventory, including any chambered round.
    pub ammo_ap: u8,
    pub ammo_he: u8,
    pub selected: Ammo,
    pub facing_center_yaw: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub gunner: Option<PlayerId>,
    pub loader: Option<PlayerId>,
    def: AtGunDef,
    reload_left: u32,
    switch_left: u32,
    reorient: Option<Reorient>,
}

impl AtGunState {
    /// `pos` is the placement point on the ground and angles are radians, with +Z forward.
    pub fn new(id: EntityId, sector: u16, pos: Vec3, yaw: f32, def: &AtGunDef) -> Self {
        let yaw = wrap_yaw(yaw);
        Self {
            id,
            team: def.team(),
            sector,
            pos,
            hp: def.hp,
            ammo_ap: def.ammo.ap,
            ammo_he: def.ammo.he,
            selected: Ammo::Ap,
            facing_center_yaw: yaw,
            yaw,
            pitch: 0.0,
            gunner: None,
            loader: None,
            def: def.clone(),
            reload_left: 0,
            switch_left: 0,
            reorient: None,
        }
    }

    /// The world checks that the actor is alive, within use range, and has no other mount.
    pub fn occupy(&mut self, actor: Actor<'_>, seat: Seat) -> bool {
        let (target, other) = match seat {
            Seat::Gunner => (&mut self.gunner, &mut self.loader),
            Seat::Loader => (&mut self.loader, &mut self.gunner),
        };
        if self.hp <= 0.0 || actor.team != self.team || target.is_some_and(|p| p != actor.player) {
            return false;
        }
        if *other == Some(actor.player) {
            *other = None;
        }
        *target = Some(actor.player);
        true
    }

    pub fn leave(&mut self, player: PlayerId) {
        if self.gunner == Some(player) {
            self.gunner = None;
        }
        if self.loader == Some(player) {
            self.loader = None;
        }
        if self.reorienting_engineer() == Some(player) {
            self.reorient = None;
        }
    }

    pub fn aim(&mut self, player: PlayerId, yaw: f32, pitch: f32) -> bool {
        if self.hp <= 0.0 || self.gunner != Some(player) || !yaw.is_finite() || !pitch.is_finite() {
            return false;
        }
        self.clamp_aim(yaw, pitch);
        true
    }

    fn clamp_aim(&mut self, yaw: f32, pitch: f32) {
        let limit = self.def.aim.traverse_deg.to_radians();
        let offset = wrap_yaw(yaw - self.facing_center_yaw).clamp(-limit, limit);
        self.yaw = wrap_yaw(self.facing_center_yaw + offset);
        self.pitch = pitch.clamp(
            self.def.aim.min_pitch_deg.to_radians(),
            self.def.aim.max_pitch_deg.to_radians(),
        );
    }

    pub fn start_reorient(&mut self, actor: Actor<'_>, yaw: f32, hz: u8) -> bool {
        if self.hp <= 0.0
            || actor.team != self.team
            || actor.role_id != "engineer"
            || !yaw.is_finite()
            || self.reorient.is_some()
        {
            return false;
        }
        self.reorient = Some(Reorient {
            engineer: actor.player,
            target_yaw: wrap_yaw(yaw),
            left: ticks(self.def.aim.reorient_s, hz),
        });
        true
    }

    pub fn reorienting_engineer(&self) -> Option<PlayerId> {
        self.reorient.map(|r| r.engineer)
    }

    /// Call once per world tick before inputs. The world can cancel an interrupted interaction
    /// via `leave` before ticking it. Fire, switch, and reorientation durations start on input.
    pub fn tick(&mut self) {
        self.reload_left = self.reload_left.saturating_sub(1);
        self.switch_left = self.switch_left.saturating_sub(1);
        if let Some(r) = &mut self.reorient {
            r.left -= 1;
            if r.left == 0 {
                self.facing_center_yaw = r.target_yaw;
                self.reorient = None;
                self.clamp_aim(self.yaw, self.pitch);
            }
        }
    }

    pub fn select_ammo(&mut self, player: PlayerId, ammo: Ammo, hz: u8) -> bool {
        if self.hp <= 0.0 || self.gunner != Some(player) || self.selected == ammo {
            return false;
        }
        self.selected = ammo;
        self.switch_left = ticks(self.def.reload.switch_s, hz);
        true
    }

    pub fn ready_in_ticks(&self) -> u32 {
        self.reload_left.max(self.switch_left)
    }

    /// The loader's current role is supplied at fire time. The reload duration is fixed for that
    /// cycle, and changing ammo cannot cancel the cooldown left by the previous shot.
    pub fn fire(
        &mut self,
        player: PlayerId,
        loader_role: Option<&str>,
        hz: u8,
        shells: &mut Shells,
    ) -> Option<MuzzleFlash> {
        if self.hp <= 0.0
            || self.gunner != Some(player)
            || self.ready_in_ticks() != 0
            || self.reorient.is_some()
        {
            return None;
        }
        let (ammo, shell_def) = match self.selected {
            Ammo::Ap => (&mut self.ammo_ap, &self.def.ap),
            Ammo::He => (&mut self.ammo_he, &self.def.he),
        };
        *ammo = ammo.checked_sub(1)?;
        let (sin_pitch, cos_pitch) = (libm::sinf(self.pitch), libm::cosf(self.pitch));
        let dir = forward(self.yaw) * cos_pitch + Vec3::Y * sin_pitch;
        let m = self.def.muzzle;
        let pos = self.pos + Vec3::Y * m.height_m + dir * m.forward_m;
        let shell_id = shells.push(shell_from_def(shell_def, self.id, self.team, pos, dir));
        let reload = self.def.reload;
        let seconds = match self.loader.and(loader_role) {
            Some("anti_tank" | "engineer") => reload.with_loader_s,
            Some(_) => reload.with_loader_s * reload.untrained_mult,
            None => reload.solo_s,
        };
        self.reload_left = ticks(seconds, hz);
        Some(MuzzleFlash {
            gun: self.id,
            team: self.team,
            pos,
            shell_id,
            ammo: self.selected,
            visual_ticks: ticks(m.visual_s, hz),
            exposure_ticks: ticks(m.exposure_s, hz),
            audience_radius_m: m.audience_radius_m,
            error_radius_m: m.error_radius_m,
        })
    }

    /// `spend` commits the authorized source (crate points or the data-defined Supplies price).
    /// Its arguments are the rounds accepted after capping and the full Supplies price.
    pub fn resupply_ap(&mut self, spend: impl FnOnce(u8, u16) -> bool) -> u8 {
        if self.hp <= 0.0 {
            return 0;
        }
        let rounds = self
            .def
            .ammo
            .ap
            .saturating_sub(self.ammo_ap)
            .min(self.def.ammo.resupply_ap);
        if rounds > 0 && spend(rounds, self.def.ammo.resupply_supplies) {
            self.ammo_ap += rounds;
            rounds
        } else {
            0
        }
    }

    /// Front-only plate. `dir` is unit length and the returned distance is in metres.
    pub fn raycast_shield(&self, origin: Vec3, dir: Vec3, len: f32) -> Option<ShellHit> {
        if self.hp <= 0.0 {
            return None;
        }
        let shield = self.def.shield;
        let fwd = forward(self.yaw);
        let incoming = Vec3::new(-dir.x, 0.0, -dir.z).try_normalize()?;
        if incoming.dot(fwd) + 1e-6 < libm::cosf(shield.arc_deg.to_radians()) {
            return None;
        }
        let right = Vec3::Y.cross(fwd);
        let slope = shield.slope_deg.to_radians();
        let normal = fwd * libm::cosf(slope) + Vec3::Y * libm::sinf(slope);
        let up = normal.cross(right);
        let [x, y, z] = shield.offset_m;
        let center = self.pos + right * x + Vec3::Y * y + fwd * z;
        let denom = dir.dot(normal);
        if denom >= -1e-6 {
            return None;
        }
        let t = (center - origin).dot(normal) / denom;
        let local = origin + dir * t - center;
        if !(0.0..=len).contains(&t)
            || local.dot(right).abs() > shield.half_width_m
            || local.dot(up).abs() > shield.half_height_m
        {
            return None;
        }
        Some(ShellHit {
            t,
            shield_normal: Some(normal),
        })
    }

    pub fn raycast_shell(&self, origin: Vec3, dir: Vec3, len: f32) -> Option<ShellHit> {
        if self.hp <= 0.0 {
            return None;
        }
        let fwd = forward(self.facing_center_yaw);
        let right = Vec3::Y.cross(fwd);
        let to_local = |v: Vec3| Vec3::new(v.dot(right), v.y, v.dot(fwd));
        let chassis = self.def.chassis;
        let body = ray_obb(
            to_local(origin - self.pos) - Vec3::from_array(chassis.offset_m),
            to_local(dir),
            Vec3::from_array(chassis.half_extents_m),
            len,
        )
        .map(|(t, _, _, _)| ShellHit {
            t,
            shield_normal: None,
        });
        self.raycast_shield(origin, dir, len)
            .filter(|shield| body.is_none_or(|body| shield.t <= body.t))
            .or(body)
    }

    /// Resolve the earliest shell hit. `factor` uses the shared armour luck distribution.
    pub fn apply_shell_hit(
        &mut self,
        hit: ShellHit,
        shell: &Shell,
        dir: Vec3,
        factor: f32,
    ) -> Option<ArmorResult> {
        let result = match shell.warhead {
            Warhead::Smoke => return None,
            Warhead::He => None,
            Warhead::Ap | Warhead::Heat => hit.shield_normal.map(|normal| {
                let distance = (shell.pos + dir * hit.t - shell.origin).length();
                armor_hit(
                    &shell.params,
                    distance,
                    normal,
                    dir,
                    self.def.shield.thickness_mm,
                    factor,
                )
            }),
        };
        if result.is_none_or(|r| matches!(r, ArmorResult::Pen { .. })) {
            self.damage(shell.params.structure_damage_hp);
        }
        result
    }

    /// Apply a resolved damage amount (e.g. from a nearby explosion). Seats are retained
    /// until the world ejects the occupants and removes the destroyed emplacement.
    pub fn damage(&mut self, hp: f32) {
        if hp.is_finite() && hp > 0.0 {
            self.hp = (self.hp - hp).max(0.0);
            if self.hp == 0.0 {
                self.reorient = None;
            }
        }
    }

    fn hash_into(&self, h: &mut HashWriter) {
        h.write_u16(self.id);
        h.write_u32(self.def.id.len() as u32);
        h.write_bytes(self.def.id.as_bytes());
        h.write_u8(self.team as u8);
        h.write_u16(self.sector);
        for f in self.pos.to_array() {
            h.write_f32(f);
        }
        h.write_f32(self.hp);
        h.write_u8(self.ammo_ap);
        h.write_u8(self.ammo_he);
        h.write_u8(self.selected as u8);
        h.write_f32(self.facing_center_yaw);
        h.write_f32(self.yaw);
        h.write_f32(self.pitch);
        h.write_u8(self.gunner.unwrap_or(u8::MAX));
        h.write_u8(self.loader.unwrap_or(u8::MAX));
        h.write_u32(self.reload_left);
        h.write_u32(self.switch_left);
        h.write_u8(u8::from(self.reorient.is_some()));
        if let Some(r) = self.reorient {
            h.write_u8(r.engineer);
            h.write_f32(r.target_yaw);
            h.write_u32(r.left);
        }
    }
}

#[derive(Resource, Debug, Default)]
pub struct AtGuns {
    pub list: BTreeMap<EntityId, AtGunState>,
}

impl AtGuns {
    /// Check AT-specific rules at placement and again before completion, in construction order.
    /// The caller checks ownership of the build site and the shared cap and Supplies transaction.
    pub fn check_build(
        &self,
        def: &AtGunDef,
        team: Team,
        role_id: &str,
        sector: u16,
    ) -> Result<(), BuildError> {
        if role_id != "anti_tank" {
            return Err(BuildError::NotAntiTank);
        }
        if def.team() != team {
            return Err(BuildError::WrongTeam);
        }
        let alive = || self.list.values().filter(|g| g.team == team && g.hp > 0.0);
        if alive().count() >= usize::from(def.build.team_limit) {
            return Err(BuildError::TeamCap);
        }
        if self
            .list
            .values()
            .filter(|g| g.hp > 0.0 && g.sector == sector)
            .count()
            >= usize::from(def.build.sector_limit)
        {
            return Err(BuildError::SectorCap);
        }
        Ok(())
    }

    /// Scan in stable ID order. The caller arbitrates against vehicles and terrain before applying
    /// the impact or emitting its burst. Shell allies are skipped as in the vehicle shell scan.
    pub fn raycast_shell(
        &self,
        shell: &Shell,
        dir: Vec3,
        len: f32,
    ) -> Option<(EntityId, ShellHit)> {
        self.list
            .values()
            .filter(|g| g.team != shell.team && g.id != shell.owner)
            .filter_map(|g| Some((g.id, g.raycast_shell(shell.pos, dir, len)?)))
            .min_by(|a, b| a.1.t.total_cmp(&b.1.t))
    }

    /// Bullet occlusion applies to both teams, like other ballistic cover.
    pub fn raycast_shield(&self, pos: Vec3, dir: Vec3, len: f32) -> Option<f32> {
        self.list
            .values()
            .filter_map(|g| g.raycast_shield(pos, dir, len).map(|hit| hit.t))
            .min_by(f32::total_cmp)
    }

    pub fn hash_into(&self, h: &mut HashWriter) {
        if self.list.is_empty() {
            return;
        }
        h.write_u8(0xD5);
        h.write_u32(self.list.len() as u32);
        for gun in self.list.values() {
            gun.hash_into(h);
        }
    }
}

fn forward(yaw: f32) -> Vec3 {
    Vec3::new(libm::sinf(yaw), 0.0, libm::cosf(yaw))
}

fn wrap_yaw(yaw: f32) -> f32 {
    use core::f32::consts::{PI, TAU};
    if (-PI..PI).contains(&yaw) {
        yaw
    } else {
        (yaw + PI).rem_euclid(TAU) - PI
    }
}

fn ticks(seconds: f32, hz: u8) -> u32 {
    (libm::ceilf(seconds * f32::from(hz)) as u32).max(1)
}

#[cfg(test)]
mod tests;
