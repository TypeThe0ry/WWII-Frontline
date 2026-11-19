//! Host-side requests for tactical bots. The brain remains a read-only client.

use frontline_bots::ctx::SoldierStatus;
use frontline_bots::perception::Sound;
use frontline_bots::role_select::{select_role, SquadSnapshot};
use frontline_bots::squad_leader::SquadOrder;
use frontline_protocol::msg::c0::{DeployRequest, SquadOp, SquadOpKind};
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::data::defs::Faction;
use frontline_sim::deploy::{validate::M1_UNLOCKED_LEVEL, SPAWN_HQ};
use frontline_sim::spawn::garrison::{
    GarrisonState, BUILD_RANGE_M, COST_FRIENDLY, MIN_OWN_M, SUPPLY_RANGE_M,
};
use frontline_sim::squad::SquadKind;
use frontline_sim::supply::{dist_xz, INTERACT_RANGE_M};
use frontline_sim::{LifeState, PlayerId, SimEvent, Team, Vec3, World};

use crate::{c0, metrics::Metrics};

const ROLE_BACKOFF_S: u32 = 30;
const MAX_SOUNDS: usize = 128;
const SOUND_HISTORY_TICKS: u32 = 90;

pub(super) fn record_sounds(sounds: &mut Vec<Sound>, tick: u32, events: &[SimEvent]) {
    sounds.retain(|s| tick.saturating_sub(s.tick) <= SOUND_HISTORY_TICKS);
    for (pos, range_m) in events.iter().filter_map(|e| match e {
        SimEvent::Shot { muzzle, .. } => Some((*muzzle, 120.0)),
        SimEvent::Explosion { pos, .. } => Some((*pos, 180.0)),
        _ => None,
    }) {
        if sounds.len() == MAX_SOUNDS {
            sounds.remove(0);
        }
        sounds.push(Sound { pos, tick, range_m });
    }
}

pub(super) fn assign_squad(world: &mut World, metrics: &Metrics, id: PlayerId, team: Team) {
    // Only join unlocked Infantry squads with a free seat. The sim owns invitations and bans.
    let available: Vec<_> = world
        .squads()
        .squads()
        .filter(|s| {
            s.team == team
                && s.kind == SquadKind::Infantry
                && !s.locked
                && s.size() < world.squads().cfg.infantry_max
        })
        .filter_map(|s| u8::try_from(s.id).ok())
        .collect();
    for (op, squad) in available
        .into_iter()
        .map(|id| (SquadOpKind::Join, id))
        .chain([(SquadOpKind::Create, 0)])
    {
        if c0::squad_op(
            world,
            metrics,
            id,
            &SquadOp {
                op,
                squad,
                target: 0,
                name_key: String::new(),
            },
        ) {
            break;
        }
    }
}

fn order_position(world: &World, order: Option<SquadOrder>) -> Option<Vec3> {
    match order? {
        SquadOrder::Move(pos) => Some(pos),
        SquadOrder::Attack(id) | SquadOrder::Defend(id) => world
            .objective_list()
            .iter()
            .find(|o| o.id == id)
            .map(|o| o.pos),
        SquadOrder::Hold => None,
    }
}

