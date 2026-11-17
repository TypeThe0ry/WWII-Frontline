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

/// Natural recovery below this hp after `REGEN_DELAY_S` without damage, at `REGEN_HPS`.
pub const REGEN_BELOW_HP: f32 = 60.0;
pub const REGEN_DELAY_S: f32 = 10.0;
pub const REGEN_HPS: f32 = 1.0;
/// A single hit of at least this much damage (or any explosion) starts bleeding (not fatal).
pub const BLEED_HIT_HP: f32 = 30.0;
pub const BLEED_HPS: f32 = 1.0;
pub const BLEED_FLOOR_HP: f32 = 5.0;
pub const SELF_BANDAGE_S: f32 = 3.5;
pub const SELF_BANDAGE_HP: f32 = 25.0;
pub const SELF_BANDAGE_CAP: f32 = 75.0;
pub const MEDIC_BANDAGE_S: f32 = 2.5;
pub const MEDIC_BANDAGE_HP: f32 = 40.0;
pub const STOP_BLEED_S: f32 = 2.5;
pub const SELF_BANDAGES: u8 = 2;
pub const MEDIC_BANDAGES: u8 = 6;
pub const BANDAGE_RANGE_M: f32 = 1.5;
/// Fall damage: free up to `FALL_FREE_M`, then per metre; `FALL_KIA_M` and more kills.
pub const FALL_FREE_M: f32 = 3.0;
pub const FALL_HP_PER_M: f32 = 12.0;
pub const FALL_KIA_M: f32 = 12.0;

/// The button a downed player holds to give up (PI-055).
pub const GIVE_UP_BUTTON: u16 = buttons::RELOAD;

fn ticks(seconds: f32, hz: u8) -> u32 {
    libm::ceilf(seconds * f32::from(hz) - 1e-3).max(0.0) as u32
}

/// Marks a player as a medic (only medics revive; set with `World::set_medic`).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Medic;

/// Wounds and first aid of a living player (M1-016b).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Vitals {
    pub last_damage: Tick,
    pub bleeding: bool,
    pub bandages: u8,
    pub action: Option<Bandage>,
}

