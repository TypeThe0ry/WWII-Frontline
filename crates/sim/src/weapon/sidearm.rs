//! Authoritative primary/secondary inventory and draw timing (M1-013c).

use bevy_ecs::prelude::*;
use frontline_protocol::msg::input::{buttons, InputCmd};

use crate::components::{EntityTable, LastInput, Team, WeaponTable};
use crate::data::{GameData, LoadoutDef};
use crate::health::LifeState;
use crate::supply::Carrying;
use crate::vehicle::seats::InVehicle;

use super::melee::MeleePhase;
use super::{advance_weapon, Fsm, MeleeState, WeaponState};

/// The accepted deploy choice, applied only on a new life.
#[derive(Component, Clone, Copy)]
pub(crate) struct SpawnLoadout(pub Option<u8>);

pub(crate) fn loadout_for(
    data: &GameData,
    team: Team,
    selected: Option<u8>,
) -> Option<&'static LoadoutDef> {
    selected
        .and_then(|id| data.loadouts.get(usize::from(id)))
        .or_else(|| default_loadout(data, crate::world::DEFAULT_ROLE, team))
}

pub(crate) fn default_loadout(
    data: &GameData,
    role: &str,
    team: Team,
) -> Option<&'static LoadoutDef> {
    let faction = match team {
        Team::Us => "us",
        Team::Ger => "ger",
    };
    let id = data
        .role(role)?
        .faction_overrides
        .get(faction)?
        .loadout_ids
        .first()?;
    data.loadout(id)
}

pub(crate) fn loadout_weapon(
    data: &GameData,
    table: &WeaponTable,
    id: &str,
    mags: u32,
) -> Option<WeaponState> {
    let idx = u16::try_from(data.weapons.iter().position(|w| w.id == id)?).ok()?;
    let p = table.params(idx)?;
    Some(initial_weapon(idx, p.mag_capacity, mags))
}

pub(crate) fn initial_weapon(idx: u16, capacity: u16, mags: u32) -> WeaponState {
    let reserve = u32::from(capacity).saturating_mul(mags.saturating_sub(1));
    let mag = if mags > 0 { capacity } else { 0 };
    WeaponState::new(idx, mag, reserve.min(u32::from(u16::MAX)) as u16)
}

/// `WeaponState` remains the equipped gun; the other gun is kept here with its own ammo/FSM.
#[derive(Component, Debug, Default)]
pub(crate) struct WeaponSlots {
    pub loadout: Option<&'static LoadoutDef>,
    pub secondary_active: bool,
    pub stowed: Option<WeaponState>,
    pub draw_ticks: u32,
    pub swap_held: bool,
    pub input_slot: u8,
}

impl WeaponSlots {
    pub fn spawn(
        data: &GameData,
        table: &WeaponTable,
        team: Team,
        selected: SpawnLoadout,
    ) -> (Self, Option<WeaponState>) {
        let loadout = loadout_for(data, team, selected.0);
        let primary = loadout.and_then(|l| loadout_weapon(data, table, &l.primary, l.ammo_mags));
        let secondary = loadout.and_then(|l| {
            loadout_weapon(
                data,
                table,
                l.secondary.as_deref()?,
                l.secondary_mags.unwrap_or(1),
            )
        });
        let (equipped, stowed, secondary_active) = if primary.is_some() {
            (primary, secondary, false)
        } else {
            (secondary, None, secondary.is_some())
        };
        (
            Self {
                loadout,
                stowed,
                secondary_active,
                ..Self::default()
            },
            equipped,
        )
    }

    fn requested(&mut self, cmd: &InputCmd) -> Option<bool> {
        let held = cmd.buttons & buttons::SWAP != 0;
        let edge = held && !self.swap_held;
        self.swap_held = held;
        // A change to the explicit selector takes precedence. Remember it so a repeated
        // command (or a client sending the default slot 0 with SWAP) cannot switch back.
        if cmd.weapon_slot <= 1 && cmd.weapon_slot != self.input_slot {
            self.input_slot = cmd.weapon_slot;
            Some(cmd.weapon_slot == 1)
        } else {
            edge.then_some(!self.secondary_active)
        }
    }

    fn swap(&mut self, active: &mut WeaponState, table: &WeaponTable) {
        let Some(stowed) = self.stowed.as_mut() else {
            return;
        };
        // No ammo moves until the reload finishes. Do not discard a pending bolt/fire cycle.
        if active.fsm.is_reloading() {
            active.fsm = if active.mag > 0 {
                Fsm::Ready
            } else {
                Fsm::Empty
            };
            active.timer_units = 0;
        }
        active.ads_units = 0;
        active.trigger_held = false;
        std::mem::swap(active, stowed);
        self.secondary_active = !self.secondary_active;
        self.draw_ticks = table.params(active.def_id).map_or(0, |p| p.swap_in_ticks);
    }
}

pub(crate) fn is_pistol(data: &GameData, weapon: &WeaponState) -> bool {
    data.weapons
        .get(usize::from(weapon.def_id))
        .is_some_and(|w| w.class == "pistol")
}

#[allow(clippy::type_complexity)]
pub(crate) fn equip(
    cfg: Res<crate::components::SimCfgRes>,
    table: Res<EntityTable>,
    wt: Res<WeaponTable>,
    mut q: Query<(
        &LastInput,
        &LifeState,
        &Carrying,
        &InVehicle,
        &MeleeState,
        &mut WeaponSlots,
        &mut WeaponState,
    )>,
) {
    for (_, e) in table.sorted() {
        let Ok((last, life, carrying, vehicle, melee, mut slots, mut active)) = q.get_mut(e) else {
            continue;
        };
        if !life.is_alive() || slots.stowed.is_none() {
            continue;
        }
        slots.draw_ticks = slots.draw_ticks.saturating_sub(1);
        let requested = slots.requested(&last.cmd);
        if let Some(w) = slots.stowed.as_mut() {
            if let Some(p) = wt.params(w.def_id) {
                advance_weapon(w, p);
            }
        }
        if vehicle.0.is_some() {
            continue;
        }
        let stowed_is_pistol = slots
            .stowed
            .as_ref()
            .is_some_and(|w| is_pistol(cfg.data, w));
        let carrying = carrying.0.is_some();
        let forced = carrying && !is_pistol(cfg.data, &active) && stowed_is_pistol;
        let manual = requested.is_some_and(|secondary| secondary != slots.secondary_active)
            && (!carrying || stowed_is_pistol)
            && !melee.executing()
            && melee.phase != MeleePhase::Windup;
        if forced || manual {
            slots.swap(&mut active, &wt);
        }
    }
}
