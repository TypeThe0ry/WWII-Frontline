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
    /// Previously commanded stance (selects the transition length, see `stance`).
    pub stance_from: Stance,
    /// `0..=stamina_max`.
    pub stamina: f32,
    /// Ticks until stamina regeneration resumes.
    pub stamina_delay: u8,
    /// Sprint / jump locked after exhaustion until stamina >= the unlock threshold.
    pub sprint_locked: bool,
    pub grounded: bool,
    /// Set when the last step could not move because the player is on or would leave the
    /// loaded collision (ARCHITECTURE 9.3 failure handling).
    pub blocked: bool,
}

impl PlayerAuthState {
    /// Serialised size of [`to_bytes`](Self::to_bytes).
    pub const BYTES: usize = 45;

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
            stance_from: Stance::Stand,
            stamina: Self::SPAWN_STAMINA,
            stamina_delay: 0,
            sprint_locked: false,
            grounded: false,
            blocked: false,
        }
    }

    /// Stamina of a freshly spawned player (the role's `stamina_max`, 100 in all M1 roles; the
    /// movement stage clamps it to the role maximum on the first tick).
    pub const SPAWN_STAMINA: f32 = 100.0;

    /// Little-endian layout (offsets in bytes): `pos` f32x3 @0, `vel` f32x3 @12, `yaw` f32
    /// @24, `pitch` f32 @28, `stance_blend` f32 @32, `stance` u8 @36, `stance_timer` u8 @37,
    /// flags u8 @38 (bit0 `grounded`, bit1 `blocked`, bit2 `sprint_locked`), `stance_from` u8 @39, `stamina` f32 bits XOR `SPAWN_STAMINA` bits @40 (so that an all-zero tail means a fresh player and the round trip is exact), `stamina_delay` u8 @44. Used for `self_state` and
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
        b[38] = u8::from(self.grounded)
            | (u8::from(self.blocked) << 1)
            | (u8::from(self.sprint_locked) << 2);
        b[39] = self.stance_from as u8;
        b[40..44].copy_from_slice(
            &(self.stamina.to_bits() ^ Self::SPAWN_STAMINA.to_bits()).to_le_bytes(),
        );
        b[44] = self.stamina_delay;
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
        let stamina = f32::from_bits(
            u32::from_le_bytes([b[40], b[41], b[42], b[43]]) ^ Self::SPAWN_STAMINA.to_bits(),
        );
        if f.iter().any(|v| !v.is_finite()) || !stamina.is_finite() || b[38] & !0b111 != 0 {
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
            stance_from: Stance::from_u8(b[39])?,
            stamina,
            stamina_delay: b[44],
            sprint_locked: b[38] & 4 != 0,
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
            stance_from: Stance::Prone,
            stamina: 42.5,
            stamina_delay: 7,
            sprint_locked: true,
            grounded: true,
            blocked: true,
        };
        let b = s.to_bytes();
        assert_eq!(b.len(), PlayerAuthState::BYTES);
        assert_eq!(PlayerAuthState::from_bytes(&b), Some(s));
        assert!(PlayerAuthState::from_bytes(&b[..44]).is_none());
        let mut bad = b;
        bad[36] = 9;
        assert!(PlayerAuthState::from_bytes(&bad).is_none());
        let mut bad = b;
        bad[0..4].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(PlayerAuthState::from_bytes(&bad).is_none());
        let mut bad = b;
        bad[38] = 8;
        assert!(PlayerAuthState::from_bytes(&bad).is_none());
    }
}
