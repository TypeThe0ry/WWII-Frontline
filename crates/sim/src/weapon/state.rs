//! [`WeaponState`]: the per-player weapon state machine data and its wire bytes.

use bevy_ecs::prelude::Component;

/// Time is kept in *tick-milli-units*: `ms * tick_hz`. One tick consumes [`TICK_UNITS`] (1000)
/// units, so a duration of `ms` milliseconds lasts `ms * hz / 1000` ticks exactly, with the
/// fractional part carried by [`WeaponState::timer_units`] (no systematic rounding bias,
/// REFERENCE_MEASUREMENTS 7.2 item 3).
pub const TICK_UNITS: i32 = 1000;

/// Which duration a reload uses (GAMEDESIGN 6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReloadKind {
    /// Rounds left in the magazine: `reload.tactical_ms`.
    Tactical = 0,
    /// Empty magazine: `reload.empty_ms`.
    Empty = 1,
}

/// Fire state machine: `Ready -> (fire) -> Cooldown | BoltCycle -> Ready | Empty`,
/// `Reloading{kind}` (any state but itself, ends in `Ready`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fsm {
    Ready,
    /// Semi / auto pacing interval after a shot (`60 / cap_rpm`).
    Cooldown,
    /// Bolt cycling after a shot (`bolt_cycle_ms`).
    BoltCycle,
    Reloading(ReloadKind),
    /// Magazine empty, nothing cycling: a trigger pull is a dry click.
    Empty,
}

impl Fsm {
    /// A reload is in progress.
    pub fn is_reloading(self) -> bool {
        matches!(self, Fsm::Reloading(_))
    }

    fn code(self) -> u8 {
        match self {
            Fsm::Ready => 0,
            Fsm::Cooldown => 1,
            Fsm::BoltCycle => 2,
            Fsm::Reloading(ReloadKind::Tactical) => 3,
            Fsm::Reloading(ReloadKind::Empty) => 4,
            Fsm::Empty => 5,
        }
    }
    fn from_code(c: u8) -> Option<Self> {
        Some(match c {
            0 => Fsm::Ready,
            1 => Fsm::Cooldown,
            2 => Fsm::BoltCycle,
            3 => Fsm::Reloading(ReloadKind::Tactical),
            4 => Fsm::Reloading(ReloadKind::Empty),
            5 => Fsm::Empty,
            _ => return None,
        })
    }
}

/// Weapon state of one player (ECS component; also advanced by `PredictSim`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WeaponState {
    /// Index into `GameData::weapons`.
    pub def_id: u16,
    /// Rounds in the magazine / clip.
    pub mag: u16,
    /// Spare rounds.
    pub reserve: u16,
    pub fsm: Fsm,
    /// Cooldown / bolt cycle / reload time left in tick-milli-units. In `Ready` / `Empty` it is
    /// the (non-positive, `> -TICK_UNITS`) credit of the last interval, carried into the next one.
    pub timer_units: i32,
    /// The fire button was down last tick (edge detection for semi / bolt).
    pub trigger_held: bool,
    /// Time since the last shot, tick-milli-units, saturating (recoil recovery delay).
    pub since_shot_units: i32,
    /// Accumulated recoil offset of the view, degrees (pitch up / yaw right positive).
    pub recoil_pitch_deg: f32,
    pub recoil_yaw_deg: f32,
    /// Spread bloom, degrees.
    pub bloom_deg: f32,
    /// Time spent aiming down sights, tick-milli-units (0 when not aiming).
    pub ads_units: i32,
}

/// `since_shot_units` saturation (10 s).
const SINCE_SHOT_CAP: i32 = 10_000 * 30;

impl WeaponState {
    /// Serialised size of [`to_bytes`](Self::to_bytes).
    pub const BYTES: usize = 33;

    /// Fresh weapon: full magazine, given reserve, ready.
    pub fn new(def_id: u16, mag: u16, reserve: u16) -> Self {
        Self {
            def_id,
            mag,
            reserve,
            fsm: if mag == 0 { Fsm::Empty } else { Fsm::Ready },
            timer_units: 0,
            trigger_held: false,
            since_shot_units: SINCE_SHOT_CAP,
            recoil_pitch_deg: 0.0,
            recoil_yaw_deg: 0.0,
            bloom_deg: 0.0,
            ads_units: 0,
        }
    }

    pub(crate) fn saturate_since(v: i32) -> i32 {
        v.min(SINCE_SHOT_CAP)
    }