/// Select the desired role, then preflight the request. The sim is authoritative about
/// loadout, cooldown and Garrison state (including block and capacity).
fn deploy_choice(world: &World, id: PlayerId, order: Option<SquadOrder>) -> Option<DeployRequest> {
    let roster = world.squads();
    let entry = roster.entry(id)?;
    let squad = roster.squad_of(id);
    let data = world.config().data;
    let held: Vec<_> = data
        .roles
        .iter()
        .map(|d| {
            let (mut team_count, mut squad_count) = (0, 0);
            for (p, r) in roster.roster() {
                if p != id && r.team == entry.team && r.role == Some(&d.id) {
                    team_count += 1;
                    squad_count += u32::from(entry.squad.is_some() && entry.squad == r.squad);
                }
            }
            (d.id.as_str(), squad_count, team_count)
        })
        .collect();
    let desired = select_role(
        data.roles,
        &SquadSnapshot {
            kind: squad.map(|s| s.kind),
            size: squad.map_or(0, |s| s.size()),
            is_leader: squad.is_some_and(|s| s.leader == id),
            held: &held,
            crewman_cap: roster.cfg.crewman_max,
        },
        &roster.cfg.armor_role,
    );
    let faction = match entry.team {
        Team::Us => Faction::Us,
        Team::Ger => Faction::Ger,
    };
    let body = world.player_state(id);
    // First deployments use HQ: the sim only applies Garrison choices to existing dead bodies.
    let mut points = Vec::new();
    if let Some(body) = body.filter(|_| squad.is_some()) {
        let dest = order_position(world, order).unwrap_or(body.pos);
        points.extend(
            world
                .garrison_list()
                .into_iter()
                .filter(|g| g.team == entry.team)
                .map(|g| (dist_xz(g.pos, dest), g.id)),
        );
        points.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    }
    let squad_id = u8::try_from(entry.squad.unwrap_or(0)).ok()?;
    for name in [desired, entry.role.unwrap_or("rifleman"), "rifleman"] {
        let Some(role) = data
            .roles
            .iter()
            .position(|r| r.id == name)
            .and_then(|i| u8::try_from(i).ok())
        else {
            continue;
        };
        let Some(loadout) = data
            .loadouts
            .iter()
            .position(|lo| {
                lo.role == name && lo.faction == faction && lo.unlock_level <= M1_UNLOCKED_LEVEL
            })
            .and_then(|i| u8::try_from(i).ok())
        else {
            continue;
        };
        for spawn_point in points.iter().map(|&(_, id)| id).chain([SPAWN_HQ]) {
            let req = DeployRequest {
                spawn_point,
                role,
                loadout,
                squad: squad_id,
            };
            if world.validate_deploy(id, &c0::deploy_req(req)).is_ok() {
                return Some(req);
            }
        }
    }
    None
}

#[derive(Clone, Copy)]
pub(super) struct Intent {
    pub pos: Vec3,
    pub range: f32,
}

impl Intent {
    pub(super) fn apply(self, from: Vec3, cmd: &mut InputCmd) {
        if dist_xz(from, self.pos) <= self.range && cmd.buttons & buttons::FIRE == 0 {
            cmd.move_x = 0;
            cmd.move_z = 0;
            cmd.buttons |= buttons::USE;
        }
    }
}

#[derive(Default)]
pub(super) struct Slot {
    deployed_for_death: Option<u32>,
    next_check: u32,
    next_role_action: u32,
    intent: Option<Intent>,
}

impl Slot {
    pub(super) fn intent(&self) -> Option<Intent> {
        self.intent
    }

    pub(super) fn survival(&self, world: &World, id: PlayerId) -> SoldierStatus {
        let Some(life) = world.player_life(id) else {
            return SoldierStatus::default();
        };
        if let LifeState::Downed { t_down, .. } = life {
            return SoldierStatus {
                downed_since: Some(t_down.0),
                ..Default::default()
            };
        }
        if life != LifeState::Alive {
            return SoldierStatus::default();
        }
        let (Some(entry), Some(body)) = (world.squads().entry(id), world.player_state(id)) else {
            return SoldierStatus::default();
        };
        let downed_ally = (entry.role == Some("medic"))
            .then(|| {
                // Downed players are not in WorldView::players, but their roster and body remain.
                world
                    .squads()
                    .roster()
                    .filter(|(p, r)| {
                        r.team == entry.team
                            && matches!(world.player_life(*p), Some(LifeState::Downed { .. }))
                    })
                    .filter_map(|(p, _)| world.player_state(p).map(|a| (p, a.pos)))
                    .min_by(|a, b| dist_xz(body.pos, a.1).total_cmp(&dist_xz(body.pos, b.1)))
            })
            .flatten();
        let ammo_crate = nearest(
            body.pos,
            world
                .supply_world()
                .ammo
                .values()
                .filter(|c| c.team == entry.team && c.points > 0)
                .map(|c| c.pos),
        );
        let (frag, smoke) = world.grenade_counts(id).unwrap_or_default();
        SoldierStatus {
            downed_since: None,
            downed_ally,
            ammo_crate,
            frag,
            smoke,
        }
    }

