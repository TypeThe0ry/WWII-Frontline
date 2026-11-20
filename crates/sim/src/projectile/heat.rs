//! HEAT flight integration for infantry launchers (GAMEDESIGN 6.9.4, 14.4).

use glam::Vec3;

use crate::vehicle::armor::{self, ArmorResult, ArmorSteps, ShellParams};

pub const GRAVITY_MPS2: f32 = 9.81;
pub const SUBSTEPS_PER_TICK: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeatProjectile {
    pub origin: Vec3,
    pub position: Vec3,
    pub velocity: Vec3,
    pub distance_m: f32,
    pub thrust_m: f32,
    pub max_range_m: f32,
    pub alive: bool,
}

impl HeatProjectile {
    pub fn new(origin: Vec3, velocity: Vec3, thrust_m: f32, max_range_m: f32) -> Self {
        Self {
            origin,
            position: origin,
            velocity,
            distance_m: 0.0,
            thrust_m,
            max_range_m,
            alive: true,
        }
    }

    /// Integrates one fixed simulation tick, returning ordered swept segments for collision.
    pub fn step(&mut self, dt: f32) -> [(Vec3, Vec3); SUBSTEPS_PER_TICK as usize] {
        let mut segments = [(self.position, self.position); SUBSTEPS_PER_TICK as usize];
        let h = dt / SUBSTEPS_PER_TICK as f32;
        for segment in &mut segments {
            let start = self.position;
            if self.distance_m >= self.thrust_m {
                self.velocity.y -= GRAVITY_MPS2 * h;
            }
            self.position += self.velocity * h;
            self.distance_m += start.distance(self.position);
            *segment = (start, self.position);
            if self.distance_m >= self.max_range_m {
                self.alive = false;
                break;
            }
        }
        segments
    }
}

pub fn resolve_heat_hit(
    params: &ShellParams,
    distance_m: f32,
    normal: Vec3,
    velocity: Vec3,
    thickness_mm: f32,
    penetration_factor: f32,
    skirt_multiplier: f32,
) -> ArmorSteps {
    let mut adjusted = *params;
    adjusted.kind = armor::ShellKind::Heat;
    adjusted.drop_pct_per_100m = 0.0;
    adjusted.p100_mm *= skirt_multiplier;
    armor::armor_hit_steps(
        &adjusted,
        distance_m,
        normal,
        velocity.normalize_or_zero(),
        thickness_mm,
        penetration_factor,
    )
}

pub fn penetrated(result: ArmorResult) -> bool {
    matches!(result, ArmorResult::Pen { .. })
}

pub fn penetration_damage(structure_damage_hp: f32, u: f32) -> f32 {
    structure_damage_hp * (0.9 + 0.2 * u.clamp(0.0, 1.0))
}

pub fn internal_length_m(remaining_pen_mm: f32) -> f32 {
    crate::vehicle::internal::internal_len(remaining_pen_mm)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(pen: f32) -> ShellParams {
        ShellParams {
            kind: armor::ShellKind::Heat,
            caliber_mm: 149.0,
            p100_mm: pen,
            drop_pct_per_100m: 0.0,
            structure_damage_hp: 900.0,
        }
    }

    #[test]
    fn thrust_segment_is_flat_then_gravity_applies() {
        let mut p = HeatProjectile::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 82.0), 20.0, 200.0);
        for _ in 0..7 {
            p.step(1.0 / 30.0);
        }
        assert_eq!(p.position.y, 0.0);
        for _ in 0..2 {
            p.step(1.0 / 30.0);
        }
        assert!(p.position.y < 0.0);
    }

    #[test]
    fn manual_heat_vectors_match_design_and_overmatch() {
        assert!(penetrated(
            resolve_heat_hit(&params(90.0), 0.0, -Vec3::X, Vec3::X, 80.0, 1.0, 1.0).result
        ));
        let n9 = Vec3::new(
            -libm::sinf(9f32.to_radians()),
            0.0,
            -libm::cosf(9f32.to_radians()),
        );
        let front = resolve_heat_hit(&params(90.0), 0.0, n9, Vec3::Z, 100.0, 1.0, 1.0);
        assert!(matches!(front.result, ArmorResult::NoPen { .. }));
        assert!((front.t_eff - 100.0 / libm::cosf(9f32.to_radians())).abs() < 0.01);
        let n47 = Vec3::new(
            -libm::sinf(47f32.to_radians()),
            0.0,
            -libm::cosf(47f32.to_radians()),
        );
        assert!(penetrated(
            resolve_heat_hit(&params(200.0), 0.0, n47, Vec3::Z, 101.0, 1.0, 1.0).result
        ));
        let jumbo = resolve_heat_hit(&params(200.0), 0.0, n47, Vec3::Z, 101.0, 1.0, 1.0);
        assert!((jumbo.t_eff - 101.0 / libm::cosf(47f32.to_radians())).abs() < 0.01);
        assert!(penetrated(
            resolve_heat_hit(&params(200.0), 0.0, -Vec3::X, Vec3::X, 30.0, 1.0, 0.6).result
        ));
    }

    #[test]
    fn penetration_damage_and_internal_length_follow_design_equations() {
        assert_eq!(penetration_damage(600.0, 0.0), 540.0);
        assert_eq!(penetration_damage(600.0, 0.5), 600.0);
        assert_eq!(penetration_damage(600.0, 1.0), 660.0);
        assert_eq!(internal_length_m(40.0), 1.6);
    }

    #[test]
    fn heat_ricochet_threshold_and_overmatch() {
        let dir = Vec3::X;
        let normal = Vec3::new(
            -libm::cosf(76f32.to_radians()),
            -libm::sinf(76f32.to_radians()),
            0.0,
        );
        assert_eq!(
            resolve_heat_hit(&params(200.0), 0.0, normal, dir, 80.0, 1.0, 1.0).result,
            ArmorResult::Ricochet
        );
        let normal = Vec3::new(
            -libm::cosf(74f32.to_radians()),
            -libm::sinf(74f32.to_radians()),
            0.0,
        );
        assert!(!matches!(
            resolve_heat_hit(&params(200.0), 0.0, normal, dir, 80.0, 1.0, 1.0).result,
            ArmorResult::Ricochet
        ));
        assert!(!matches!(
            resolve_heat_hit(&params(200.0), 0.0, normal, dir, 60.0, 1.0, 1.0).result,
            ArmorResult::Ricochet
        ));
    }
}
