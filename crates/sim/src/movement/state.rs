//! [`PlayerAuthState`]: the per-player state that `step_player` advances, and its wire bytes.

use glam::Vec3;

use super::stance::Stance;

/// Authoritative movement state of one player (server) / predicted state (client).
///
/// Orientation convention: x east, z north, y up; `yaw` 0 faces +z and grows toward +x
/// (forward = `(sin yaw, cos yaw)`), see [`super`] for the input mapping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerAuthState {
    /// Foot position (capsule bottom), metres.
    pub pos: Vec3,
    /// Velocity, m/s. Horizontal part: achieved velocity of the last tick in the air; on the ground the
    /// ramped (commanded) velocity unless the move was obstructed by more than half (see `step_player`).
    pub vel: Vec3,
    /// Radians `[0, 2pi)`.
    pub yaw: f32,
    /// Radians `[-pi/2, pi/2]`.
    pub pitch: f32,
    /// Commanded (target) stance.
    pub stance: Stance,
    /// Crouch fraction `0..=1` of the capsule-height transition.
    pub stance_blend: f32,
    /// Ticks until the stance transition completes (0 = settled).
    pub stance_timer: u8,
    pub grounded: bool,
    /// Set when the last step could not move because the player is on or would leave the
    /// loaded collision (ARCHITECTURE 9.3 failure handling).
    pub blocked: bool,
}

impl PlayerAuthState {
    /// Serialised size of [`to_bytes`](Self::to_bytes).
    pub const BYTES: usize = 39;

    /// A standing, airborne (it will settle onto the ground) player at `pos`.
    pub fn spawn(pos: Vec3, yaw: f32) -> Self {
        Self {
            pos,
            vel: Vec3::ZERO,
            yaw,
            pitch: 0.0,
            stance: Stance::Stand,
            stance_blend: 0.0,
            stance_timer: 0,
            grounded: false,
            blocked: false,
        }
    }

    /// Little-endian layout (offsets in bytes): `pos` f32x3 @0, `vel` f32x3 @12, `yaw` f32
    /// @24, `pitch` f32 @28, `stance_blend` f32 @32, `stance` u8 @36, `stance_timer` u8 @37,
    /// flags u8 @38 (bit0 `grounded`, bit1 `blocked`). Used for `self_state` and
    /// `Predictor.reset(authState)` (M0-014 / M0-022); a layout change must be mirrored there.
    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        let mut b = [0u8; Self::BYTES];
        let f = [
            self.pos.x,
            self.pos.y,
            self.pos.z,
            self.vel.x,
            self.vel.y,
            self.vel.z,
            self.yaw,
            self.pitch,
            self.stance_blend,
        ];
        for (i, v) in f.iter().enumerate() {
            b[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        b[36] = self.stance as u8;
        b[37] = self.stance_timer;
        b[38] = u8::from(self.grounded) | (u8::from(self.blocked) << 1);
        b
    }

    /// Inverse of [`to_bytes`](Self::to_bytes); `None` for a wrong length, an unknown stance,
    /// reserved flag bits or a non-finite float.
    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        if b.len() != Self::BYTES {
            return None;
        }
        let mut f = [0f32; 9];
        for (i, v) in f.iter_mut().enumerate() {
            *v = f32::from_le_bytes([b[i * 4], b[i * 4 + 1], b[i * 4 + 2], b[i * 4 + 3]]);
        }
        if f.iter().any(|v| !v.is_finite()) || b[38] & !0b11 != 0 {
            return None;
        }
        Some(Self {
            pos: Vec3::new(f[0], f[1], f[2]),
            vel: Vec3::new(f[3], f[4], f[5]),
            yaw: f[6],
            pitch: f[7],
            stance_blend: f[8],
            stance: Stance::from_u8(b[36])?,
            stance_timer: b[37],
            grounded: b[38] & 1 != 0,
            blocked: b[38] & 2 != 0,
        })
    }

    /// Bit-exact equality (distinguishes `-0.0`, never equal for NaN-free states otherwise).
    pub fn bit_eq(&self, o: &Self) -> bool {
        self.to_bytes() == o.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_roundtrip() {
        let s = PlayerAuthState {
            pos: Vec3::new(1.5, -2.0, 300.25),
            vel: Vec3::new(0.1, 3.4, -4.4),
            yaw: 3.0,
            pitch: -1.0,
            stance: Stance::Crouch,
            stance_blend: 0.4,
            stance_timer: 5,
            grounded: true,
            blocked: true,
        };
        let b = s.to_bytes();
        assert_eq!(b.len(), PlayerAuthState::BYTES);
        assert_eq!(PlayerAuthState::from_bytes(&b), Some(s));
        assert!(PlayerAuthState::from_bytes(&b[..38]).is_none());
        let mut bad = b;
        bad[36] = 9;
        assert!(PlayerAuthState::from_bytes(&bad).is_none());
        let mut bad = b;
        bad[0..4].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(PlayerAuthState::from_bytes(&bad).is_none());
        let mut bad = b;
        bad[38] = 4;
        assert!(PlayerAuthState::from_bytes(&bad).is_none());
    }
}
