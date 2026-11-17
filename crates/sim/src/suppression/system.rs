//! Suppression systems: near-miss detection (called from the projectile stage), the per-tick
//! decay / explosion / event stage, and the anti-farm assist stage.

use std::collections::BTreeSet;

use bevy_ecs::prelude::*;
use glam::Vec3;

use super::anti_farm::{self, assisting};
use super::detect::{amount, axis, seg_seg_distance};
use super::state::{decay, SuppressionState};
use super::{
    ASSIST_WINDOW_S, EVENT_MAX_HZ, EXPLOSION_R_M, EXPLOSION_S, PINNED_HOLD_S, PINNED_S, WALL_MULT,
    WINDOW_S,
};
use crate::components::{
    CollisionRes, CurrentTick, EntityTable, EventBuf, Motion, PlayerIndex, SimCfgRes, Team,
    Transform,
};
use crate::health::LifeState;
use crate::ids::EntityId;
use crate::step::SimEvent;
use crate::weapon::hitbox;

/// `(projectile id, target)` pairs already suppressed (a bullet suppresses a player once).
#[derive(Resource, Debug, Default)]
pub struct SuppressSeen(pub BTreeSet<(u32, EntityId)>);

pub fn ticks(s: f32, hz: u8) -> u32 {
    (s * f32::from(hz)).ceil() as u32
}

/// World bearing of `from -> to` quantised to 256 steps (`0` = +z, clockwise towards +x).
pub fn bearing_q(from: Vec3, to: Vec3) -> u8 {
    let a = libm::atan2f(to.x - from.x, to.z - from.z);
    let a = if a < 0.0 {
        a + core::f32::consts::TAU
    } else {
        a
    };
    (a / core::f32::consts::TAU * 256.0 + 0.5) as u32 as u8
}

/// Applies `amount` to `st`; sends a throttled `Suppress` event.
#[allow(clippy::too_many_arguments)]
pub fn apply(
    st: &mut SuppressionState,
    amt: f32,
    tick: u32,
    hz: u8,
    target: EntityId,
    target_pos: Vec3,
    src: Vec3,
    events: &mut Vec<SimEvent>,
) {
    if !st.add(amt, tick, ticks(WINDOW_S, hz)) {
        return;
    }
    st.src_pos = src;
    if st.s >= PINNED_S {
        st.pinned_until = tick + ticks(PINNED_HOLD_S, hz);
    }
    let gap = u32::from(hz / EVENT_MAX_HZ as u8).max(1);
    if st.last_event == 0 || tick.saturating_sub(st.last_event) >= gap {
        st.last_event = tick.max(1);
        events.push(SimEvent::Suppress {
            target,
            source_dir: bearing_q(target_pos, src),
            intensity: (st.s * 2.55 + 0.5).min(255.0) as u8,
        });
    }
}

/// One sub-step of a projectile: suppresses enemies it passed. Called from the projectile stage
/// with the segment actually flown (cut at a hit), excluding the player that was hit.
#[allow(clippy::too_many_arguments)]
pub fn pass_segment(
    p0: Vec3,
    p1: Vec3,
    speed: f32,
    proj_id: u32,
    owner: EntityId,
    team: Team,
    class: &str,
    tick: u32,
    hz: u8,
    hit_target: Option<EntityId>,
    targets: impl Iterator<Item = (EntityId, Entity, Team, Vec3, f32)>,
    seen: &mut SuppressSeen,
    sq: &mut Query<&mut SuppressionState>,
    events: &mut Vec<SimEvent>,
) {
    for (id, e, tteam, foot, blend) in targets {
        if id == owner || tteam == team || Some(id) == hit_target {
            continue;
        }
        if seen.0.contains(&(proj_id, id)) {
            continue;
        }
        if !super::detect::may_pass(p0, p1, foot) {
            continue;
        }
        let head = hitbox::volumes(blend)[0].y1;
        let (a, b) = axis(foot, head);
        let d = seg_seg_distance(p0, p1, a, b);
        if d > super::D_MAX_M {
            continue;
        }
        let amt = amount(class, d, speed);
        seen.0.insert((proj_id, id));
        if let Ok(mut st) = sq.get_mut(e) {
            super::anti_farm::record(&mut st.sources, owner, team, tick);
            apply(&mut st, amt, tick, hz, id, foot, p0, events);
        }
    }
}

