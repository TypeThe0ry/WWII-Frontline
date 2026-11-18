//! The snapshot scheduler: which clients get a packet this tick, which entities go in it, and
//! the encoding (ARCHITECTURE §6.1, §6.5, §6.9; task M0-019).
//!
//! Pipeline per tick (sim thread, steps 6-7 of §6.1):
//! 1. refresh the per-entity frame table and the AOI grid from the world, update the
//!    firing / hurt trackers and route the tick's `SimEvent`s into the per-client windows;
//! 2. for every client due this tick (20 Hz, staggered by `player_id % 3`; 15 Hz on WebSocket
//!    or while the degradation guard is active): process its ack and `pred_hash`, run the
//!    candidate / priority pass, and encode one packet with a [`SnapshotBuilder`].
//!
//! Wire entity id = `PlayerId` (0..=127): the client's own record is never sent (its own state
//! travels in `self_state`), so the client's `player_id` doubles as its own entity id.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use frontline_protocol::quant::{quant_pitch, quant_yaw};
use frontline_protocol::snapshot::pred_hash::pred_hash_q;
use frontline_protocol::snapshot::{EntityKind, EntityState, Event, EventEntry, QPos};
use frontline_sim::{EntityId, PlayerId, SimEvent, Team, Vec3, World};

use super::aoi_grid::AoiGrid;
use super::baseline_book::ClientBook;
use super::events_out::{damage_angle, explosion_event, impact_event, shot_event};
use super::priority::*;
use super::self_state;
use crate::metrics::Metrics;

pub const TICK_HZ: f32 = 30.0;
/// Default packet budget (bytes of snapshot payload).
pub const PACKET_BUDGET: usize = 1150;
/// Ticks the degradation guard keeps the reduced rate after the last trigger (3 s).
pub const DEGRADE_HOLD_TICKS: u32 = 90;
/// Full-health damage used to scale `HitConfirm.damage_q` / `DamageDir.dmg_q`.
pub const DAMAGE_Q_FULL_HP: f32 = 100.0;
/// Cap on stored samples in [`SchedStats`].
const MAX_SAMPLES: usize = 400_000;

/// What the scheduler needs to know about the connection of a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkInfo {
    /// Largest snapshot payload for this connection.
    pub budget: usize,
    /// CRC-16 trailer (datagram transports; TCP already checks).
    pub crc: bool,
    /// 15 Hz instead of 20 Hz (WebSocket, ARCHITECTURE §6.6.2).
    pub hz15: bool,
}

impl LinkInfo {
    /// WebSocket: no datagram limit, no CRC, 15 Hz.
    pub const WS: LinkInfo = LinkInfo {
        budget: PACKET_BUDGET,
        crc: false,
        hz15: true,
    };
    /// A datagram transport with the given `ClientHandle::max_datagram_size()`; without a
    /// datagram path the link behaves like WebSocket.
    pub fn from_max_datagram(max_datagram: Option<usize>) -> LinkInfo {
        match max_datagram {
            None => LinkInfo::WS,
            Some(m) => LinkInfo {
                budget: PACKET_BUDGET.min(m.saturating_sub(1)).max(64),
                crc: true,
                hz15: false,
            },
        }
    }
    /// A 20 Hz datagram link with the default budget (tests, benches).
    pub const DATAGRAM: LinkInfo = LinkInfo {
        budget: PACKET_BUDGET,
        crc: true,
        hz15: false,
    };
}

/// Per-client inputs of one scheduler pass (read from the session layer).
#[derive(Debug, Clone, Copy)]
pub struct ClientIn {
    pub player: PlayerId,
    pub team: Team,
    /// Low 16 bits of the newest snapshot tick the client decoded.
    pub ack_low: u16,
    pub pred_hash: u8,
    /// Sequence number of the input the sim applied this tick.
    pub ack_input_seq: u16,
    pub input_slack: u8,
    pub link: LinkInfo,
}

/// One finished packet.
#[derive(Debug, Clone)]
pub struct Outgoing {
    pub player: PlayerId,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Default, Clone)]
