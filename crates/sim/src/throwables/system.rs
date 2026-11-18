//! ECS side of grenades and smoke: the `Gadgets` component (counts and the cooking state), the
//! `Throwables` / `Smokes` resources and the `throwables` system.
//!
//! Input (PI-054): holding `GADGET` starts cooking the frag grenade, `GADGET` with `AIM` held at
//! the start cooks the smoke grenade instead; releasing throws. A frag grenade held past its
//! fuse explodes in the hand. A player who dies while cooking drops the grenade where they fell.

use bevy_ecs::prelude::*;
use glam::Vec3;

use frontline_protocol::msg::input::buttons;

use crate::components::{
    CollisionRes, CurrentTick, EntityTable, EventBuf, LastInput, Motion, SimCfgRes, Team, Transform,
};
use crate::data::defs::{ExplosiveClass, ExplosiveGameplay, Faction};
use crate::data::GameData;
use crate::health::{Health, HitLog, HitRec, LifeState};
use crate::ids::EntityId;
use crate::step::SimEvent;
use crate::throwables::explosion::{explosion_damage, Occlusion, FF_MULT};
use crate::throwables::grenade::{self, Flight, THROW_LIFT_DEG};
use crate::throwables::smoke::{SmokeCloud, SmokeParams};
use crate::throwables::EXPLOSIVE_WEAPON_IDX;
use crate::weapon::spread::view_dir;
use crate::weapon::HitZone;

/// Frag / smoke grenades every player starts with (loadout gadget counts come with M1-020).
pub const START_FRAG: u8 = 2;
pub const START_SMOKE: u8 = 1;
/// Height of the throwing hand above the feet and of the chest (blast centre of a victim).
pub const HAND_HEIGHT_M: f32 = 1.4;
pub const CHEST_HEIGHT_M: f32 = 1.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Frag,
    Smoke,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThrowableParams {
    pub kind: Kind,
    pub fuse_ticks: u16,
    pub speed_mps: f32,
    pub restitution: f32,
    pub friction: f32,
    pub dmax: f32,
    pub radius_m: f32,
    pub can_cook: bool,
    pub smoke: Option<SmokeParams>,
}

/// Grenade parameters per team (the faction's frag and smoke grenade).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct ThrowableTable {
    frag: [Option<ThrowableParams>; 2],
    smoke: [Option<ThrowableParams>; 2],
}

fn ticks(seconds: f64, hz: u8) -> u32 {
    libm::ceilf((seconds * f64::from(hz)) as f32 - 1e-4).max(0.0) as u32
}

impl ThrowableTable {
    pub fn new(data: &GameData, hz: u8) -> Self {
        let mut t = Self::default();
        for e in data.explosives {
            let team = match e.faction {
                Faction::Us => 0,
                Faction::Ger => 1,
            };
            match (&e.class, &e.gameplay) {
                (ExplosiveClass::FragGrenade, ExplosiveGameplay::Frag(g)) => {
                    t.frag[team] = Some(ThrowableParams {
                        kind: Kind::Frag,
                        fuse_ticks: ticks(g.fuse_s, hz).max(1) as u16,
                        speed_mps: g.throw_speed_mps as f32,
                        restitution: g.bounce_mult as f32,
                        friction: g.friction_mult as f32,
                        dmax: g.damage_max_hp as f32,
                        radius_m: g.damage_radius_m as f32,
                        can_cook: g.can_cook,
                        smoke: None,
                    });
                }
                (ExplosiveClass::SmokeGrenade, ExplosiveGameplay::Smoke(g)) => {
                    t.smoke[team] = Some(ThrowableParams {
                        kind: Kind::Smoke,
                        fuse_ticks: ticks(g.fuse_s, hz).max(1) as u16,
                        speed_mps: g.throw_speed_mps as f32,
                        restitution: 0.4,
                        friction: 0.6,
                        dmax: 0.0,
                        radius_m: 0.0,
                        can_cook: false,
                        smoke: Some(SmokeParams {
                            radius_m: g.cloud_radius_m as f32,
                            expand_ticks: ticks(g.expand_s, hz),
                            duration_ticks: ticks(g.duration_s, hz),
                            decay_ticks: ticks(g.decay_s, hz),
                        }),
                    });
                }
                _ => {}
            }
        }
        t
    }

