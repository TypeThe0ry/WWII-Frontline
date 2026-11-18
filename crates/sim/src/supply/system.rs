//! Depot timers, crate carrying and the per-tick upkeep of crates.

use bevy_ecs::prelude::*;
use frontline_protocol::msg::input::buttons;

use crate::components::{EntityTable, EventBuf, LastInput, Player, Team, Transform};
use crate::health::LifeState;
use crate::rules::Objectives;
use crate::step::SimEvent;

use super::depot::tick_store;
use super::{
    can_place_supply_crate, dist_xz, limits, Carrying, SupplyCfg, SupplyCrate, SupplyWorld,
    Support, INTERACT_RANGE_M,
};

/// Notice codes of `SimEvent::SupplyNotice`.
pub const NOTICE_NO_PLACE: u8 = 1;

#[allow(clippy::type_complexity)]
pub fn supply(
    cfg: Res<SupplyCfg>,
    table: Res<EntityTable>,
    objs: Res<Objectives>,
    mut sw_res: ResMut<SupplyWorld>,
    mut events: ResMut<EventBuf>,
    mut q: Query<(
        &Player,
        &Team,
        &Transform,
        &LifeState,
        &LastInput,
        &Support,
        &mut Carrying,
    )>,
) {
    let sw = &mut *sw_res;
    if !sw.active() {
        return;
    }
    // Stores.
    for d in &mut sw.depots {
        tick_store(
            &mut d.stock,
            &mut d.timer,
            cfg.depot_interval,
            cfg.depot_max,
        );
    }
    for p in &mut sw.points {
        let owner = objs
            .0
            .iter()
            .find(|o| o.id == p.objective)
            .and_then(|o| o.owner);
        if owner != p.owner {
            p.owner = owner;
            p.stock = 0;
            p.timer = 0;
        }
        if owner.is_some() {
            tick_store(
                &mut p.stock,
                &mut p.timer,
                cfg.point_interval,
                cfg.point_max,
            );
        }
    }
    for team in [Team::Us, Team::Ger] {
        for gone in limits::evict(sw, team, cfg.ground_max) {
            events.0.push(SimEvent::CrateDestroyed { id: gone });
        }
    }
    // Players.
    for (_, e) in table.sorted() {
        let Ok((pl, team, tr, life, last, support, mut carry)) = q.get_mut(e) else {
            continue;
        };
        let pid = pl.id;
        let alive = !life.is_dead() && !life.is_downed();
        if !alive {
            if let Some(cid) = carry.0.take() {
                let seq = sw.next_seq;
                if let Some(c) = sw.crates.get_mut(&cid) {
                    c.carried_by = None;
                    c.pos = tr.pos;
                    c.seq = seq;
                    sw.next_seq += 1;
                }
            }
            sw.hold.remove(&pid);
            continue;
        }
        // A destroyed crate vanishes from the carrier's hands.
        if carry.0.is_some_and(|cid| !sw.crates.contains_key(&cid)) {
            carry.0 = None;
        }
        if let Some(cid) = carry.0 {
            if let Some(c) = sw.crates.get_mut(&cid) {
                c.pos = tr.pos;
            }
        }
        if !support.0 || last.cmd.buttons & buttons::USE == 0 {
            sw.hold.remove(&pid);
            continue;
        }
        let h = sw.hold.entry(pid).or_insert(0);
        *h += 1;
        if *h != cfg.interact_ticks {
            continue;
        }
        match carry.0 {
            Some(cid) => {
                if !can_place_supply_crate(&objs.0, *team, tr.pos) {
                    events.0.push(SimEvent::SupplyNotice {
                        player: pid,
                        code: NOTICE_NO_PLACE,
                    });
                    continue;
                }
                let seq = sw.next_seq;
                sw.next_seq += 1;
                if let Some(c) = sw.crates.get_mut(&cid) {
                    c.carried_by = None;
                    c.pos = tr.pos;
                    c.seq = seq;
                    events.0.push(SimEvent::CratePlaced {
                        id: cid,
                        team: *team,
                        pos: tr.pos,
                    });
                }
                carry.0 = None;
                for gone in limits::evict(sw, *team, cfg.ground_max) {
                    events.0.push(SimEvent::CrateDestroyed { id: gone });
                }
            }
            None => {
                // A friendly ground crate within reach first, else the HQ Depot / point store.
                let mut best: Option<(f32, u32)> = None;
                for c in sw.crates.values() {
                    if c.team == *team && c.carried_by.is_none() {
                        let d = dist_xz(c.pos, tr.pos);
                        if d <= INTERACT_RANGE_M && best.is_none_or(|(bd, _)| d < bd) {
                            best = Some((d, c.id));
                        }
                    }
                }
                if let Some((_, cid)) = best {
                    sw.crates.get_mut(&cid).unwrap().carried_by = Some(pid);
                    carry.0 = Some(cid);
                    continue;
                }
                let supplies = cfg.crate_supplies;
                let hp = cfg.crate_hp;
                let mut took = false;
                if let Some(d) = sw.depots.iter_mut().find(|d| {
                    d.team == *team && d.stock > 0 && dist_xz(d.pos, tr.pos) <= INTERACT_RANGE_M
                }) {
                    d.stock -= 1;
                    took = true;
                } else if let Some(p) = sw.points.iter_mut().find(|p| {
                    p.owner == Some(*team)
                        && p.stock > 0
                        && dist_xz(p.pos, tr.pos) <= INTERACT_RANGE_M
                }) {
                    p.stock -= 1;
                    took = true;
                }
                if took {
                    let id = sw.next_id;
                    sw.next_id += 1;
                    sw.crates.insert(
                        id,
                        SupplyCrate {
                            id,
                            team: *team,
                            pos: tr.pos,
                            supplies,
                            hp,
                            carried_by: Some(pid),
                            seq: u32::MAX,
                        },
                    );
                    carry.0 = Some(id);
                }
            }
        }
    }
}
