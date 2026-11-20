//! C0 comms ingress and per-client C2 map marker delivery.

use std::collections::BTreeMap;

use frontline_protocol::msg::c0::{Chat, MarkerPlace, NoticeC0, ServerMsgC0};
use frontline_protocol::msg::ping::{PingKind, PingPlace};
use frontline_protocol::msg::team::{TeamMarker, TeamMarkerKind, MARKER_CELL_M, MAX_MARKER_CELL};
use frontline_protocol::snapshot::{Event, MarkerKind as WireKind};
use frontline_sim::comms::{
    chat, ping, Marker, MarkerErr, MarkerId, MarkerKind, MarkerRegistry, Member,
};
use frontline_sim::{GameData, PlayerId, Tick, Vec3, World};

use super::Outbox;
use crate::metrics::Metrics;
use crate::snapshot::{team_channel::MapFrame, ClientIn, SnapshotScheduler};

const PLAYER_SLOTS: usize = 128;

pub(crate) fn notice(key: &str) -> ServerMsgC0 {
    ServerMsgC0::Notice(NoticeC0 {
        key: key.to_owned(),
    })
}

#[derive(Default)]
pub(super) struct ChatRouting {
    windows: BTreeMap<PlayerId, chat::ChatWindow>,
}

impl ChatRouting {
    pub fn forget(&mut self, player: PlayerId) {
        self.windows.remove(&player);
    }

    pub fn route(
        &mut self,
        world: &World,
        metrics: &Metrics,
        from: PlayerId,
        request: Chat,
        muted: bool,
        out: &mut Outbox,
    ) -> bool {
        let result = (|| {
            if muted {
                return Err("muted");
            }
            let recipients = chat::recipients(world.squads(), from, request.channel)
                .map_err(chat::ChatReject::label)?;
            let line = chat::sanitize_chat(&request.text).map_err(chat::ChatReject::label)?;
            self.windows
                .entry(from)
                .or_default()
                .admit(world.tick(), u32::from(world.config().tick_hz), &line)
                .map_err(chat::ChatReject::label)?;
            out.extend(recipients.map(|to| {
                (
                    to,
                    ServerMsgC0::Chat(Chat {
                        channel: request.channel,
                        text: line.clone(),
                        sender: from,
                    }),
                )
            }));
            Ok(())
        })();
        match result {
            Ok(()) => {
                metrics.chat_routed_total.inc();
                true
            }
            Err(reason) => {
                metrics
                    .chat_rejected_total
                    .with_label_values(&[reason])
                    .inc();
                out.push((from, notice(&format!("comms.chat.{reason}"))));
                false
            }
        }
    }
}

struct Published {
    marker: Marker,
    viewers: u128,
    wire_id: u16,
}

pub(crate) struct Markers {
    registry: MarkerRegistry,
    hz: u32,
    map: MapFrame,
    members: [Option<Member>; PLAYER_SLOTS],
    published: BTreeMap<MarkerId, Published>,
    next_wire_id: u16,
}

impl Markers {
    pub fn new(hz: u8, map: MapFrame) -> Self {
        let hz = u32::from(hz);
        Self {
            registry: MarkerRegistry::new(GameData::embedded().comms, hz),
            hz,
            map,
            members: [None; PLAYER_SLOTS],
            published: BTreeMap::new(),
            next_wire_id: 1,
        }
    }

