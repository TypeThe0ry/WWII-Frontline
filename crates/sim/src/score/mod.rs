//! Authoritative, tick-driven score ledger. Sim producers report outcomes, not points.

use std::collections::{BTreeMap, VecDeque};

use smallvec::SmallVec;

use crate::components::Team;
use crate::hash::HashWriter;
use crate::ids::PlayerId;
use crate::medical::CreditKind;
use crate::step::SimEvent;

mod rules;
pub mod stats;
#[cfg(test)]
mod tests;
mod world;
pub mod xp;

pub use rules::{Category, ScoreKind, ScoreRules};
use stats::*;
pub use world::Afk;
use ScoreKind as K;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreDelta {
    pub player: PlayerId,
    pub kind: ScoreKind,
    pub category: Category,
    pub value: i16,
    pub role_at_time: u8,
}

/// Input from authoritative gameplay producers (never a client-accepted message).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Heal(u32),
    Resupply,
    AmmoCrateUse,
    Build(u32),
    NodeBuilt,
    GarrisonBuild(u32),
    GarrisonBuilt,
    GarrisonSpawn {
        forward: bool,
    },
    OutpostSpawn,
    SupplyDelivery,
    Repair(u32),
    ModuleRepair,
    VehicleDestroyed(VehicleClass),
    VehicleDamage(u32),
    Drive(u32),
    CrewMember,
    GunDestroyed,
    GarrisonDestroyed,
    NodeDestroyed,
    GarrisonDefense,
    SpotAssist {
        target: PlayerId,
        spotter: bool,
    },
    AssaultCapture,
    ResourceSample([[i32; 3]; 2]),
    GarrisonPlaced {
        id: u16,
        supplies: u32,
        forward: bool,
    },
    SpawnedAtGarrison(u16),
    AmmoCratePlaced(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleClass {
    Light,
    Medium,
    Heavy,
    Soft,
}

#[derive(Debug, Clone, Copy)]
pub struct Actor {
    pub role: u8,
    pub team: Team,
    pub squad: Option<u16>,
    pub alive: bool,
    pub afk: bool,
}

#[derive(Default)]
struct Ledger {
    stats: PlayerStats,
    actor: Option<Actor>,
    minutes: BTreeMap<K, VecDeque<(u32, i16)>>,
    streak: VecDeque<u32>,
    killed_by: Option<(PlayerId, u32)>,
    role: BTreeMap<u8, (i64, i64)>,
    remainder: BTreeMap<K, u32>,
}

#[derive(Default)]
struct Point {
    last_capture: Option<u32>,
    threatened: bool,
    players: BTreeMap<PlayerId, Presence>,
}

#[derive(Default)]
struct Presence {
    attack: u32,
    defend: u32,
    participation: u32,
    present: bool,
}

pub type Deltas = SmallVec<[ScoreDelta; 4]>;

pub struct ScoreEngine {
    rules: &'static ScoreRules,
    roles: &'static [crate::data::RoleDef],
    awards: [Option<rules::Award>; 256],
    bonuses: Vec<[bool; 256]>,
    bonus_hundredths: i64,
    hz: u8,
    now: u32,
    duration: u32,
    players: [Option<Ledger>; 128],
    departed: Vec<Ledger>,
    pairs: BTreeMap<(PlayerId, PlayerId), VecDeque<u32>>,
    revives: BTreeMap<PlayerId, u32>,
    spotted: BTreeMap<(PlayerId, PlayerId), ()>,
    points: BTreeMap<u16, Point>,
    timeline: Vec<CaptureRecord>,
    resource_curve: Vec<ResourceSample>,
    garrisons: BTreeMap<u16, (PlayerKey, bool)>,
    ammo: BTreeMap<u32, PlayerKey>,
    digest: Option<HashWriter>,
}

impl ScoreEngine {
    pub fn new(hz: u8, roles: &'static [crate::data::RoleDef]) -> Self {
        let rules = ScoreRules::embedded();
        let mut awards = [None; 256];
        for (&kind, &award) in &rules.awards {
            awards[kind as usize] = Some(award);
        }
        let bonuses = roles
            .iter()
            .map(|r| {
                let mut mask = [false; 256];
                for k in rules.xp_role_bonus.get(&r.id).into_iter().flatten() {
                    mask[*k as usize] = true;
                }
                mask
            })
            .collect();
        Self {
            rules,
            roles,
            awards,
            bonuses,
            bonus_hundredths: (rules.xp_role_bonus_mult * 100.0).round() as i64,
            hz,
            now: 0,
            duration: 0,
            players: std::array::from_fn(|_| None),
            departed: Vec::new(),
            pairs: BTreeMap::new(),
            revives: BTreeMap::new(),
            spotted: BTreeMap::new(),
            points: BTreeMap::new(),
            timeline: Vec::new(),
            resource_curve: Vec::new(),
            garrisons: BTreeMap::new(),
            ammo: BTreeMap::new(),
            digest: None,
        }
    }

    pub fn role(&self, name: &str) -> u8 {
        self.roles
            .iter()
            .position(|r| r.id == name)
            .and_then(|r| u8::try_from(r).ok())
            .unwrap_or(u8::MAX)
    }

    pub fn advance(&mut self, tick: u32) {
        self.now = tick;
        self.duration += 1;
    }

    pub fn observe(&mut self, player: PlayerId, actor: Actor) {
        let now = self.now;
        let Some(slot) = self.players.get_mut(usize::from(player)) else {
            return;
        };
        let l = slot.get_or_insert_with(|| Ledger {
            stats: PlayerStats {
                player,
                session: now,
                ..PlayerStats::default()
            },
            ..Ledger::default()
        });
        l.actor = Some(actor);
        l.stats.team = actor.team as u8;
        l.stats.squad = actor.squad;
        l.stats.online_ticks += 1;
        if actor.alive {
            l.role.entry(actor.role).or_default();
        }
    }

    pub fn disconnect(&mut self, player: PlayerId) {
        if let Some(ledger) = self
            .players
            .get_mut(usize::from(player))
            .and_then(Option::take)
        {
            self.departed.push(ledger);
            self.pairs.retain(|&(a, b), _| a != player && b != player);
            self.spotted.retain(|&(a, b), _| a != player && b != player);
            self.revives.remove(&player);
            for l in self.players.iter_mut().flatten() {
                if l.killed_by.is_some_and(|(by, _)| by == player) {
                    l.killed_by = None;
                }
            }
            for p in self.points.values_mut() {
                p.players.remove(&player);
            }
        }
    }

    fn ledger(&mut self, p: PlayerId) -> Option<&mut Ledger> {
        self.players.get_mut(p as usize)?.as_mut()
    }
    fn actor(&self, player: PlayerId) -> Option<Actor> {
        self.players.get(player as usize)?.as_ref()?.actor
    }
    fn current(&self, key: PlayerKey) -> bool {
        self.players
            .get(key.player as usize)
            .and_then(Option::as_ref)
            .is_some_and(|l| l.stats.key() == key)
    }
    fn ticks(&self, seconds: u32) -> u32 {
        seconds.saturating_mul(u32::from(self.hz))
    }

    fn award(&mut self, player: PlayerId, kind: K, quantity: u32, out: &mut Deltas) {
        let Some(rule) = self.awards[kind as usize].filter(|a| a.enabled) else {
            return;
        };
        let now = self.now;
        let window = self.ticks(self.rules.cap_window_s);
        let role_mult = self.bonus_hundredths;
        let bonuses = &self.bonuses;
        let Some(l) = self
            .players
            .get_mut(player as usize)
            .and_then(Option::as_mut)
        else {
            return;
        };
        let Some(actor) = l.actor.filter(|a| !a.afk) else {
            return;
        };
        let units = rule.units.max(1);
        let quantity = if kind == K::Repair {
            let carry = l.remainder.entry(kind).or_default();
            let total = u64::from(*carry) + u64::from(quantity);
            *carry = (total % u64::from(units)) as u32;
            total / u64::from(units)
        } else {
            u64::from(quantity / units)
        };
        let mut value = (i64::from(rule.points) * quantity as i64)
            .min(i64::from(rule.action_cap_points.unwrap_or(i16::MAX as u16)))
            .clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16;
        if let Some(cap) = rule.minute_cap_points {
            let history = l.minutes.entry(kind).or_default();
            history.retain(|&(t, _)| now.wrapping_sub(t) < window);
            let used = history.iter().map(|&(_, v)| i32::from(v)).sum::<i32>();
            value = i16::try_from(i32::from(value).min((i32::from(cap) - used).max(0)))
                .unwrap_or(value);
            if value > 0 {
                history.push_back((now, value));
            }
        }
        if value == 0 {
            return;
        }
        let category = kind.category();
        l.stats.categories[category as usize] += i64::from(value);
        l.stats.total_score += i64::from(value);
        if actor.alive
            || matches!(
                kind,
                K::GarrisonSpawn | K::ForwardGarrisonSpawn | K::OutpostSpawn | K::AmmoCrateUse
            )
        {
            let bonus = bonuses
                .get(actor.role as usize)
                .is_some_and(|mask| mask[kind as usize]);
            let role = l.role.entry(actor.role).or_default();
            role.0 += i64::from(value);
            role.1 += i64::from(value) * if bonus { role_mult } else { 100 };
        }
        let h = self.digest.get_or_insert_with(HashWriter::new);
        h.write_u32(now);
        h.write_u8(player);
        h.write_u8(kind as u8);
        h.write_i16(value);
        h.write_u8(actor.role);
        out.push(ScoreDelta {
            player,
            kind,
            category,
            value,
            role_at_time: actor.role,
        });
    }

    pub(crate) fn hash(&self, h: &mut HashWriter) {
        if let Some(digest) = &self.digest {
            h.write_u8(0xB6);
            h.write_u64(digest.finish());
        }
    }

    pub fn on_event(&mut self, event: &SimEvent) -> Deltas {
        let mut out = Deltas::new();
        if let SimEvent::ScoreAction { player, action } = *event {
            self.action(player, action, &mut out);
        }
        out
    }

    fn action(&mut self, p: PlayerId, action: Action, out: &mut Deltas) {
        use Action::*;
        let (kind, quantity) = match action {
            Heal(hp) => (K::Heal, hp),
            Resupply => (K::Resupply, 1),
            AmmoCrateUse => (K::AmmoCrateUse, 1),
            Build(n) => (K::Build, n),
            NodeBuilt => (K::NodeBuilt, 1),
            GarrisonBuild(n) => (K::GarrisonBuild, n),
            GarrisonBuilt => (K::GarrisonBuilt, 1),
            GarrisonSpawn { forward } => {
                if forward {
                    self.award(p, K::ForwardGarrisonSpawn, 1, out);
                }
                (K::GarrisonSpawn, 1)
            }
            OutpostSpawn => (K::OutpostSpawn, 1),
            SupplyDelivery => (K::SupplyDelivery, 1),
            Repair(n) => (K::Repair, n),
            ModuleRepair => (K::ModuleRepair, 1),
            VehicleDestroyed(c) => (
                match c {
                    VehicleClass::Light => K::LightVehicle,
                    VehicleClass::Medium => K::MediumVehicle,
                    VehicleClass::Heavy => K::HeavyVehicle,
                    VehicleClass::Soft => K::SoftVehicle,
                },
                1,
            ),
            VehicleDamage(n) => {
                if let Some(l) = self.ledger(p) {
                    l.stats.vehicle_damage_hp += u64::from(n);
                }
                return;
            }
            Drive(n) => {
                if let Some(l) = self.ledger(p) {
                    l.stats.driving_distance_mm += u64::from(n);
                }
                return;
            }
            CrewMember => (K::CrewMember, 1),
            GunDestroyed => (K::GunDestroyed, 1),
            GarrisonDestroyed => (K::GarrisonDestroyed, 1),
            NodeDestroyed => (K::NodeDestroyed, 1),
            GarrisonDefense => (K::GarrisonDefense, 1),
            SpotAssist { target, spotter } => {
                if spotter || self.spotted.insert((p, target), ()).is_some() {
                    return;
                }
                (K::SpotAssist, 1)
            }
            AssaultCapture => (K::AssaultCapture, 1),
            ResourceSample(teams) => {
                self.resource_curve.push(stats::ResourceSample {
                    tick: self.now,
                    teams,
                });
                return;
            }
            GarrisonPlaced {
                id,
                supplies,
                forward,
            } => {
                if let Some(l) = self.ledger(p) {
                    let key = l.stats.key();
                    self.garrisons.insert(id, (key, forward));
                }
                self.action(p, GarrisonBuild(supplies), out);
                return;
            }
            SpawnedAtGarrison(id) => {
                if let Some(&(key, forward)) = self.garrisons.get(&id) {
                    if key.player != p
                        && self.current(key)
                        && self
                            .actor(key.player)
                            .zip(self.actor(p))
                            .is_some_and(|(owner, spawned)| owner.team == spawned.team)
                    {
                        self.action(key.player, GarrisonSpawn { forward }, out);
                    }
                }
                return;
            }
            AmmoCratePlaced(id) => {
                if let Some(l) = self.ledger(p) {
                    let key = l.stats.key();
                    self.ammo.insert(id, key);
                }
                return;
            }
        };
        if let Some(l) = self.ledger(p) {
            match action {
                Heal(n) => l.stats.healed_hp += u64::from(n),
                Build(n) | GarrisonBuild(n) => l.stats.build_supplies += u64::from(n),
                GarrisonSpawn { .. } => l.stats.garrison_spawns += 1,
                Repair(n) => l.stats.repaired_hp += u64::from(n),
                SpotAssist { .. } => l.stats.spot_assists += 1,
                VehicleDestroyed(_) => l.stats.vehicles_destroyed += 1,
                _ => {}
            }
        }
        self.award(p, kind, quantity, out);
    }

    fn revive(&mut self, medic: PlayerId, target: PlayerId, out: &mut Deltas) {
        let previous = self.revives.insert(target, self.now);
        let repeat = previous
            .is_some_and(|t| self.now.wrapping_sub(t) <= self.ticks(self.rules.revive_repeat_s));
        if let Some(l) = self.ledger(medic) {
            l.stats.revives += 1;
        }
        self.award(
            medic,
            if repeat { K::RepeatRevive } else { K::Revive },
            1,
            out,
        );
    }

    fn death(&mut self, victim: PlayerId) {
        if let Some(l) = self.ledger(victim) {
            l.stats.deaths += 1;
            l.streak.clear();
            l.killed_by = None;
        }
    }

    fn credit(&mut self, kill: Kill<'_>, out: &mut Deltas) -> bool {
        let Kill {
            killer,
            victim,
            team_kill,
            kind,
            assists,
            headshot,
            dist_mm,
            defense,
        } = kill;
        if killer == victim {
            return false;
        }
        if team_kill {
            if kind != CreditKind::Finish {
                self.award(killer, K::TeamKill, 1, out);
            }
            return false;
        }
        let now = self.now;
        let window = self.ticks(self.rules.duplicate_kill_s);
        let history = self.pairs.entry((killer, victim)).or_default();
        history.retain(|&t| now.wrapping_sub(t) < window);
        if kind != CreditKind::Finish {
            history.push_back(now);
            if history.len() > self.rules.duplicate_kill_count.saturating_add(1) {
                history.pop_front();
            }
        }
        let eligible = history.len() <= self.rules.duplicate_kill_count;
        if kind == CreditKind::Finish {
            if eligible {
                self.award(killer, K::Finish, 1, out);
            }
            if let Some(l) = self.ledger(victim) {
                l.killed_by = Some((killer, now));
            }
            return eligible;
        }
        let streak_window = self.ticks(self.rules.streak_s);
        let streak_count = self.rules.streak_count;
        let revenge_window = self.ticks(self.rules.revenge_s);
        let Some(l) = self.ledger(killer) else {
            return false;
        };
        l.stats.kills += 1;
        l.stats.headshots += u32::from(headshot);
        l.stats.farthest_kill_mm = l.stats.farthest_kill_mm.max(dist_mm);
        l.streak.retain(|&t| now.wrapping_sub(t) <= streak_window);
        let revenge = l
            .killed_by
            .is_some_and(|(p, t)| p == victim && now.wrapping_sub(t) <= revenge_window);
        if revenge {
            l.killed_by = None;
        }
        if eligible && l.actor.is_some_and(|actor| !actor.afk) {
            l.streak.push_back(now);
        }
        let streak = l.streak.len();
        l.stats.longest_streak = l.stats.longest_streak.max(streak as u32);
        if kind == CreditKind::Direct {
            if let Some(l) = self.ledger(victim) {
                l.killed_by = Some((killer, now));
            }
        }
        for &p in assists {
            if let Some(l) = self.ledger(p) {
                l.stats.assists += 1;
            }
            if eligible {
                self.award(p, K::Assist, 1, out);
            }
        }
        if eligible {
            self.award(killer, K::Kill, 1, out);
            if headshot {
                self.award(killer, K::Headshot, 1, out);
            }
            if streak == streak_count {
                self.award(killer, K::KillStreak, 1, out);
            }
            if revenge {
                self.award(killer, K::Revenge, 1, out);
            }
        }
        if defense {
            self.award(killer, K::DefenseKill, 1, out);
        }
        eligible
    }

    fn player_stats(&self, l: &Ledger, winner: Option<Team>, ended: bool) -> PlayerStats {
        let mut p = l.stats.clone();
        p.kd_milli = (p.deaths > 0).then(|| u64::from(p.kills) * 1000 / u64::from(p.deaths));
        let rate = |num: u32, denom: u32| {
            (u64::from(num) * 10_000 / u64::from(denom.max(1))).min(10_000) as u32
        };
        p.hit_bps = rate(p.hits, p.shots);
        p.headshot_bps = rate(p.headshots, p.kills);
        p.role_xp = l
            .role
            .iter()
            .map(|(&role, &(score, xp_hundredths))| RoleXpDelta {
                role,
                score,
                xp_hundredths,
            })
            .collect();
        p.match_xp = xp::match_xp(
            &self.rules.xp,
            p.total_score,
            match winner {
                Some(t) if t as u8 == p.team => xp::MatchOutcome::Win,
                Some(_) => xp::MatchOutcome::Loss,
                None if ended => xp::MatchOutcome::Draw,
                None => xp::MatchOutcome::Unfinished,
            },
            p.online_ticks,
            self.duration,
            self.hz,
        );
        p.account_level = xp::account_level(p.match_xp.total_xp);
        p
    }

    pub fn summary(
        &self,
        map: u16,
        mode: &str,
        winner: Option<Team>,
        reason: Option<String>,
    ) -> MatchSummary {
        let players: Vec<_> = self
            .departed
            .iter()
            .chain(self.players.iter().flatten())
            .map(|l| self.player_stats(l, winner, reason.is_some()))
            .collect();
        let mut top: Vec<_> = players.iter().collect();
        top.sort_by_key(|p| (std::cmp::Reverse(p.total_score), p.player, p.session));
        let mut squads: BTreeMap<_, SquadRank> = BTreeMap::new();
        for p in &top {
            if let Some(squad) = p.squad {
                squads
                    .entry((p.team, squad))
                    .or_insert(SquadRank {
                        team: p.team,
                        squad,
                        score: 0,
                        mvp: p.key(),
                    })
                    .score += p.total_score;
            }
        }
        let mut squad_ranking: Vec<_> = squads.into_values().collect();
        squad_ranking.sort_by_key(|r| (std::cmp::Reverse(r.score), r.team, r.squad));
        let team_ranking = std::array::from_fn(|team| {
            let on_team = || top.iter().copied().filter(|p| usize::from(p.team) == team);
            TeamRank {
                top: on_team().take(10).map(PlayerStats::key).collect(),
                category_leaders: std::array::from_fn(|c| {
                    on_team()
                        .min_by_key(|p| (std::cmp::Reverse(p.categories[c]), p.player, p.session))
                        .map(PlayerStats::key)
                }),
            }
        });
        let mvp = top.first().map(|p| p.key());
        MatchSummary {
            map,
            mode: mode.to_owned(),
            tick_hz: self.hz,
            duration_ticks: self.duration,
            winner: winner.map(|t| t as u8),
            reason,
            players,
            timeline: self.timeline.clone(),
            squad_ranking,
            team_ranking,
            mvp,
            resource_curve: self.resource_curve.clone(),
        }
    }
}

struct Kill<'a> {
    killer: PlayerId,
    victim: PlayerId,
    team_kill: bool,
    kind: CreditKind,
    assists: &'a [PlayerId],
    headshot: bool,
    dist_mm: u32,
    defense: bool,
}
