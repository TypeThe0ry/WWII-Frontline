//! Effect parameters derived from `S` (GAMEDESIGN 8.3). Pure; the client turns them into audio
//! and visuals, the sim applies the gameplay ones (M1-017b).

use super::PINNED_S;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SuppressionEffects {
    /// Movement speed multiplier `1 - 0.10 e`.
    pub move_mult: f32,
    /// ADS time multiplier `1 + 0.30 e`.
    pub ads_mult: f32,
    /// Spread multiplier `1 + 1.5 e` (hip and ADS).
    pub spread_mult: f32,
    /// Camera / ray shake amplitude `0.6 deg * e` (at 12 Hz).
    pub shake_deg: f32,
    /// Saturation multiplier `1 - 0.55 e`.
    pub desat: f32,
    /// Vignette strength `0.6 e`.
    pub vignette: f32,
    /// Radial blur `6 e` px.
    pub blur_px: f32,
    /// Hearing attenuation `3 dB e`.
    pub hearing_db: f32,
    /// `S >= 85`.
    pub pinned: bool,
}

/// `effects(S)`, `S` clamped to `[0, 100]`.
pub fn effects(s: f32) -> SuppressionEffects {
    let s = s.clamp(0.0, 100.0);
    let e = s / 100.0;
    SuppressionEffects {
        move_mult: 1.0 - 0.10 * e,
        ads_mult: 1.0 + 0.30 * e,
        spread_mult: 1.0 + 1.5 * e,
        shake_deg: 0.6 * e,
        desat: 1.0 - 0.55 * e,
        vignette: 0.6 * e,
        blur_px: 6.0 * e,
        hearing_db: 3.0 * e,
        pinned: s >= PINNED_S,
    }
}