    pub fn place_manual(
        &mut self,
        world: &World,
        player: PlayerId,
        request: MarkerPlace,
    ) -> Result<(), &'static str> {
        let kind = TeamMarkerKind::from_u8(request.kind).ok_or("comms.marker.unsupported")?;
        let author = self.author(world, player)?;
        let kind = manual_kind(kind);
        self.registry.can_place(author, kind).map_err(marker_key)?;
        let pos = self.map_point(request.pos)?;
        let (pos, target) = if let Some(class) = team_mark_class(kind) {
            let hit = ping::identify(world, player, optics_range(author))
                .filter(|t| t.team != author.team && t.key.kind == class)
                .ok_or("comms.marker.no_target")?;
            (hit.pos, Some(hit.key.id))
        } else {
            (pos, None)
        };
        self.place(author, kind, pos, target, world.tick())
    }

    pub fn place_ping(
        &mut self,
        world: &World,
        player: PlayerId,
        request: PingPlace,
    ) -> Result<(), &'static str> {
        use PingKind::*;
        let author = self.author(world, player)?;
        let mut pos = self.map_point(request.pos)?;
        let mut target = None;
        let kind = match request.kind {
            Attention | Enemy => {
                let hit = ping::identify(world, player, 400.0);
                match hit {
                    Some(hit) if hit.team != author.team => {
                        target = Some(hit.key.id);
                        pos = hit.pos;
                        MarkerKind::Enemy
                    }
                    Some(hit) if request.kind == Attention => {
                        pos = hit.pos;
                        if hit.downed {
                            MarkerKind::NeedMedic
                        } else {
                            if hit.key.kind != ping::TargetKind::Building {
                                target = Some(hit.key.id);
                            }
                            MarkerKind::Thanks
                        }
                    }
                    _ if request.kind == Enemy => return Err("comms.marker.no_target"),
                    _ => MarkerKind::Attention,
                }
            }
            NeedMedic => MarkerKind::NeedMedic,
            NeedAmmo => MarkerKind::NeedAmmo,
            NeedEngineer => MarkerKind::NeedEngineer,
            NeedAt => MarkerKind::NeedAt,
            NeedSupplies => MarkerKind::NeedSupplies,
            Thanks => MarkerKind::Thanks,
        };
        self.place(author, kind, pos, target, world.tick())
    }

    fn author(&self, world: &World, player: PlayerId) -> Result<Member, &'static str> {
        Member::from_roster(world.squads(), player).ok_or("comms.marker.not_on_roster")
    }

    fn map_point(&self, codes: [u16; 2]) -> Result<Vec3, &'static str> {
        let x = self.map.origin_x + f32::from(codes[0]) / f32::from(u16::MAX) * self.map.extent_m;
        let z = self.map.origin_z + f32::from(codes[1]) / f32::from(u16::MAX) * self.map.extent_m;
        (x.is_finite() && z.is_finite() && self.map.extent_m > 0.0)
            .then_some(Vec3::new(x, 0.0, z))
            .ok_or("comms.marker.invalid_position")
    }

    fn place(
        &mut self,
        author: Member,
        kind: MarkerKind,
        pos: Vec3,
        target: Option<u16>,
        now: Tick,
    ) -> Result<(), &'static str> {
        // The registry uses coordinates relative to the map origin so quantization and the
        // wire grid coincide even when the playable map has an offset.
        let relative = Vec3::new(pos.x - self.map.origin_x, 0.0, pos.z - self.map.origin_z);
        let cell = f32::from(MARKER_CELL_M);
        let max = f32::from(MAX_MARKER_CELL);
        if ![relative.x, relative.z]
            .into_iter()
            .all(|v| (0.0..=max).contains(&(v / cell).round()))
        {
            return Err("comms.marker.invalid_position");
        }
        self.registry
            .place(author, kind, relative, target, now)
            .map(|_| ())
            .map_err(marker_key)
    }

    /// Trusted match-host hook. No C0 message can report or spoof mutes.
    pub fn report_squad_mute(&mut self, world: &World, by: PlayerId, target: PlayerId) {
        if let (Ok(by), Ok(target)) = (self.author(world, by), self.author(world, target)) {
            self.registry.report_squad_mute(by, target, world.tick());
        }
    }

    pub fn forget(&mut self, player: PlayerId, scheduler: &mut SnapshotScheduler, now: Tick) {
        let mask = 1u128 << player;
        scheduler.clear_comms(player);
        self.members[usize::from(player)] = None;
        for item in self.published.values_mut() {
            item.viewers &= !mask;
        }
        for marker in self.registry.forget(player) {
            self.expire(marker.id, scheduler, now, mask);
        }
    }

    pub fn tick(&mut self, world: &World, clients: &[ClientIn], scheduler: &mut SnapshotScheduler) {
        let now = world.tick();
        let mut members = [None; PLAYER_SLOTS];
        for client in clients {
            members[usize::from(client.player)] =
                Member::from_roster(world.squads(), client.player)
                    .filter(|member| member.team == client.team);
        }
        let changed = members != self.members;
        let mut new_session = 0u128;
        if changed {
            for (p, (old, new)) in self.members.iter().zip(&members).enumerate() {
                if old.map(|m| m.joined) != new.map(|m| m.joined) {
                    new_session |= 1u128 << p;
                    scheduler.clear_comms(p as PlayerId);
                }
            }
            self.members = members;
            for marker in self.registry.reconcile(world.squads()) {
                self.expire(marker.id, scheduler, now, new_session);
            }
        }
        for marker in self.registry.tick(now) {
            self.expire(marker.id, scheduler, now, new_session);
        }
        let refresh_ticks = self.hz * u32::from(GameData::embedded().comms.enemy_refresh_s);
        // Spread moving observations across ticks, and only run LOS for marked vehicles.
        for (&id, item) in &self.published {
            if item.marker.kind != MarkerKind::TeamVehicle
                || now.0 % refresh_ticks != id % refresh_ticks
            {
                continue;
            }
            if let Some(pos) = item.marker.target.and_then(|target| {
                let key = ping::TargetId {
                    kind: ping::TargetKind::Vehicle,
                    id: target,
                };
                ping::observed_position(
                    world,
                    item.marker.author.player,
                    key,
                    optics_range(item.marker.author),
                )
            }) {
                let relative = Vec3::new(pos.x - self.map.origin_x, 0.0, pos.z - self.map.origin_z);
                let cell = f32::from(MARKER_CELL_M);
                if [relative.x, relative.z]
                    .into_iter()
                    .all(|v| (0.0..=f32::from(MAX_MARKER_CELL)).contains(&(v / cell).round()))
                {
                    let _ = self
                        .registry
                        .move_marker(item.marker.author, id, relative, now);
                }
            }
        }
        for marker in self.registry.take_due(now) {
            let audience = audience(&self.registry, &self.members, &marker, now);
            let wire_id = match self.published.get(&marker.id) {
                Some(item) => {
                    let removed = item.viewers & !audience & !new_session;
                    send(
                        scheduler,
                        removed,
                        Event::MarkerExpired { id: item.wire_id },
                        now,
                    );
                    item.wire_id
                }
                None => self.allocate_wire_id(),
            };
            self.published.insert(
                marker.id,
                Published {
                    marker,
                    viewers: audience,
                    wire_id,
                },
            );
            send(
                scheduler,
                audience,
                event(marker, wire_id, now, self.hz),
                now,
            );
        }
        let audience_changed = self.registry.take_audience_changed();
        if changed || audience_changed {
            for item in self.published.values_mut() {
                let audience = audience(&self.registry, &self.members, &item.marker, now);
                let old = item.viewers & !new_session;
                send(
                    scheduler,
                    old & !audience,
                    Event::MarkerExpired { id: item.wire_id },
                    now,
                );
                send(
                    scheduler,
                    audience & !old,
                    event(item.marker, item.wire_id, now, self.hz),
                    now,
                );
                item.viewers = audience;
            }
        }
    }

    fn expire(
        &mut self,
        id: MarkerId,
        scheduler: &mut SnapshotScheduler,
        now: Tick,
        new_session: u128,
    ) {
        if let Some(item) = self.published.remove(&id) {
            send(
                scheduler,
                item.viewers & !new_session,
                Event::MarkerExpired { id: item.wire_id },
                now,
            );
        }
    }

    fn allocate_wire_id(&mut self) -> u16 {
        loop {
            let id = self.next_wire_id;
            self.next_wire_id = id.wrapping_add(1);
            if !self.published.values().any(|m| m.wire_id == id) {
                return id;
            }
        }
    }

    /// Existing map UI consumes C3. Only project the original non-intelligence kinds there.
    pub fn legacy(&self, viewer: PlayerId, now: Tick) -> Vec<TeamMarker> {
        let Some(Some(viewer)) = self.members.get(usize::from(viewer)) else {
            return Vec::new();
        };
        self.registry
            .visible(*viewer, now)
            .filter_map(|marker| {
                let kind = match marker.kind {
                    MarkerKind::Attention => TeamMarkerKind::Attention,
                    MarkerKind::NeedMedic => TeamMarkerKind::NeedMedic,
                    MarkerKind::NeedAmmo => TeamMarkerKind::NeedAmmo,
                    MarkerKind::OrderAttack => TeamMarkerKind::OrderAttack,
                    MarkerKind::OrderDefend => TeamMarkerKind::OrderDefend,
                    MarkerKind::OrderMove => TeamMarkerKind::OrderMove,
                    _ => return None,
                };
                // Keep the legacy C3 floor projection for existing map consumers.
                let [x, z] = [marker.pos.x, marker.pos.z]
                    .map(|axis| (axis / f32::from(MARKER_CELL_M)).floor() as u16);
                Some(TeamMarker {
                    kind,
                    x,
                    z,
                    ttl_s: ttl(*marker, now, self.hz),
                })
            })
            .take(frontline_protocol::msg::team::MAX_TEAM_MARKERS)
            .collect()
    }
}

