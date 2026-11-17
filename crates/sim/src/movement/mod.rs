//! Player movement (M0-011): walk / run / sprint / crouch / jump on the terrain height field.
//!
//! [`step_player`] is a **pure function** of `(state, input, params, collision)`; it never
//! touches the ECS `World`. The server (`World::step`) and the client prediction
//! (`PredictSim`, feature `client-predict`) both call it, so they cannot diverge (P2).
//!
//! # Input mapping (`InputCmd`)
//! * `move_x` (+ right) / `move_z` (+ forward), `-127..=127`, divided by 127, clamped to `[-1, 1]`
//!   (-128 is treated as -127) and normalised to length <= 1 (`libm::sqrtf`); `yaw` (u16 ->
//!   `[0, 2pi)`) and `pitch` (i16 -> +-pi/2) become the view angles. Forward is
//!   `(sin yaw, cos yaw)` on (x, z), right is `(cos yaw, -sin yaw)` (x east, z north).
//! * `buttons`: `SPRINT` (bit 6) sprints when the forward component is > 0 and the target stance is
//!   standing (M0: unlimited stamina); `CROUCH` (bit 4) held = crouch target; `JUMP` (bit 3)
//!   jumps when grounded and standing. `PRONE` (bit 5) is ignored until M1-011.
//! * `aux` bit 0 (`AUX_WALK`) = walk modifier ("Shift", GAMEDESIGN 9.1: 2.0 m/s standing,
//!   1.5 m/s crouched). Other `aux` bits are reserved (PLAN_ISSUES PI-014).
//!
//! # Model
//! Ground: horizontal velocity jumps to the wish velocity (instant acceleration, ground
//! friction stops at once; smoother ramps wait for REF M-24..M-26). Air: horizontal velocity
//! is kept; while a direction is held it closes `air_control_mult` of the gap to the wish
//! velocity per tick. Vertical: exact constant-gravity integration per tick. Speeds, jump
//! height, gravity, slope limit and capsule come from `data/roles/*.json`.

mod kcc;
mod params;
mod stance;
mod state;

pub use kcc::{move_capsule, KccMove};
pub use params::{MoveParams, ParamsError};
pub use stance::{capsule_height, Stance};
pub use state::PlayerAuthState;

use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_protocol::quant::{dequant_yaw, pitch_i16_to_rad};
use glam::Vec3;

use crate::collision::CollisionData;

/// `InputCmd::aux` bit 0: walk modifier.
pub const AUX_WALK: u8 = 1;

/// Normalised movement intent in `[-1, 1]^2`, length <= 1. Non-finite axes count as 0.
pub fn normalize_move(x: f32, z: f32) -> (f32, f32) {
    let fix = |v: f32| {
        if v.is_finite() {
            v.clamp(-1.0, 1.0)
        } else {
            0.0
        }
    };
    let (x, z) = (fix(x), fix(z));
    let len = libm::sqrtf(x * x + z * z);
    if len > 1.0 {
        (x / len, z / len)
    } else {
        (x, z)
    }
}

/// Decodes the quantised move axes of a command.
pub fn decode_move(cmd: &InputCmd) -> (f32, f32) {
    normalize_move(f32::from(cmd.move_x) / 127.0, f32::from(cmd.move_z) / 127.0)
}

/// Horizontal speed (m/s at full deflection) chosen by the buttons and stance.
fn select_speed(cmd: &InputCmd, forward: f32, want: Stance, p: &MoveParams) -> f32 {
    let walk = cmd.aux & AUX_WALK != 0;
    match want {
        Stance::Crouch => {
            if walk {
                p.crouch_walk_mps
            } else {
                p.crouch_run_mps
            }
        }
        Stance::Stand => {
            if cmd.buttons & buttons::SPRINT != 0 && forward > 0.0 {
                p.sprint_mps
            } else if walk {
                p.walk_mps
            } else {
                p.run_mps
            }
        }
    }
}

/// Advances one player by one tick. Pure: the result depends only on the arguments.
///
/// Out of loaded collision (or a non-finite state) the position is kept and `blocked` is set.
pub fn step_player(
    state: &PlayerAuthState,
    cmd: &InputCmd,
    p: &MoveParams,
    col: &CollisionData,
) -> PlayerAuthState {
    let mut s = *state;
    s.yaw = dequant_yaw(cmd.yaw, 16);
    s.pitch = pitch_i16_to_rad(cmd.pitch);

    let want = if cmd.buttons & buttons::CROUCH != 0 {
        Stance::Crouch
    } else {
        Stance::Stand
    };
    stance::advance(&mut s, want, p);

    // Out-of-bounds protection (ARCHITECTURE 9.3): never move on or into unloaded terrain.
    if !s.pos.is_finite() || !s.vel.is_finite() || col.height_at(s.pos.x, s.pos.z).is_none() {
        s.vel = Vec3::ZERO;
        s.blocked = true;
        if !state.pos.is_finite() {
            s.pos = Vec3::ZERO;
        }
        return s;
    }

    // Wish velocity.
    let (mx, mz) = decode_move(cmd);
    let speed = select_speed(cmd, mz, want, p);
    let (sy, cy) = (libm::sinf(s.yaw), libm::cosf(s.yaw));
    let wish_x = (mx * cy + mz * sy) * speed;
    let wish_z = (mz * cy - mx * sy) * speed;
    let has_input = mx != 0.0 || mz != 0.0;

    let mut vel = s.vel;
    let mut grounded = s.grounded;
    if grounded {
        vel.x = wish_x;
        vel.z = wish_z;
        vel.y = 0.0;
        if cmd.buttons & buttons::JUMP != 0 && want == Stance::Stand {
            vel.y = p.jump_speed_mps;
            grounded = false;
        }
    } else if has_input {
        vel.x += (wish_x - vel.x) * p.air_control;
        vel.z += (wish_z - vel.z) * p.air_control;
    }

    // Vertical displacement for this tick (exact for constant gravity).
    let g = p.gravity_mps2;
    let dy = if grounded {
        0.0
    } else {
        vel.y * p.dt - 0.5 * g * p.dt * p.dt
    };
    if !grounded {
        vel.y -= g * p.dt;
    }
    let desired = Vec3::new(vel.x * p.dt, dy, vel.z * p.dt);

    let height = capsule_height(s.stance_blend, p);
    let m = move_capsule(col, p, s.pos, height, desired, state.grounded);
    let new_pos = s.pos + m.translation;
    if !new_pos.is_finite() || col.height_at(new_pos.x, new_pos.z).is_none() {
        s.vel = Vec3::ZERO;
        s.blocked = true;
        return s;
    }

    s.pos = new_pos;
    s.blocked = false;
    // Achieved horizontal velocity (sliding, slopes and blocking reduce it).
    vel.x = m.translation.x * p.hz;
    vel.z = m.translation.z * p.hz;
    if dy > 0.0 && m.translation.y < dy - 1e-4 && vel.y > 0.0 {
        vel.y = 0.0; // bumped a ceiling
    }
    // A rising character is never grounded (the jump apex may still be within the skin).
    s.grounded = m.grounded && vel.y <= 0.0;
    if s.grounded {
        vel.y = 0.0;
    }
    s.vel = vel;
    s
}
