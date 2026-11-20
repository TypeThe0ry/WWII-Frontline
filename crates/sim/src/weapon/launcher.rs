//! Deterministic infantry launcher state and back-blast rules (GAMEDESIGN 6.9.4).

use glam::Vec3;

use crate::data::{GameData, OrdnanceRulesDef};
use crate::ids::Tick;

pub const TICK_HZ: u32 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherKind {
    Bazooka,
    Panzerschreck,
    Panzerfaust,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LauncherParams {
    pub muzzle_velocity_mps: f32,
    pub effective_range_m: f32,
    pub max_range_m: f32,
    pub heat_pen_mm: f32,
    pub caliber_mm: f32,
    pub reload_ticks: u16,
    pub loader_reload_ticks: u16,
    pub thrust_m: f32,
    pub backblast_m: f32,
    pub backblast_half_angle_deg: f32,
    pub backblast_damage_hp: f32,
    pub backblast_duration_ticks: u16,
    pub structure_damage_hp: f32,
    pub infantry_damage_hp: f32,
    pub blast_radius_m: f32,
    pub single_use: bool,
}

impl LauncherKind {
    pub fn params(self, rules: &OrdnanceRulesDef) -> LauncherParams {
        let def = match self {
            Self::Bazooka => &rules.launchers.bazooka,
            Self::Panzerschreck => &rules.launchers.panzerschreck,
            Self::Panzerfaust => &rules.launchers.panzerfaust,
        };
        let ticks = |seconds: f64| libm::ceilf((seconds * f64::from(TICK_HZ)) as f32 - 1e-4) as u16;
        LauncherParams {
            muzzle_velocity_mps: def.muzzle_velocity_mps as f32,
            effective_range_m: def.effective_range_m as f32,
            max_range_m: def.max_range_m as f32,
            heat_pen_mm: def.heat_pen_mm as f32,
            caliber_mm: def.caliber_mm as f32,
            reload_ticks: ticks(def.reload_s),
            loader_reload_ticks: ticks(def.loader_reload_s),
            thrust_m: def.thrust_m as f32,
            backblast_m: def.backblast_m as f32,
            backblast_half_angle_deg: def.backblast_half_angle_deg as f32,
            backblast_damage_hp: def.backblast_damage_hp as f32,
            backblast_duration_ticks: ticks(def.backblast_duration_s).max(1),
            structure_damage_hp: def.structure_damage_hp as f32,
            infantry_damage_hp: def.infantry_damage_hp as f32,
            blast_radius_m: def.blast_radius_m as f32,
            single_use: def.single_use,
        }
    }
}

impl LauncherParams {
    pub fn from_data(data: &GameData, kind: LauncherKind) -> Self {
        let _ = data;
        kind.params(GameData::ordnance())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireError {
    Empty,
    Reloading,
}

/// Per-launcher ammunition and reload clock. The clock is advanced by simulation ticks only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LauncherState {
    pub ammunition: u8,
    pub ready_at: Tick,
}

impl LauncherState {
    pub const fn new(ammunition: u8, now: Tick) -> Self {
        Self {
            ammunition,
            ready_at: now,
        }
    }

    pub fn fire(
        &mut self,
        kind: LauncherKind,
        now: Tick,
        has_loader: bool,
    ) -> Result<(), FireError> {
        let params = LauncherParams::from_data(GameData::embedded(), kind);
        if self.ammunition == 0 {
            return Err(FireError::Empty);
        }
        if now.lt(self.ready_at) {
            return Err(FireError::Reloading);
        }
        self.ammunition -= 1;
        let reload = if has_loader {
            params.loader_reload_ticks
        } else {
            params.reload_ticks
        };
        self.ready_at = Tick(now.0.wrapping_add(u32::from(reload)));
        Ok(())
    }
}

/// Whether a target is inside the horizontal rear cone. A wall within 5 m returns the
/// reflected-damage flag for the shooter; callers perform the actual collision raycast.
pub fn backblast_hit(
    shooter: Vec3,
    forward: Vec3,
    target: Vec3,
    params: LauncherParams,
    blocked_by_wall: bool,
) -> Option<f32> {
    if blocked_by_wall {
        return Some(40.0);
    }
    let delta = target - shooter;
    let flat = Vec3::new(delta.x, 0.0, delta.z);
    let len = flat.length();
    if len <= 1e-5 || len > params.backblast_m {
        return None;
    }
    let back = -Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let min_dot = libm::cosf(params.backblast_half_angle_deg.to_radians());
    if (flat / len).dot(back) < min_dot {
        return None;
    }
    let duration_s = f32::from(params.backblast_duration_ticks) / TICK_HZ as f32;
    let damage = if params.backblast_duration_ticks > 1 {
        params.backblast_damage_hp * duration_s
    } else {
        params.backblast_damage_hp
    };
    Some(damage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_and_loader_use_sim_ticks() {
        let mut state = LauncherState::new(2, Tick(0));
        state.fire(LauncherKind::Bazooka, Tick(0), false).unwrap();
        assert_eq!(state.ready_at, Tick(135));
        assert_eq!(
            state.fire(LauncherKind::Bazooka, Tick(134), false),
            Err(FireError::Reloading)
        );
        state.fire(LauncherKind::Bazooka, Tick(135), true).unwrap();
        assert_eq!(state.ready_at, Tick(210));
    }

    #[test]
    fn backblast_has_range_and_angle_and_wall_reflection() {
        let p = LauncherParams::from_data(GameData::embedded(), LauncherKind::Bazooka);
        let origin = Vec3::ZERO;
        let forward = Vec3::Z;
        let at_11m = Vec3::new(
            11.0 * libm::sinf(10f32.to_radians()),
            0.0,
            -11.0 * libm::cosf(10f32.to_radians()),
        );
        assert_eq!(backblast_hit(origin, forward, at_11m, p, false), Some(18.0));
        assert_eq!(
            backblast_hit(origin, forward, Vec3::new(0.0, 0.0, -13.0), p, false),
            None
        );
        assert_eq!(
            backblast_hit(origin, forward, Vec3::new(0.0, 0.0, -4.0), p, true),
            Some(40.0)
        );
    }
}
