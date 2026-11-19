//! Ammo Crates and resupply (GAMEDESIGN 12.3; M1-020b).
//!
//! A crate holds 600 points; refilling a loadout item costs its price in points. Order:
//! primary magazines, then grenades, then bandages. A player never gets more than its
//! loadout's initial amount.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use frontline_protocol::msg::input::buttons;
use glam::Vec3;

use crate::components::{
    CurrentTick, EntityTable, EventBuf, LastInput, Player, SimCfgRes, Team, Transform,
};
use crate::data::GameData;
use crate::health::LifeState;
use crate::ids::PlayerId;
use crate::medical::{Vitals, SELF_BANDAGES};
use crate::step::SimEvent;
use crate::throwables::system::{Gadgets, START_FRAG, START_SMOKE};
use crate::weapon::{WeaponParams, WeaponState};

use super::{dist_xz, SupplyWorld};

/// Resupply range and hold time (the hold is `SupplyCfg::interact_ticks`).
pub const RESUPPLY_RANGE_M: f32 = 4.0;
/// Ammo Crates stacked within this distance of each other are limited to [`MAX_STACK`].
pub const STACK_RADIUS_M: f32 = 1.0;
pub const MAX_STACK: usize = 3;

#[derive(Resource, Debug, Clone)]
pub struct AmmoCfg {
    pub content_points: u32,
    pub max_uses: u8,
    pub cooldown_ticks: u32,
    pub lifetime_ticks: u32,
    pub hp: f32,
    pub prices: AmmoCosts,
}

/// Point prices per item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmmoCosts(pub BTreeMap<String, u32>);

impl AmmoCosts {
    fn get(&self, k: &str) -> u32 {
        self.0.get(k).copied().unwrap_or(0)
    }

    /// Price of one magazine / clip / belt of a weapon of `class`.
    pub fn mag_price(&self, class: &str) -> u32 {
        self.get(match class {
            "pistol" => "pistol_mag",
            "smg" => "smg_mag",
            "bar" => "bar_mag",
            c if c.starts_with("mg") || c == "hmg" => "mg_belt",
            _ => "rifle_clip",
        })
    }
}

