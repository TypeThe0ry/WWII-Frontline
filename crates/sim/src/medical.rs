//! Life-state transitions, bleeding, give-up, revive and kill credit (M1-016a, GAMEDESIGN 7).
//!
//! `Alive -> Downed -> Dead` (the existing [`LifeState::Dead`] is KIA). While downed,
//! [`Health::hp`](crate::health::Health) holds the downed hit points; bleeding and further hits
//! lower it and `hp <= 0` is KIA. Credit: the downing counts as a kill for the player who dealt it,
//! the player who finishes a downed enemy gets a "finish" instead, and anyone else who dealt at
//! least [`ASSIST_MIN_HP`] in the last 15 s is an assist (GAMEDESIGN 7.5).
//!
//! Numbers are constants here (PI-055), not yet in `/data/health.json`.

use bevy_ecs::prelude::*;
use glam::Vec3;

use frontline_protocol::msg::input::buttons;

use crate::combat::credit::RecentDamage;
use crate::combat::DamageKind;
use crate::components::RespawnPolicyRes;
use crate::components::{
    CollisionRes, CurrentTick, EntityTable, EventBuf, LastInput, Player, SimCfgRes, Team, Transform,
};
use crate::health::{Health, HitLog, HitRec, LifeState};
use crate::ids::{EntityId, PlayerId, Tick};
use crate::respawn::{respawn_time, RespawnQueue};
use crate::step::SimEvent;
use crate::weapon::HitZone;

/// Hit points a player has right after going down or being revived.
pub const DOWNED_HP: f32 = 40.0;
/// Leaves hp at or below this: killed outright.
pub const OVERKILL_HP: f32 = -60.0;
/// Seconds a downed player takes to bleed out (40 hp at 0.5 hp/s).
pub const BLEED_OUT_S: f32 = 80.0;
/// Each revive in the fatigue window shortens the next bleed-out by this much, down to the floor.
pub const FATIGUE_STEP_S: f32 = 20.0;
pub const BLEED_FLOOR_S: f32 = 20.0;
pub const FATIGUE_WINDOW_S: u32 = 120;
/// Seconds after going down before giving up is allowed, and how long the button is held.
pub const GIVE_UP_DELAY_S: f32 = 5.0;
pub const GIVE_UP_HOLD_S: f32 = 3.0;
pub const REVIVE_S: f32 = 5.0;
pub const REVIVE_RANGE_M: f32 = 1.5;
/// A broken revive keeps its progress this long.
pub const REVIVE_RESUME_S: f32 = 3.0;
/// The medic loses progress when they lose this much hp during the revive.
pub const REVIVE_INTERRUPT_HP: f32 = 20.0;
pub const REVIVE_FIRE_LOCK_S: f32 = 2.0;
/// Damage inside this window from other players counts for assists, and must reach the minimum.
pub const ASSIST_MIN_HP: f32 = 25.0;

/// The button a downed player holds to give up (PI-055).
pub const GIVE_UP_BUTTON: u16 = buttons::RELOAD;

fn ticks(seconds: f32, hz: u8) -> u32 {
    libm::ceilf(seconds * f32::from(hz) - 1e-3).max(0.0) as u32
}

/// Marks a player as a medic (only medics revive; set with `World::set_medic`).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Medic;

/// A revive in progress on a medic.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct ReviveAction(pub Option<Revive>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Revive {
    pub target: EntityId,
    pub ticks: u16,
    pub last_active: Tick,
    pub medic_hp: f32,
}

/// Ticks of this player's recent revives (for the fatigue rule).
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviveHistory(pub Vec<Tick>);

/// The player cannot fire before this tick (after a revive).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FireLock(pub Tick);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreditKind {
    /// The hit that downed the victim.
    Down,
    /// The hit that killed an already downed victim.
    Finish,
    /// Straight from alive to KIA.
    Direct,
}

/// Bleed-out duration for a player who was revived `fatigue` times lately.
pub fn bleed_out_ticks(fatigue: u8, hz: u8) -> u32 {
    let s = (BLEED_OUT_S - FATIGUE_STEP_S * f32::from(fatigue)).max(BLEED_FLOOR_S);
    ticks(s, hz)
}

/// Whether a hit that left the victim at `hp_after` kills outright instead of downing.
pub fn kills_outright(rec: &HitRec, hp_after: f32) -> bool {
    hp_after <= OVERKILL_HP
        || rec.direct_kia
        || matches!(rec.kind, DamageKind::Explosion | DamageKind::Vehicle)
        || (rec.kind == DamageKind::Bullet && rec.zone == HitZone::Head)
}

