//! In-process synthetic clients for the snapshot tests (M0-019): a real `World`, the real
//! `SnapshotScheduler`, and per client a `SnapshotDecoder` with ack feedback, optional packet
//! loss and ack delay. No network involved, so the numbers isolate AOI + encoding.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use frontline_protocol::codec::CodecError;
use frontline_protocol::msg::input::InputCmd;
use frontline_protocol::snapshot::pred_hash::pred_hash_q;
use frontline_protocol::snapshot::{EntityState, EventEntry, QPos, SnapshotDecoder};
use frontline_server::snapshot::{ClientIn, LinkInfo, Outgoing, SnapshotScheduler};
use frontline_server::world_loader::LoadedMap;
use frontline_sim::{
    CollisionData, GameData, GameMode, MapId, SimConfig, SimEvent, SpawnReq, Team, Vec3, World,
};

pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn f64(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[derive(Debug, Clone, Default)]
pub struct PacketLog {
    pub ids: Vec<u8>,
    pub kf_ids: Vec<u8>,
    pub tick: u32,
    pub bytes: usize,
    pub entities: usize,
    pub keyframes: usize,
    pub removed: usize,
    pub has_corr: bool,
    pub has_baseline: bool,
    pub events: usize,
}

pub struct SynthClient {
    pub player: u8,
    pub team: Team,
    pub link: LinkInfo,
    pub dec: SnapshotDecoder,
    pub ack_tick: Option<u32>,
    pub hash: u8,
    /// Reported to the server `ack_delay` ticks late.
    pending_acks: VecDeque<(u32, Option<u32>, u8)>,
    pub known: BTreeMap<u8, EntityState>,
    pub seen_events: BTreeSet<u16>,
    pub events: Vec<(u32, EventEntry)>,
    pub duplicate_events: u64,
    /// How often each event id arrived (before de-duplication).
    pub event_arrivals: BTreeMap<u16, u32>,
    pub packets_ok: u64,
    pub packets_lost: u64,
    pub missing_baseline: u64,
    pub miss_since: Option<u32>,
    pub max_recovery_ticks: u32,
    pub log: Vec<PacketLog>,
    /// Every entity id that ever appeared (record or event shooter).
    pub ever_ids: BTreeSet<u8>,
    /// Wrong hash to report instead of the true one (position-correction tests).
    pub tamper: Option<u8>,
    /// Drop every packet while set (ack blackout).
    pub blackout: bool,
    pub last: Option<frontline_protocol::snapshot::DecodedSnapshot>,
}

impl SynthClient {
    fn new(player: u8, team: Team, link: LinkInfo) -> Self {
        SynthClient {
            player,
            team,
            link,
            dec: SnapshotDecoder::new(),
            ack_tick: None,
            hash: 0,
            pending_acks: VecDeque::new(),
            known: BTreeMap::new(),
            seen_events: BTreeSet::new(),
            events: Vec::new(),
            duplicate_events: 0,
            event_arrivals: BTreeMap::new(),
            packets_ok: 0,
            packets_lost: 0,
            missing_baseline: 0,
            miss_since: None,
            max_recovery_ticks: 0,
            log: Vec::new(),
            ever_ids: BTreeSet::new(),
            tamper: None,
            blackout: false,
            last: None,
        }
    }
}

pub struct Synth {
    pub world: World,
    pub sched: SnapshotScheduler,
    pub clients: Vec<SynthClient>,
    pub rng: Rng,
    pub loss: f64,
    pub ack_delay: u32,
    out: Vec<Outgoing>,
    pub tick: u32,
}

pub fn new_world(map: Option<&Arc<LoadedMap>>) -> World {
    let empty = CollisionData::empty();
    let mut w = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 7,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        map.map_or(&empty, |m| &m.collision),
    );
    if let Some(m) = map {
        m.apply_to(&mut w);
    }
    w
}

impl Synth {
    pub fn new(world: World) -> Self {
        Synth {
            world,
            sched: SnapshotScheduler::new(None),
            clients: Vec::new(),
            rng: Rng(0x9E37_79B9_7F4A_7C15),
            loss: 0.0,
            ack_delay: 2,
            out: Vec::new(),
            tick: 0,
        }
    }

    /// Spawns a player (a client when `client` is set).
    pub fn add(&mut self, player: u8, team: Team, pos: Vec3, yaw: f32, client: Option<LinkInfo>) {
        self.world
            .spawn_player(SpawnReq {
                player,
                team,
                pos,
                yaw,
            })
            .expect("spawn");
        if let Some(link) = client {
            self.clients.push(SynthClient::new(player, team, link));
        }
    }

    pub fn client(&self, p: u8) -> &SynthClient {
        self.clients.iter().find(|c| c.player == p).expect("client")
    }

    pub fn client_mut(&mut self, p: u8) -> &mut SynthClient {
        self.clients
            .iter_mut()
            .find(|c| c.player == p)
            .expect("client")
    }

