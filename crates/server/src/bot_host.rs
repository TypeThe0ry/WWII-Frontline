//! Server-side bot host (M0-026; ARCHITECTURE 16.1, 6.10 stage 6).
//!
//! A bot is "a client without a network connection": the host gives every bot a `PlayerId` and a
//! session (through [`SimBridge::join`], exactly like a human's join), asks its
//! [`BotBrain`] for one [`InputCmd`] per tick from the **read-only** world, and pushes that command
//! through [`SimBridge::ingest_cmd`] - the same window check + `validate_input` + input buffer
//! that human C1 bundles use. Brains only receive `&dyn WorldView`; the host mediates squad,
//! deploy and role requests through the same authoritative rules used for players.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use frontline_bots::ctx::SoldierStatus;
use frontline_bots::fill::{BotInfo, BotState, FillAction, FillManager, REJOIN_GRACE_S};
use frontline_bots::perception::Sound;
use frontline_bots::squad_leader::{ObjView, SquadLeader, SquadOrder, REPICK_TICKS};
use frontline_bots::{BotBrain, BotCtx, BotMap, Difficulty, SoldierBrain, Tuning};
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::squad::SquadId;
use frontline_sim::{GameData, LifeState, PlayerId, SimEvent, Team, WeaponState, World, WorldView};

use crate::metrics::Metrics;
use crate::net::conn::Sessions;
use crate::sim_bridge::SimBridge;
use crate::world_loader::LoadedMap;

mod gameplay;

/// Nickname prefix of every bot (GAMEDESIGN 20.2).
pub use frontline_bots::fill::BOT_NAME_PREFIX;
/// Cap on stored timing samples (one per tick; ~37 h at 30 Hz).
const MAX_SAMPLES: usize = 4_000_000;

/// `PlayerId` and team of the `index`-th bot: ids count down from 127 (humans take the lowest
/// free id), teams alternate US, GER, ... so `--bots N` splits evenly.
pub fn bot_slot(index: u16) -> (PlayerId, Team) {
    let id = 127u16.saturating_sub(index) as PlayerId;
    (
        id,
        if index.is_multiple_of(2) {
            Team::Us
        } else {
            Team::Ger
        },
    )
}

/// Display name of the `index`-th bot.
pub fn bot_nickname(index: u16) -> String {
    format!("{BOT_NAME_PREFIX} {:02}", index + 1)
}

struct BotSlot {
    player: PlayerId,
    team: Team,
    name: String,
    brain: Box<dyn BotBrain>,
    tactics: Option<gameplay::Slot>,
}

/// Timing of one bot tick, as measured by the host.
#[derive(Debug, Clone, Copy, Default)]
pub struct BotTickTiming {
    /// Whole host tick: fill, gameplay requests, `think` calls and input ingestion.
    pub total_ns: u64,
    /// Time inside path queries.
    pub path_ns: u64,
}

pub struct BotHost {
    slots: Vec<BotSlot>,
    seed: u64,
    difficulty: Difficulty,
    metrics: Arc<Metrics>,
    timings: Vec<BotTickTiming>,
    /// FNV-1a of all commands of the last [`tick`](Self::tick) (determinism checks).
    last_hash: u64,
    record_hashes: bool,
    hashes: Vec<u64>,
    stuck_total: u64,
    /// Shared nav data for soldier bots spawned by [`reconcile`](Self::reconcile).
    soldier_map: Option<Arc<BotMap>>,
    tactical_map: Option<Arc<BotMap>>,
    sessions: Option<Arc<Sessions>>,
    fill: [FillManager; 2],
    disconnected: [BTreeMap<PlayerId, u32>; 2],
    leaders: BTreeMap<SquadId, (PlayerId, SquadLeader)>,
    sounds: Vec<Sound>,
}

