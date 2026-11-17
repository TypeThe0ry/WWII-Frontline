//! Identifier types (ARCHITECTURE 8.1).

/// Player slot, `0..=127`.
pub type PlayerId = u8;
/// Highest valid [`PlayerId`].
pub const MAX_PLAYER_ID: PlayerId = 127;

/// Stable entity id: low 12 bits = slot, high 4 bits = generation.
pub type EntityId = u16;

pub const SLOT_BITS: u32 = 12;
pub const MAX_SLOTS: usize = 1 << SLOT_BITS;
pub const GEN_MASK: u8 = 0x0f;

#[inline]
pub fn entity_id(slot: u16, generation: u8) -> EntityId {
    debug_assert!((slot as usize) < MAX_SLOTS);
    ((generation & GEN_MASK) as u16) << SLOT_BITS | (slot & (MAX_SLOTS as u16 - 1))
}

#[inline]
pub fn entity_slot(id: EntityId) -> u16 {
    id & (MAX_SLOTS as u16 - 1)
}

#[inline]
pub fn entity_gen(id: EntityId) -> u8 {
    (id >> SLOT_BITS) as u8
}

/// Simulation tick with wrapping comparison (serial-number arithmetic).
///
/// Deliberately does not implement `Ord`: ordering is not transitive across the wrap point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Tick(pub u32);

impl Tick {
    /// Signed distance `self - other` (positive when `self` is later), wrapping.
    #[inline]
    pub fn diff(self, other: Tick) -> i32 {
        self.0.wrapping_sub(other.0) as i32
    }
    #[inline]
    pub fn lt(self, other: Tick) -> bool {
        self.diff(other) < 0
    }
    #[inline]
    pub fn le(self, other: Tick) -> bool {
        self.diff(other) <= 0
    }
    #[inline]
    pub fn next(self) -> Tick {
        Tick(self.0.wrapping_add(1))
    }
}

/// Slot allocator giving stable ids with a generation counter (4 bits, wraps).
///
/// Always hands out the lowest free slot, which keeps allocation deterministic.
#[derive(Debug, Clone, Default)]
pub struct EntityAllocator {
    /// Per-slot generation; `alive[slot]` tells whether the slot is in use.
    gens: Vec<u8>,
    alive: Vec<bool>,
}

impl EntityAllocator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocates the lowest free slot, or `None` when all `MAX_SLOTS` are in use.
    pub fn alloc(&mut self) -> Option<EntityId> {
        if let Some(slot) = self.alive.iter().position(|a| !*a) {
            self.alive[slot] = true;
            return Some(entity_id(slot as u16, self.gens[slot]));
        }
        if self.gens.len() >= MAX_SLOTS {
            return None;
        }
        self.gens.push(0);
        self.alive.push(true);
        Some(entity_id((self.gens.len() - 1) as u16, 0))
    }

    /// Whether `id` refers to a live slot of the same generation.
    pub fn is_alive(&self, id: EntityId) -> bool {
        let slot = entity_slot(id) as usize;
        slot < self.gens.len() && self.alive[slot] && self.gens[slot] == entity_gen(id)
    }

    /// Frees `id`'s slot and bumps its generation. Returns `false` for stale ids.
    pub fn free(&mut self, id: EntityId) -> bool {
        if !self.is_alive(id) {
            return false;
        }
        let slot = entity_slot(id) as usize;
        self.alive[slot] = false;
        self.gens[slot] = (self.gens[slot] + 1) & GEN_MASK;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_roundtrip() {
        let id = entity_id(0xabc, 0x5);
        assert_eq!(id, 0x5abc);
        assert_eq!(entity_slot(id), 0xabc);
        assert_eq!(entity_gen(id), 5);
    }

    #[test]
    fn tick_wraps() {
        let a = Tick(u32::MAX - 1);
        let b = Tick(3);
        assert!(a.lt(b));
        assert!(a.le(b));
        assert!(!b.lt(a));
        assert_eq!(b.diff(a), 5);
        assert_eq!(a.diff(b), -5);
        assert!(a.le(a) && !a.lt(a));
        assert_eq!(Tick(u32::MAX).next(), Tick(0));
    }

    #[test]
    fn slot_reuse_bumps_generation() {
        let mut al = EntityAllocator::new();
        let a = al.alloc().unwrap();
        let b = al.alloc().unwrap();
        assert_eq!((entity_slot(a), entity_gen(a)), (0, 0));
        assert_eq!(entity_slot(b), 1);
        assert!(al.free(a));
        assert!(!al.is_alive(a));
        assert!(!al.free(a), "double free rejected");
        let a2 = al.alloc().unwrap();
        assert_eq!(entity_slot(a2), 0);
        assert_eq!(entity_gen(a2), 1);
        assert!(!al.is_alive(a), "old id is stale");
        assert!(al.is_alive(a2));
    }

    #[test]
    fn generation_wraps_at_16() {
        let mut al = EntityAllocator::new();
        let mut id = al.alloc().unwrap();
        for _ in 0..16 {
            al.free(id);
            id = al.alloc().unwrap();
        }
        assert_eq!(entity_gen(id), 0);
    }
}
