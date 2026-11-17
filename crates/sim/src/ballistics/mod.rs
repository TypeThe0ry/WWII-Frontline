//! Ballistics refinement (M1-012; GAMEDESIGN 6.4, 6.7): per-ammunition drag, material
//! penetration and a minimal ricochet. Pure functions of their arguments; the only randomness
//! is the exit deflection, drawn from a caller-provided deterministic `Pcg64Mcg`.
//!
//! * [`integrate`]: the 4-sub-step flight of one tick ([`ProjectileStep`]).
//! * [`materials`]: the surface material table from `data/materials/`.
//! * [`penetration`]: [`penetration::trace_with_penetration`] walks a shot through at most
//!   three segments (two penetrations) over the layers a [`penetration::LayerWorld`] reports.
//! * [`ricochet`]: shallow hits on hard materials bounce with 40 % of the energy.
//!
//! The collision world has only terrain today, so the layer source is a trait; M1 greybox buildings
//! and props plug into it when their colliders exist (PLAN_ISSUES PI-050).

pub mod integrate;
pub mod materials;
pub mod penetration;
pub mod ricochet;

pub use integrate::{step_tick, ProjectileStep};
pub use materials::{Material, MaterialId, MaterialTable};
pub use penetration::{
    trace_with_penetration, Layer, LayerWorld, PenAmmo, PenResult, TraceSegment, MAX_PENETRATIONS,
    MAX_SEGMENTS,
};