    /// Little-endian layout (offsets in bytes): `def_id` u16 @0, `mag` u16 @2, `reserve` u16
    /// @4, fsm code u8 @6 (0 Ready, 1 Cooldown, 2 BoltCycle, 3 Reloading tactical, 4 Reloading
    /// empty, 5 Empty), flags u8 @7 (bit0 `trigger_held`), `timer_units` i32 @8,
    /// `since_shot_units` i32 @12, `recoil_pitch_deg` f32 @16, `recoil_yaw_deg` f32 @20,
    /// `bloom_deg` f32 @24, `ads_units` i32 @28, reserved u8 @32 (0). Used for `self_state`
    /// and `Predictor.reset` (M0-014 / M0-022); a layout change must be mirrored there.
    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        let mut b = [0u8; Self::BYTES];
        b[0..2].copy_from_slice(&self.def_id.to_le_bytes());
        b[2..4].copy_from_slice(&self.mag.to_le_bytes());
        b[4..6].copy_from_slice(&self.reserve.to_le_bytes());
        b[6] = self.fsm.code();
        b[7] = u8::from(self.trigger_held);
        b[8..12].copy_from_slice(&self.timer_units.to_le_bytes());
        b[12..16].copy_from_slice(&self.since_shot_units.to_le_bytes());
        b[16..20].copy_from_slice(&self.recoil_pitch_deg.to_le_bytes());
        b[20..24].copy_from_slice(&self.recoil_yaw_deg.to_le_bytes());
        b[24..28].copy_from_slice(&self.bloom_deg.to_le_bytes());
        b[28..32].copy_from_slice(&self.ads_units.to_le_bytes());
        b
    }

    /// Inverse of [`to_bytes`](Self::to_bytes); `None` for a wrong length, an unknown state,
    /// reserved bits or a non-finite float.
    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        if b.len() != Self::BYTES || b[7] & !1 != 0 || b[32] != 0 {
            return None;
        }
        let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        let i32_at = |i: usize| i32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        let f32_at = |i: usize| f32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        let (rp, ry, bl) = (f32_at(16), f32_at(20), f32_at(24));
        if !(rp.is_finite() && ry.is_finite() && bl.is_finite()) {
            return None;
        }
        Some(Self {
            def_id: u16_at(0),
            mag: u16_at(2),
            reserve: u16_at(4),
            fsm: Fsm::from_code(b[6])?,
            timer_units: i32_at(8),
            trigger_held: b[7] & 1 != 0,
            since_shot_units: i32_at(12),
            recoil_pitch_deg: rp,
            recoil_yaw_deg: ry,
            bloom_deg: bl,
            ads_units: i32_at(28),
        })
    }

    /// Bit-exact equality (the state bytes are equal).
    pub fn bit_eq(&self, o: &Self) -> bool {
        self.to_bytes() == o.to_bytes()
    }

    /// Fraction `0..=1` of the ADS transition (`ads_ms` from the weapon data).
    pub fn ads_progress(&self, ads_total_units: i32) -> f32 {
        if ads_total_units <= 0 {
            return if self.ads_units > 0 { 1.0 } else { 0.0 };
        }
        (self.ads_units as f32 / ads_total_units as f32).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_roundtrip() {
        let mut w = WeaponState::new(1, 8, 48);
        w.fsm = Fsm::Reloading(ReloadKind::Empty);
        w.timer_units = -123;
        w.trigger_held = true;
        w.recoil_pitch_deg = 1.5;
        w.recoil_yaw_deg = -0.25;
        w.bloom_deg = 0.4;
        w.ads_units = 4000;
        let b = w.to_bytes();
        assert_eq!(WeaponState::from_bytes(&b), Some(w));
        assert!(WeaponState::from_bytes(&b[..32]).is_none());
        let mut bad = b;
        bad[6] = 9;
        assert!(WeaponState::from_bytes(&bad).is_none());
        let mut bad = b;
        bad[7] = 2;
        assert!(WeaponState::from_bytes(&bad).is_none());
        let mut bad = b;
        bad[16..20].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(WeaponState::from_bytes(&bad).is_none());
        let mut bad = b;
        bad[32] = 1;
        assert!(WeaponState::from_bytes(&bad).is_none());
    }

    #[test]
    fn empty_mag_starts_empty() {
        assert_eq!(WeaponState::new(0, 0, 5).fsm, Fsm::Empty);
        assert_eq!(WeaponState::new(0, 3, 5).fsm, Fsm::Ready);
    }
}
