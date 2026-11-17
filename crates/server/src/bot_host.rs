//! Server-side bot host (M0-026; ARCHITECTURE 16.1, 6.10 stage 6).
//!
//! A bot is "a client without a network connection": the host gives every bot a `PlayerId` and a
//! session (through [`SimBridge::join`], exactly like a human's join), asks its
//! [`BotBrain`] for one [`InputCmd`] per tick from the **read-only** world, and pushes that command
//! through [`SimBridge::ingest_cmd`] - the same window check + `validate_input` + input buffer
//! that human C1 bundles use. The host holds `&World` only; it can neither change the world nor
//! skip validation (`input_rejected_total` counts bot violations like anyone's).

use std::sync::Arc;
use std::time::Instant;

use frontline_bots::{BotBrain, BotCtx, BotMap, Difficulty, SoldierBrain, Tuning};
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::{GameData, PlayerId, Team, WeaponState, World, WorldView};

use crate::metrics::Metrics;
use crate::net::conn::Sessions;
use crate::sim_bridge::SimBridge;
use crate::world_loader::LoadedMap;

/// Nickname prefix of every bot (GAMEDESIGN 20.2).
pub const BOT_NAME_PREFIX: &str = "[BOT]";
/// Cap on stored timing samples (one per tick; ~37 h at 30 Hz).
const MAX_SAMPLES: usize = 4_000_000;

/// `PlayerId` and team of the `index`-th bot: ids count down from 127 (humans take the lowest
/// free id), teams alternate US, GER, ... so `--bots N` splits evenly.
pub fn bot_slot(index: u16) -> (PlayerId, Team) {
    let id = 127u16.saturating_sub(index) as PlayerId;
    (id, if index.is_multiple_of(2) { Team::Us } else { Team::Ger })
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
}

/// Timing of one bot tick, as measured by the host.
#[derive(Debug, Clone, Copy, Default)]
pub struct BotTickTiming {
    /// Whole host tick: all `think` calls plus input ingestion.
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
            if let Some(s) = sessions {
                s.reserve_bot(id, team);
            }
            if let Err(e) = bridge.join(world, id, team) {
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
        });
        self.metrics.bots.set(self.slots.len() as i64);
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
        let t0 = Instant::now();
        let now = world.tick().0 + 1;
        let weapon = |p: PlayerId| world.weapon_state(p);
        let mut path_ns = 0u64;
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut accepted = 0usize;
        let view: &dyn WorldView = world.view();
        let alive: Vec<PlayerId> = view.players().iter().map(|p| p.player).collect();
        for s in &mut self.slots {
            let ctx = BotCtx {
                bot_id: s.player,
                team: s.team,
                tick: now,
                difficulty: self.difficulty,
                rng_seed: self.seed,
                self_weapon: alive
                    .contains(&s.player)
                    .then(|| weapon(s.player))
                    .flatten(),
            };
            let cmd = s.brain.think(view, &ctx);
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