/// Rules stage, deaths (ARCHITECTURE 6.10 stage 5): applies the hits of this tick to the life
/// state, schedules respawns of the dead and emits the kill credit.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn deaths(
    tick: Res<CurrentTick>,
    cfg: Res<SimCfgRes>,
    policy: Res<RespawnPolicyRes>,
    table: Res<EntityTable>,
    mut log: ResMut<HitLog>,
    mut queue: ResMut<RespawnQueue>,
    mut events: ResMut<EventBuf>,
    mut q: Query<(
        &Player,
        &Team,
        &mut Health,
        &mut LifeState,
        &mut RecentDamage,
        &ReviveHistory,
    )>,
) {
    if log.0.is_empty() {
        return;
    }
    for rec in std::mem::take(&mut log.0) {
        let Some(victim_e) = table.get(rec.target) else {
            continue;
        };
        let killer = table
            .get(rec.shooter)
            .and_then(|e| q.get(e).ok())
            .map(|(p, t, ..)| (p.id, *t));
        // Read the victim first (copies), so the assist lookup can borrow the query again.
        let (vid, vteam, hp_now, downed_before, fatigue_n, mut attackers) = {
            let Ok((vp, vteam, hp, life, recent, hist, ..)) = q.get(victim_e) else {
                continue;
            };
            if life.is_dead() {
                continue;
            }
            let n = hist
                .0
                .iter()
                .filter(|t| tick.0 .0.saturating_sub(t.0) <= FATIGUE_WINDOW_S * u32::from(cfg.hz))
                .count()
                .min(255) as u8;
            let mut att = recent.attackers();
            if rec.damage > 0.0 && rec.shooter != rec.target {
                match att.iter_mut().find(|(a, _)| *a == rec.shooter) {
                    Some((_, t)) => *t += rec.damage,
                    None => att.push((rec.shooter, rec.damage)),
                }
            }
            (vp.id, *vteam, hp.hp, life.is_downed(), n, att)
        };
        if let Ok((_, _, _, _, mut recent, ..)) = q.get_mut(victim_e) {
            if rec.damage > 0.0 && rec.shooter != rec.target {
                recent.record(rec.shooter, rec.damage, tick.0, cfg.hz);
            }
        }
        if hp_now > 0.0 {
            continue;
        }
        attackers.retain(|&(a, amount)| a != rec.shooter && amount >= ASSIST_MIN_HP);
        let killer_id = killer.map(|(id, _)| id);
        let team_kill = killer.is_some_and(|(id, t)| t == vteam && id != vid);
        let headshot = rec.zone == HitZone::Head;
        let weapon = rec.weapon as u8;
        let outright = kills_outright(&rec, hp_now);
        let assists: Vec<PlayerId> = attackers
            .into_iter()
            .filter_map(|(a, _)| table.get(a))
            .filter_map(|e| q.get(e).ok().map(|(p, t, ..)| (p.id, *t)))
            .filter(|&(_, t)| t != vteam)
            .map(|(id, _)| id)
            .collect();
        let Ok((_, _, mut hp, mut life, mut recent, ..)) = q.get_mut(victim_e) else {
            continue;
        };
        if !downed_before && !outright {
            hp.hp = DOWNED_HP;
            *life = LifeState::Downed {
                t_down: tick.0,
                killer: killer_id,
                weapon,
                fatigue: fatigue_n,
                give_up_ticks: 0,
            };
            events.0.push(SimEvent::Downed {
                victim: vid,
                killer: killer_id,
                weapon,
                headshot,
            });
            events.0.push(SimEvent::KillCredit {
                victim: vid,
                killer: killer_id,
                assists,
                kind: CreditKind::Down,
                team_kill,
            });
            recent.prune(tick.0, cfg.hz);
            continue;
        }
        // KIA.
        hp.hp = 0.0;
        let t_kia = tick.0;
        let respawn_tick =
            respawn_time(t_kia, policy.0.pick_tick(t_kia, cfg.hz), &policy.0, cfg.hz);
        *life = LifeState::Dead {
            t_kia,
            killer: killer_id,
            weapon,
            headshot,
            respawn_tick,
        };
        queue.schedule(vid, respawn_tick);
        recent.clear();
        events.0.push(SimEvent::Kill {
            victim: vid,
            killer: killer_id,
            weapon,
            headshot,
            dist_m: rec.dist_m,
        });
        events.0.push(SimEvent::KillCredit {
            victim: vid,
            killer: killer_id,
            assists,
            kind: if downed_before {
                CreditKind::Finish
            } else {
                CreditKind::Direct
            },
            team_kill,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn kia_no_credit(
    tick: Tick,
    cfg: &SimCfgRes,
    policy: &RespawnPolicyRes,
    queue: &mut RespawnQueue,
    events: &mut Vec<SimEvent>,
    p: PlayerId,
    hp: &mut Health,
    life: &mut LifeState,
    weapon: u8,
) {
    hp.hp = 0.0;
    let respawn_tick = respawn_time(tick, policy.0.pick_tick(tick, cfg.hz), &policy.0, cfg.hz);
    *life = LifeState::Dead {
        t_kia: tick,
        killer: None,
        weapon,
        headshot: false,
        respawn_tick,
    };
    queue.schedule(p, respawn_tick);
    events.push(SimEvent::Kill {
        victim: p,
        killer: None,
        weapon,
        headshot: false,
        dist_m: 0.0,
    });
}

/// Medical stage (after `deaths`): bleeding, giving up and reviving.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn medical(
    tick: Res<CurrentTick>,
    cfg: Res<SimCfgRes>,
    policy: Res<RespawnPolicyRes>,
    col: Res<CollisionRes>,
    table: Res<EntityTable>,
    mut queue: ResMut<RespawnQueue>,
    mut events: ResMut<EventBuf>,
    mut q: Query<(
        &Player,
        &Team,
        &Transform,
        &LastInput,
        &mut Health,
        &mut LifeState,
        &mut RecentDamage,
        &mut ReviveHistory,
        &mut ReviveAction,
        &mut FireLock,
        Option<&Medic>,
    )>,
) {
    let now = tick.0;
    let hz = cfg.hz;
    // 1. Bleeding and giving up.
    for (_, e) in table.sorted() {
        let Ok((p, _, _, last, mut hp, mut life, ..)) = q.get_mut(e) else {
            continue;
        };
        let LifeState::Downed {
            t_down,
            weapon,
            fatigue,
            give_up_ticks,
            killer,
        } = *life
        else {
            continue;
        };
        hp.hp -= DOWNED_HP / bleed_out_ticks(fatigue, hz) as f32;
        let can_give_up = now.0.saturating_sub(t_down.0) >= ticks(GIVE_UP_DELAY_S, hz);
        let holding = last.cmd.buttons & GIVE_UP_BUTTON != 0 && can_give_up;
        let held = if holding { give_up_ticks + 1 } else { 0 };
        if hp.hp <= 0.0 || u32::from(held) >= ticks(GIVE_UP_HOLD_S, hz) {
            kia_no_credit(
                now,
                &cfg,
                &policy,
                &mut queue,
                &mut events.0,
                p.id,
                &mut hp,
                &mut life,
                weapon,
            );
            continue;
        }
        *life = LifeState::Downed {
            t_down,
            killer,
            weapon,
            fatigue,
            give_up_ticks: held,
        };
    }
    // 2. Revive interactions, medics in ascending id order.
    let medics: Vec<(EntityId, Entity)> = table
        .sorted()
        .filter(|(_, e)| q.get(*e).is_ok_and(|r| r.10.is_some()))
        .collect();
    let need = ticks(REVIVE_S, hz).min(65_535) as u16;
    let resume = ticks(REVIVE_RESUME_S, hz);
    for (mid, me) in medics {
        let (mpos, mteam, held, mhp, alive, act) = {
            let Ok((_, team, t, last, hp, life, _, _, act, _, _)) = q.get(me) else {
                continue;
            };
            (
                t.pos,
                *team,
                last.cmd.buttons & buttons::USE != 0,
                hp.hp,
                life.is_alive(),
                act.0,
            )
        };
        if !alive {
            if let Ok(mut r) = q.get_mut(me) {
                r.8 .0 = None;
            }
            continue;
        }
        // Nearest downed teammate in range with a clear line.
        let target = if held {
            table
                .sorted()
                .filter(|&(id, _)| id != mid)
                .filter_map(|(id, e)| {
                    let (_, team, t, _, _, life, ..) = q.get(e).ok()?;
                    let d = (t.pos - mpos).length();
                    (life.is_downed()
                        && *team == mteam
                        && d <= REVIVE_RANGE_M
                        && col.0.los(mpos + Vec3::Y * 0.9, t.pos + Vec3::Y * 0.3))
                    .then_some((id, e, d))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2).then(a.0.cmp(&b.0)))
        } else {
            None
        };
        let mut next = act;
        if let Some(a) = act {
            let stale = now.0.saturating_sub(a.last_active.0) > resume;
            let hurt = a.medic_hp - mhp >= REVIVE_INTERRUPT_HP;
            if stale || hurt {
                next = None;
            }
        }
        if let Some((tid, te, _)) = target {
            let mut a = match next {
                Some(a) if a.target == tid => a,
                _ => Revive {
                    target: tid,
                    ticks: 0,
                    last_active: now,
                    medic_hp: mhp,
                },
            };
            a.ticks += 1;
            a.last_active = now;
            if a.ticks >= need {
                next = None;
                if let Ok((
                    vp,
                    _,
                    _,
                    _,
                    mut vhp,
                    mut vlife,
                    mut vrecent,
                    mut vhist,
                    _,
                    mut lock,
                    _,
                )) = q.get_mut(te)
                {
                    vhp.hp = DOWNED_HP;
                    *vlife = LifeState::Alive;
                    vrecent.clear();
                    vhist.0.push(now);
                    vhist
                        .0
                        .retain(|t| now.0.saturating_sub(t.0) <= FATIGUE_WINDOW_S * u32::from(hz));
                    lock.0 = Tick(now.0 + ticks(REVIVE_FIRE_LOCK_S, hz));
                    let target_player = vp.id;
                    events.0.push(SimEvent::Revive {
                        medic: mid,
                        target: tid,
                        target_player,
                    });
                }
            } else {
                next = Some(a);
            }
        }
        if let Ok(mut r) = q.get_mut(me) {
            r.8 .0 = next;
        }
    }
}