/// Decay, explosions, dead reset (runs after the projectile stage, before `deaths`).
#[allow(clippy::type_complexity)]
pub fn suppression(
    tick: Res<CurrentTick>,
    cfg: Res<SimCfgRes>,
    col: Res<CollisionRes>,
    table: Res<EntityTable>,
    mut events: ResMut<EventBuf>,
    mut q: Query<(&Transform, &Motion, &LifeState, &mut SuppressionState)>,
) {
    let now = tick.0 .0;
    let hz = cfg.hz;
    let blasts: Vec<Vec3> = events
        .0
        .iter()
        .filter_map(|e| match e {
            SimEvent::Explosion { pos, kind: 0, .. } => Some(*pos),
            _ => None,
        })
        .collect();
    for (id, e) in table.sorted() {
        let Ok((t, m, life, mut st)) = q.get_mut(e) else {
            continue;
        };
        // Dead players reset (a lethal hit of this tick is still on record for the assists).
        if life.is_dead() {
            if st.is_active() {
                *st = SuppressionState::default();
            }
            continue;
        }
        for &c in &blasts {
            let r = (t.pos - c).length();
            if r >= EXPLOSION_R_M {
                continue;
            }
            let f = 1.0 - r / EXPLOSION_R_M;
            let mut amt = EXPLOSION_S * f * f;
            let chest = t.pos + Vec3::Y * 1.2;
            if !col.0.los(c, chest) {
                amt *= WALL_MULT;
            }
            apply(&mut st, amt, now, hz, id, t.pos, c, &mut events.0);
        }
        if st.s > 0.0 {
            let cover = !col.0.los(st.src_pos, t.pos + Vec3::Y * 1.2);
            decay(&mut st, now, hz, m.stance_blend, cover);
        }
        // Forget suppressors older than the assist window.
        let w = ticks(ASSIST_WINDOW_S, hz);
        if !st.sources.is_empty() {
            st.sources.retain(|s| now.saturating_sub(s.tick) <= w);
        }
    }
}

/// `SuppressionAssist` for each suppressor of a victim killed by a teammate within 5 s.
#[allow(clippy::type_complexity)]
pub fn suppression_assists(
    tick: Res<CurrentTick>,
    cfg: Res<SimCfgRes>,
    players: Res<PlayerIndex>,
    mut events: ResMut<EventBuf>,
    mut q: Query<(&Team, &mut SuppressionState)>,
) {
    let now = tick.0 .0;
    let w = ticks(ASSIST_WINDOW_S, cfg.hz);
    let kills: Vec<_> = events
        .0
        .iter()
        .filter_map(|e| match e {
            SimEvent::Kill { victim, killer, .. } => Some((*victim, *killer)),
            _ => None,
        })
        .collect();
    let mut out = Vec::new();
    for (victim, killer) in kills {
        let Some(&(_, ve)) = players.map.get(&victim) else {
            continue;
        };
        let (kid, kteam) = match killer.and_then(|k| players.map.get(&k).copied()) {
            Some((kid, ke)) => (Some(kid), q.get(ke).ok().map(|r| *r.0)),
            None => (None, None),
        };
        let Ok((_, mut st)) = q.get_mut(ve) else {
            continue;
        };
        for s in assisting(&st.sources, now, w, kid, kteam) {
            out.push(SimEvent::SuppressionAssist {
                suppressor: s,
                victim,
            });
        }
        *st = SuppressionState::default();
    }
    events.0.extend(out);
    let _ = anti_farm::record;
}