impl BotHost {
    /// An empty host (tests add brains with [`add`](Self::add)).
    pub fn new(seed: u64, difficulty: Difficulty, metrics: Arc<Metrics>) -> Self {
        BotHost {
            slots: Vec::new(),
            seed,
            difficulty,
            metrics,
            timings: Vec::new(),
            last_hash: 0,
            record_hashes: false,
            hashes: Vec::new(),
            stuck_total: 0,
            soldier_map: None,
            tactical_map: None,
            sessions: None,
            fill: [FillManager::new(), FillManager::new()],
            disconnected: Default::default(),
            leaders: BTreeMap::new(),
            sounds: Vec::new(),
        }
    }

    /// Creates `count` soldier bots: joins them to the world (spiral around their HQ, like any
    /// joining player), reserves their ids in `sessions`, and builds the shared nav grid from
    /// the loaded terrain. Bots that cannot join (id clash) are skipped with a warning.
    pub fn with_soldiers(
        count: u16,
        seed: u64,
        world: &mut World,
        bridge: &mut SimBridge,
        map: Option<&LoadedMap>,
        sessions: Option<&Arc<Sessions>>,
        metrics: Arc<Metrics>,
    ) -> Self {
        let rules = &GameData::embedded().rules.bots;
        let tuning = Tuning::from_rules(rules);
        let bot_map = Arc::new(match map {
            Some(m) => BotMap::build(
                world.view(),
                (m.size_m, m.size_m),
                m.patrol_points.clone(),
                tuning,
            ),
            None => BotMap::without_nav(tuning, Vec::new()),
        });
        let mut host = BotHost::new(seed, Difficulty::default_from(rules), metrics.clone());
        for i in 0..count {
            let (id, team) = bot_slot(i);
            if sessions.is_some_and(|s| !s.reserve_bot(id, team)) {
                continue;
            }
            if let Err(e) = bridge.join(world, id, team) {
                if let Some(s) = sessions {
                    s.unreserve_bot(id);
                }
                tracing::warn!(event = "bot_join_failed", player_id = id as u64, error = ?e, "bot could not join");
                continue;
            }
            host.add(
                id,
                team,
                bot_nickname(i),
                Box::new(SoldierBrain::new(bot_map.clone())),
            );
        }
        host.soldier_map = Some(bot_map);
        host.sessions = sessions.cloned();
        metrics.bots.set(host.len() as i64);
        host
    }

    /// Registers a brain for a player that already joined the world / bridge.
    pub fn add(&mut self, player: PlayerId, team: Team, name: String, brain: Box<dyn BotBrain>) {
        self.slots.push(BotSlot {
            player,
            team,
            name,
            brain,
            tactics: None,
        });
        self.metrics.bots.set(self.slots.len() as i64);
    }

    fn reserve_bot_id(&self, bridge: &SimBridge, world: &World, team: Team) -> Option<PlayerId> {
        (0..=127u8).rev().find(|&i| {
            bridge.session(i).is_none()
                && world.squads().entry(i).is_none()
                && world.player_life(i).is_none()
                && !self.is_bot(i)
                && self
                    .sessions
                    .as_ref()
                    .is_none_or(|s| s.reserve_bot(i, team))
        })
    }

    pub(crate) fn human_left(&mut self, player: PlayerId, team: Team, tick: u32) {
        if !self.is_bot(player) {
            self.disconnected[team as usize].insert(player, tick);
        }
    }

    pub(crate) fn forget(&mut self, player: PlayerId) {
        if let Some(i) = self.slots.iter().position(|s| s.player == player) {
            self.slots.remove(i);
            if let Some(s) = &self.sessions {
                s.unreserve_bot(player);
            }
            self.metrics.bots.set(self.len() as i64);
        }
    }

