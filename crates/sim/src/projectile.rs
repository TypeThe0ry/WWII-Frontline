//! Server-authoritative projectiles (GAMEDESIGN 6.4, M0-012): flight, terrain / body hits, damage.
//!
//! Each tick a projectile integrates [`SUBSTEPS`] sub-steps (gravity + quadratic drag, see
//! [`crate::weapon::ballistics`]); every sub-step segment is tested against the terrain
//! (`CollisionData::raycast`) and the hit volumes of the enemy players
//! ([`crate::weapon::hitbox`]); the nearest hit ends the projectile. The shooter is ignored;
//! teammates take `ff_mult` damage ([`crate::combat::damage`], M1-014; no penetration yet).
//! A projectile that leaves the loaded collision, lives longer than 4 s or drops below 150 m/s
//! is removed silently. Damage `D = damage_torso_hp * falloff(d) * part_mult` is subtracted from
//! the target's [`Health`] and reported as [`SimEvent::Hit`] (and logged in [`HitLog`] for the
//! rules stage, which turns a lethal hit into a kill: `crate::schedule::deaths`).

use bevy_ecs::prelude::*;
use glam::Vec3;

use crate::combat::damage::{damage_amount, DamageKind, HitInfo, TargetCtx};
use crate::combat::lagcomp::{protected_by_cover, LagComp, LagCompRules, ProjectileClock};
use crate::combat::lagcomp_history::{HitboxFrame, LagCompHistory, PlayerHitboxes};
use crate::components::{
    CollisionRes, CurrentTick, EntityTable, EventBuf, Motion, MoveParamsRes, SimCfgRes, Team,
    Transform, WeaponTable,
};
use crate::data::defs::WeaponDef;
use crate::health::{Health, HitLog, HitRec};
use crate::ids::{EntityId, Tick};
use crate::step::SimEvent;
use crate::suppression::detect::Grid;
use crate::suppression::{system::pass_segment, SuppressSeen, SuppressionState};
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
    /// Initial lag-compensation rewind in ticks (M1-025); 0 = live from spawn.
    pub rewind: u8,
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
            rewind: 0,
        });
        id
    }

    /// Sets the rewind of projectile `id` (just spawned).
    pub fn set_rewind(&mut self, id: u32, rewind: u8) {
        if let Some(p) = self.list.iter_mut().rev().find(|p| p.id == id) {
            p.rewind = rewind;
        }
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

/// `D = dmg * falloff(d) * part_mult` against an enemy wearing a helmet (GAMEDESIGN 6.6); the
/// full model with friendly fire and penetration is [`crate::combat::compute_damage`].
pub fn damage(def: &WeaponDef, zone: HitZone, dist_m: f32) -> f32 {
    crate::combat::damage::damage_amount(
        def,
        &HitInfo {
            zone,
            travelled_m: dist_m,
            pen_mult: 1.0,
            kind: DamageKind::Bullet,
        },
        &TargetCtx {
            hp: 0.0,
            same_team: false,
            helmet: true,
            friendly_fire: true,
        },
    )
}

enum Fate {
    Alive,
    Gone,
    WallRejected,
}

struct RewindCollision<'a> {
    frame: &'a HitboxFrame,
    next: Option<&'a HitboxFrame>,
    next_is_live: bool,
    history: &'a LagCompHistory,
    rules: LagCompRules,
    shooter_eye: Option<Vec3>,
}