    pub fn players(&self) -> Vec<u8> {
        self.world
            .view()
            .players()
            .iter()
            .map(|p| p.player)
            .collect()
    }

    pub fn outgoing(&self) -> &[Outgoing] {
        &self.out
    }

    /// One tick: `input(player, tick)` for every player alive in the world (ascending ids).
    pub fn step(&mut self, input: impl FnMut(u8, u32) -> InputCmd) -> Vec<SimEvent> {
        self.step_with_events(input, &[])
    }

    pub fn step_with_events(
        &mut self,
        mut input: impl FnMut(u8, u32) -> InputCmd,
        injected: &[SimEvent],
    ) -> Vec<SimEvent> {
        let t = self.world.tick().0 + 1;
        self.tick = t;
        let mut inputs: Vec<(u8, InputCmd)> = Vec::new();
        for p in self.players() {
            inputs.push((p, input(p, t)));
        }
        // dead players are not in `players()`: they get no input (the sim keeps them dead)
        let mut out = self.world.step(&inputs);
        out.events.extend_from_slice(injected);
        let mut cin = Vec::new();
        for c in &mut self.clients {
            while c.pending_acks.front().is_some_and(|a| a.0 <= t) {
                let (_, ack, h) = c.pending_acks.pop_front().unwrap();
                if let Some(a) = ack {
                    c.ack_tick = Some(a);
                }
                c.hash = h;
            }
            cin.push(ClientIn {
                player: c.player,
                team: c.team,
                ack_low: c.ack_tick.map_or(0, |a| a as u16),
                pred_hash: c.hash,
                ack_input_seq: t as u16,
                input_slack: 1,
                link: c.link,
            });
        }
        self.out.clear();
        self.sched
            .tick(&self.world, &out.events, &cin, t, &mut self.out);
        let outs = std::mem::take(&mut self.out);
        for o in &outs {
            self.deliver(o, t);
        }
        self.out = outs;
        out.events
    }

    fn deliver(&mut self, o: &Outgoing, t: u32) {
        let lost = {
            let c = self.clients.iter().find(|c| c.player == o.player).unwrap();
            c.blackout || self.rng.f64() < self.loss
        };
        let truth = self
            .world
            .player_state(o.player)
            .map(|s| pred_hash_q(QPos::from_meters(s.pos.x, s.pos.y, s.pos.z)));
        let delay = self.ack_delay;
        let c = self
            .clients
            .iter_mut()
            .find(|c| c.player == o.player)
            .unwrap();
        if lost {
            c.packets_lost += 1;
            return;
        }
        match c.dec.decode(&o.bytes) {
            Ok(s) => {
                c.packets_ok += 1;
                if let Some(since) = c.miss_since.take() {
                    c.max_recovery_ticks = c.max_recovery_ticks.max(t - since);
                }
                let mut log = PacketLog {
                    ids: s.entities.ids.clone(),
                    kf_ids: s
                        .entities
                        .ids
                        .iter()
                        .zip(&s.entities.keyframe)
                        .filter(|(_, &k)| k)
                        .map(|(&i, _)| i)
                        .collect(),
                    tick: s.tick,
                    bytes: o.bytes.len(),
                    entities: s.entities.len(),
                    keyframes: s.entities.keyframe.iter().filter(|&&k| k).count(),
                    removed: s.removed_ids.len(),
                    has_corr: s.self_state.is_some_and(|x| x.pos_corr.is_some()),
                    has_baseline: s.entities.keyframe.iter().any(|&k| !k),
                    events: s.events.len(),
                };
                log.has_baseline = s.entities.keyframe.iter().any(|&k| !k);
                c.log.push(log);
                for st in s.entities.to_states() {
                    c.known.insert(st.id, st);
                    c.ever_ids.insert(st.id);
                }
                for id in &s.removed_ids {
                    c.known.remove(id);
                }
                for e in &s.events {
                    *c.event_arrivals.entry(e.id).or_insert(0) += 1;
                    if c.seen_events.insert(e.id) {
                        c.events.push((s.tick, *e));
                    } else {
                        c.duplicate_events += 1;
                    }
                }
                let hash = c.tamper.or(truth).unwrap_or(0);
                c.pending_acks.push_back((t + delay, Some(s.tick), hash));
                c.last = Some(s);
            }
            Err(CodecError::MissingBaseline) => {
                c.missing_baseline += 1;
                c.miss_since.get_or_insert(t);
            }
            Err(e) => panic!("decode error {e:?} for player {}", o.player),
        }
    }
}

/// Deterministic spread of `n` players around the map's HQs (real terrain), alternating teams.
pub fn spawn_squads(s: &mut Synth, map: &LoadedMap, n: usize, clients: usize, link: LinkInfo) {
    for i in 0..n {
        let team = if i % 2 == 0 { Team::Us } else { Team::Ger };
        let (pos, yaw) = map.join_spawn(i as u8, team);
        s.add(i as u8, team, pos, yaw, (i < clients).then_some(link));
    }
}
