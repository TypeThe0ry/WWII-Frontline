//! Garrison: a placeable spawn point (GAMEDESIGN 10.2; M1-019b).

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use frontline_protocol::msg::input::buttons;
use glam::Vec3;

use crate::components::{
    CurrentTick, EntityTable, EventBuf, LastInput, Player, SimCfgRes, Team, Transform,
};
use crate::health::LifeState;
use crate::ids::PlayerId;
use crate::step::SimEvent;

pub const HEALTH_HP: f32 = 1200.0;
pub const BUILD_S: f32 = 20.0;
pub const START_HP_FRAC: f32 = 0.2;
/// Builders beyond the first add this much speed each, up to [`MAX_BUILDERS`].
pub const BUILD_BONUS: f32 = 0.6;
pub const MAX_BUILDERS: usize = 3;
/// A builder must hold USE within this distance.
pub const BUILD_RANGE_M: f32 = 6.0;
pub const SLOTS: f32 = 24.0;
pub const SLOT_REGEN_S: f32 = 5.0;
pub const BASE_WAIT_S: u32 = 8;
pub const BLOCK_RADIUS_M: f32 = 30.0;
pub const BLOCK_CLEAR_S: u32 = 3;
pub const OVERRUN_RADIUS_M: f32 = 20.0;
pub const OVERRUN_S: u32 = 25;
pub const COST_FRIENDLY: u32 = 50;
pub const COST_ENEMY: u32 = 100;
pub const TEAM_CAP: usize = 6;
pub const SECTOR_CAP: usize = 2;
pub const MIN_OWN_M: f32 = 150.0;
pub const MIN_ENEMY_M: f32 = 250.0;
pub const MIN_HQ_M: f32 = 200.0;
pub const OBJECTIVE_EXTRA_M: f32 = 100.0;
pub const MAX_SLOPE_DEG: f32 = 25.0;
pub const ENEMY_NEAR_M: f32 = 50.0;
/// Supplies must be within this distance of the placer.
pub const SUPPLY_RANGE_M: f32 = 15.0;
/// Spawns per second a Garrison lets through.
pub const THROUGHPUT_PER_S: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GarrisonState {
    Building,
    Active,
    Destroyed,
}

/// Damage classes (GAMEDESIGN 10.2: small arms barely scratch it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GarrisonDamage {
    Bullet,
    Explosion,
}

impl GarrisonDamage {
    pub fn mult(self) -> f32 {
        match self {
            GarrisonDamage::Bullet => 0.05,
            GarrisonDamage::Explosion => 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Garrison {
    pub id: u16,
    pub team: Team,
    pub pos: Vec3,
    pub owner: PlayerId,
    pub state: GarrisonState,
    pub hp: f32,
    pub max_hp: f32,
    /// Build progress in `[0, 1]`.
    pub progress: f32,
    pub slots: f32,
    /// Tick an enemy was last within the block radius.
    pub last_enemy: Option<u32>,
    pub overrun_ticks: u32,
    /// Tick of the last spawn through this point (throughput).
    pub last_spawn: Option<u32>,
    /// Spawn ticks in the flood window.
    pub recent: Vec<u32>,
}

impl Garrison {
    pub fn blocked(&self, now: u32, hz: u8) -> bool {
        self.last_enemy
            .is_some_and(|t| now.saturating_sub(t) < BLOCK_CLEAR_S * u32::from(hz))
    }
}

#[derive(Resource, Debug, Default, Clone)]
pub struct Garrisons {
    pub list: BTreeMap<u16, Garrison>,
    pub next_id: u16,
}

/// Whether the player may place Garrisons (an Officer; real roles arrive with M1-021).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Officer(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    WrongRole,
    TooCloseHq,
    TooCloseOwnGarrison,
    TooCloseEnemyGarrison,
    TooCloseObjective,
    Slope,
    Water,
    EnemyNear,
    NoSupplies,
    TeamCap,
    SectorCap,
    NotAlive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GarrisonCost {
    pub supplies: u32,
}

fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// Ticks of `s` seconds at `hz`.
fn t(s: u32, hz: u8) -> u32 {
    s * u32::from(hz)
}

/// Progress, health and slot bookkeeping, blocking and Overrun. One call per tick.
#[allow(clippy::type_complexity)]
pub fn garrisons(
    tick: Res<CurrentTick>,
    cfg: Res<SimCfgRes>,
    table: Res<EntityTable>,
    mut gs: ResMut<Garrisons>,
    mut events: ResMut<EventBuf>,
    q: Query<(&Player, &Team, &Transform, &LifeState, &LastInput)>,
) {
    if gs.list.is_empty() {
        return;
    }
    let now = tick.0 .0;
    let hz = cfg.hz;
    // (team, pos, USE held), living and on their feet.
    let mut people: Vec<(Team, Vec3, bool)> = Vec::new();
    for (_, e) in table.sorted() {
        if let Ok((_, team, tr, life, last)) = q.get(e) {
            if life.is_dead() || life.is_downed() {
                continue;
            }
            people.push((*team, tr.pos, last.cmd.buttons & buttons::USE != 0));
        }
    }
    let regen = 1.0 / (SLOT_REGEN_S * f32::from(hz));
    for g in gs.list.values_mut() {
        if g.state == GarrisonState::Destroyed {
            continue;
        }
        if g.state == GarrisonState::Building {
            let n = people
                .iter()
                .filter(|(tm, p, usd)| *tm == g.team && *usd && dist_xz(*p, g.pos) <= BUILD_RANGE_M)
                .count()
                .min(MAX_BUILDERS);
            if n > 0 {
                let rate = 1.0 + BUILD_BONUS * (n as f32 - 1.0);
                let dp = rate / (BUILD_S * f32::from(hz));
                g.progress = (g.progress + dp).min(1.0);
                g.hp = (g.max_hp * (START_HP_FRAC + (1.0 - START_HP_FRAC) * g.progress)).max(g.hp);
                if g.progress >= 1.0 {
                    g.state = GarrisonState::Active;
                    events.0.push(SimEvent::SpawnPointState {
                        id: g.id,
                        team: g.team,
                        pos: g.pos,
                        state: GarrisonState::Active,
                    });
                }
            }
            continue;
        }
        // Active.
        g.slots = (g.slots + regen).min(SLOTS);
        let mut enemy_block = false;
        let mut enemy_overrun = false;
        let mut friend_overrun = false;
        for (tm, p, _) in &people {
            let d = dist_xz(*p, g.pos);
            if *tm != g.team {
                enemy_block |= d <= BLOCK_RADIUS_M;
                enemy_overrun |= d <= OVERRUN_RADIUS_M;
            } else {
                friend_overrun |= d <= OVERRUN_RADIUS_M;
            }
        }
        if enemy_block {
            g.last_enemy = Some(now);
        }
        if enemy_overrun && !friend_overrun {
            g.overrun_ticks += 1;
        } else {
            g.overrun_ticks = 0;
        }
        if g.overrun_ticks >= t(OVERRUN_S, hz) {
            g.state = GarrisonState::Destroyed;
            g.hp = 0.0;
            events.0.push(SimEvent::SpawnPointState {
                id: g.id,
                team: g.team,
                pos: g.pos,
                state: GarrisonState::Destroyed,
            });
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnRequestError {
    NotDead,
    NoSuchPoint,
    WrongTeam,
    NotActive,
    Blocked,
    NoSlots,
}