    /// Fill-to-target (M1-039c): roster new bots through the squad/deploy path and make room for
    /// humans (deploying bots first, then bots on foot, then vehicle occupants).
    /// `target` is the per-team size. Returns `(spawned, removed)`. Needs a host built by
    /// [`with_soldiers`](Self::with_soldiers) (it owns the nav data); otherwise it does nothing.
    pub fn reconcile(
        &mut self,
        tick: u32,
        world: &mut World,
        bridge: &mut SimBridge,
        target: u8,
    ) -> (usize, usize) {
        let Some(map) = self.soldier_map.clone() else {
            return (0, 0);
        };
        let map = self
            .tactical_map
            .get_or_insert_with(|| {
                let Some(loaded) = bridge.map().filter(|m| m.has_bot_navigation()) else {
                    return map;
                };
                let tuning = Tuning::from_rules(&world.config().data.rules.bots);
                let map = BotMap::build(
                    world.view(),
                    (loaded.size_m, loaded.size_m),
                    loaded.patrol_points.clone(),
                    tuning,
                );
                Arc::new(loaded.bot_navigation(map))
            })
            .clone();
        let (mut spawned, mut removed) = (0, 0);
        let teams = [Team::Us, Team::Ger];
        let mut actions = teams.map(|team| {
            let mut humans: Vec<(PlayerId, bool)> = bridge
                .sessions()
                .filter(|(id, s)| s.team == team && !self.is_bot(**id))
                .map(|(id, _)| (*id, true))
                .collect();
            let ti = team as usize;
            let grace = (REJOIN_GRACE_S * f32::from(world.config().tick_hz)) as u32;
            self.disconnected[ti].retain(|&id, left| {
                bridge.session(id).is_none_or(|s| s.team != team)
                    && tick.saturating_sub(*left) < grace
            });
            humans.extend(self.disconnected[ti].keys().map(|&id| (id, false)));
            let bots: Vec<BotInfo> = self
                .slots
                .iter()
                .filter(|b| b.team == team)
                .map(|b| BotInfo {
                    id: b.player,
                    state: if matches!(
                        world.player_life(b.player),
                        None | Some(LifeState::Dead { .. })
                    ) {
                        BotState::Deploying
                    } else if world.seat_of(b.player).is_some() {
                        BotState::InVehicle
                    } else {
                        BotState::Alive
                    },
                })
                .collect();
            self.fill[ti]
                .reconcile(tick, &humans, &bots, target)
                .into_iter()
        });
        // Interleave joins so the sim's team-balance check sees both teams grow together.
        loop {
            let mut any = false;
            for (team, iter) in teams.into_iter().zip(&mut actions) {
                let Some(a) = iter.next() else { continue };
                any = true;
                match a {
                    FillAction::SpawnBot => {
                        let Some(id) = self.reserve_bot_id(bridge, world, team) else {
                            continue;
                        };
                        let rostered = world.roster_add(id, team).is_ok();
                        if !rostered {
                            if let Some(s) = &self.sessions {
                                s.unreserve_bot(id);
                            }
                            continue;
                        }
                        gameplay::assign_squad(world, &self.metrics, id, team);
                        let (_, yaw) = crate::world_loader::join_spawn_at(
                            bridge.map().map(AsRef::as_ref),
                            id,
                            team,
                        );
                        bridge.open_session(id, team, yaw);
                        self.add(
                            id,
                            team,
                            bot_nickname(127 - u16::from(id)),
                            Box::new(SoldierBrain::with_tactics(map.clone())),
                        );
                        self.slots.last_mut().unwrap().tactics = Some(gameplay::Slot::default());
                        spawned += 1;
                    }
                    FillAction::RemoveBot { id, .. } => {
                        bridge.leave(world, id);
                        self.forget(id);
                        removed += 1;
                    }
                }
            }
            if !any {
                break;
            }
        }
        (spawned, removed)
    }

