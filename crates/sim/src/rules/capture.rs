//! Capture formula (GAMEDESIGN 3.2).

use crate::data::defs::CaptureRulesDef;

/// Capture coefficients in `p` percent per second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptureParams {
    pub k_cap: f32,
    pub regress: f32,
    pub height_tol_m: f32,
    pub full_players: u32,
    pub max_net: u32,
    pub partial_mult: f32,
    pub vehicle_occupants_counted: bool,
}

impl CaptureParams {
    pub fn from_def(d: &CaptureRulesDef) -> Self {
        Self {
            k_cap: d.k_cap_pct_per_s as f32,
            regress: d.regress_pct_per_s as f32,
            height_tol_m: d.height_tolerance_m as f32,
            full_players: d.eff.full_weight_players,
            max_net: d.eff.max_net_players,
            partial_mult: d.eff.partial_weight_mult as f32,
            vehicle_occupants_counted: d.vehicle_occupants_counted,
        }
    }
}

/// `min(|d|, 4) + 0.5 * max(0, min(|d|, 10) - 4)` with the default coefficients.
pub fn eff(d_abs: u32) -> f32 {
    eff_with(d_abs, 4, 10, 0.5)
}

pub fn eff_with(d_abs: u32, full: u32, max_net: u32, partial: f32) -> f32 {
    let d = d_abs.min(max_net);
    d.min(full) as f32 + partial * d.saturating_sub(full) as f32
}

/// Signed rate (percent per second) for the net advantage `d = n_us - n_ger`.
pub fn capture_rate(d: i32, p: &CaptureParams) -> f32 {
    if d == 0 {
        return 0.0;
    }
    let e = eff_with(d.unsigned_abs(), p.full_players, p.max_net, p.partial_mult);
    p.k_cap * e * if d > 0 { 1.0 } else { -1.0 }
}
