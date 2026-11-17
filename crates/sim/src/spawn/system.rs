//! Spawn protection and the HQ protection zone (run before `deaths`).

use std::collections::BTreeSet;

use bevy_ecs::prelude::*;
use frontline_protocol::msg::input::buttons;

use crate::components::{CurrentTick, EntityTable, LastInput, Player, SimCfgRes, Team, Transform};
use crate::health::{Health, HitLog, HitRec, LifeState};
use crate::step::SimEvent;
use crate::throwables::EXPLOSIVE_WEAPON_IDX;
use crate::weapon::HitZone;

use super::{SpawnProtection, SpawnRules, HQ_ZONE_DPS, HQ_ZONE_RADIUS_M, HQ_ZONE_WARN_S};

/// Spawn protection: damage x0 until `until`; moving or shooting cancels it. Implemented as
/// "undo this tick's damage" so no damage source needs to know about it.
pub fn protection(
    tick: Res<CurrentTick>,
    table: Res<EntityTable>,
    mut log: ResMut<HitLog>,
    mut q: Query<(&mut SpawnProtection, &mut Health, &LastInput, &LifeState)>,
) {
    let now = tick.0 .0;
    let mut shielded: BTreeSet<crate::ids::EntityId> = BTreeSet::new();
    for (id, e) in table.sorted() {
        let Ok((mut sp, mut hp, last, life)) = q.get_mut(e) else {
            continue;
        };
        if sp.until == 0 {
            continue;
        }
        let c = &last.cmd;
        if now >= sp.until
            || life.is_dead()
            || c.move_x != 0
            || c.move_z != 0
            || c.buttons & buttons::FIRE != 0
        {
            sp.until = 0;
            continue;
        }
        hp.hp = hp.max_hp;
        shielded.insert(id);
    }
    if !shielded.is_empty() {
        log.0.retain(|r| !shielded.contains(&r.target));
    }
}

/// HQ zone: an enemy in the 150 m zone gets a warning on entry, then 40 HP/s after 3 s.
#[allow(clippy::type_complexity)]
pub fn hq_zone(
    tick: Res<CurrentTick>,
    cfg: Res<SimCfgRes>,
    table: Res<EntityTable>,
    mut rules: ResMut<SpawnRules>,
    mut log: ResMut<HitLog>,
    mut events: ResMut<crate::components::EventBuf>,
    mut q: Query<(&Player, &Team, &Transform, &mut Health, &LifeState)>,
) {
    if !rules.active() {
        return;
    }
    let now = tick.0 .0;
    let warn = HQ_ZONE_WARN_S * u32::from(cfg.hz);
    let mut inside = BTreeSet::new();
    for (eid, e) in table.sorted() {
        let Ok((p, team, t, mut hp, life)) = q.get_mut(e) else {
            continue;
        };
        if life.is_dead() {
            continue;
        }
        let pid = p.id;
        let Some(h) = rules.hq_of(match *team {
            Team::Us => Team::Ger,
            Team::Ger => Team::Us,
        }) else {
            continue;
        };
        let d2 = (t.pos.x - h.x).powi(2) + (t.pos.z - h.z).powi(2);
        if d2 > HQ_ZONE_RADIUS_M * HQ_ZONE_RADIUS_M {
            continue;
        }
        inside.insert(pid);
        let since = *rules.zone_since.entry(pid).or_insert_with(|| {
            events.0.push(SimEvent::HqWarning { player: pid });
            now
        });
        if now.saturating_sub(since) >= warn {
            let d = HQ_ZONE_DPS / f32::from(cfg.hz);
            hp.hp -= d;
            log.0.push(HitRec {
                target: eid,
                shooter: eid,
                weapon: EXPLOSIVE_WEAPON_IDX,
                zone: HitZone::Torso,
                dist_m: 0.0,
                damage: d,
                kind: crate::combat::DamageKind::Explosion,
                direct_kia: hp.hp <= 0.0,
            });
        }
    }
    rules.zone_since.retain(|p, _| inside.contains(p));
}