impl RewindCollision<'_> {
    fn pose(&self, target: &Target, frac: f32) -> Option<PlayerHitboxes> {
        let pose = self.frame.player(target.id)?;
        if frac == 0.0 {
            return Some(pose);
        }
        let next = if self.next_is_live {
            // The current tick has moved the player but has yet to record its history frame.
            Some(PlayerHitboxes {
                foot: target.foot,
                blend: target.blend,
                ..pose
            })
        } else {
            self.next.and_then(|frame| frame.player(target.id))
        };
        Some(next.map_or(pose, |p| pose.lerp(p, frac)))
    }
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
    tick: Res<CurrentTick>,
    mut lag: ResMut<LagComp>,
    move_params: Res<MoveParamsRes>,
    mut seen: ResMut<SuppressSeen>,
    mut sq: Query<&mut SuppressionState>,
    mut q: Query<(&Transform, &Motion, &Team, &mut Health)>,
    mut profiler: Option<ResMut<crate::step::StepProfiler>>,
) {
    let _sample = crate::step::StepProfiler::measure(&mut profiler, |t| &mut t.projectiles);
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
    let mut cand: Vec<u16> = Vec::new();
    let mut grid = Grid::default();
    for (i, t) in targets.iter().enumerate() {
        grid.insert(i as u16, t.foot);
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
        let rewind = match ProjectileClock::at(tick.0, pr.rewind, pr.substeps) {
            ProjectileClock::ViewTime(view_tick) if lag.enabled => {
                let lag = &mut *lag;
                let selected = lag.history.frame_clamped(view_tick);
                if selected.is_none_or(|(_, clamped)| clamped) {
                    lag.stats.history_clamped += 1;
                }
                selected.map(|(frame, clamped)| RewindCollision {
                    frame,
                    next: (!clamped)
                        .then(|| lag.history.frame(view_tick.next()))
                        .flatten(),
                    next_is_live: !clamped && view_tick.next() == tick.0,
                    history: &lag.history,
                    rules: lag.rules,
                    shooter_eye: move_params.0.and_then(|params| {
                        let (transform, motion, _, _) = q.get(table.get(pr.owner)?).ok()?;
                        let auth = motion.to_auth(transform);
                        let height = crate::movement::eye_height(auth.stance_blend, &params);
                        Some(
                            auth.pos
                                + Vec3::Y * height
                                + crate::movement::lean::head_offset(&auth, &params),
                        )
                    }),
                })
            }
            _ => None,
        };
        let fate = advance(
            &mut pr,
            def,
            &bp,
            h,
            max_sub,
            &col.0,
            &mut targets,
            &grid,
            &mut cand,
            &mut q,
            &mut events.0,
            &mut hitlog.0,
            (tick.0, cfg.hz),
            &mut seen,
            &mut sq,
            rewind.as_ref(),
        );
        match fate {
            Fate::Alive => keep.push(pr),
            Fate::WallRejected => lag.stats.wall_rejected += 1,
            Fate::Gone => {}
        }
    }
    if !seen.0.is_empty() {
        let mut live: Vec<u32> = keep.iter().map(|p| p.id).collect();
        live.sort_unstable();
        seen.0.retain(|(id, _)| live.binary_search(id).is_ok());
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
    grid: &Grid,
    cand: &mut Vec<u16>,
    q: &mut Query<(&Transform, &Motion, &Team, &mut Health)>,
    events: &mut Vec<SimEvent>,
    hitlog: &mut Vec<HitRec>,
    (tick, hz): (Tick, u8),
    seen: &mut SuppressSeen,
    sq: &mut Query<&mut SuppressionState>,
    rewind: Option<&RewindCollision<'_>>,
) -> Fate {
    for substep in 0..SUBSTEPS {
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
            if !t.alive || t.id == pr.owner {
                continue;
            }
            let (foot, blend) = if let Some(rewind) = rewind {
                let Some(pose) = rewind.pose(t, substep as f32 / SUBSTEPS as f32) else {
                    continue; // No such target at the sampled view time.
                };
                (pose.foot, pose.blend)
            } else {
                (t.foot, t.blend)
            };
            if !hitbox::seg_aabb_overlap(p0, p1, foot) {
                continue;
            }
            if let Some(hit) = hitbox::segment_hit(foot, blend, p0, p1) {
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
        // Near misses along the part of the segment flown (M1-017).
        {
            let (end, hit_id) = match (body, ground_t) {
                (Some((tb, _, i)), gt) if gt.is_none_or(|g| tb < g) => (tb, Some(targets[i].id)),
                (_, Some(g)) => (g, None),
                _ => (1.0, None),
            };
            let speed = pr.vel.length();
            pass_segment(
                p0,
                p0 + seg * end,
                speed,
                pr.id,
                pr.owner,
                pr.team,
                &def.class,
                tick.0,
                hz,
                hit_id,
                {
                    grid.query(p0, p0 + seg * end, cand);
                    cand.iter().filter_map(|&i| {
                        let t = &targets[usize::from(i)];
                        t.alive.then_some((t.id, t.entity, t.team, t.foot, t.blend))
                    })
                },
                seen,
                sq,
                events,
            );
        }
        match (body, ground, ground_t) {
            (Some((tb, zone, i)), _, gt) if gt.is_none_or(|g| tb < g) => {
                let hit_pos = p0 + seg * tb;
                let dist = (hit_pos - pr.origin).length();
                let tgt = &mut targets[i];
                if rewind.is_some_and(|r| {
                    r.shooter_eye.is_some_and(|shooter_eye| {
                        protected_by_cover(
                            col,
                            r.history,
                            r.rules,
                            tick,
                            hz,
                            shooter_eye,
                            PlayerHitboxes {
                                id: tgt.id,
                                foot: tgt.foot,
                                blend: tgt.blend,
                                head_dx: 0.0,
                                head_dz: 0.0,
                            },
                        )
                    })
                }) {
                    return Fate::WallRejected;
                }
                let same_team = tgt.team == pr.team;
                let hp_before = q.get(tgt.entity).map_or(0.0, |(_, _, _, h)| h.hp);
                let dmg = damage_amount(
                    def,
                    &HitInfo {
                        zone,
                        travelled_m: dist,
                        pen_mult: 1.0,
                        kind: DamageKind::Bullet,
                    },
                    &TargetCtx {
                        hp: hp_before,
                        same_team,
                        helmet: true,
                        friendly_fire: true,
                    },
                );
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
                    damage: dmg,
                    kind: DamageKind::Bullet,
                    direct_kia: false,
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
