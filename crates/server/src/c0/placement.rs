//! Placement slots are sim-thread actions, not client-provided positions or role flags.

use std::collections::BTreeMap;

use frontline_protocol::msg::c0::{
    PlacementAction, PlacementReason as Reason, PlacementRequest, PlacementResult,
    PlacementSlot as Slot,
};
use frontline_protocol::quant::{dequant_yaw, pitch_i16_to_rad};
use frontline_sim::spawn::garrison::PlaceError;
use frontline_sim::supply::ammo::{AmmoCfg, AmmoPlaceError};
use frontline_sim::supply::{check_supply_crate_place, SupplyPlaceError};
use frontline_sim::{PlayerId, Team, Vec3, World};

#[derive(Default)]
pub(super) struct PlacementSlots(BTreeMap<PlayerId, Preview>);

struct Preview {
    id: u16,
    tick: u32,
    slot: Slot,
    yaw: u16,
    pitch: i16,
    target: Vec3,
}

impl PlacementSlots {
    pub(super) fn forget(&mut self, player: PlayerId) {
        self.0.remove(&player);
    }

    pub(super) fn handle(
        &mut self,
        world: &mut World,
        player: PlayerId,
        req: PlacementRequest,
    ) -> PlacementResult {
        let mut result = PlacementResult {
            id: req.id,
            reason: Reason::Rejected,
            supply_cost: 0,
            target_cm: None,
        };
        match req.action {
            PlacementAction::Preview { slot, yaw, pitch } => {
                self.forget(player);
                let checked = (|| {
                    let team = eligible(world, player, slot)?;
                    let target = aim(world, player, slot, yaw, pitch)?;
                    result.target_cm = Some(target.to_array().map(|m| (m * 100.0).round() as i32));
                    result.supply_cost = preview(world, player, team, slot, target)?;
                    self.0.insert(
                        player,
                        Preview {
                            id: req.id,
                            tick: world.tick().0,
                            slot,
                            yaw,
                            pitch,
                            target,
                        },
                    );
                    Ok(())
                })();
                result.reason = checked.err().unwrap_or(Reason::Ok);
            }
            PlacementAction::Commit { preview_id } => {
                let checked = (|| {
                    let p = self
                        .0
                        .remove(&player)
                        .filter(|p| p.id == preview_id)
                        .filter(|p| {
                            world.tick().0.wrapping_sub(p.tick) < u32::from(world.config().tick_hz)
                        })
                        .ok_or(Reason::NoTarget)?;
                    let team = eligible(world, player, p.slot)?;
                    let target = aim(world, player, p.slot, p.yaw, p.pitch)?;
                    if p.target.distance_squared(target) > 0.75 * 0.75 {
                        return Err(Reason::NoTarget);
                    }
                    // Recheck public facts before mutating. Hidden-state failures carry no coordinates
                    // or specific enemy reason, and never affect the preview replies.
                    preview(world, player, team, p.slot, target)?;
                    match p.slot {
                        Slot::Garrison => world
                            .place_garrison(player, target)
                            .map(|_| ())
                            .map_err(garrison_reason),
                        Slot::Supply => world
                            .drop_supply_crate(player)
                            .map(|_| ())
                            .map_err(supply_reason),
                        Slot::Ammo => world.place_ammo_crate(player).map(|_| ()).map_err(|e| {
                            if e == AmmoPlaceError::Stacked {
                                Reason::Rejected
                            } else {
                                ammo_reason(e)
                            }
                        }),
                    }
                })();
                result.reason = checked.err().unwrap_or(Reason::Ok);
            }
        }
        result
    }
}

fn eligible(world: &World, player: PlayerId, slot: Slot) -> Result<Team, Reason> {
    if !world
        .player_life(player)
        .is_some_and(|life| life.is_alive())
    {
        return Err(Reason::NotAlive);
    }
    if world.seat_of(player).is_some() {
        return Err(Reason::NoTarget);
    }
    let roster = world.squads();
    let entry = roster.entry(player);
    let role = entry.filter(|e| !e.role_invalid).and_then(|e| e.role);
    match slot {
        Slot::Garrison if role != Some("officer") => return Err(Reason::WrongRole),
        Slot::Garrison if !roster.squad_of(player).is_some_and(|s| s.size() >= 2) => {
            return Err(Reason::NeedSquad)
        }
        Slot::Supply | Slot::Ammo if role != Some("support") => return Err(Reason::NotSupport),
        _ => (),
    }
    entry.map(|e| e.team).ok_or(Reason::Rejected)
}