fn audience(
    registry: &MarkerRegistry,
    members: &[Option<Member>; PLAYER_SLOTS],
    marker: &Marker,
    now: Tick,
) -> u128 {
    members
        .iter()
        .flatten()
        .filter(|&&m| registry.visible_to(marker, m, now))
        .fold(0, |mask, m| mask | (1u128 << m.player))
}

fn send(scheduler: &mut SnapshotScheduler, mask: u128, event: Event, now: Tick) {
    if mask != 0 {
        scheduler.queue_comms(mask, event, now.0);
    }
}

fn cells(pos: Vec3) -> [u16; 2] {
    [pos.x, pos.z].map(|axis| (axis / f32::from(MARKER_CELL_M)).round() as u16)
}

fn ttl(marker: Marker, now: Tick, hz: u32) -> u8 {
    marker.expires_at.map_or(u8::MAX, |end| {
        (end.diff(now).max(1) as u32).div_ceil(hz).min(255) as u8
    })
}

fn event(marker: Marker, id: u16, now: Tick, hz: u32) -> Event {
    Event::MarkerUpsert {
        id,
        kind: wire_kind(marker.kind),
        pos: cells(marker.pos),
        ttl: ttl(marker, now, hz),
        owner: marker.author.player,
        target: (marker.kind == MarkerKind::Thanks)
            .then_some(marker.target)
            .flatten(),
    }
}