    /// Updates tactical orders and performs gameplay requests before the read-only brain pass.
    pub(crate) fn gameplay(
        &mut self,
        world: &mut World,
        events: &[SimEvent],
        map: Option<&LoadedMap>,
    ) {
        if !self.slots.iter().any(|s| s.tactics.is_some()) {
            self.sounds.clear();
            self.leaders.clear();
            return;
        }
        let now = world.tick().0;
        gameplay::record_sounds(&mut self.sounds, now, events);
        if now.is_multiple_of(REPICK_TICKS) {
            self.update_orders(world, now);
        }
        for s in &mut self.slots {
            let Some(state) = &mut s.tactics else {
                continue;
            };
            let squad = world.squads().squad_of(s.player);
            let order = squad
                .and_then(|sq| self.leaders.get(&sq.id).filter(|(p, _)| *p == sq.leader))
                .map(|(_, l)| l.order());
            let site = map.and_then(|m| m.garrison_sites[s.team as usize]);
            state.update(world, &self.metrics, s.player, order, events, site);
        }
    }

    fn update_orders(&mut self, world: &World, now: u32) {
        self.leaders.retain(|id, (leader, _)| {
            world
                .squads()
                .squad(*id)
                .is_some_and(|sq| sq.leader == *leader)
        });
        let view = world.view();
        if view.objectives().is_empty() {
            self.leaders.clear();
            return;
        }
        for team in [Team::Us, Team::Ger] {
            let objs: Vec<_> = view
                .objectives()
                .iter()
                .map(|o| {
                    let friends_near = view
                        .players()
                        .iter()
                        .filter(|p| {
                            p.team == team
                                && frontline_sim::supply::dist_xz(p.pos, o.pos) <= o.radius_m
                        })
                        .count() as u8;
                    // Public contested state is a threat proxy, never a count of unseen opponents.
                    let enemies_near = if o.contested { 4 } else { 0 };
                    ObjView {
                        id: o.id,
                        pos: o.pos,
                        own: o.owner == Some(team),
                        held: o.p.abs() / 100.0,
                        friends_near,
                        enemies_near,
                    }
                })
                .collect();
            for s in self
                .slots
                .iter()
                .filter(|s| s.team == team && s.tactics.is_some())
            {
                let Some(sq) = world
                    .squads()
                    .squad_of(s.player)
                    .filter(|sq| sq.leader == s.player)
                else {
                    continue;
                };
                let Some(from) = world.player_state(s.player).map(|p| p.pos) else {
                    continue;
                };
                self.leaders
                    .entry(sq.id)
                    .or_insert_with(|| (s.player, SquadLeader::new()))
                    .1
                    .update(now, from, &objs);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn is_bot(&self, p: PlayerId) -> bool {
        self.slots.iter().any(|s| s.player == p)
    }

    pub fn count_team(&self, t: Team) -> usize {
        self.slots.iter().filter(|s| s.team == t).count()
    }

    pub fn nickname(&self, p: PlayerId) -> Option<&str> {
        self.slots
            .iter()
            .find(|s| s.player == p)
            .map(|s| s.name.as_str())
    }

    pub fn players(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.slots.iter().map(|s| s.player)
    }

    pub fn record_hashes(&mut self, on: bool) {
        self.record_hashes = on;
    }

    /// Per-tick command hashes recorded since [`record_hashes`](Self::record_hashes)`(true)`.
    pub fn hashes(&self) -> &[u64] {
        &self.hashes
    }

    pub fn last_hash(&self) -> u64 {
        self.last_hash
    }

    pub fn timings(&self) -> &[BotTickTiming] {
        &self.timings
    }

    pub fn stuck_total(&self) -> u64 {
        self.stuck_total
    }

    /// One bot step for the tick after `world.tick()`: every bot thinks, then its command is
    /// ingested through the bridge (validation included). Returns `(accepted, total)`.
    pub fn tick(&mut self, world: &World, bridge: &mut SimBridge) -> (usize, usize) {
        self.tick_started(world, bridge, Instant::now())
    }

    pub(crate) fn tick_started(
        &mut self,
        world: &World,
        bridge: &mut SimBridge,
        t0: Instant,
    ) -> (usize, usize) {
        let now = world.tick().0 + 1;
        let mut path_ns = 0u64;
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut accepted = 0usize;
        let view: &dyn WorldView = world.view();
        for s in &mut self.slots {
            let alive = world.player_life(s.player) == Some(LifeState::Alive);
            let squad = s
                .tactics
                .as_ref()
                .and_then(|_| world.squads().squad_of(s.player));
            let order =
                squad.and_then(|sq| self.leaders.get(&sq.id).filter(|(p, _)| *p == sq.leader));
            let role_order = s.tactics.as_ref().and_then(gameplay::Slot::intent);
            let ctx = BotCtx {
                bot_id: s.player,
                team: s.team,
                tick: now,
                difficulty: self.difficulty,
                rng_seed: self.seed,
                self_weapon: alive.then(|| world.weapon_state(s.player)).flatten(),
                squad_order: role_order
                    .map(|i| SquadOrder::Move(i.pos))
                    .or_else(|| order.map(|(_, l)| l.order())),
                squad_leader: squad.map(|sq| sq.leader).filter(|&p| p != s.player),
                hearing: if s.tactics.is_some() {
                    &self.sounds
                } else {
                    &[]
                },
                survival: s
                    .tactics
                    .as_ref()
                    .map_or_else(SoldierStatus::default, |st| st.survival(world, s.player)),
            };
            let mut cmd = s.brain.think(view, &ctx);
            if let Some((intent, from)) = role_order
                .filter(|_| alive)
                .zip(world.player_state(s.player))
            {
                intent.apply(from.pos, &mut cmd);
            }
            let st = s.brain.take_stats();
            path_ns += st.path_ns;
            self.stuck_total += u64::from(st.stuck_events);
            hash = fold(hash, s.player, &cmd);
            if bridge.ingest_cmd(s.player, &cmd, now).is_accepted() {
                accepted += 1;
            }
        }
        let total_ns = t0.elapsed().as_nanos() as u64;
        self.metrics.bot_think_ms.observe(total_ns as f64 / 1e6);
        self.metrics.bot_path_ms.observe(path_ns as f64 / 1e6);
        if self.timings.len() < MAX_SAMPLES {
            self.timings.push(BotTickTiming { total_ns, path_ns });
        }
        self.last_hash = hash;
        if self.record_hashes {
            self.hashes.push(hash);
        }
        (accepted, self.slots.len())
    }

    /// Think-only variant for benches: no bridge, the commands are returned.
    pub fn think_all(
        &mut self,
        view: &dyn WorldView,
        weapon: &dyn Fn(PlayerId) -> Option<WeaponState>,
        now: u32,
        out: &mut Vec<(PlayerId, InputCmd)>,
    ) {
        out.clear();
        for s in &mut self.slots {
            let ctx = BotCtx {
                bot_id: s.player,
                team: s.team,
                tick: now,
                difficulty: self.difficulty,
                rng_seed: self.seed,
                self_weapon: weapon(s.player),
                squad_order: None,
                squad_leader: None,
                hearing: &[],
                survival: SoldierStatus::default(),
            };
            let cmd = s.brain.think(view, &ctx);
            out.push((s.player, cmd));
            let _ = s.brain.take_stats();
        }
    }
}

fn fold(mut h: u64, p: PlayerId, c: &InputCmd) -> u64 {
    let mut put = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    };
    put(u64::from(p));
    put(u64::from(c.seq));
    put(u64::from(c.tick));
    put((c.move_x as u8) as u64 | ((c.move_z as u8) as u64) << 8);
    put(u64::from(c.yaw) | (c.pitch as u16 as u64) << 16);
    put(u64::from(c.buttons) | u64::from(c.aux) << 16 | u64::from(c.weapon_slot) << 24);
    h
}

#[cfg(test)]
mod fill_tests {
    use super::*;
    use crate::validate::ValidateCfg;
    use frontline_sim::{CollisionData, GameMode, MapId, SimConfig};

    fn setup(bots: u16) -> (World, SimBridge, BotHost) {
        let cfg = SimConfig {
            tick_hz: 30,
            match_seed: 1,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        };
        let empty = CollisionData::empty();
        let mut world = World::new(cfg, &empty);
        let metrics = Metrics::new();
        let mut bridge = SimBridge::new(metrics.clone(), None, None, ValidateCfg::default());
        let host = BotHost::with_soldiers(bots, 1, &mut world, &mut bridge, None, None, metrics);
        (world, bridge, host)
    }

    /// 2 bots at start (one per team) topped up to 3 per team, then a human joins a full team and one bot leaves.
    #[test]
    fn reconcile_tops_up_and_makes_room_for_humans() {
        let (mut world, mut bridge, mut host) = setup(2);
        assert_eq!(host.reconcile(0, &mut world, &mut bridge, 3), (4, 0));
        assert_eq!(
            (host.count_team(Team::Us), host.count_team(Team::Ger)),
            (3, 3)
        );
        assert_eq!(
            host.reconcile(1, &mut world, &mut bridge, 3),
            (0, 0),
            "stable at target"
        );
        bridge.join(&mut world, 5, Team::Us).unwrap();
        assert_eq!(host.reconcile(2, &mut world, &mut bridge, 3), (0, 1));
        assert_eq!(
            (host.count_team(Team::Us), host.count_team(Team::Ger)),
            (2, 3)
        );
        assert!(bridge.session(5).is_some(), "the human keeps their seat");
        assert!(host.players().all(|p| p != 5), "no bot took the human's id");
    }

    #[test]
    fn spawned_bots_carry_the_prefix_and_unique_ids() {
        let (mut world, mut bridge, mut host) = setup(0);
        // a host without nav data (no `with_soldiers` map) does nothing; with 0 bots it still owns a map
        host.reconcile(0, &mut world, &mut bridge, 2);
        let ids: Vec<_> = host.players().collect();
        let mut u = ids.clone();
        u.sort();
        u.dedup();
        assert_eq!(ids.len(), u.len());
        assert!(ids
            .iter()
            .all(|i| host.nickname(*i).unwrap().starts_with(BOT_NAME_PREFIX)));
    }

    #[test]
    fn fill_human_replaces_deploying_bot_and_honors_flap_grace() {
        let (mut world, mut bridge, mut host) = setup(2);
        assert_eq!(host.reconcile(0, &mut world, &mut bridge, 2), (2, 0));
        let waiting = host
            .slots
            .iter()
            .find(|s| s.team == Team::Us && s.tactics.is_some())
            .unwrap()
            .player;
        bridge.join(&mut world, 0, Team::Us).unwrap();
        assert_eq!(host.reconcile(30, &mut world, &mut bridge, 2), (0, 1));
        assert!(
            bridge.session(waiting).is_none(),
            "DEPLOY bot must be replaced first"
        );
        assert!(
            world.squads().entry(waiting).is_none(),
            "bot's roster seat must also leave"
        );
        assert!(host.is_bot(127), "living bot must keep its seat");

        for t in [60, 390] {
            host.human_left(0, Team::Us, t);
            assert!(bridge.leave(&mut world, 0));
            assert_eq!(host.reconcile(t, &mut world, &mut bridge, 2), (0, 0));
            assert_eq!(host.reconcile(t + 299, &mut world, &mut bridge, 2), (0, 0));
            if t == 60 {
                bridge.join(&mut world, 0, Team::Us).unwrap();
                assert_eq!(host.reconcile(t + 300, &mut world, &mut bridge, 2), (0, 0));
            } else {
                assert_eq!(host.reconcile(t + 300, &mut world, &mut bridge, 2), (1, 0));
            }
        }
    }
}
