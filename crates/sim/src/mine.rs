//! Deterministic anti-personnel and anti-tank mine state (GAMEDESIGN 6.9.5, 12.4).

use glam::Vec3;
use std::collections::BTreeMap;

use crate::hash::HashWriter;
use crate::ids::{EntityId, PlayerId, Tick};
use crate::{data::GameData, Team};

pub const AP_TRIGGER_M: f32 = 0.9;
pub const AT_MIN_MASS_T: f32 = 1.0;
pub const MINE_HP: f32 = 1.0;
pub const CHAIN_MIN_DAMAGE_HP: f32 = 30.0;
pub const CHAIN_RADIUS_M: f32 = 2.0;
pub const AP_LIMIT: usize = 20;
pub const AT_LIMIT: usize = 16;
pub const SPATIAL_CELL_M: f32 = 64.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MineKind {
    Ap,
    At,
}

impl MineKind {
    pub const fn arming_ticks(self) -> u32 {
        match self {
            Self::Ap => 90,
            Self::At => 150,
        }
    }

    pub const fn disarm_ticks(self) -> u32 {
        match self {
            Self::Ap => 45,
            Self::At => 90,
        }
    }

    pub const fn team_limit(self) -> usize {
        match self {
            Self::Ap => AP_LIMIT,
            Self::At => AT_LIMIT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mine {
    pub id: u32,
    pub kind: MineKind,
    pub owner: PlayerId,
    pub team: Team,
    pub pos: Vec3,
    pub placed_at: Tick,
    pub armed_at: Tick,
    pub hp: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InfantryContact {
    pub player: PlayerId,
    pub pos: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleContact {
    pub id: EntityId,
    pub pos: Vec3,
    pub mass_t: f32,
    /// Left/right wheel or track chosen from the impact side by the caller.
    pub trigger_side: u8,
    pub lightweight_destruction: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MineDetonation {
    pub mine: Mine,
    pub infantry: Option<PlayerId>,
    pub vehicle: Option<VehicleContact>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtMineDamage {
    pub track_or_wheel_hp: f32,
    pub hull_hp: f32,
    pub destroys_light_vehicle: bool,
    pub trigger_side: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MineError {
    InvalidGround,
    TeamLimit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MineRules {
    ap_trigger_m: f32,
    at_min_mass_kg: f32,
    ap_arming_ticks: u32,
    at_arming_ticks: u32,
    ap_disarm_ticks: u32,
    at_disarm_ticks: u32,
    health_hp: f32,
    chain_damage_hp: f32,
    chain_radius_m: f32,
    ap_limit: usize,
    at_limit: usize,
}

impl Default for MineRules {
    fn default() -> Self {
        let r = &GameData::ordnance().mines;
        let ticks = |s: f64| libm::ceilf((s * 30.0) as f32 - 1e-4) as u32;
        Self {
            ap_trigger_m: r.ap.trigger_distance_m as f32,
            at_min_mass_kg: r.at.min_vehicle_mass_kg as f32,
            ap_arming_ticks: ticks(r.ap.arming_s),
            at_arming_ticks: ticks(r.at.arming_s),
            ap_disarm_ticks: ticks(r.ap.disarm_s),
            at_disarm_ticks: ticks(r.at.disarm_s),
            health_hp: GameData::ordnance().mines.health_hp as f32,
            chain_damage_hp: GameData::ordnance().mines.chain_damage_hp as f32,
            chain_radius_m: GameData::ordnance().mines.chain_radius_m as f32,
            ap_limit: r.ap.team_slots,
            at_limit: r.at.team_slots,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Minefield {
    mines: Vec<Mine>,
    next_id: u32,
    rules: MineRules,
}

// The default rules are loaded from hashed game data, so this cannot use derived Default.
#[allow(clippy::derivable_impls)]
impl Default for Minefield {
    fn default() -> Self {
        Self {
            mines: Vec::new(),
            next_id: 0,
            rules: MineRules::default(),
        }
    }
}

impl Minefield {
    pub fn place(
        &mut self,
        kind: MineKind,
        owner: PlayerId,
        team: Team,
        pos: Vec3,
        now: Tick,
        valid_ground: bool,
    ) -> Result<u32, MineError> {
        if !valid_ground {
            return Err(MineError::InvalidGround);
        }
        let limit = match kind {
            MineKind::Ap => self.rules.ap_limit,
            MineKind::At => self.rules.at_limit,
        };
        if self
            .mines
            .iter()
            .filter(|m| m.team == team && m.kind == kind)
            .count()
            >= limit
        {
            return Err(MineError::TeamLimit);
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.mines.push(Mine {
            id,
            kind,
            owner,
            team,
            pos,
            placed_at: now,
            armed_at: Tick(now.0.wrapping_add(match kind {
                MineKind::Ap => self.rules.ap_arming_ticks,
                MineKind::At => self.rules.at_arming_ticks,
            })),
            hp: self.rules.health_hp,
        });
        Ok(id)
    }

    pub fn as_slice(&self) -> &[Mine] {
        &self.mines
    }

    /// Removes and returns armed mines triggered this tick in stable mine-id order.
    pub fn step(
        &mut self,
        now: Tick,
        infantry: &[InfantryContact],
        vehicles: &[VehicleContact],
    ) -> Vec<MineDetonation> {
        let infantry_grid = contact_grid(infantry, |p| (u16::from(p.player), p.pos));
        let vehicle_grid = contact_grid(vehicles, |v| (v.id, v.pos));
        let mut detonations = Vec::new();
        self.mines.retain(|mine| {
            if now.lt(mine.armed_at) {
                return true;
            }
            let victim = match mine.kind {
                MineKind::Ap => nearby(&infantry_grid, mine.pos)
                    .into_iter()
                    .filter(|p| p.pos.distance(mine.pos) <= self.rules.ap_trigger_m)
                    .min_by_key(|p| p.player)
                    .map(|p| (Some(p.player), None)),
                MineKind::At => nearby(&vehicle_grid, mine.pos)
                    .into_iter()
                    .filter(|v| {
                        v.mass_t * 1000.0 >= self.rules.at_min_mass_kg
                            && v.pos.distance(mine.pos) <= 2.0
                    })
                    .min_by_key(|v| v.id)
                    .map(|v| (None, Some(v))),
            };
            if let Some((infantry, vehicle)) = victim {
                detonations.push(MineDetonation {
                    mine: *mine,
                    infantry,
                    vehicle,
                });
                false
            } else {
                true
            }
        });
        detonations
    }

    /// Gunfire does nothing. Explosions detonate mines only at both documented thresholds.
    pub fn damage_by_explosion(&mut self, center: Vec3, dmax_hp: f32, radius_m: f32) -> Vec<Mine> {
        let mut detonated = Vec::new();
        self.mines.retain(|m| {
            let r = m.pos.distance(center);
            let at_mine = crate::throwables::explosion_damage(
                dmax_hp,
                r,
                radius_m,
                crate::throwables::explosion::Occlusion::Clear,
            );
            if r <= self.rules.chain_radius_m && at_mine >= self.rules.chain_damage_hp {
                detonated.push(*m);
                false
            } else {
                true
            }
        });
        detonated
    }

    pub fn detonation_damage(
        &self,
        kind: MineKind,
        vehicle: Option<VehicleContact>,
    ) -> AtMineDamage {
        match kind {
            MineKind::Ap => AtMineDamage {
                track_or_wheel_hp: 0.0,
                hull_hp: 0.0,
                destroys_light_vehicle: false,
                trigger_side: 0,
            },
            MineKind::At => {
                let rules = &GameData::ordnance().mines.at;
                let contact = vehicle.expect("AT mine detonation has vehicle contact");
                AtMineDamage {
                    track_or_wheel_hp: rules.track_damage_hp as f32,
                    hull_hp: rules.hull_damage_hp as f32,
                    destroys_light_vehicle: contact.lightweight_destruction,
                    trigger_side: contact.trigger_side,
                }
            }
        }
    }

    pub fn ap_blast(&self) -> (f32, f32) {
        let r = &GameData::ordnance().mines.ap;
        (r.blast_damage_hp as f32, r.blast_radius_m as f32)
    }

    pub fn detonation_center(mine: Mine) -> Vec3 {
        match mine.kind {
            MineKind::Ap => mine.pos + Vec3::Y * 1.2,
            MineKind::At => mine.pos,
        }
    }

    pub fn visible_to(&self, viewer_team: Team, engineer: bool) -> impl Iterator<Item = &Mine> {
        self.mines
            .iter()
            .filter(move |m| engineer && m.team == viewer_team)
    }

    pub fn disarm(&mut self, id: u32, now: Tick, elapsed_ticks: u32) -> bool {
        let Some(index) = self.mines.iter().position(|m| m.id == id) else {
            return false;
        };
        let mine = self.mines[index];
        let required = match mine.kind {
            MineKind::Ap => self.rules.ap_disarm_ticks,
            MineKind::At => self.rules.at_disarm_ticks,
        };
        if elapsed_ticks < required || now.lt(mine.armed_at) {
            return false;
        }
        self.mines.remove(index);
        true
    }

    #[allow(clippy::should_implement_trait)]
    pub fn hash(&self, h: &mut HashWriter) {
        h.write_u8(0xB6);
        h.write_u32(self.next_id);
        for m in &self.mines {
            h.write_u32(m.id);
            h.write_u8(match m.kind {
                MineKind::Ap => 0,
                MineKind::At => 1,
            });
            h.write_u8(m.owner);
            h.write_u8(m.team as u8);
            h.write_u32(m.placed_at.0);
            h.write_u32(m.armed_at.0);
            h.write_f32(m.pos.x);
            h.write_f32(m.pos.y);
            h.write_f32(m.pos.z);
        }
    }
}

fn spatial_cell(pos: Vec3) -> (i32, i32) {
    (
        libm::floorf(pos.x / SPATIAL_CELL_M) as i32,
        libm::floorf(pos.z / SPATIAL_CELL_M) as i32,
    )
}

fn contact_grid<T: Copy>(
    contacts: &[T],
    identity_position: impl Fn(T) -> (u16, Vec3),
) -> BTreeMap<(i32, i32), Vec<T>> {
    let mut grid: BTreeMap<(i32, i32), Vec<T>> = BTreeMap::new();
    for contact in contacts.iter().copied() {
        grid.entry(spatial_cell(identity_position(contact).1))
            .or_default()
            .push(contact);
    }
    for entries in grid.values_mut() {
        entries.sort_by_key(|entry| identity_position(*entry).0);
    }
    grid
}

fn nearby<T: Copy>(grid: &BTreeMap<(i32, i32), Vec<T>>, pos: Vec3) -> Vec<T> {
    let (x, z) = spatial_cell(pos);
    let mut candidates = Vec::new();
    for cx in (x - 1)..=(x + 1) {
        for cz in (z - 1)..=(z + 1) {
            if let Some(entries) = grid.get(&(cx, cz)) {
                candidates.extend(entries.iter().copied());
            }
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ap_and_at_trigger_boundaries_and_arming_delay() {
        let mut field = Minefield::default();
        field
            .place(MineKind::Ap, 1, Team::Us, Vec3::ZERO, Tick(0), true)
            .unwrap();
        assert!(field
            .step(
                Tick(89),
                &[InfantryContact {
                    player: 2,
                    pos: Vec3::new(0.9, 0.0, 0.0)
                }],
                &[]
            )
            .is_empty());
        assert_eq!(
            field
                .step(
                    Tick(90),
                    &[InfantryContact {
                        player: 2,
                        pos: Vec3::new(0.9, 0.0, 0.0)
                    }],
                    &[]
                )
                .len(),
            1
        );
        let mut field = Minefield::default();
        field
            .place(MineKind::Ap, 1, Team::Us, Vec3::ZERO, Tick(0), true)
            .unwrap();
        assert!(field
            .step(
                Tick(90),
                &[InfantryContact {
                    player: 2,
                    pos: Vec3::new(1.0, 0.0, 0.0)
                }],
                &[]
            )
            .is_empty());
        field
            .place(MineKind::At, 1, Team::Us, Vec3::ZERO, Tick(0), true)
            .unwrap();
        assert!(field
            .step(
                Tick(150),
                &[],
                &[VehicleContact {
                    id: 4,
                    pos: Vec3::ZERO,
                    mass_t: 0.9,
                    trigger_side: 0,
                    lightweight_destruction: true
                }]
            )
            .is_empty());
        assert_eq!(
            field
                .step(
                    Tick(150),
                    &[],
                    &[VehicleContact {
                        id: 5,
                        pos: Vec3::ZERO,
                        mass_t: 1.0,
                        trigger_side: 1,
                        lightweight_destruction: false
                    }]
                )
                .len(),
            1
        );
    }

    #[test]
    fn caps_visibility_disarm_and_chain_conditions() {
        let mut field = Minefield::default();
        for i in 0..AP_LIMIT as u8 {
            field
                .place(MineKind::Ap, i, Team::Us, Vec3::ZERO, Tick(0), true)
                .unwrap();
        }
        assert_eq!(
            field.place(MineKind::Ap, 22, Team::Us, Vec3::ZERO, Tick(0), true),
            Err(MineError::TeamLimit)
        );
        assert_eq!(field.visible_to(Team::Us, true).count(), AP_LIMIT);
        assert_eq!(field.visible_to(Team::Ger, true).count(), 0);
        assert_eq!(field.visible_to(Team::Us, false).count(), 0);
        assert!(field.damage_by_explosion(Vec3::ZERO, 29.99, 4.0).is_empty());
        assert_eq!(
            field.damage_by_explosion(Vec3::ZERO, 40.0, 4.0).len(),
            AP_LIMIT
        );
    }

    #[test]
    fn at_damage_and_contact_order_are_stable() {
        let mut field = Minefield::default();
        field
            .place(
                MineKind::Ap,
                1,
                Team::Us,
                Vec3::new(63.8, 0.0, 0.0),
                Tick(0),
                true,
            )
            .unwrap();
        let contacts = [
            InfantryContact {
                player: 9,
                pos: Vec3::new(64.1, 0.0, 0.0),
            },
            InfantryContact {
                player: 3,
                pos: Vec3::new(64.1, 0.0, 0.0),
            },
        ];
        let det = field.step(Tick(90), &contacts, &[]);
        assert_eq!(det[0].infantry, Some(3));

        let mine = Minefield::default();
        assert_eq!(mine.ap_blast(), (180.0, 6.0));
        let damage = mine.detonation_damage(
            MineKind::At,
            Some(VehicleContact {
                id: 2,
                pos: Vec3::ZERO,
                mass_t: 1.0,
                trigger_side: 1,
                lightweight_destruction: true,
            }),
        );
        assert_eq!(
            (
                damage.track_or_wheel_hp,
                damage.hull_hp,
                damage.destroys_light_vehicle,
                damage.trigger_side
            ),
            (200.0, 300.0, true, 1)
        );
    }

    #[test]
    fn at_team_cap_invalid_placement_and_disarm_duration() {
        let mut field = Minefield::default();
        assert_eq!(
            field.place(MineKind::Ap, 1, Team::Us, Vec3::ZERO, Tick(0), false),
            Err(MineError::InvalidGround)
        );
        for i in 0..AT_LIMIT as u8 {
            field
                .place(MineKind::At, i, Team::Ger, Vec3::ZERO, Tick(0), true)
                .unwrap();
        }
        assert_eq!(
            field.place(MineKind::At, 30, Team::Ger, Vec3::ZERO, Tick(0), true),
            Err(MineError::TeamLimit)
        );
        assert!(!field.disarm(0, Tick(150), 89));
        assert!(field.disarm(0, Tick(150), 90));
    }
}