fn aim(world: &World, player: PlayerId, slot: Slot, yaw: u16, pitch: i16) -> Result<Vec3, Reason> {
    let actor = world
        .player_state(player)
        .filter(|p| p.grounded)
        .ok_or(Reason::NoTarget)?;
    match slot {
        Slot::Garrison => world
            .placement_aim(player, dequant_yaw(yaw, 16), pitch_i16_to_rad(pitch))
            .ok_or(Reason::NoTarget),
        _ => Ok(actor.pos),
    }
}

fn preview(
    world: &World,
    player: PlayerId,
    team: Team,
    slot: Slot,
    pos: Vec3,
) -> Result<u8, Reason> {
    match slot {
        Slot::Garrison => world
            .preview_garrison_place(player, pos)
            .map(|cost| cost.supplies as u8)
            .map_err(garrison_reason),
        Slot::Supply => {
            let id = world.carrying(player).ok_or(Reason::NotCarrying)?;
            if world
                .supply_world()
                .crates
                .get(&id)
                .is_none_or(|c| c.carried_by != Some(player))
            {
                return Err(Reason::NotCarrying);
            }
            check_supply_crate_place(
                world.objective_list().iter().map(|o| (o.owner, o.pos)),
                team,
                pos,
                world.no_build_zones(),
            )
            .map(|()| 0)
            .map_err(supply_reason)
        }
        Slot::Ammo => {
            let config = world.config();
            let cfg = AmmoCfg::from_data(config.data, config.tick_hz);
            let supply = world.supply_world();
            cfg.check_place(
                world.tick().0,
                supply.ammo_count.get(&player).copied().unwrap_or(0),
                supply.ammo_last.get(&player).copied(),
                pos,
                world.no_build_zones(),
                supply
                    .ammo
                    .values()
                    .filter(|c| c.team == team)
                    .map(|c| c.pos),
            )
            .map(|_| 0)
            .map_err(ammo_reason)
        }
    }
}

fn garrison_reason(err: PlaceError) -> Reason {
    match err {
        PlaceError::WrongRole => Reason::WrongRole,
        PlaceError::TooCloseHq => Reason::TooCloseHq,
        PlaceError::TooCloseOwnGarrison => Reason::TooCloseOwnGarrison,
        PlaceError::TooCloseEnemyGarrison | PlaceError::EnemyNear => Reason::Rejected,
        PlaceError::TooCloseObjective => Reason::TooCloseObjective,
        PlaceError::Slope => Reason::Slope,
        PlaceError::Water => Reason::Water,
        PlaceError::NoSupplies => Reason::NoSupplies,
        PlaceError::TeamCap => Reason::TeamCap,
        PlaceError::SectorCap => Reason::SectorCap,
        PlaceError::NotAlive => Reason::NotAlive,
        PlaceError::InNoBuildZone => Reason::NoBuildZone,
    }
}

fn supply_reason(err: SupplyPlaceError) -> Reason {
    match err {
        SupplyPlaceError::NotAlive => Reason::NotAlive,
        SupplyPlaceError::NotSupport => Reason::NotSupport,
        SupplyPlaceError::NotCarrying => Reason::NotCarrying,
        SupplyPlaceError::TooCloseObjective => Reason::TooCloseObjective,
        SupplyPlaceError::InNoBuildZone => Reason::NoBuildZone,
    }
}

fn ammo_reason(err: AmmoPlaceError) -> Reason {
    match err {
        AmmoPlaceError::NotAlive => Reason::NotAlive,
        AmmoPlaceError::NotSupport => Reason::NotSupport,
        AmmoPlaceError::LimitPerLife => Reason::LimitPerLife,
        AmmoPlaceError::Cooldown => Reason::Cooldown,
        AmmoPlaceError::Stacked => Reason::Stacked,
        AmmoPlaceError::InNoBuildZone => Reason::NoBuildZone,
    }
}
