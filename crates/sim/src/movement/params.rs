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
    /// Ground acceleration time (s): rest -> target speed, linear. `0` = instantaneous.
    pub accel_time_s: f32,
    /// Ground deceleration time (s): reference speed -> stop, linear. `0` = instantaneous.
    pub decel_time_s: f32,
    /// Eye height above the feet when standing / crouched (muzzle origin, M0-012).
    pub eye_height_stand_m: f32,
    pub eye_height_crouch_m: f32,
    /// Prone capsule and eye height (M1-011). The capsule is at least `2 * radius` tall, so it
    /// is taller than the 0.5 m prone *hit* height (`hitbox_height_m[2]`).
    pub height_prone_m: f32,
    pub eye_height_prone_m: f32,
    /// Prone speeds: slow (`aux` walk / default) and fast (SPRINT held, costs stamina).
    pub prone_mps: f32,
    pub prone_fast_mps: f32,
    pub prone_fast_cost_per_s: f32,
    /// Stance transition lengths in ticks (fractional): crouch->prone, prone->crouch,
    /// stand->prone (dive), prone->stand. Stand <-> crouch is `stance_ticks`.
    pub ticks_c2p: f32,
    pub ticks_p2c: f32,
    pub ticks_s2p: f32,
    pub ticks_p2s: f32,
    /// Aim-down-sights speed multipliers by stance, and while reloading.
    pub ads_mult: [f32; 3],
    pub reload_move_mult: f32,
    /// Downed crawl speed (used by M1-016).
    pub crawl_mps: f32,
    /// Load modifier: `clamp(1 - penalty * (load - ref), min, 1)`.
    pub load_ref_kg: f32,
    pub load_penalty_per_kg: f32,
    pub load_min_mult: f32,
    pub stamina_max: f32,
    pub sprint_cost_per_s: f32,
    pub stamina_regen_per_s: f32,
    /// Regeneration starts this many ticks after the last spend.
    pub stamina_regen_delay_ticks: u8,
    pub stamina_regen_mult: [f32; 3],
    pub stamina_low: f32,
    pub stamina_unlock: f32,
    pub jump_cost: f32,
    /// Stance tables of GAMEDESIGN 9.5, indexed by `Stance as usize`.
    pub recoil_mult: [f32; 3],
    pub spread_mult: [f32; 3],
    pub visibility_mult: [f32; 3],
    pub hitbox_height_m: [f32; 3],
    pub move_spread_walk_mult: f32,
    pub move_spread_run_mult: f32,
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
        let eye_s = pos(
            need(cap.eye_height_stand_m, "capsule.eye_height_stand_m")?,
            "capsule.eye_height_stand_m",
        )?;
        let eye_c = pos(
            need(cap.eye_height_crouch_m, "capsule.eye_height_crouch_m")?,
            "capsule.eye_height_crouch_m",
        )?;
        if eye_s > hs || eye_c > hc {
            return Err(ParamsError::Invalid("capsule.eye_height"));
        }
        // Absent ramp times keep the pre-M0-012 behaviour (instantaneous), so older data works.
        let ramp = |v: Option<f64>, k: &'static str| match v {
            Some(v) => nonneg(v, k),
            None => Ok(0.0),
        };
        let req = |v: Option<f64>, k: &'static str| -> Result<f32, ParamsError> {
            nonneg(need(v, k)?, k)
        };
        let reqp =
            |v: Option<f64>, k: &'static str| -> Result<f32, ParamsError> { pos(need(v, k)?, k) };
        let hp = reqp(cap.height_prone_m, "capsule.height_prone_m")?;
        let eye_p = reqp(cap.eye_height_prone_m, "capsule.eye_height_prone_m")?;
        if hp < 2.0 * r || hp > hc || eye_p > hp {
            return Err(ParamsError::Invalid("capsule.height_prone_m"));
        }
        let ms_ticks = |v: Option<f64>, k: &'static str| -> Result<f32, ParamsError> {
            Ok(reqp(v, k)? * hz / 1000.0)
        };
        let regen_delay =
            libm::roundf(reqp(m.stamina_regen_delay_s, "movement.stamina_regen_delay_s")? * hz)
                .clamp(0.0, 255.0) as u8;
        let stamina_max = pos(m.stamina_max, "movement.stamina_max")?;
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
            accel_time_s: ramp(m.accel_time_s, "movement.accel_time_s")?,
            decel_time_s: ramp(m.decel_time_s, "movement.decel_time_s")?,
            eye_height_stand_m: eye_s,
            eye_height_crouch_m: eye_c,
            height_prone_m: hp,
            eye_height_prone_m: eye_p,
            prone_mps: pos(m.prone_mps, "movement.prone_mps")?,
            prone_fast_mps: reqp(m.prone_fast_mps, "movement.prone_fast_mps")?,
            prone_fast_cost_per_s: req(m.prone_fast_cost_per_s, "movement.prone_fast_cost_per_s")?,
            ticks_c2p: ms_ticks(m.crouch_to_prone_ms, "movement.crouch_to_prone_ms")?,
            ticks_p2c: ms_ticks(m.prone_to_crouch_ms, "movement.prone_to_crouch_ms")?,
            ticks_s2p: ms_ticks(m.stand_to_prone_ms, "movement.stand_to_prone_ms")?,
            ticks_p2s: ms_ticks(m.prone_to_stand_ms, "movement.prone_to_stand_ms")?,
            ads_mult: [
                reqp(m.ads_stand_mult, "movement.ads_stand_mult")?,
                reqp(m.ads_crouch_mult, "movement.ads_crouch_mult")?,
                reqp(m.ads_prone_mult, "movement.ads_prone_mult")?,
            ],
            reload_move_mult: reqp(m.reload_move_mult, "movement.reload_move_mult")?,
            crawl_mps: reqp(m.crawl_mps, "movement.crawl_mps")?,
            load_ref_kg: req(m.load_ref_kg, "movement.load_ref_kg")?,
            load_penalty_per_kg: req(m.load_penalty_per_kg, "movement.load_penalty_per_kg")?,
            load_min_mult: reqp(m.load_min_mult, "movement.load_min_mult")?,
            stamina_max,
            sprint_cost_per_s: req(m.sprint_cost_per_s, "movement.sprint_cost_per_s")?,
            stamina_regen_per_s: req(m.stamina_regen_per_s, "movement.stamina_regen_per_s")?,
            stamina_regen_delay_ticks: regen_delay,
            stamina_regen_mult: [
                1.0,
                reqp(
                    m.stamina_regen_crouch_mult,
                    "movement.stamina_regen_crouch_mult",
                )?,
                reqp(
                    m.stamina_regen_prone_mult,
                    "movement.stamina_regen_prone_mult",
                )?,
            ],
            stamina_low: req(m.stamina_low_points, "movement.stamina_low_points")?,
            stamina_unlock: req(m.stamina_unlock_points, "movement.stamina_unlock_points")?,
            jump_cost: req(m.jump_cost_points, "movement.jump_cost_points")?,
            recoil_mult: [
                1.0,
                reqp(
                    m.stance_recoil_crouch_mult,
                    "movement.stance_recoil_crouch_mult",
                )?,
                reqp(
                    m.stance_recoil_prone_mult,
                    "movement.stance_recoil_prone_mult",
                )?,
            ],
            spread_mult: [
                1.0,
                reqp(
                    m.stance_spread_crouch_mult,
                    "movement.stance_spread_crouch_mult",
                )?,
                reqp(
                    m.stance_spread_prone_mult,
                    "movement.stance_spread_prone_mult",
                )?,
            ],
            visibility_mult: [
                1.0,
                reqp(m.visibility_crouch_mult, "movement.visibility_crouch_mult")?,
                reqp(m.visibility_prone_mult, "movement.visibility_prone_mult")?,
            ],
            hitbox_height_m: [
                reqp(m.hitbox_height_stand_m, "movement.hitbox_height_stand_m")?,
                reqp(m.hitbox_height_crouch_m, "movement.hitbox_height_crouch_m")?,
                reqp(m.hitbox_height_prone_m, "movement.hitbox_height_prone_m")?,
            ],
            move_spread_walk_mult: reqp(m.move_spread_walk_mult, "movement.move_spread_walk_mult")?,
            move_spread_run_mult: reqp(m.move_spread_run_mult, "movement.move_spread_run_mult")?,
        })
    }
}