pub struct SchedStats {
    pub packets: u64,
    pub bytes: u64,
    pub keyframe_records: u64,
    pub delta_records: u64,
    pub remove_records: u64,
    pub starved: u64,
    pub degraded_ticks: u64,
    pub encode_errors: u64,
    /// Enemy records that broke the R3 rules (audit; must stay 0).
    pub enemy_leaks: u64,
    /// Packet sizes in bytes.
    pub sizes: Vec<u16>,
    /// Per-client 1 s window rates (kbps): payload only / payload + 58 B per packet.
    pub kbps_payload: Vec<f32>,
    pub kbps_wire: Vec<f32>,
    /// Wall time of one scheduler pass (steps 6+7), nanoseconds.
    pub pass_ns: Vec<u32>,
    pub budget_util: Vec<f32>,
}

impl SchedStats {
    pub fn keyframe_share(&self) -> f64 {
        let t = self.keyframe_records + self.delta_records;
        if t == 0 {
            0.0
        } else {
            self.keyframe_records as f64 / t as f64
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Frame {
    present: bool,
    entity: EntityId,
    team: Team,
    pos: Vec3,
    state: EntityState,
    hurt_q: bool,
    firing: bool,
}

impl Default for Frame {
    fn default() -> Self {
        Frame {
            present: false,
            entity: 0,
            team: Team::Us,
            pos: Vec3::ZERO,
            state: EntityState::default(),
            hurt_q: false,
            firing: false,
        }
    }
}

/// Wire entity ids: players use 0..=127, vehicles 128..=255.
pub const MAX_SLOTS: usize = 256;
/// First wire id of vehicles.
pub const VEHICLE_ID_BASE: u8 = 128;

pub struct SnapshotScheduler {
    clients: BTreeMap<PlayerId, ClientBook>,
    grid: AoiGrid,
    frames: Vec<Frame>,
    team_of: [Team; 128],
    ent_of: [Option<EntityId>; 128],
    player_of: BTreeMap<EntityId, PlayerId>,
    last_fire: [u32; 128],
    last_hurt: [u32; 128],
    next_event_id: u16,
    degraded_left: u32,
    metrics: Option<Arc<Metrics>>,
    pub stats: SchedStats,
    /// Keep the sample vectors of [`SchedStats`] (tests / benches; always on, bounded).
    pub collect: bool,
    /// Entities within 96 m go into every packet (default off, see `client_pass`).
    pub near_every_packet: bool,
    // scratch
    cand_ids: Vec<u8>,
    cands: Vec<Cand>,
    near: [bool; MAX_SLOTS],
    /// Wire id (128..=255) of each live vehicle.
    veh_slot: BTreeMap<EntityId, u8>,
    params: AoiParams,
}

fn snap_stance(s: frontline_sim::Stance) -> u8 {
    s as u8
}

impl SnapshotScheduler {
    pub fn new(metrics: Option<Arc<Metrics>>) -> Self {
        SnapshotScheduler {
            clients: BTreeMap::new(),
            grid: AoiGrid::new(),
            frames: vec![Frame::default(); MAX_SLOTS],
            team_of: [Team::Us; 128],
            ent_of: [None; 128],
            player_of: BTreeMap::new(),
            last_fire: [0; 128],
            last_hurt: [0; 128],
            next_event_id: 0,
            degraded_left: 0,
            metrics,
            stats: SchedStats::default(),
            collect: true,
            near_every_packet: false,
            cand_ids: Vec::new(),
            cands: Vec::new(),
            near: [false; MAX_SLOTS],
            veh_slot: BTreeMap::new(),
            params: AoiParams::from_rules(&frontline_sim::GameData::embedded().rules.aoi, TICK_HZ),
        }
    }

    /// Reduced-rate guard: all clients drop to 15 Hz for the next `ticks` ticks (3 consecutive
    /// tick overruns, ARCHITECTURE §6.10).
    pub fn degrade(&mut self, ticks: u32) {
        self.degraded_left = self.degraded_left.max(ticks);
    }

    pub fn is_degraded(&self) -> bool {
        self.degraded_left > 0
    }

    pub fn client(&self, p: PlayerId) -> Option<&ClientBook> {
        self.clients.get(&p)
    }

    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Whether `player`'s packet is due at `tick`: 20 Hz = every tick except those with
    /// `tick % 3 == player % 3` (two packets per three ticks, staggered per client); 15 Hz =
    /// every second tick (`(tick + player) % 2 == 0`), the "every 2 ticks" equivalent of 3
    /// packets per 4 ticks.
    pub fn is_due(hz15: bool, player: PlayerId, tick: u32) -> bool {
        if hz15 {
            (tick.wrapping_add(player as u32)).is_multiple_of(2)
        } else {
            player as u32 % 3 != tick % 3
        }
    }

    fn event_id(&mut self) -> u16 {
        let id = self.next_event_id;
        self.next_event_id = self.next_event_id.wrapping_add(1);
        id
    }

    fn observer(world: &World, p: PlayerId, team: Team) -> Option<Observer> {
        let s = world.player_state(p)?;
        Some(Observer {
            pos: s.pos,
            yaw: s.yaw,
            pitch: s.pitch,
            team: team as u8,
            prev_yaw: s.yaw,
            prev_pitch: s.pitch,
        })
    }

    /// One pass. `now` is the tick the world just simulated.
    pub fn tick(
        &mut self,
        world: &World,
        events: &[SimEvent],
        clients: &[ClientIn],
        now: u32,
        out: &mut Vec<Outgoing>,
    ) {
        let t0 = Instant::now();
        self.refresh_frames(world, events, now);

        // client set
        self.clients
            .retain(|p, _| clients.iter().any(|c| c.player == *p));
        for c in clients {
            self.clients
                .entry(c.player)
                .or_insert_with(|| ClientBook::new(now));
        }
        let mut obs: BTreeMap<PlayerId, Observer> = BTreeMap::new();
        let union_ticks = self.params.turn_union_ticks;
        for c in clients {
            if let Some(mut o) = Self::observer(world, c.player, c.team) {
                if let Some(b) = self.clients.get_mut(&c.player) {
                    // Orientation `turn_union_ms` ago: the newest sample at least that old.
                    if let Some(&(_, y, p)) = b
                        .view_hist
                        .iter()
                        .rev()
                        .find(|(t, _, _)| now.wrapping_sub(*t) >= union_ticks)
                    {
                        o.prev_yaw = y;
                        o.prev_pitch = p;
                    }
                    b.view_hist.push_back((now, o.yaw, o.pitch));
                    while b.view_hist.len() > 2 * union_ticks as usize + 2 {
                        b.view_hist.pop_front();
                    }
                }
                obs.insert(c.player, o);
            }
        }
        self.route_events(world, events, clients, &obs);

        let degraded = self.degraded_left > 0;
        if degraded {
            self.degraded_left -= 1;
            self.stats.degraded_ticks += 1;
            if let Some(m) = &self.metrics {
                m.snapshot_degraded_ticks_total.inc();
            }
        }

        for c in clients {
            let Some(o) = obs.get(&c.player).copied() else {
                continue;
            };
            self.client_pass(world, c, &o, now, degraded, out);
        }

        // 1 s bandwidth windows (ticks without a packet count too)
        for c in clients {
            if let Some(b) = self.clients.get_mut(&c.player) {
                let sample = b.meter.tick(b.tick_bytes, b.tick_packets);
                b.tick_bytes = 0;
                b.tick_packets = 0;
                if let Some((pl, wire)) = sample {
                    if let Some(m) = &self.metrics {
                        m.net_bytes_out_per_client_kbps
                            .with_label_values(&["payload"])
                            .observe(pl);
                        m.net_bytes_out_per_client_kbps
                            .with_label_values(&["wire"])
                            .observe(wire);
                    }
                    if self.collect && self.stats.kbps_payload.len() < MAX_SAMPLES {
                        self.stats.kbps_payload.push(pl as f32);
                        self.stats.kbps_wire.push(wire as f32);
                    }
                }
            }
        }
        let ns = t0.elapsed().as_nanos().min(u32::MAX as u128) as u32;
        if self.collect && self.stats.pass_ns.len() < MAX_SAMPLES {
            self.stats.pass_ns.push(ns);
        }
        if let Some(m) = &self.metrics {
            m.aoi_encode_ms.observe(ns as f64 / 1e6);
        }
    }

    fn refresh_frames(&mut self, world: &World, events: &[SimEvent], now: u32) {
        for e in events {
            match *e {
                SimEvent::PlayerSpawned { player, entity } => {
                    if (player as usize) < 128 {
                        self.ent_of[player as usize] = Some(entity);
                    }
                    self.player_of.insert(entity, player);
                }
                SimEvent::PlayerRemoved { player, entity } => {
                    if (player as usize) < 128 {
                        self.ent_of[player as usize] = None;
                    }
                    self.player_of.remove(&entity);
                }
                SimEvent::Shot { shooter, .. } => {
                    if let Some(&p) = self.player_of.get(&shooter) {
                        self.last_fire[p as usize & 127] = now;
                    }
                }
                SimEvent::Hit {
                    shooter, target, ..
                } => {
                    if let Some(&p) = self.player_of.get(&target) {
                        self.last_hurt[p as usize & 127] = now;
                    }
                    if let Some(&p) = self.player_of.get(&shooter) {
                        self.last_hurt[p as usize & 127] = now;
                    }
                }
                SimEvent::Kill { victim, killer, .. } => {
                    self.last_hurt[victim as usize & 127] = now;
                    if let Some(k) = killer {
                        self.last_hurt[k as usize & 127] = now;
                    }
                }
                _ => {}
            }
        }

        let view = world.view();
        let mut present = [false; MAX_SLOTS];
        for pl in view.players() {
            let id = pl.player as usize & 127;
            present[id] = true;
            self.ent_of[id] = Some(pl.entity);
            self.team_of[id] = pl.team;
            let (anim, stance) = match world.player_state(pl.player) {
                Some(m) => {
                    let speed = m.vel.x.hypot(m.vel.z);
                    let mut a = if !m.grounded {
                        4
                    } else if speed < 0.3 {
                        0
                    } else if speed < 2.5 {
                        1
                    } else if speed < 5.0 {
                        2
                    } else {
                        3
                    };
                    let mut weapon_reload = false;
                    if let Some(w) = world.weapon_state(pl.player) {
                        let code = w.to_bytes()[6];
                        weapon_reload = code == 3 || code == 4;
                    }
                    if weapon_reload {
                        a |= 8;
                    }
                    (a, snap_stance(m.stance))
                }
                None => (0, 0),
            };
            let weapon = world
                .weapon_state(pl.player)
                .map_or(0, |w| (w.def_id & 63) as u8);
            let health_q = view.entity_state(pl.entity).map_or(0, |s| s.health_q);
            let firing =
                now.wrapping_sub(self.last_fire[id]) <= FIRING_TICKS && self.last_fire[id] != 0;
            let hurt =
                now.wrapping_sub(self.last_hurt[id]) <= HURT_TICKS && self.last_hurt[id] != 0;
            let flags = firing as u8 | (hurt as u8) << 1 | ((pl.team == Team::Ger) as u8) << 7;
            let f = &mut self.frames[id];
            f.present = true;
            f.entity = pl.entity;
            f.team = pl.team;
            f.pos = pl.pos;
            f.firing = firing;
            f.hurt_q = hurt;
            f.state = EntityState {
                id: id as u8,
                kind: EntityKind::Player,
                pos: QPos::from_meters(pl.pos.x, pl.pos.y, pl.pos.z),
                yaw: quant_yaw(pl.yaw, 10).code,
                pitch: quant_pitch(pl.pitch, 6).code as u8,
                vel: None,
                anim,
                stance,
                weapon,
                flags,
                health_q,
                vehicle: None,
            };
            self.grid.update(id as u8, pl.pos.x, pl.pos.z);
        }
        self.update_vehicle_frames(world, &mut present);
        for (id, p) in present.iter().enumerate() {
            if !p && self.frames[id].present {
                self.frames[id].present = false;
                self.grid.remove(id as u8);
            }
        }
    }

    /// Fills the frames of vehicles (wire ids 128..=255; a vehicle keeps its slot while it lives).
    fn update_vehicle_frames(&mut self, world: &World, present: &mut [bool; MAX_SLOTS]) {
        use frontline_protocol::snapshot::{QVel, VehicleExtra};
        let ids = world.vehicle_ids();
        self.veh_slot.retain(|e, _| ids.contains(e));
        for e in ids {
            let Some(v) = world.vehicle(e) else { continue };
            if v.gone {
                continue;
            }
            let slot = match self.veh_slot.get(&e) {
                Some(&s) => s,
                None => {
                    let used: Vec<u8> = self.veh_slot.values().copied().collect();
                    let Some(s) = (VEHICLE_ID_BASE..=255).find(|s| !used.contains(s)) else {
                        continue;
                    };
                    self.veh_slot.insert(e, s);
                    s
                }
            };
            let id = slot as usize;
            present[id] = true;
            let seats = v
                .seats
                .iter()
                .take(3)
                .enumerate()
                .fold(0u8, |m, (i, o)| m | (u8::from(o.is_some()) << i));
            let damage = u8::from(v.knocked_out)
                | u8::from(v.burn_left > 0) << 1
                | u8::from(v.overturned) << 2;
            let hp_q = if v.hull_hp > 0.0 && v.params.hull_hp > 0.0 {
                ((v.hull_hp / v.params.hull_hp).min(1.0) * 255.0 + 0.5) as u8
            } else {
                0
            };
            let tau = std::f32::consts::TAU;
            let pi = std::f32::consts::PI;
            let ty = (v.turret.yaw.rem_euclid(tau) / tau * 4096.0) as u16 & 0xFFF;
            let gp = (((v.turret.pitch + pi / 2.0) / pi).clamp(0.0, 1.0) * 4095.0 + 0.5) as u16;
            let f = &mut self.frames[id];
            f.present = true;
            f.entity = e;
            f.team = v.team;
            f.pos = v.pos;
            f.firing = false;
            f.hurt_q = false;
            f.state = EntityState {
                id: slot,
                kind: EntityKind::Vehicle,
                pos: QPos::from_meters(v.pos.x, v.pos.y, v.pos.z),
                yaw: 0,
                pitch: 0,
                vel: Some(QVel::from_mps(v.vel.x, v.vel.y, v.vel.z)),
                anim: 0,
                stance: 0,
                weapon: 0,
                flags: ((v.team == Team::Ger) as u8) << 7,
                health_q: hp_q,
                vehicle: Some(VehicleExtra {
                    quat: frontline_protocol::quant::quat_pack([
                        v.rot.x, v.rot.y, v.rot.z, v.rot.w,
                    ]),
                    turret_yaw: ty,
                    gun_pitch: gp,
                    track_flags: 0,
                    seats,
                    damage_flags: damage,
                }),
            };
            self.grid.update(slot, v.pos.x, v.pos.z);
        }
    }

    fn route_events(
        &mut self,
        world: &World,
        events: &[SimEvent],
        clients: &[ClientIn],
        obs: &BTreeMap<PlayerId, Observer>,
    ) {
        for e in events {
            match *e {
                SimEvent::Shot {
                    shooter,
                    weapon,
                    muzzle,
                    dir,
                } => {
                    let Some(&sp) = self.player_of.get(&shooter) else {
                        continue;
                    };
                    let id = self.event_id();
                    let entry = EventEntry {
                        id,
                        event: shot_event(sp, weapon, muzzle, dir),
                    };
                    let steam = self.team_of[sp as usize & 127];
                    for c in clients {
                        if c.player == sp {
                            continue;
                        }
                        let Some(o) = obs.get(&c.player) else {
                            continue;
                        };
                        let rel = relation(o, muzzle);
                        if event_visible(&self.params, c.team == steam, &rel) {
                            if let Some(b) = self.clients.get_mut(&c.player) {
                                b.events.push(entry);
                            }
                        }
                    }
                }
                SimEvent::Impact {
                    pos,
                    normal,
                    material,
                } => {
                    let id = self.event_id();
                    let entry = EventEntry {
                        id,
                        event: impact_event(pos, normal, material),
                    };
                    for c in clients {
                        let Some(o) = obs.get(&c.player) else {
                            continue;
                        };
                        if o.pos.distance(pos) <= ENTER_M {
                            if let Some(b) = self.clients.get_mut(&c.player) {
                                b.events.push(entry);
                            }
                        }
                    }
                }
                SimEvent::Explosion {
                    pos,
                    radius_m,
                    kind,
                } => {
                    let id = self.event_id();
                    let entry = EventEntry {
                        id,
                        event: explosion_event(pos, radius_m, kind),
                    };
                    for c in clients {
                        let Some(o) = obs.get(&c.player) else {
                            continue;
                        };
                        if o.pos.distance(pos) <= ENTER_M {
                            if let Some(b) = self.clients.get_mut(&c.player) {
                                b.events.push(entry);
                            }
                        }
                    }
                }
                SimEvent::Suppress {
                    target,
                    source_dir,
                    intensity,
                } => {
                    let Some(&tp) = self.player_of.get(&target) else {
                        continue;
                    };
                    if clients.iter().any(|c| c.player == tp) {
                        let id = self.event_id();
                        if let Some(b) = self.clients.get_mut(&tp) {
                            b.events.push(EventEntry {
                                id,
                                event: Event::Suppress {
                                    source_dir,
                                    intensity,
                                },
                            });
                        }
                    }
                }
                SimEvent::Hit {
                    shooter,
                    target,
                    zone,
                    damage_hp,
                    ..
                } => {
                    let dmg_q =
                        ((damage_hp / DAMAGE_Q_FULL_HP).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                    let (Some(&sp), Some(&tp)) =
                        (self.player_of.get(&shooter), self.player_of.get(&target))
                    else {
                        continue;
                    };
                    if clients.iter().any(|c| c.player == sp) {
                        let id = self.event_id();
                        if let Some(b) = self.clients.get_mut(&sp) {
                            b.events.push(EventEntry {
                                id,
                                event: Event::HitConfirm {
                                    target: tp,
                                    zone: zone as u8,
                                    damage_q: dmg_q,
                                },
                            });
                        }
                    }
                    if clients.iter().any(|c| c.player == tp) {
                        if let (Some(o), Some(sstate)) =
                            (obs.get(&tp), world.view().entity_state(shooter))
                        {
                            let id = self.event_id();
                            let angle = damage_angle(o.yaw, sstate.pos, o.pos);
                            if let Some(b) = self.clients.get_mut(&tp) {
                                b.events.push(EventEntry {
                                    id,
                                    event: Event::DamageDir { angle, dmg_q },
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn client_pass(
        &mut self,
        world: &World,
        c: &ClientIn,
        o: &Observer,
        now: u32,
        degraded: bool,
        out: &mut Vec<Outgoing>,
    ) {
        let p = c.player;
        let self_state_pos = world.player_state(p).map(|s| s.pos).unwrap_or(o.pos);
        let q = QPos::from_meters(self_state_pos.x, self_state_pos.y, self_state_pos.z);
        let hz15 = c.link.hz15 || degraded;

        let Some(book) = self.clients.get_mut(&p) else {
            return;
        };
        book.on_client_report(c.ack_low, c.pred_hash);
        if !Self::is_due(hz15, p, now) {
            return;
        }
        let packet_hz = if hz15 { 15.0 } else { 20.0 };

        // ---- candidate / priority pass ----
        let dt = book
            .prio
            .last_eval_tick
            .map_or(1.0 / packet_hz, |t| now.wrapping_sub(t) as f32 / TICK_HZ);
        book.prio.last_eval_tick = Some(now);
        book.prio.eval_counter = book.prio.eval_counter.wrapping_add(1);
        let ec = book.prio.eval_counter;
        self.cand_ids.clear();
        self.grid.query_radius(
            o.pos.x,
            o.pos.z,
            self.params.leave_max_m,
            &mut self.cand_ids,
        );
        self.cands.clear();
        for &id in &self.cand_ids {
            if id == p {
                continue;
            }
            let f = &self.frames[id as usize];
            if !f.present {
                continue;
            }
            let slot = &mut book.prio.slots[id as usize];
            let was = slot.state == SlotState::InRange;
            let rel = relation(o, f.pos);
            if !is_candidate(&self.params, f.team as u8 == o.team, &rel, was) {
                continue;
            }
            slot.touched = ec;
            if !was {
                slot.state = SlotState::InRange;
                slot.acc = 1.0;
                slot.remove_left = 0;
            }
            // Option (default off): PI-010 notes that a delta needs the entity to be in the
            // acknowledged baseline packet. Forcing everything within 96 m into every packet
            // makes them all deltas but measured 20 % MORE bytes than the spec rates with
            // keyframes (deltas of moving players are 8-10 B, keyframes 12.5 B, T1 is 10 Hz):
            // see `near_every_packet_policy_comparison`.
            let rate = if self.near_every_packet && rel.dist <= self.params.enemy_close_m {
                packet_hz
            } else {
                rate_hz(&rel, f.firing, f.hurt_q).min(packet_hz)
            };
            slot.acc = (slot.acc + rate * dt).min(ACC_CAP);
            self.near[id as usize] = rel.dist <= self.params.enemy_close_m;
            self.cands.push(Cand { id, acc: slot.acc });
        }
        for (id, slot) in book.prio.slots.iter_mut().enumerate() {
            if slot.state == SlotState::InRange && slot.touched != ec {
                slot.state = SlotState::Leaving;
                slot.remove_left = REMOVE_REPEATS;
                slot.acc = 0.0;
                let _ = id;
            }
        }
        order(&mut self.cands);

        // ---- encode ----
        let health_q = self.ent_of[p as usize]
            .and_then(|e| world.view().entity_state(e))
            .map_or(0, |s| s.health_q);
        let corr = book.corr.pending.then_some(q);
        let st = self_state::build(world, p, health_q, corr);
        let mut b = book
            .baselines
            .begin(now, c.link.budget)
            .with_crc(c.link.crc);
        let _ = b.set_ack(c.ack_input_seq, c.input_slack.min(15));
        if !b.add_self(&st).unwrap_or(false) {
            self.stats.encode_errors += 1;
        }
        let evs = book.events.entries();
        let n_ev = b.add_events(&evs).unwrap_or(0);

        let mut removes: [u8; MAX_SLOTS] = [0; MAX_SLOTS];
        let mut n_rm = 0usize;
        for (id, slot) in book.prio.slots.iter().enumerate() {
            if slot.state == SlotState::Leaving && slot.remove_left > 0 {
                removes[n_rm] = id as u8;
                n_rm += 1;
            }
        }
        let mut rm_written = 0u64;
        for &id in &removes[..n_rm] {
            if b.add_removed(id).unwrap_or(false) {
                rm_written += 1;
            }
        }
        let mut starved = 0u64;
        let mut sent_ids: [u8; MAX_SLOTS] = [0; MAX_SLOTS];
        let mut n_sent = 0usize;
        for cd in &self.cands {
            let f = &self.frames[cd.id as usize];
            match b.add_entity(&f.state, self.near[cd.id as usize]) {
                Ok(true) => {
                    sent_ids[n_sent] = cd.id;
                    n_sent += 1;
                }
                Ok(false) => starved += 1,
                Err(_) => self.stats.encode_errors += 1,
            }
        }
        let (kf, dl) = b.record_counts();
        let bytes = b.finish();

        // ---- audit (R3): every enemy record must satisfy the rules, with the leave limits
        // (hysteresis) as the hard bound ----
        let mut leaks = 0u64;
        for &id in &sent_ids[..n_sent] {
            let f = &self.frames[id as usize];
            if f.team as u8 == o.team {
                continue;
            }
            let rel = relation(o, f.pos);
            if !is_candidate(&self.params, false, &rel, true) {
                leaks += 1;
            }
        }
        if leaks > 0 {
            self.stats.enemy_leaks += leaks;
            if let Some(m) = &self.metrics {
                m.aoi_enemy_leak_total.inc_by(leaks);
            }
        }

        // ---- bookkeeping ----
        for &id in &sent_ids[..n_sent] {
            let s = &mut book.prio.slots[id as usize];
            s.acc = (s.acc - 1.0).max(0.0);
        }
        for &id in &removes[..n_rm] {
            let s = &mut book.prio.slots[id as usize];
            s.remove_left = s.remove_left.saturating_sub(1);
            if s.remove_left == 0 {
                *s = Slot::default();
            }
        }
        book.events.mark_sent(n_ev, now);
        book.record_sent(now, pred_hash_q(q));
        book.packets += 1;
        book.bytes += bytes.len() as u64;
        book.keyframe_records += kf as u64;
        book.delta_records += dl as u64;
        book.starved += starved;
        book.tick_bytes += bytes.len();
        book.tick_packets += 1;

        let st = &mut self.stats;
        st.packets += 1;
        st.bytes += bytes.len() as u64;
        st.keyframe_records += kf as u64;
        st.delta_records += dl as u64;
        st.remove_records += rm_written;
        st.starved += starved;
        if self.collect && st.sizes.len() < MAX_SAMPLES {
            st.sizes.push(bytes.len() as u16);
            st.budget_util
                .push(bytes.len() as f32 / c.link.budget as f32);
        }
        if let Some(m) = &self.metrics {
            m.snapshot_bytes.observe(bytes.len() as f64);
            m.snapshot_budget_utilization
                .observe(bytes.len() as f64 / c.link.budget as f64);
            m.snapshots_sent_total.inc();
            m.snapshot_keyframe_records_total.inc_by(kf as u64);
            m.snapshot_delta_records_total.inc_by(dl as u64);
            if starved > 0 {
                m.aoi_starved_total.inc_by(starved);
            }
        }
        out.push(Outgoing { player: p, bytes });
    }
}
