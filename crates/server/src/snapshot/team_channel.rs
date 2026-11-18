//! Team channel C3 (ARCHITECTURE §6.9.2, GD §17.4, R3): once per `team_channel_period_s`
//! every client gets the whole team as 32-bit icons (`x:11 z:11 role:4 state:3 facing:3`),
//! so teammates beyond the 400 m snapshot range still appear on the map. Entry `i` belongs
//! to the `i`-th teammate in ascending `player_id` (the C4 roster order).

use frontline_protocol::msg::team::{encode_team_state, TeamMember, TeamState, TEAM_X_BITS};
use frontline_sim::{GameData, PlayerId, Team, Vec3, World};

/// State codes of [`TeamMember::state`].
pub const STATE_ABSENT: u8 = 0;
pub const STATE_ALIVE: u8 = 1;
pub const STATE_IN_VEHICLE: u8 = 2;

/// Map position code: `extent_m / 2048` metres per step, clamped to the grid.
pub fn quant_map(v: f32, origin: f32, extent_m: f32) -> u16 {
    let max = (1u32 << TEAM_X_BITS) - 1;
    (((v - origin) / extent_m * (max + 1) as f32)
        .floor()
        .max(0.0) as u32)
        .min(max) as u16
}

/// Inverse of [`quant_map`] (cell centre).
pub fn dequant_map(c: u16, origin: f32, extent_m: f32) -> f32 {
    origin + (f32::from(c) + 0.5) / (1u32 << TEAM_X_BITS) as f32 * extent_m
}

/// 8-way facing, 0 = +z, increasing clockwise seen from above.
pub fn facing8(yaw: f32) -> u8 {
    let t = yaw.rem_euclid(core::f32::consts::TAU) / core::f32::consts::TAU;
    ((t * 8.0 + 0.5).floor() as u8) & 7
}

#[derive(Debug, Clone, Copy)]
pub struct MapFrame {
    pub origin_x: f32,
    pub origin_z: f32,
    pub extent_m: f32,
}

/// Builds the team state of `team` at `tick`. Teammates without a body (not spawned / dead)
/// are `STATE_ABSENT`. Returns the message and the number of members dropped by the cap.
pub fn build(
    world: &World,
    data: &GameData,
    team: Team,
    tick: u32,
    map: MapFrame,
) -> (TeamState, usize) {
    let bodies = world.view().players();
    let mut members = Vec::new();
    for (p, e) in world.squads().roster() {
        if e.team != team {
            continue;
        }
        let role = e
            .role
            .and_then(|r| data.roles.iter().position(|d| d.id == r))
            .map_or(0, |i| (i as u8 + 1).min(15));
        let body = bodies.iter().find(|b| b.player == p);
        members.push(match body {
            Some(b) if e.dead_since.is_none() => TeamMember {
                x: quant_map(b.pos.x, map.origin_x, map.extent_m),
                z: quant_map(b.pos.z, map.origin_z, map.extent_m),
                role,
                state: if world.seat_of(p).is_some() {
                    STATE_IN_VEHICLE
                } else {
                    STATE_ALIVE
                },
                facing: facing8(b.yaw),
            },
            _ => TeamMember {
                x: 0,
                z: 0,
                role,
                state: STATE_ABSENT,
                facing: 0,
            },
        });
    }
    TeamState::from_members(tick as u16, &members)
}

/// Sends the team state to every client once per period.
#[derive(Debug, Clone)]
pub struct TeamChannel {
    period_ticks: u32,
    pub map: MapFrame,
    pub sent: u64,
}

impl TeamChannel {
    pub fn new(period_s: f32, tick_hz: f32, map: MapFrame) -> Self {
        TeamChannel {
            period_ticks: (period_s * tick_hz).round().max(1.0) as u32,
            map,
            sent: 0,
        }
    }

    pub fn due(&self, tick: u32) -> bool {
        tick.is_multiple_of(self.period_ticks)
    }

    /// Payloads for `clients` (player, team) when due; one encode per team.
    pub fn tick(
        &mut self,
        world: &World,
        data: &GameData,
        clients: &[(PlayerId, Team)],
        tick: u32,
    ) -> Vec<(PlayerId, Vec<u8>)> {
        if !self.due(tick) {
            return Vec::new();
        }
        let mut per_team: [Option<Vec<u8>>; 2] = [None, None];
        let mut out = Vec::with_capacity(clients.len());
        for &(p, team) in clients {
            let slot = &mut per_team[team as usize];
            if slot.is_none() {
                let (ts, _) = build(world, data, team, tick, self.map);
                *slot = encode_team_state(&ts).ok();
            }
            if let Some(b) = slot {
                out.push((p, b.clone()));
                self.sent += 1;
            }
        }
        out
    }
}

/// Unused helper kept for the HUD side: centre of a code as a world point (y = 0).
pub fn code_to_world(x: u16, z: u16, map: MapFrame) -> Vec3 {
    Vec3::new(
        dequant_map(x, map.origin_x, map.extent_m),
        0.0,
        dequant_map(z, map.origin_z, map.extent_m),
    )
}
