//! Collision data placeholder (filled by M0-010).

#[derive(Debug, Clone, Default)]
pub struct CollisionData {}

impl CollisionData {
    /// Empty shell: no geometry.
    pub fn empty() -> Self {
        CollisionData {}
    }
}
