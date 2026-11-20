//! Satchel charge lifecycle and deterministic damage allocation (GAMEDESIGN 6.9.3).

use glam::Vec3;

use crate::hash::HashWriter;
use crate::ids::{EntityId, PlayerId, Tick};
use crate::throwables::explosion::Occlusion;
use crate::vehicle::modules::ModuleId;
use crate::{data::GameData, Team};

pub const PLACEMENT_DISTANCE_M: f32 = 2.5;
pub const PLACEMENT_TICKS: u32 = 45;
pub const FUSE_TICKS: u32 = 180;
pub const DISARM_TICKS: u32 = 90;
pub const DISARM_RANGE_M: f32 = 40.0;
pub const MAX_ACTIVE_PER_TEAM: usize = 8;
pub const VEHICLE_DAMAGE_HP: f32 = 700.0;
pub const STRUCTURE_DAMAGE_HP: f32 = 2500.0;
pub const INFANTRY_DMAX_HP: f32 = 300.0;
pub const INFANTRY_RADIUS_M: f32 = 10.0;
pub const THROW_RANGE_M: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attachment {
    pub entity: EntityId,
    pub module: Option<ModuleId>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Satchel {
    pub id: u32,
    pub owner: PlayerId,
    pub team: Team,
    pub pos: Vec3,
    pub attached: Option<Attachment>,
    pub placed_at: Tick,
    pub fuse_tick: Tick,
    pub thrown: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SatchelError {
    TeamLimit,
    TooFar,
    PlacementIncomplete,
    NotOwner,
    OutOfRange,
    DisarmWindowExpired,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleBlastDamage {
    pub hull_hp: f32,
    pub attached_module: Option<(ModuleId, f32)>,
    pub adjacent_modules: [(ModuleId, f32); 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SatchelRules {
    placement_distance_m: f32,
    placement_ticks: u32,
    fuse_ticks: u32,
    disarm_ticks: u32,
    disarm_distance_m: f32,
    vehicle_damage_hp: f32,
    adjacent_module_mult: f32,
    structure_damage_hp: f32,
    infantry_damage_hp: f32,
    infantry_radius_m: f32,
    throw_range_m: f32,
    thrown_vehicle_mult: f32,
    team_limit: usize,
}

impl Default for SatchelRules {
    fn default() -> Self {
        let r = &GameData::ordnance().satchel;
        let ticks = |s: f64| libm::ceilf((s * 30.0) as f32 - 1e-4) as u32;
        Self {
            placement_distance_m: r.placement_distance_m as f32,
            placement_ticks: ticks(r.placement_s),
            fuse_ticks: ticks(r.fuse_s),
            disarm_ticks: ticks(r.disarm_s),
            disarm_distance_m: r.disarm_distance_m as f32,
            vehicle_damage_hp: r.vehicle_damage_hp as f32,
            adjacent_module_mult: r.adjacent_module_mult as f32,
            structure_damage_hp: r.structure_damage_hp as f32,
            infantry_damage_hp: r.infantry_damage_hp as f32,
            infantry_radius_m: r.infantry_radius_m as f32,
            throw_range_m: r.throw_range_m as f32,
            thrown_vehicle_mult: r.thrown_vehicle_mult as f32,
            team_limit: r.team_slots,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Satchels {
    active: Vec<Satchel>,
    next_id: u32,
    rules: SatchelRules,
}

// The default rules are loaded from hashed game data, so this cannot use derived Default.
#[allow(clippy::derivable_impls)]
impl Default for Satchels {
    fn default() -> Self {
        Self {
            active: Vec::new(),
            next_id: 0,
            rules: SatchelRules::default(),
        }
    }
}

impl Satchels {
    pub fn as_slice(&self) -> &[Satchel] {
        &self.active
    }

    pub fn place(
        &mut self,
        owner: PlayerId,
        team: Team,
        pos: Vec3,
        target_pos: Vec3,
        attachment: Attachment,
        now: Tick,
    ) -> Result<u32, SatchelError> {
        if self.active.iter().filter(|s| s.team == team).count() >= self.rules.team_limit {
            return Err(SatchelError::TeamLimit);
        }
        if pos.distance(target_pos) > self.rules.placement_distance_m {
            return Err(SatchelError::TooFar);
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.active.push(Satchel {
            id,
            owner,
            team,
            pos,
            attached: Some(attachment),
            placed_at: now,
            fuse_tick: Tick(
                now.0
                    .wrapping_add(self.rules.placement_ticks + self.rules.fuse_ticks),
            ),
            thrown: false,
        });
        Ok(id)
    }

    pub fn throw_ground(
        &mut self,
        owner: PlayerId,
        team: Team,
        pos: Vec3,
        target: Vec3,
        now: Tick,
    ) -> Result<u32, SatchelError> {
        if self.active.iter().filter(|s| s.team == team).count() >= self.rules.team_limit {
            return Err(SatchelError::TeamLimit);
        }
        if pos.distance(target) > self.rules.throw_range_m {
            return Err(SatchelError::TooFar);
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.active.push(Satchel {
            id,
            owner,
            team,
            pos: target,
            attached: None,
            placed_at: now,
            fuse_tick: Tick(now.0.wrapping_add(self.rules.fuse_ticks)),
            thrown: true,
        });
        Ok(id)
    }

    pub fn step(&mut self, now: Tick) -> Vec<Satchel> {
        let mut detonated = Vec::new();
        self.active.retain(|s| {
            if !now.lt(s.fuse_tick) {
                detonated.push(*s);
                false
            } else {
                true
            }
        });
        detonated
    }

    pub fn detonate(
        &mut self,
        id: u32,
        owner: PlayerId,
        now: Tick,
    ) -> Result<Satchel, SatchelError> {
        let i = self
            .active
            .iter()
            .position(|s| s.id == id)
            .ok_or(SatchelError::PlacementIncomplete)?;
        let s = self.active[i];
        if s.owner != owner {
            return Err(SatchelError::NotOwner);
        }
        if now.lt(Tick(s.placed_at.0.wrapping_add(self.rules.placement_ticks))) {
            return Err(SatchelError::PlacementIncomplete);
        }
        self.active.remove(i);
        Ok(s)
    }

    pub fn disarm(
        &mut self,
        id: u32,
        owner: PlayerId,
        now: Tick,
        elapsed_ticks: u32,
        distance_m: f32,
    ) -> Result<Satchel, SatchelError> {
        let i = self
            .active
            .iter()
            .position(|s| s.id == id)
            .ok_or(SatchelError::PlacementIncomplete)?;
        let s = self.active[i];
        if s.owner != owner {
            return Err(SatchelError::NotOwner);
        }
        if distance_m > self.rules.disarm_distance_m {
            return Err(SatchelError::OutOfRange);
        }
        if now.lt(Tick(s.placed_at.0.wrapping_add(self.rules.placement_ticks))) {
            return Err(SatchelError::PlacementIncomplete);
        }
        if elapsed_ticks < self.rules.disarm_ticks || !now.lt(s.fuse_tick) {
            return Err(SatchelError::DisarmWindowExpired);
        }
        self.active.remove(i);
        Ok(s)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn hash(&self, h: &mut HashWriter) {
        h.write_u8(0xB7);
        h.write_u32(self.next_id);
        for s in &self.active {
            h.write_u32(s.id);
            h.write_u8(s.owner);
            h.write_u8(s.team as u8);
            h.write_u32(s.placed_at.0);
            h.write_u32(s.fuse_tick.0);
            for f in s.pos.to_array() {
                h.write_f32(f);
            }
            h.write_u8(u8::from(s.thrown));
            if let Some(a) = s.attached {
                h.write_u16(a.entity);
                h.write_u8(a.module.map_or(0xff, |m| m as u8));
            }
        }
    }
}

pub fn infantry_damage(distance_m: f32) -> f32 {
    let r = &GameData::ordnance().satchel;
    crate::throwables::explosion_damage(
        r.infantry_damage_hp as f32,
        distance_m,
        r.infantry_radius_m as f32,
        Occlusion::Clear,
    )
}

pub fn structure_damage() -> f32 {
    GameData::ordnance().satchel.structure_damage_hp as f32
}

pub fn vehicle_damage(attached_module: Option<ModuleId>, thrown: bool) -> VehicleBlastDamage {
    let r = &GameData::ordnance().satchel;
    let total = r.vehicle_damage_hp as f32
        * if thrown {
            r.thrown_vehicle_mult as f32
        } else {
            1.0
        };
    let Some(module) = attached_module else {
        return VehicleBlastDamage {
            hull_hp: total,
            attached_module: None,
            adjacent_modules: [(ModuleId::Engine, 0.0); 2],
        };
    };
    let neighbors = adjacent_modules(module);
    VehicleBlastDamage {
        hull_hp: 0.0,
        attached_module: Some((module, total)),
        adjacent_modules: [
            (neighbors[0], total * r.adjacent_module_mult as f32),
            (neighbors[1], total * r.adjacent_module_mult as f32),
        ],
    }
}

fn adjacent_modules(module: ModuleId) -> [ModuleId; 2] {
    use ModuleId::*;
    match module {
        Engine => [TrackL, TrackR],
        TrackL => [Engine, TrackR],
        TrackR => [Engine, TrackL],
        TurretDrive => [MainGun, Engine],
        MainGun => [TurretDrive, AmmoRack],
        AmmoRack => [MainGun, TurretDrive],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn fuse_placement_and_owner_controls() {
        let mut charges = Satchels::default();
        charges
            .place(
                2,
                Team::Ger,
                Vec3::ZERO,
                Vec3::X,
                Attachment {
                    entity: 4,
                    module: Some(ModuleId::TrackL),
                },
                Tick(0),
            )
            .unwrap();
        assert_eq!(charges.as_slice()[0].fuse_tick, Tick(225));
        assert!(charges.step(Tick(224)).is_empty());
        assert_eq!(charges.step(Tick(225)).len(), 1);
        let id = charges
            .place(
                2,
                Team::Ger,
                Vec3::ZERO,
                Vec3::X,
                Attachment {
                    entity: 4,
                    module: None,
                },
                Tick(100),
            )
            .unwrap();
        assert_eq!(
            charges.detonate(id, 3, Tick(145)),
            Err(SatchelError::NotOwner)
        );
        assert_eq!(
            charges.detonate(id, 2, Tick(144)),
            Err(SatchelError::PlacementIncomplete)
        );
        charges.detonate(id, 2, Tick(145)).unwrap();
    }

    #[test]
    fn limits_vehicle_distribution_throw_multiplier_and_blast_curve() {
        let mut charges = Satchels::default();
        for i in 0..MAX_ACTIVE_PER_TEAM as u8 {
            charges
                .place(
                    i,
                    Team::Us,
                    Vec3::ZERO,
                    Vec3::X,
                    Attachment {
                        entity: 1,
                        module: None,
                    },
                    Tick(0),
                )
                .unwrap();
        }
        assert_eq!(
            charges.place(
                10,
                Team::Us,
                Vec3::ZERO,
                Vec3::X,
                Attachment {
                    entity: 1,
                    module: None
                },
                Tick(0)
            ),
            Err(SatchelError::TeamLimit)
        );
        assert_eq!(
            vehicle_damage(Some(ModuleId::TrackL), false).attached_module,
            Some((ModuleId::TrackL, 700.0))
        );
        assert_eq!(
            vehicle_damage(Some(ModuleId::TrackL), false).adjacent_modules[0].1,
            350.0
        );
        assert_eq!(vehicle_damage(None, true).hull_hp, 280.0);
        assert_eq!(infantry_damage(0.0), 300.0);
        assert_eq!(infantry_damage(5.0), 225.0);
        assert_eq!(infantry_damage(10.0), 0.0);
        assert_eq!(structure_damage(), 2500.0);
        let id = charges.as_slice()[0].id;
        assert_eq!(
            charges.disarm(id, 0, Tick(199), DISARM_TICKS - 1, 1.0),
            Err(SatchelError::DisarmWindowExpired)
        );
        charges.disarm(id, 0, Tick(200), DISARM_TICKS, 1.0).unwrap();
    }

    proptest::proptest! {
        #[test]
        fn blast_damage_is_monotone_with_distance(a in 0.0f32..9.9, delta in 0.0f32..0.1) {
            let b = (a + delta).min(10.0);
            prop_assert!(infantry_damage(a) >= infantry_damage(b));
        }
    }
}
