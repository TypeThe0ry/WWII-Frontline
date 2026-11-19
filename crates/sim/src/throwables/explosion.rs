//! Blast damage `D = Dmax x (1 - (r / R)^2) x occlusion` (GAMEDESIGN 6.9.1, 6.6).

/// How much of the blast reaches the victim through what lies between (GAMEDESIGN 6.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Occlusion {
    Clear,
    /// Thin wood or canvas.
    Thin,
    /// Solid cover: terrain, masonry, steel.
    Solid,
}

impl Occlusion {
    pub fn mult(self) -> f32 {
        match self {
            Occlusion::Clear => 1.0,
            Occlusion::Thin => 0.6,
            Occlusion::Solid => 0.25,
        }
    }
}

/// Friendly-fire coefficient of explosions on teammates (GAMEDESIGN 6.6; the owner takes 1.0).
pub const FF_MULT: f32 = 0.7;
/// Lift the blast ray off the terrain at the resting projectile's position.
pub(crate) const RAY_LIFT_M: f32 = 0.05;

/// Damage at distance `r` from the blast centre; zero at and beyond `big_r`.
pub fn explosion_damage(dmax: f32, r: f32, big_r: f32, occlusion: Occlusion) -> f32 {
    if r >= big_r || big_r <= 0.0 {
        return 0.0;
    }
    let q = r / big_r;
    dmax * (1.0 - q * q) * occlusion.mult()
}