fn optics_range(author: Member) -> f32 {
    if author.role == Some("officer") {
        500.0
    } else {
        800.0
    }
}

fn team_mark_class(kind: MarkerKind) -> Option<ping::TargetKind> {
    use ping::TargetKind;
    Some(match kind {
        MarkerKind::TeamInfantry => TargetKind::Infantry,
        MarkerKind::TeamVehicle => TargetKind::Vehicle,
        MarkerKind::TeamArtillery => TargetKind::Artillery,
        MarkerKind::TeamAtGun => TargetKind::AntiTankGun,
        MarkerKind::TeamGarrison => TargetKind::Building,
        _ => return None,
    })
}

fn manual_kind(kind: TeamMarkerKind) -> MarkerKind {
    use TeamMarkerKind::*;
    match kind {
        Attention => MarkerKind::Attention,
        NeedMedic => MarkerKind::NeedMedic,
        NeedAmmo => MarkerKind::NeedAmmo,
        OrderMove => MarkerKind::OrderMove,
        OrderAttack => MarkerKind::OrderAttack,
        OrderDefend => MarkerKind::OrderDefend,
        OrderHold => MarkerKind::OrderHold,
        OrderBuild => MarkerKind::OrderBuild,
        MarkInfantry => MarkerKind::TeamInfantry,
        MarkVehicle => MarkerKind::TeamVehicle,
        MarkArtillery => MarkerKind::TeamArtillery,
        MarkAtGun => MarkerKind::TeamAtGun,
        MarkGarrison => MarkerKind::TeamGarrison,
        CommandAttack => MarkerKind::CommandAttack,
        CommandDefend => MarkerKind::CommandDefend,
        FireRequest => MarkerKind::FireRequest,
        GarrisonRequest => MarkerKind::GarrisonRequest,
        SupplyRequest => MarkerKind::SupplyRequest,
    }
}

fn wire_kind(kind: MarkerKind) -> WireKind {
    use MarkerKind::*;
    WireKind::Ping(match kind {
        Attention => PingKind::Attention,
        Enemy => PingKind::Enemy,
        NeedMedic => PingKind::NeedMedic,
        NeedAmmo => PingKind::NeedAmmo,
        NeedEngineer => PingKind::NeedEngineer,
        NeedAt => PingKind::NeedAt,
        NeedSupplies => PingKind::NeedSupplies,
        Thanks => PingKind::Thanks,
        other => {
            return WireKind::Manual(match other {
                OrderMove => TeamMarkerKind::OrderMove,
                OrderAttack => TeamMarkerKind::OrderAttack,
                OrderDefend => TeamMarkerKind::OrderDefend,
                OrderHold => TeamMarkerKind::OrderHold,
                OrderBuild => TeamMarkerKind::OrderBuild,
                TeamInfantry => TeamMarkerKind::MarkInfantry,
                TeamVehicle => TeamMarkerKind::MarkVehicle,
                TeamArtillery => TeamMarkerKind::MarkArtillery,
                TeamAtGun => TeamMarkerKind::MarkAtGun,
                TeamGarrison => TeamMarkerKind::MarkGarrison,
                CommandAttack => TeamMarkerKind::CommandAttack,
                CommandDefend => TeamMarkerKind::CommandDefend,
                FireRequest => TeamMarkerKind::FireRequest,
                GarrisonRequest => TeamMarkerKind::GarrisonRequest,
                SupplyRequest => TeamMarkerKind::SupplyRequest,
                _ => unreachable!("only supported kinds enter the registry"),
            })
        }
    })
}

fn marker_key(error: MarkerErr) -> &'static str {
    match error {
        MarkerErr::UnsupportedKind => "comms.marker.unsupported",
        MarkerErr::NeedSquad => "comms.marker.need_squad",
        MarkerErr::NotSquadLeader => "comms.marker.not_squad_leader",
        MarkerErr::RoleDenied => "comms.marker.role_denied",
        MarkerErr::Cooldown | MarkerErr::RefreshTooSoon => "comms.marker.cooldown",
        MarkerErr::PlayerCap | MarkerErr::TeamCap => "comms.marker.quota",
        MarkerErr::InvalidPosition => "comms.marker.invalid_position",
        MarkerErr::NotFound | MarkerErr::NotOwner => "comms.marker.unavailable",
    }
}
