//! Server-authoritative projectiles (GAMEDESIGN 6.4, M0-012): flight, terrain / body hits, damage.
//!
//! Each tick a projectile integrates [`SUBSTEPS`] sub-steps (gravity + quadratic drag, see
//! [`crate::weapon::ballistics`]); every sub-step segment is tested against the terrain
//! (`CollisionData::raycast`) and the hit volumes of the enemy players
//! ([`crate::weapon::hitbox`]); the nearest hit ends the projectile. Players of the shooter's
//! team and the shooter are ignored (friendly fire is M1-014; there is no penetration in M0).
//! A projectile that leaves the loaded collision, lives longer than 4 s or drops below 150 m/s
//! is removed silently. Damage `D = damage_torso_hp * falloff(d) * part_mult` is subtracted from
//! the target's [`Health`] and reported as [`SimEvent::Hit`] (and logged in [`HitLog`] for the
//! rules stage, which turns a lethal hit into a kill: `crate::schedule::deaths`).

use bevy_ecs::prelude::*;
use glam::Vec3;

use crate::components::{
    CollisionRes, EntityTable, EventBuf, Motion, SimCfgRes, Team, Transform, WeaponTable,
};
use crate::data::defs::WeaponDef;
use crate::health::{Health, HitLog, HitRec};
use crate::ids::EntityId;
use crate::step::SimEvent;
use crate::weapon::ballistics::{self, BallisticsParams, MIN_SPEED_MPS, SUBSTEPS};
use crate::weapon::hitbox::{self, HitZone};

/// One projectile in flight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projectile {
    pub id: u32,
    pub owner: EntityId,
    pub team: Team,
    /// Index into `GameData::weapons`.
    pub weapon: u16,
    /// Muzzle position (damage falloff distance is measured from here).
    pub origin: Vec3,
    pub pos: Vec3,
    pub vel: Vec3,
    /// Sub-steps flown so far (lifetime).
    pub substeps: u32,
}

/// All projectiles in spawn order.
#[derive(Resource, Debug, Default)]
pub struct Projectiles {
    list: Vec<Projectile>,
    next_id: u32,
}

impl Projectiles {
    pub fn spawn(
        &mut self,
        owner: EntityId,
        team: Team,
        weapon: u16,
        origin: Vec3,
        vel: Vec3,
    ) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.list.push(Projectile {
            id,
            owner,
            team,
            weapon,
            origin,
            pos: origin,
            vel,
            substeps: 0,
        });
        id
    }

    pub fn as_slice(&self) -> &[Projectile] {
        &self.list
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn next_id(&self) -> u32 {
        self.next_id
    }
}

struct Target {
    id: EntityId,
    entity: Entity,
    team: Team,
    foot: Vec3,
    blend: f32,
    /// Cleared when a hit this tick kills it (later projectiles pass through the corpse).
    alive: bool,
}

fn part_mult(def: &WeaponDef, zone: HitZone) -> f32 {
    let m = &def.gameplay.part_mult;
    (match zone {
        HitZone::Head => m.head,
        HitZone::Neck => m.neck,
        HitZone::Torso => m.torso,
        // PI-007: a missing pelvis multiplier falls back to the torso one.
        HitZone::Pelvis => m.pelvis.unwrap_or(m.torso),
        HitZone::Limb => m.limb,
    }) as f32
}

/// `D = dmg * falloff(d) * part_mult` (GAMEDESIGN 6.6); no other modifiers in M0.
pub fn damage(def: &WeaponDef, zone: HitZone, dist_m: f32) -> f32 {
    def.gameplay.damage_torso_hp as f32
        * ballistics::falloff(&def.gameplay.falloff, dist_m)
        * part_mult(def, zone)
}

enum Fate {
    Alive,
    Gone,
}