impl AmmoCfg {
    pub fn from_data(data: &GameData, hz: u8) -> Self {
        let a = &data.rules.supplies.ammo_crate;
        let t = |s: f64| libm::ceil(s * f64::from(hz)) as u32;
        Self {
            content_points: a.content_points,
            max_uses: a.max_uses.min(255) as u8,
            cooldown_ticks: t(a.place_cooldown_s),
            lifetime_ticks: t(a.lifetime_s),
            hp: a.health_hp as f32,
            prices: AmmoCosts(a.price_points.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AmmoCrate {
    pub id: u32,
    pub team: Team,
    pub pos: Vec3,
    pub points: u32,
    pub hp: f32,
    pub expires_at: u32,
    pub owner: PlayerId,
}

/// What a loadout starts with (the cap of a resupply).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadoutTarget {
    pub mag_capacity: u16,
    pub reserve: u16,
    pub frag: u8,
    pub smoke: u8,
    pub bandages: u8,
}

/// The starting loadout of `role` for `team` (mirrors `components::starting_weapon`).
pub fn loadout_target(
    data: &GameData,
    role: &str,
    team: Team,
    hz: u8,
) -> Option<(LoadoutTarget, String)> {
    let key = match team {
        Team::Us => "us",
        Team::Ger => "ger",
    };
    let lid = data
        .role(role)?
        .faction_overrides
        .get(key)?
        .loadout_ids
        .first()?;
    let lo = data.loadout(lid)?;
    let def = data.weapons.iter().find(|w| w.id == lo.primary)?;
    let p = WeaponParams::from_def(def, hz).ok()?;
    let reserve = u32::from(p.mag_capacity) * lo.ammo_mags.saturating_sub(1);
    Some((
        LoadoutTarget {
            mag_capacity: p.mag_capacity,
            reserve: reserve.min(u32::from(u16::MAX)) as u16,
            frag: START_FRAG,
            smoke: START_SMOKE,
            bandages: SELF_BANDAGES,
        },
        def.class.clone(),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Mag,
    Frag,
    Smoke,
    Bandage,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResupplyResult {
    /// Units given per item kind, in refill order.
    pub items: Vec<(Item, u32)>,
    pub points_spent: u32,
}

/// What a player carries that a resupply tops up.
pub struct Inventory<'a> {
    pub weapon: Option<&'a mut WeaponState>,
    pub gadgets: &'a mut Gadgets,
    pub bandages: &'a mut u8,
}

/// Refills `inv` towards `target` from `points` (spent in place). `class` is the primary
/// weapon's class (its magazine price).
pub fn resupply(
    inv: &mut Inventory<'_>,
    points: &mut u32,
    target: &LoadoutTarget,
    class: &str,
    costs: &AmmoCosts,
) -> ResupplyResult {
    let mut out = ResupplyResult::default();
    let mut spend = |units: u32, price: u32, points: &mut u32| -> u32 {
        if units == 0 || price == 0 {
            return units;
        }
        let n = units.min(*points / price);
        *points -= n * price;
        out.points_spent += n * price;
        n
    };
    // Primary magazines.
    if let Some(w) = inv.weapon.as_deref_mut() {
        let cap = u32::from(target.mag_capacity.max(1));
        let missing = u32::from(target.reserve.saturating_sub(w.reserve));
        let clips = missing.div_ceil(cap);
        let got = spend(clips, costs.mag_price(class), points);
        if got > 0 {
            let rounds = (got * cap).min(missing);
            w.reserve += rounds as u16;
            out.items.push((Item::Mag, got));
        }
    }
    let g = costs.get("grenade");
    let n = spend(
        u32::from(target.frag.saturating_sub(inv.gadgets.frag)),
        g,
        points,
    );
    if n > 0 {
        inv.gadgets.frag += n as u8;
        out.items.push((Item::Frag, n));
    }
    let n = spend(
        u32::from(target.smoke.saturating_sub(inv.gadgets.smoke)),
        g,
        points,
    );
    if n > 0 {
        inv.gadgets.smoke += n as u8;
        out.items.push((Item::Smoke, n));
    }
    let n = spend(
        u32::from(target.bandages.saturating_sub(*inv.bandages)),
        costs.get("bandage"),
        points,
    );
    if n > 0 {
        *inv.bandages += n as u8;
        out.items.push((Item::Bandage, n));
    }
    out
}

/// Why `World::place_ammo_crate` refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmmoPlaceError {
    NotSupport,
    NotAlive,
    LimitPerLife,
    Cooldown,
    Stacked,
    InNoBuildZone,
}

impl AmmoCfg {
    /// Read-only rules shared by the server placement action and its preview.
    pub fn check_place(
        &self,
        now: u32,
        used: u8,
        last: Option<u32>,
        pos: Vec3,
        no_build_zones: &crate::NoBuildZones,
        crates: impl Iterator<Item = Vec3>,
    ) -> Result<u8, AmmoPlaceError> {
        if no_build_zones.contains(pos) {
            return Err(AmmoPlaceError::InNoBuildZone);
        }
        if used >= self.max_uses {
            return Err(AmmoPlaceError::LimitPerLife);
        }
        if last.is_some_and(|t| now.saturating_sub(t) < self.cooldown_ticks) {
            return Err(AmmoPlaceError::Cooldown);
        }
        let stacked = crates
            .filter(|&p| dist_xz(p, pos) <= STACK_RADIUS_M)
            .take(MAX_STACK)
            .count();
        if stacked == MAX_STACK {
            return Err(AmmoPlaceError::Stacked);
        }
        Ok(self.max_uses - used)
    }
}

/// Upkeep (expiry, empty crates) and the hold-to-resupply interaction.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn ammo(
    tick: Res<CurrentTick>,
    sim: Res<SimCfgRes>,
    cfg: Res<super::SupplyCfg>,
    acfg: Res<AmmoCfg>,
    table: Res<EntityTable>,
    mut sw_res: ResMut<SupplyWorld>,
    mut events: ResMut<EventBuf>,
    mut q: Query<(
        &Player,
        &Team,
        &Transform,
        &LifeState,
        &LastInput,
        Option<&mut WeaponState>,
        &mut Gadgets,
        &mut Vitals,
    )>,
) {
    let sw = &mut *sw_res;
    if sw.ammo.is_empty() && sw.ammo_hold.is_empty() {
        return;
    }
    let now = tick.0 .0;
    // Expired or empty crates go.
    let gone: Vec<u32> = sw
        .ammo
        .values()
        .filter(|c| c.points == 0 || now >= c.expires_at)
        .map(|c| c.id)
        .collect();
    for id in gone {
        sw.ammo.remove(&id);
        events.0.push(SimEvent::CrateDestroyed { id });
    }
    for (_, e) in table.sorted() {
        let Ok((pl, team, tr, life, last, weapon, mut gadgets, mut vitals)) = q.get_mut(e) else {
            continue;
        };
        let pid = pl.id;
        if life.is_dead() {
            // A new life: the per-life crate counter starts over.
            sw.ammo_count.remove(&pid);
        }
        if life.is_dead() || life.is_downed() || last.cmd.buttons & buttons::USE == 0 {
            sw.ammo_hold.remove(&pid);
            continue;
        }
        let near = sw
            .ammo
            .values()
            .filter(|c| {
                c.team == *team && c.points > 0 && dist_xz(c.pos, tr.pos) <= RESUPPLY_RANGE_M
            })
            .min_by(|a, b| {
                dist_xz(a.pos, tr.pos)
                    .total_cmp(&dist_xz(b.pos, tr.pos))
                    .then(a.id.cmp(&b.id))
            })
            .map(|c| c.id);
        let Some(cid) = near else {
            sw.ammo_hold.remove(&pid);
            continue;
        };
        let h = sw.ammo_hold.entry(pid).or_insert(0);
        *h += 1;
        if *h != cfg.interact_ticks {
            continue;
        }
        let Some((target, class)) =
            loadout_target(sim.data, crate::world::DEFAULT_ROLE, *team, sim.hz)
        else {
            continue;
        };
        let c = sw.ammo.get_mut(&cid).unwrap();
        let mut inv = Inventory {
            weapon: weapon.map(|w| w.into_inner()),
            gadgets: &mut gadgets,
            bandages: &mut vitals.bandages,
        };
        let res = resupply(&mut inv, &mut c.points, &target, &class, &acfg.prices);
        if res.points_spent > 0 {
            events.0.push(SimEvent::Resupplied {
                player: pid,
                crate_id: cid,
                points: res.points_spent,
            });
        } else {
            events.0.push(SimEvent::SupplyNotice {
                player: pid,
                code: super::system::NOTICE_NOTHING_TO_RESUPPLY,
            });
        }
    }
}