    pub(super) fn update(
        &mut self,
        world: &mut World,
        metrics: &Metrics,
        id: PlayerId,
        order: Option<SquadOrder>,
        events: &[SimEvent],
        site: Option<Vec3>,
    ) {
        let now = world.tick().0;
        let hz = u32::from(world.config().tick_hz);
        if events.iter().any(|e| matches!(e, SimEvent::SupplyNotice { player, code: frontline_sim::supply::system::NOTICE_NO_PLACE } if *player == id)) {
            self.next_role_action = now + ROLE_BACKOFF_S * hz;
            self.intent = None;
        }
        let life = world.player_life(id);
        if life != Some(LifeState::Alive) {
            self.intent = None;
        }
        if now < self.next_check {
            return;
        }
        self.next_check = now + hz;
        match life {
            None | Some(LifeState::Dead { .. }) => {
                let death = match life {
                    Some(LifeState::Dead { t_kia, .. }) => Some(t_kia.0),
                    _ => None,
                };
                if death.is_some() && self.deployed_for_death == death {
                    return;
                }
                if let Some(req) = deploy_choice(world, id, order) {
                    let result = c0::deploy(world, metrics, id, req);
                    if result.ok {
                        self.deployed_for_death = death;
                    }
                }
            }
            Some(LifeState::Alive) => {
                self.deployed_for_death = None;
                self.intent = None;
                if let (Some(entry), Some(from)) = (
                    world.squads().entry(id),
                    world.player_state(id).map(|p| p.pos),
                ) {
                    let team = entry.team;
                    match entry.role.filter(|_| !entry.role_invalid) {
                        Some("officer") => self.officer(world, id, team, from, now, hz),
                        Some("support") => self.support(world, id, team, from, now, hz, site),
                        _ => {}
                    }
                }
            }
            Some(LifeState::Downed { .. }) => {}
        }
    }