impl Default for Vitals {
    fn default() -> Self {
        Self {
            last_damage: Tick(0),
            bleeding: false,
            bandages: SELF_BANDAGES,
            action: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BandageKind {
    /// On yourself: +25 hp up to 75, stops bleeding.
    Own,
    /// A medic on a wounded teammate: +40 hp up to 100, stops bleeding.
    Friend,
    /// Anyone on a downed teammate: stops the bleeding, restores nothing.
    Stabilise,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bandage {
    pub kind: BandageKind,
    pub target: EntityId,
    pub ticks: u16,
}

/// Peak height of the current fall (for fall damage).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct FallTracker {
    pub peak_y: f32,
    pub airborne: bool,
}

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
        &mut Vitals,
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
        if let Ok((_, _, _, _, mut recent, _, mut vitals)) = q.get_mut(victim_e) {
            if rec.damage > 0.0 && rec.shooter != rec.target {
                recent.record(rec.shooter, rec.damage, tick.0, cfg.hz);
            }
            if rec.damage > 0.0 {
                vitals.last_damage = tick.0;
                if hp_now > 0.0 && (rec.damage >= BLEED_HIT_HP || rec.kind == DamageKind::Explosion)
                {
                    vitals.bleeding = true;
                }
            }
        }
        if hp_now > 0.0 {
            continue;
        }
        attackers.retain(|&(a, amount)| a != rec.shooter && amount >= ASSIST_MIN_HP);
        let killer_id = killer
            .map(|(id, _)| id)
            .filter(|_| rec.shooter != rec.target);
        let team_kill =
            killer_id.is_some() && killer.is_some_and(|(id, t)| t == vteam && id != vid);
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
        let Ok((_, _, mut hp, mut life, mut recent, _, mut vitals)) = q.get_mut(victim_e) else {
            continue;
        };
        vitals.action = None;
        if !downed_before && !outright {
            hp.hp = DOWNED_HP;
            *life = LifeState::Downed {
                t_down: tick.0,
                killer: killer_id,
                weapon,
                fatigue: fatigue_n,
                give_up_ticks: 0,
                stable: false,
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
            stable,
        } = *life
        else {
            continue;
        };
        if !stable {
            hp.hp -= DOWNED_HP / bleed_out_ticks(fatigue, hz) as f32;
        }
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
            stable,
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

/// Fall damage (M1-016b, GAMEDESIGN 7.6): tracks the peak of each airborne stretch and charges
/// the drop on landing. Runs after movement.
#[allow(clippy::type_complexity)]
pub fn falls(
    table: Res<EntityTable>,
    mut log: ResMut<HitLog>,
    mut q: Query<(
        &Transform,
        &crate::components::Motion,
        &mut FallTracker,
        &LifeState,
        &mut Health,
    )>,
) {
    for (id, e) in table.sorted() {
        let Ok((t, m, mut f, life, mut hp)) = q.get_mut(e) else {
            continue;
        };
        if life.is_dead() {
            f.airborne = false;
            continue;
        }
        if !m.grounded {
            if f.airborne {
                f.peak_y = f.peak_y.max(t.pos.y);
            } else {
                f.airborne = true;
                f.peak_y = t.pos.y;
            }
            continue;
        }
        if !f.airborne {
            continue;
        }
        f.airborne = false;
        let drop = f.peak_y - t.pos.y;
        if drop <= FALL_FREE_M || hp.hp <= 0.0 {
            continue;
        }
        let kills = drop >= FALL_KIA_M;
        let dmg = (drop - FALL_FREE_M) * FALL_HP_PER_M;
        hp.hp -= dmg;
        log.0.push(HitRec {
            target: id,
            shooter: id,
            weapon: 0xFD,
            zone: HitZone::Limb,
            dist_m: drop,
            damage: dmg,
            kind: DamageKind::Fall,
            direct_kia: kills,
        });
    }
}

/// Vitals stage (after `medical`): bleeding, natural recovery and bandaging (M1-016b).
///
/// `USE` held: a medic near a downed teammate revives (handled in `medical`); otherwise a medic
/// bandages the nearest wounded teammate, anyone else stabilises a downed teammate, and with
/// nobody to help the player bandages themselves (PI-056).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn vitals(
    tick: Res<CurrentTick>,
    cfg: Res<SimCfgRes>,
    table: Res<EntityTable>,
    mut q: Query<(
        &Team,
        &Transform,
        &LastInput,
        &mut Health,
        &mut LifeState,
        &mut Vitals,
        Option<&Medic>,
    )>,
) {
    let now = tick.0;
    let hz = cfg.hz;
    // 1. Bleeding, recovery, dead reset.
    for (_, e) in table.sorted() {
        let Ok((_, _, _, mut hp, life, mut v, medic)) = q.get_mut(e) else {
            continue;
        };
        if life.is_dead() {
            *v = Vitals {
                bandages: if medic.is_some() {
                    MEDIC_BANDAGES
                } else {
                    SELF_BANDAGES
                },
                ..Vitals::default()
            };
            continue;
        }
        if !life.is_alive() {
            v.bleeding = false;
            v.action = None;
            continue;
        }
        if v.bleeding {
            // Bleeding never kills: it stops at the floor.
            if hp.hp > BLEED_FLOOR_HP {
                hp.hp = (hp.hp - BLEED_HPS / f32::from(hz)).max(BLEED_FLOOR_HP);
            }
        } else if hp.hp < REGEN_BELOW_HP
            && now.0.saturating_sub(v.last_damage.0) >= ticks(REGEN_DELAY_S, hz)
        {
            hp.hp = (hp.hp + REGEN_HPS / f32::from(hz)).min(REGEN_BELOW_HP);
        }
    }
    // 2. Bandages, players in ascending id order.
    let ids: Vec<(EntityId, Entity)> = table.sorted().collect();
    for &(id, e) in &ids {
        let (pos, team, held, alive, medic, action, hp_self, bleeding, bandages) = {
            let Ok((team, t, last, hp, life, v, medic)) = q.get(e) else {
                continue;
            };
            (
                t.pos,
                *team,
                last.cmd.buttons & buttons::USE != 0,
                life.is_alive(),
                medic.is_some(),
                v.action,
                hp.hp,
                v.bleeding,
                v.bandages,
            )
        };
        if !alive || !held {
            if let Ok((.., mut v, _)) = q.get_mut(e) {
                v.action = None;
            }
            continue;
        }
        // Teammates within reach.
        let near: Vec<(EntityId, Entity, bool, bool, f32)> = ids
            .iter()
            .filter(|&&(oid, _)| oid != id)
            .filter_map(|&(oid, oe)| {
                let (oteam, ot, _, ohp, olife, ov, _) = q.get(oe).ok()?;
                ((*oteam == team) && (ot.pos - pos).length() <= BANDAGE_RANGE_M).then_some((
                    oid,
                    oe,
                    olife.is_downed(),
                    olife.is_alive() && (ohp.hp < ohp.max_hp || ov.bleeding),
                    (ot.pos - pos).length(),
                ))
            })
            .collect();
        let best = |pred: &dyn Fn(&(EntityId, Entity, bool, bool, f32)) -> bool| {
            near.iter()
                .filter(|n| pred(n))
                .min_by(|a, b| a.4.total_cmp(&b.4).then(a.0.cmp(&b.0)))
                .copied()
        };
        let downed_near = best(&|n| n.2);
        if medic && downed_near.is_some() {
            // The revive in `medical` owns this input.
            if let Ok((.., mut v, _)) = q.get_mut(e) {
                v.action = None;
            }
            continue;
        }
        let want = if let Some((tid, ..)) = downed_near {
            let stable = q
                .get(downed_near.map_or(e, |d| d.1))
                .map(|r| matches!(*r.4, LifeState::Downed { stable: true, .. }))
                .unwrap_or(true);
            (!stable && bandages > 0).then_some((
                BandageKind::Stabilise,
                tid,
                ticks(STOP_BLEED_S, hz),
            ))
        } else if let Some((tid, ..)) = best(&|n| n.3).filter(|_| medic) {
            (bandages > 0).then_some((BandageKind::Friend, tid, ticks(MEDIC_BANDAGE_S, hz)))
        } else if hp_self < SELF_BANDAGE_CAP || bleeding {
            (bandages > 0).then_some((BandageKind::Own, id, ticks(SELF_BANDAGE_S, hz)))
        } else {
            None
        };
        let Some((kind, target, need)) = want else {
            if let Ok((.., mut v, _)) = q.get_mut(e) {
                v.action = None;
            }
            continue;
        };
        let mut a = match action {
            Some(a) if a.kind == kind && a.target == target => a,
            _ => Bandage {
                kind,
                target,
                ticks: 0,
            },
        };
        a.ticks += 1;
        if u32::from(a.ticks) < need {
            if let Ok((.., mut v, _)) = q.get_mut(e) {
                v.action = Some(a);
            }
            continue;
        }
        // Done: apply and spend a bandage.
        if let Ok((.., mut v, _)) = q.get_mut(e) {
            v.action = None;
            v.bandages -= 1;
        }
        let te = table.get(target).unwrap_or(e);
        if let Ok((_, _, _, mut thp, mut tlife, mut tv, _)) = q.get_mut(te) {
            match kind {
                BandageKind::Own => {
                    thp.hp = thp.hp.max((thp.hp + SELF_BANDAGE_HP).min(SELF_BANDAGE_CAP));
                    tv.bleeding = false;
                }
                BandageKind::Friend => {
                    thp.hp = (thp.hp + MEDIC_BANDAGE_HP).min(thp.max_hp);
                    tv.bleeding = false;
                }
                BandageKind::Stabilise => {
                    if let LifeState::Downed {
                        t_down,
                        killer,
                        weapon,
                        fatigue,
                        give_up_ticks,
                        ..
                    } = *tlife
                    {
                        *tlife = LifeState::Downed {
                            t_down,
                            killer,
                            weapon,
                            fatigue,
                            give_up_ticks,
                            stable: true,
                        };
                    }
                }
            }
        }
    }
}