    pub fn params(&self, team: Team, kind: Kind) -> Option<&ThrowableParams> {
        let i = team as usize;
        match kind {
            Kind::Frag => self.frag[i].as_ref(),
            Kind::Smoke => self.smoke[i].as_ref(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cook {
    pub kind: Kind,
    pub ticks: u16,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gadgets {
    pub frag: u8,
    pub smoke: u8,
    pub cook: Option<Cook>,
}

impl Default for Gadgets {
    fn default() -> Self {
        Self {
            frag: START_FRAG,
            smoke: START_SMOKE,
            cook: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thrown {
    pub id: u32,
    pub owner: EntityId,
    pub team: Team,
    pub kind: Kind,
    pub flight: Flight,
    pub fuse_left: u16,
}

#[derive(Resource, Debug, Default)]
pub struct Throwables {
    pub list: Vec<Thrown>,
    pub next_id: u32,
}

#[derive(Resource, Debug, Default)]
pub struct Smokes {
    pub clouds: Vec<SmokeCloud>,
}

/// Blast at `center`: damages every living player in range. Returns nothing; writes events and
/// the hit log. `owner` takes the full damage, teammates `FF_MULT`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn explode(
    center: Vec3,
    dmax: f32,
    radius_m: f32,
    kind: u8,
    owner: EntityId,
    team: Team,
    col: &crate::collision::CollisionData,
    table: &EntityTable,
    q: &mut Query<(&Transform, &Team, &LifeState, &mut Health)>,
    events: &mut Vec<SimEvent>,
    log: &mut Vec<HitRec>,
) {
    events.push(SimEvent::Explosion {
        pos: center,
        radius_m,
        kind,
    });
    for (id, e) in table.sorted() {
        let Ok((t, vteam, life, mut hp)) = q.get_mut(e) else {
            continue;
        };
        if life.is_dead() || hp.hp <= 0.0 {
            continue;
        }
        let chest = t.pos + Vec3::Y * CHEST_HEIGHT_M;
        let r = (chest - center).length();
        if r >= radius_m {
            continue;
        }
        let occ = if col.los(center + Vec3::Y * 0.05, chest) {
            Occlusion::Clear
        } else {
            Occlusion::Solid
        };
        let mut d = explosion_damage(dmax, r, radius_m, occ);
        if id != owner && *vteam == team {
            d *= FF_MULT;
        }
        if d <= 0.0 {
            continue;
        }
        hp.hp -= d;
        events.push(SimEvent::Hit {
            shooter: owner,
            target: id,
            zone: HitZone::Torso,
            damage_hp: d,
            dist_m: r,
        });
        log.push(HitRec {
            target: id,
            shooter: owner,
            weapon: EXPLOSIVE_WEAPON_IDX,
            zone: HitZone::Torso,
            dist_m: r,
            damage: d,
            kind: crate::combat::DamageKind::Explosion,
            direct_kia: false,
        });
    }
}

/// Throwables stage (between weapons and projectiles): cooking, throws, flight, fuses, blasts and
/// smoke. Players are processed in ascending `EntityId`, then grenades in ascending id.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn throwables(
    tick: Res<CurrentTick>,
    cfg: Res<SimCfgRes>,
    tt: Res<ThrowableTable>,
    col: Res<CollisionRes>,
    table: Res<EntityTable>,
    mut thrown: ResMut<Throwables>,
    mut smokes: ResMut<Smokes>,
    mut events: ResMut<EventBuf>,
    mut log: ResMut<HitLog>,
    mut players: Query<(
        &Transform,
        &Motion,
        &LastInput,
        &Team,
        &LifeState,
        &mut Gadgets,
    )>,
    mut victims: Query<(&Transform, &Team, &LifeState, &mut Health)>,
) {
    // 1. Cooking and throwing.
    let mut spawns: Vec<Thrown> = Vec::new();
    let mut hand_blasts: Vec<(Vec3, ThrowableParams, EntityId, Team)> = Vec::new();
    for (id, e) in table.sorted() {
        let Ok((t, _m, last, team, life, mut g)) = players.get_mut(e) else {
            continue;
        };
        let held = last.cmd.buttons & buttons::GADGET != 0;
        let alive = life.is_alive();
        if g.cook.is_none() && held && alive {
            let kind = if last.cmd.buttons & buttons::AIM != 0 {
                Kind::Smoke
            } else {
                Kind::Frag
            };
            let have = match kind {
                Kind::Frag => g.frag,
                Kind::Smoke => g.smoke,
            };
            if have > 0 && tt.params(*team, kind).is_some() {
                g.cook = Some(Cook { kind, ticks: 0 });
            }
        } else if let Some(mut c) = g.cook {
            let Some(p) = tt.params(*team, c.kind) else {
                g.cook = None;
                continue;
            };
            c.ticks = c.ticks.saturating_add(1);
            let over = p.can_cook && c.ticks >= p.fuse_ticks;
            if over && alive {
                // Held past the fuse: it goes off in the hand.
                match c.kind {
                    Kind::Frag => g.frag -= 1,
                    Kind::Smoke => g.smoke -= 1,
                }
                g.cook = None;
                hand_blasts.push((t.pos + Vec3::Y * HAND_HEIGHT_M, *p, id, *team));
            } else if !held || !alive {
                // Release (or death): throw, or drop at the feet when dead.
                match c.kind {
                    Kind::Frag => g.frag -= 1,
                    Kind::Smoke => g.smoke -= 1,
                }
                g.cook = None;
                let fuse_left = p
                    .fuse_ticks
                    .saturating_sub(if p.can_cook { c.ticks } else { 0 });
                let (pos, vel) = if alive {
                    let pitch = t.pitch + THROW_LIFT_DEG.to_radians();
                    let dir = view_dir(t.yaw, pitch);
                    (
                        t.pos + Vec3::Y * HAND_HEIGHT_M + dir * 0.4,
                        dir * p.speed_mps,
                    )
                } else {
                    (t.pos + Vec3::Y * 0.3, Vec3::ZERO)
                };
                spawns.push(Thrown {
                    id: 0,
                    owner: id,
                    team: *team,
                    kind: c.kind,
                    flight: Flight { pos, vel, rest: 0 },
                    fuse_left: fuse_left.max(1),
                });
            } else {
                g.cook = Some(c);
            }
        }
    }
    for (center, p, owner, team) in hand_blasts {
        explode(
            center,
            p.dmax,
            p.radius_m,
            0,
            owner,
            team,
            &col.0,
            &table,
            &mut victims,
            &mut events.0,
            &mut log.0,
        );
    }
    // 2. Flight and fuses.
    let list = std::mem::take(&mut thrown.list);
    let mut keep = Vec::with_capacity(list.len());
    for mut gr in list {
        let Some(p) = tt.params(gr.team, gr.kind).copied() else {
            continue;
        };
        grenade::step(&mut gr.flight, &col.0, cfg.hz, p.restitution, p.friction);
        gr.fuse_left = gr.fuse_left.saturating_sub(1);
        if gr.fuse_left > 0 {
            keep.push(gr);
            continue;
        }
        match (gr.kind, p.smoke) {
            (Kind::Frag, _) => explode(
                gr.flight.pos,
                p.dmax,
                p.radius_m,
                0,
                gr.owner,
                gr.team,
                &col.0,
                &table,
                &mut victims,
                &mut events.0,
                &mut log.0,
            ),
            (Kind::Smoke, Some(sp)) => {
                smokes.clouds.push(SmokeCloud {
                    pos: gr.flight.pos,
                    start: tick.0,
                    params: sp,
                });
                events.0.push(SimEvent::SmokeStarted {
                    pos: gr.flight.pos,
                    radius_m: sp.radius_m,
                });
            }
            _ => {}
        }
    }
    thrown.list = keep;
    // New throws start flying (and burning their fuse) on the next tick.
    for mut s in spawns {
        s.id = thrown.next_id;
        thrown.next_id += 1;
        events.0.push(SimEvent::Thrown {
            thrower: s.owner,
            pos: s.flight.pos,
            smoke: s.kind == Kind::Smoke,
        });
        thrown.list.push(s);
    }
    // 3. Clouds that ran out.
    let now = tick.0;
    let before = smokes.clouds.len();
    smokes.clouds.retain(|c| !c.ended(now));
    if smokes.clouds.len() != before {
        events.0.push(SimEvent::SmokeEnded {
            count: (before - smokes.clouds.len()) as u8,
        });
    }
}
