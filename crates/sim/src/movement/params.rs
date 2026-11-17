//! Movement parameters, built from a [`RoleDef`] (all numbers come from `/data/roles`).

use crate::data::RoleDef;

/// Why a role cannot be used for movement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamsError {
    /// A field the movement code needs is absent from the role file.
    Missing(&'static str),
    /// A field is non-finite or out of range.
    Invalid(&'static str),
}

impl core::fmt::Display for ParamsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParamsError::Missing(k) => write!(f, "role is missing `{k}`"),
            ParamsError::Invalid(k) => write!(f, "role field `{k}` is invalid"),
        }
    }
}

impl std::error::Error for ParamsError {}

/// Everything `step_player` needs, in SI units and `f32`. Plain data, `Copy`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveParams {
    /// Tick length in seconds (`1 / tick_hz`).
    pub dt: f32,
    /// Ticks per second as `f32` (exact for integer rates; avoids dividing by `dt`).
    pub hz: f32,
    pub walk_mps: f32,
    pub run_mps: f32,
    pub sprint_mps: f32,
    /// Crouched, default speed (GAMEDESIGN 9.1 "蹲姿移动 2.4 (Run)").
    pub crouch_run_mps: f32,
    /// Crouched with the walk modifier ("1.5 (Walk)").
    pub crouch_walk_mps: f32,
    /// `sqrt(2 g h)` for `jump_height_m`.
    pub jump_speed_mps: f32,
    pub gravity_mps2: f32,
    /// Fraction of the gap to the wish velocity closed per tick while airborne.
    pub air_control: f32,
    /// Stand <-> crouch transition length in (fractional) ticks: `ms * hz / 1000`.
    pub stance_ticks: f32,
    pub radius_m: f32,
    pub height_stand_m: f32,
    pub height_crouch_m: f32,
    pub max_slope_rad: f32,
    pub autostep_max_m: f32,
    pub autostep_min_width_m: f32,
    pub snap_to_ground_m: f32,
}

impl MoveParams {
    /// Builds the parameters of `role` for a `tick_hz` simulation.
    pub fn from_role(role: &RoleDef, tick_hz: u8) -> Result<Self, ParamsError> {
        if tick_hz == 0 {
            return Err(ParamsError::Invalid("tick_hz"));
        }
        let m = &role.movement;
        let cap = role
            .capsule
            .as_ref()
            .ok_or(ParamsError::Missing("capsule"))?;
        let need = |v: Option<f64>, k: &'static str| v.ok_or(ParamsError::Missing(k));
        let pos = |v: f64, k: &'static str| {
            if v.is_finite() && v > 0.0 {
                Ok(v as f32)
            } else {
                Err(ParamsError::Invalid(k))
            }
        };
        let nonneg = |v: f64, k: &'static str| {
            if v.is_finite() && v >= 0.0 {
                Ok(v as f32)
            } else {
                Err(ParamsError::Invalid(k))
            }
        };
        let hz = f32::from(tick_hz);
        let gravity = pos(
            need(m.gravity_mps2, "movement.gravity_mps2")?,
            "movement.gravity_mps2",
        )?;
        let jump_h = pos(
            need(m.jump_height_m, "movement.jump_height_m")?,
            "movement.jump_height_m",
        )?;
        let stance_ms = pos(
            need(m.stance_transition_ms, "movement.stance_transition_ms")?,
            "movement.stance_transition_ms",
        )?;
        let air = nonneg(
            need(m.air_control_mult, "movement.air_control_mult")?,
            "movement.air_control_mult",
        )?
        .min(1.0);
        let slope_deg = pos(
            need(m.max_slope_deg, "movement.max_slope_deg")?,
            "movement.max_slope_deg",
        )?;
        if slope_deg >= 90.0 {
            return Err(ParamsError::Invalid("movement.max_slope_deg"));
        }
        let r = pos(cap.radius_m, "capsule.radius_m")?;
        let hs = pos(cap.height_stand_m, "capsule.height_stand_m")?;
        let hc = pos(cap.height_crouch_m, "capsule.height_crouch_m")?;
        // The capsule needs a non-negative cylinder part: height >= 2 r.
        if hc < 2.0 * r || hs < hc {
            return Err(ParamsError::Invalid("capsule"));
        }
        Ok(Self {
            dt: 1.0 / hz,
            hz,
            walk_mps: pos(m.walk_mps, "movement.walk_mps")?,
            run_mps: pos(m.run_mps, "movement.run_mps")?,
            sprint_mps: pos(m.sprint_mps, "movement.sprint_mps")?,
            crouch_run_mps: pos(m.crouch_mps, "movement.crouch_mps")?,
            crouch_walk_mps: pos(
                need(m.crouch_walk_mps, "movement.crouch_walk_mps")?,
                "movement.crouch_walk_mps",
            )?,
            jump_speed_mps: libm::sqrtf(2.0 * gravity * jump_h),
            gravity_mps2: gravity,
            air_control: air,
            stance_ticks: stance_ms * hz / 1000.0,
            radius_m: r,
            height_stand_m: hs,
            height_crouch_m: hc,
            max_slope_rad: slope_deg.to_radians(),
            autostep_max_m: nonneg(
                need(m.autostep_max_m, "movement.autostep_max_m")?,
                "movement.autostep_max_m",
            )?,
            autostep_min_width_m: nonneg(
                need(m.autostep_min_width_m, "movement.autostep_min_width_m")?,
                "movement.autostep_min_width_m",
            )?,
            snap_to_ground_m: nonneg(
                need(m.snap_to_ground_m, "movement.snap_to_ground_m")?,
                "movement.snap_to_ground_m",
            )?,
        })
    }
}
