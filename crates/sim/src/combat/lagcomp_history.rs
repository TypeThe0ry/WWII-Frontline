//! Hitbox history ring for lag compensation (ARCHITECTURE 6.7.1 / 8.1). One frame per tick holds
//! every player's pose (feet, stance blend, head offset); the four capsules are rebuilt from it by
//! [`crate::weapon::hitbox::volumes`]. The rewind logic is M1-025; this is the data structure.

use glam::Vec3;

use crate::ids::{EntityId, Tick};

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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct HitboxFrame {
    pub tick: Tick,
    pub players: Vec<PlayerHitboxes>,
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
        let f = &mut self.frames[self.head];
        f.tick = tick;
        f.players.clear();
        f.players.extend_from_slice(players);
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
        self.frames[..]
            .iter()
            .take(HISTORY_TICKS)
            .enumerate()
            .filter(|(i, _)| self.is_live(*i))
            .map(|(_, f)| f)
            .find(|f| f.tick == tick)
    }

    fn is_live(&self, idx: usize) -> bool {
        let back = (self.head + HISTORY_TICKS - 1 - idx) % HISTORY_TICKS;
        back < self.len
    }

    /// Hitboxes at `tick + frac` (`frac` in `0..1`), linearly interpolated between two frames;
    /// players missing from the later frame are dropped, players new in it are taken as is.
    /// `None` when `tick` is no longer (or not yet) in the ring.
    pub fn hitboxes_at(&self, tick: Tick, frac: f32) -> Option<Vec<PlayerHitboxes>> {
        let a = self.frame(tick)?;
        let Some(b) = self.frame(Tick(tick.0 + 1)).filter(|_| frac > 0.0) else {
            return Some(a.players.clone());
        };
        let f = frac.clamp(0.0, 1.0);
        let lerp = |x: f32, y: f32| x + (y - x) * f;
        Some(
            b.players
                .iter()
                .map(|pb| match a.players.iter().find(|pa| pa.id == pb.id) {
                    Some(pa) => PlayerHitboxes {
                        id: pb.id,
                        foot: pa.foot.lerp(pb.foot, f),
                        blend: lerp(pa.blend, pb.blend),
                        head_dx: lerp(pa.head_dx, pb.head_dx),
                        head_dz: lerp(pa.head_dz, pb.head_dz),
                    },
                    None => *pb,
                })
                .collect(),
        )
    }
}