/// The projectile stage (ARCHITECTURE 6.10 stage 4).
#[allow(clippy::too_many_arguments)]
pub fn projectile_system(
    cfg: Res<SimCfgRes>,
    wt: Res<WeaponTable>,
    col: Res<CollisionRes>,
    table: Res<EntityTable>,
    mut projs: ResMut<Projectiles>,
    mut events: ResMut<EventBuf>,
    mut hitlog: ResMut<HitLog>,
    mut q: Query<(&Transform, &Motion, &Team, &mut Health)>,
) {
    if projs.list.is_empty() {
        return;
    }
    let mut targets: Vec<Target> = Vec::new();
    for (id, entity) in table.sorted() {
        if let Ok((t, m, team, h)) = q.get(entity) {
            if h.hp > 0.0 {
                targets.push(Target {
                    id,
                    entity,
                    team: *team,
                    foot: t.pos,
                    blend: m.stance_blend,
                    alive: true,
                });
            }
        }
    }
    let max_sub = ballistics::max_substeps(cfg.hz);
    let h = 1.0 / (f32::from(cfg.hz) * SUBSTEPS as f32);
    let list = std::mem::take(&mut projs.list);
    let mut keep = Vec::with_capacity(list.len());
    for mut pr in list {
        let Some(def) = cfg.data.weapons.get(usize::from(pr.weapon)) else {
            continue;
        };
        let bp = wt
            .ballistics(pr.weapon)
            .unwrap_or_else(|| BallisticsParams::from_def(def));
        let fate = advance(
            &mut pr,
            def,
            &bp,
            h,
            max_sub,
            &col.0,
            &mut targets,
            &mut q,
            &mut events.0,
            &mut hitlog.0,
        );
        if matches!(fate, Fate::Alive) {
            keep.push(pr);
        }
    }
    projs.list = keep;
}

#[allow(clippy::too_many_arguments)]
fn advance(
    pr: &mut Projectile,
    def: &WeaponDef,
    bp: &BallisticsParams,
    h: f32,
    max_substeps: u32,
    col: &crate::collision::CollisionData,
    targets: &mut [Target],
    q: &mut Query<(&Transform, &Motion, &Team, &mut Health)>,
    events: &mut Vec<SimEvent>,
    hitlog: &mut Vec<HitRec>,
) -> Fate {
    for _ in 0..SUBSTEPS {
        let p0 = pr.pos;
        ballistics::substep(
            &mut pr.pos,
            &mut pr.vel,
            bp.drag_k_per_m,
            bp.gravity_mps2,
            h,
        );
        pr.substeps += 1;
        let p1 = pr.pos;
        let seg = p1 - p0;
        let len = libm::sqrtf(seg.x * seg.x + seg.y * seg.y + seg.z * seg.z);

        // Nearest body hit on this segment.
        let mut body: Option<(f32, HitZone, usize)> = None;
        for (i, t) in targets.iter().enumerate() {
            if !t.alive || t.id == pr.owner || t.team == pr.team {
                continue;
            }
            if !hitbox::seg_aabb_overlap(p0, p1, t.foot) {
                continue;
            }
            if let Some(hit) = hitbox::segment_hit(t.foot, t.blend, p0, p1) {
                if body.is_none_or(|b| hit.t < b.0) {
                    body = Some((hit.t, hit.zone, i));
                }
            }
        }
        // Terrain.
        let ground = if len > 1e-6 {
            col.raycast(p0, seg, len)
        } else {
            None
        };
        let ground_t = ground.map(|g| g.t / len);
        match (body, ground, ground_t) {
            (Some((tb, zone, i)), _, gt) if gt.is_none_or(|g| tb < g) => {
                let hit_pos = p0 + seg * tb;
                let dist = (hit_pos - pr.origin).length();
                let dmg = damage(def, zone, dist);
                let tgt = &mut targets[i];
                if let Ok((_, _, _, mut hp)) = q.get_mut(tgt.entity) {
                    hp.hp -= dmg;
                    if hp.hp <= 0.0 {
                        tgt.alive = false;
                    }
                }
                events.push(SimEvent::Hit {
                    shooter: pr.owner,
                    target: tgt.id,
                    zone,
                    damage_hp: dmg,
                    dist_m: dist,
                });
                hitlog.push(HitRec {
                    target: tgt.id,
                    shooter: pr.owner,
                    weapon: pr.weapon,
                    zone,
                    dist_m: dist,
                });
                return Fate::Gone;
            }
            (_, Some(g), _) => {
                events.push(SimEvent::Impact {
                    pos: g.point,
                    normal: g.normal,
                    material: g.material,
                });
                return Fate::Gone;
            }
            _ => {}
        }
        // Out of the loaded world, too old or too slow.
        if col.height_at(p1.x, p1.z).is_none() || pr.substeps >= max_substeps {
            return Fate::Gone;
        }
        let v = pr.vel;
        if v.x * v.x + v.y * v.y + v.z * v.z < MIN_SPEED_MPS * MIN_SPEED_MPS {
            return Fate::Gone;
        }
    }
    Fate::Alive
}