    fn officer(
        &mut self,
        world: &mut World,
        id: PlayerId,
        team: Team,
        from: Vec3,
        now: u32,
        hz: u32,
    ) {
        if let Some(g) = world.garrison_list().into_iter().find(|g| {
            g.owner == id
                && g.state == GarrisonState::Building
                && dist_xz(from, g.pos) <= BUILD_RANGE_M
        }) {
            self.intent = Some(Intent {
                pos: g.pos,
                range: BUILD_RANGE_M,
            });
            return;
        }
        if now < self.next_role_action {
            return;
        }
        let supplies = world.supply_world().crates.values().filter(|c| {
            c.team == team && c.carried_by.is_none() && dist_xz(c.pos, from) <= SUPPLY_RANGE_M
        });
        if supplies.clone().map(|c| c.supplies).sum::<u32>() < COST_FRIENDLY {
            return;
        }
        let Some(sq) = world.squads().squad_of(id) else {
            return;
        };
        let near = world
            .view()
            .players()
            .iter()
            .filter(|p| p.player != id && sq.has(p.player) && dist_xz(p.pos, from) <= 25.0)
            .take(2)
            .count();
        if near < 2 {
            return;
        }
        let Some(pos) = nearest(from, supplies.map(|c| c.pos)) else {
            return;
        };
        if world
            .objective_list()
            .iter()
            .min_by(|a, b| dist_xz(a.pos, pos).total_cmp(&dist_xz(b.pos, pos)))
            .and_then(|o| o.owner)
            .is_some_and(|owner| owner != team)
        {
            return;
        }
        self.next_role_action = now + ROLE_BACKOFF_S * hz;
        match world.place_garrison(id, pos) {
            Ok(_) => {
                self.intent = Some(Intent {
                    pos,
                    range: BUILD_RANGE_M,
                })
            }
            Err(reason) => tracing::debug!(event = "bot_garrison_rejected", player = id, ?reason),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn support(
        &mut self,
        world: &mut World,
        id: PlayerId,
        team: Team,
        from: Vec3,
        now: u32,
        hz: u32,
        site: Option<Vec3>,
    ) {
        if now < self.next_role_action {
            return;
        }
        let sw = world.supply_world();
        let low_ammo = world.view().players().iter().any(|p| {
            p.team == team
                && dist_xz(p.pos, from) <= 12.0
                && world
                    .weapon_state(p.player)
                    .is_some_and(|w| w.reserve <= w.mag)
        });
        let stocked_near = sw
            .ammo
            .values()
            .any(|c| c.team == team && c.points > 0 && dist_xz(c.pos, from) <= 12.0);
        if low_ammo && !stocked_near {
            match world.place_ammo_crate(id) {
                Ok(_) => {}
                Err(reason) => tracing::debug!(event = "bot_ammo_rejected", player = id, ?reason),
            }
            self.next_role_action = now + ROLE_BACKOFF_S * hz;
            return;
        }
        let Some(site) = site else { return };
        let carrying = sw.crates.values().any(|c| c.carried_by == Some(id));
        if carrying {
            self.intent = Some(Intent {
                pos: site,
                range: INTERACT_RANGE_M,
            });
        } else if !sw.crates.values().any(|c| {
            c.team == team && c.carried_by.is_none() && dist_xz(c.pos, site) <= SUPPLY_RANGE_M
        }) && !world.garrison_list().iter().any(|g| {
            g.team == team
                && g.state != GarrisonState::Destroyed
                && dist_xz(g.pos, site) < MIN_OWN_M
        }) {
            let pickup = nearest(
                from,
                sw.depots
                    .iter()
                    .filter(|d| d.team == team && d.stock > 0)
                    .map(|d| d.pos)
                    .chain(
                        sw.points
                            .iter()
                            .filter(|p| p.owner == Some(team) && p.stock > 0)
                            .map(|p| p.pos),
                    ),
            );
            self.intent = pickup.map(|pos| Intent {
                pos,
                range: INTERACT_RANGE_M,
            });
        }
    }
}

fn nearest(from: Vec3, points: impl Iterator<Item = Vec3>) -> Option<Vec3> {
    points.min_by(|a, b| dist_xz(from, *a).total_cmp(&dist_xz(from, *b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontline_sim::{
        CollisionChunk, CollisionData, GameData, GameMode, MapId, SimConfig, SpawnReq,
    };

    #[test]
    fn support_supply_use_feeds_officer_garrison_request() {
        let col = CollisionData::from_chunks(vec![CollisionChunk::new(
            5,
            0,
            vec![0.0; 65 * 65],
            vec![0; 64 * 64],
        )]);
        let data = GameData::embedded();
        let mut world = World::new(
            SimConfig {
                tick_hz: 30,
                match_seed: 4,
                map: MapId(0),
                mode: GameMode::Sandbox,
                data,
            },
            &col,
        );
        let metrics = Metrics::new();
        let site = Vec3::new(350.0, 0.0, 32.0);
        world.add_depot(Team::Us, site);
        world.set_hq(Team::Us, Vec3::ZERO);
        world.set_hq(Team::Ger, Vec3::new(900.0, 0.0, 0.0));
        for (player, team) in [(0, Team::Us), (9, Team::Ger), (1, Team::Us), (2, Team::Us)] {
            world.roster_add(player, team).unwrap();
            if team == Team::Us {
                assign_squad(&mut world, &metrics, player, team);
                world
                    .spawn_player(SpawnReq {
                        player,
                        team,
                        pos: site,
                        yaw: 0.0,
                    })
                    .unwrap();
            }
        }
        world.set_role(0, Some("officer")).unwrap();
        world.set_role(1, Some("support")).unwrap();
        for _ in 0..45 * 30 {
            world.step(&[]);
        }
        assert_eq!(world.supply_world().depots[0].stock, 1);
        let mut support = Slot::default();
        let mut officer = Slot::default();
        for carrying in [true, false] {
            let now = world.tick().0;
            let from = world.player_state(1).unwrap().pos;
            support.support(&mut world, 1, Team::Us, from, now, 30, Some(site));
            let intent = support.intent().unwrap();
            let mut cmd = InputCmd {
                seq: 0,
                tick: 0,
                move_x: 0,
                move_z: 0,
                yaw: 0,
                pitch: 0,
                buttons: 0,
                weapon_slot: 0,
                aux: 0,
                view_tick: 0,
            };
            intent.apply(from, &mut cmd);
            assert_ne!(cmd.buttons & buttons::USE, 0);
            for _ in 0..frontline_sim::supply::SupplyCfg::from_data(data, 30).interact_ticks {
                world.step(&[(1, cmd)]);
            }
            assert_eq!(world.supply_crates()[0].carried_by, carrying.then_some(1));
            cmd.buttons = 0;
            world.step(&[(1, cmd)]);
        }
        let now = world.tick().0;
        officer.officer(&mut world, 0, Team::Us, site, now, 30);
        assert_eq!(world.garrison_list().len(), 1);
        assert_eq!(world.garrison_list()[0].state, GarrisonState::Building);
        assert_eq!(officer.next_role_action, now + 30 * 30);
        assert!(officer.intent().is_some());
    }
}
