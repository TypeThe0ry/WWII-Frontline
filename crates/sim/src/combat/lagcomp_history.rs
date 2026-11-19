//! Hitbox history ring for lag compensation (ARCHITECTURE 6.7.1 / 8.1). One frame per tick holds
//! every player's pose (feet, stance blend, head offset); the four capsules are rebuilt from it by
//! [`crate::weapon::hitbox::volumes`]. The rewind logic is M1-025; this is the data structure.

use glam::{Quat, Vec3};

use crate::ids::{EntityId, Tick};
use crate::vehicle::{shell, Vehicle};

/// Frames kept (about half a second at 30 Hz).
pub const HISTORY_TICKS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerHitboxes {
    pub id: EntityId,
    pub foot: Vec3,
    /// Stance coordinate `0 stand .. 1 crouch .. 2 prone`.
    pub blend: f32,
    /// Horizontal head offset from leaning (metres, world x / z).
    pub head_dx: f32,
    pub head_dz: f32,
}

impl PlayerHitboxes {
    pub fn lerp(self, other: Self, frac: f32) -> Self {
        debug_assert_eq!(self.id, other.id);
        Self {
            id: self.id,
            foot: self.foot.lerp(other.foot, frac),
            blend: self.blend + (other.blend - self.blend) * frac,
            head_dx: self.head_dx + (other.head_dx - self.head_dx) * frac,
            head_dz: self.head_dz + (other.head_dz - self.head_dz) * frac,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientedBox {
    pub center: Vec3,
    pub rotation: Quat,
    pub half_extents: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleHitboxes {
    pub id: EntityId,
    pub hull: OrientedBox,
    pub turret: OrientedBox,
}

impl From<&Vehicle> for VehicleHitboxes {
    fn from(vehicle: &Vehicle) -> Self {
        let half = vehicle.params.half_extents;
        // Match the placeholder hull/turret split in vehicle::shell::locate.
        let roof = shell::TURRET_BASE_Y;
        Self {
            id: vehicle.id,
            hull: OrientedBox {
                center: vehicle.pos + vehicle.rot * Vec3::Y * (0.5 * (roof - half.y)),
                rotation: vehicle.rot,
                half_extents: Vec3::new(half.x, 0.5 * (roof + half.y), half.z),
            },
            turret: OrientedBox {
                center: vehicle.pos + vehicle.rot * Vec3::Y * (0.5 * (roof + half.y)),
                rotation: vehicle.rot * Quat::from_rotation_y(vehicle.turret.yaw),
                half_extents: Vec3::new(
                    shell::TURRET_HALF_XZ.0,
                    0.5 * (half.y - roof),
                    shell::TURRET_HALF_XZ.1,
                ),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct HitboxFrame {
    pub tick: Tick,
    pub players: Vec<PlayerHitboxes>,
    pub vehicles: Vec<VehicleHitboxes>,
}

impl HitboxFrame {
    pub fn player(&self, id: EntityId) -> Option<PlayerHitboxes> {
        self.players
            .binary_search_by_key(&id, |p| p.id)
            .ok()
            .map(|i| self.players[i])
    }
}

#[derive(Debug, Clone)]
pub struct LagCompHistory {
    frames: Vec<HitboxFrame>,
    /// Index the next frame is written to.
    head: usize,
    len: usize,
}

impl Default for LagCompHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl LagCompHistory {
    pub fn new() -> Self {
        Self {
            frames: vec![HitboxFrame::default(); HISTORY_TICKS],
            head: 0,
            len: 0,
        }
    }

    /// Writes the frame of `tick`, replacing the oldest once the ring is full. Frames are
    /// recorded in ascending tick order; players are expected ascending by id.
    pub fn record(&mut self, tick: Tick, players: &[PlayerHitboxes]) {
        self.record_frame(tick, players.iter().copied(), std::iter::empty());
    }

    /// Reuse the ring's buffers. Both inputs are in ascending entity-id order.
    pub fn record_frame(
        &mut self,
        tick: Tick,
        players: impl IntoIterator<Item = PlayerHitboxes>,
        vehicles: impl IntoIterator<Item = VehicleHitboxes>,
    ) {
        let f = &mut self.frames[self.head];
        f.tick = tick;
        f.players.clear();
        f.players.extend(players);
        f.vehicles.clear();
        f.vehicles.extend(vehicles);
        self.head = (self.head + 1) % HISTORY_TICKS;
        self.len = (self.len + 1).min(HISTORY_TICKS);
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn frame(&self, tick: Tick) -> Option<&HitboxFrame> {
        // Most frames are consecutive. Index those directly, with a scan for sparse recordings.
        let newest = self.newest_first().next()?;
        let back = usize::try_from(newest.tick.diff(tick)).ok()?;
        if back < self.len {
            let candidate = &self.frames[(self.head + HISTORY_TICKS - 1 - back) % HISTORY_TICKS];
            if candidate.tick == tick {
                return Some(candidate);
            }
        }
        self.newest_first().find(|f| f.tick == tick)
    }

    /// Exact frame or nearest retained tick, preferring the newer frame on a tie.
    pub fn frame_clamped(&self, tick: Tick) -> Option<(&HitboxFrame, bool)> {
        if let Some(frame) = self.frame(tick) {
            return Some((frame, false));
        }
        self.newest_first()
            .min_by_key(|f| f.tick.diff(tick).unsigned_abs())
            .map(|f| (f, true))
    }

    fn newest_first(&self) -> impl Iterator<Item = &HitboxFrame> {
        (0..self.len)
            .map(|back| &self.frames[(self.head + HISTORY_TICKS - 1 - back) % HISTORY_TICKS])
    }

    /// Hitboxes at `tick + frac` (`frac` in `0..1`), linearly interpolated between two frames;
    /// players missing from the later frame are dropped, players new in it are taken as is.
    /// `None` when `tick` is no longer (or not yet) in the ring.
    pub fn hitboxes_at(&self, tick: Tick, frac: f32) -> Option<Vec<PlayerHitboxes>> {
        let a = self.frame(tick)?;
        let Some(b) = self.frame(tick.next()).filter(|_| frac > 0.0) else {
            return Some(a.players.clone());
        };
        let f = frac.clamp(0.0, 1.0);
        Some(
            b.players
                .iter()
                .map(|pb| a.player(pb.id).map_or(*pb, |pa| pa.lerp(*pb, f)))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose(x: f32) -> PlayerHitboxes {
        PlayerHitboxes {
            id: 3,
            foot: Vec3::X * x,
            blend: x * 0.5,
            head_dx: x * 0.25,
            head_dz: -x * 0.25,
        }
    }

    #[test]
    fn interpolation_and_clamped_queries_across_tick_wrap() {
        let mut history = LagCompHistory::new();
        assert!(history.frame_clamped(Tick(0)).is_none());
        history.record(Tick(u32::MAX), &[pose(0.0)]);
        history.record(Tick(0), &[pose(2.0)]);
        assert_eq!(
            history.hitboxes_at(Tick(u32::MAX), 0.5),
            Some(vec![pose(1.0)])
        );
        for tick in 1..HISTORY_TICKS as u32 {
            history.record(Tick(tick), &[pose(2.0)]);
        }
        assert_eq!(history.len(), HISTORY_TICKS);
        assert!(history.frame(Tick(u32::MAX)).is_none());
        let select = |tick| {
            history
                .frame_clamped(tick)
                .map(|(f, clamped)| (f.tick, clamped))
        };
        assert_eq!(select(Tick(u32::MAX)), Some((Tick(0), true)));
        assert_eq!(select(Tick(0)), Some((Tick(0), false)));
        assert_eq!(select(Tick(16)), Some((Tick(15), true)));
    }

    #[test]
    fn sparse_history_prefers_the_newer_frame_on_ties() {
        let mut history = LagCompHistory::new();
        for tick in [1, 3, 5] {
            history.record(Tick(tick), &[]);
        }
        assert_eq!(history.frame(Tick(1)).unwrap().tick, Tick(1));
        assert!(history.frame(Tick(2)).is_none());
        let (nearest, clamped) = history.frame_clamped(Tick(2)).unwrap();
        assert_eq!((nearest.tick, clamped), (Tick(3), true));
    }
}
