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
//! Ground: horizontal velocity moves toward the wish velocity by at most `ref_speed / accel_time_s * dt`
//! per tick (linear ramp; `ref_speed` = the speed of the selected move mode, so rest -> target
//! takes `accel_time_s`), and when slowing down (including releasing the keys) by
//! `ref_speed / decel_time_s * dt`; a time of 0 / absent in the data means instantaneous
//! (GAMEDESIGN 9.1, PLAN_ISSUES PI-015: 0.25 s / 0.20 s, to be tuned, REF M-24..M-26). Air: horizontal velocity
//! is kept; while a direction is held it closes `air_control_mult` of the gap to the wish
//! velocity per tick. Vertical: exact constant-gravity integration per tick. Speeds, jump
//! height, gravity, slope limit and capsule come from `data/roles/*.json`.

mod kcc;
pub mod modifiers;
mod params;
pub mod stamina;
mod stance;
mod state;
pub mod validate;

pub use kcc::{move_capsule, KccMove};
pub use params::{MoveParams, ParamsError};
pub use stamina::quantised as stamina_quantised;
pub use stance::{capsule_height, duration_ticks, eye_height, stance_height, Stance};
pub use state::PlayerAuthState;
pub use validate::{check_displacement, max_speed_for, MoveBound, Violation};

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

/// Per-tick context that is not part of the movement state: weapon and loadout facts.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MoveCtx {
    /// A reload is in progress (speed x `reload_move_mult`).
    pub reloading: bool,
    /// Carried load; `None` = the reference load (no penalty).
    pub load_kg: Option<f32>,
}

/// What the commanded move costs / selects, see [`select_speed`].
struct Speed {
    mps: f32,
    sprint: bool,
    prone_fast: bool,
}

/// Horizontal speed (m/s at full deflection) chosen by the buttons, stance and stamina.
fn select_speed(
    cmd: &InputCmd,
    forward: f32,
    want: Stance,
    can_exert: bool,
    p: &MoveParams,
) -> Speed {
    let walk = cmd.aux & AUX_WALK != 0;
    let sprint_btn = cmd.buttons & buttons::SPRINT != 0;
    let (mut sprint, mut prone_fast) = (false, false);
    let mps = match want {
        Stance::Prone if sprint_btn && can_exert && !walk => {
            prone_fast = true;
            p.prone_fast_mps
        }
        Stance::Prone => p.prone_mps,
        Stance::Crouch if walk => p.crouch_walk_mps,
        Stance::Crouch => p.crouch_run_mps,
        Stance::Stand if sprint_btn && forward > 0.0 && can_exert => {
            sprint = true;
            p.sprint_mps
        }
        Stance::Stand if walk => p.walk_mps,
        Stance::Stand => p.run_mps,
    };
    Speed {
        mps,
        sprint,
        prone_fast,
    }
}

/// Moves the horizontal velocity `cur` toward `wish` with the linear ramps of `p`.
fn ramp_horizontal(
    cur: (f32, f32),
    wish: (f32, f32),
    ref_speed: f32,
    p: &MoveParams,
) -> (f32, f32) {
    let (dx, dz) = (wish.0 - cur.0, wish.1 - cur.1);
    let dist = libm::sqrtf(dx * dx + dz * dz);
    let cur_len = libm::sqrtf(cur.0 * cur.0 + cur.1 * cur.1);
    let wish_len = libm::sqrtf(wish.0 * wish.0 + wish.1 * wish.1);
    // Slowing down (also: releasing the keys) uses the deceleration time, otherwise acceleration.
    let time = if wish_len < cur_len {
        p.decel_time_s
    } else {
        p.accel_time_s
    };
    if time <= 0.0 || dist == 0.0 {
        return wish;
    }
    let max_step = ref_speed * p.dt / time;
    if dist <= max_step {
        wish
    } else {
        let k = max_step / dist;
        (cur.0 + dx * k, cur.1 + dz * k)
    }
}

/// Headroom above the feet: how tall a capsule may be at `pos` (terrain overhangs only; the
/// collision world has no other ceilings yet).
fn headroom(col: &CollisionData, pos: Vec3, p: &MoveParams) -> f32 {
    let probe = 0.05;
    match col.raycast(pos + Vec3::new(0.0, probe, 0.0), Vec3::Y, p.height_stand_m) {
        Some(h) => h.t + probe,
        None => p.height_stand_m,
    }
}

/// Advances one player by one tick with the default context (no reload, reference load).
pub fn step_player(
    state: &PlayerAuthState,
    cmd: &InputCmd,
    p: &MoveParams,
    col: &CollisionData,
) -> PlayerAuthState {
    step_player_ex(state, cmd, p, col, &MoveCtx::default())
}

/// Advances one player by one tick. Pure: the result depends only on the arguments.
///
/// Out of loaded collision (or a non-finite state) the position is kept and `blocked` is set.
pub fn step_player_ex(
    state: &PlayerAuthState,
    cmd: &InputCmd,
    p: &MoveParams,
    col: &CollisionData,
    ctx: &MoveCtx,
) -> PlayerAuthState {
    let mut s = *state;
    s.yaw = dequant_yaw(cmd.yaw, 16);
    s.pitch = pitch_i16_to_rad(cmd.pitch);
    s.stamina = s.stamina.clamp(0.0, p.stamina_max);

    let mut want = if cmd.buttons & buttons::PRONE != 0 {
        Stance::Prone
    } else if cmd.buttons & buttons::CROUCH != 0 {
        Stance::Crouch
    } else {
        Stance::Stand
    };
    // Rising needs room: pick the tallest stance that fits under the ceiling.
    if want.pose() < s.stance.pose() && s.pos.is_finite() {
        let room = headroom(col, s.pos, p);
        while want != s.stance && stance::stance_height(want, p) > room {
            want = if want == Stance::Stand {
                Stance::Crouch
            } else {
                Stance::Prone
            };
        }
        if want.pose() > s.stance.pose() {
            want = s.stance;
        }
    }
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
    let exert = stamina::can_exert(&s);
    let sel = select_speed(cmd, mz, want, exert, p);
    let mut speed = sel.mps;
    if cmd.buttons & buttons::AIM != 0 && !sel.sprint && !ctx.reloading {
        speed *= p.ads_mult[want as usize];
    }
    if ctx.reloading {
        speed *= p.reload_move_mult;
    }
    if let Some(kg) = ctx.load_kg {
        speed *= modifiers::load_mult(kg, p);
    }
    let (sy, cy) = (libm::sinf(s.yaw), libm::cosf(s.yaw));
    let wish_x = (mx * cy + mz * sy) * speed;
    let wish_z = (mz * cy - mx * sy) * speed;
    let has_input = mx != 0.0 || mz != 0.0;
    // Stamina is only spent while actually moving.
    let spend_sprint = sel.sprint && has_input;
    let spend_fast = sel.prone_fast && has_input;
    let mut jumped = false;

    let mut vel = s.vel;
    let mut grounded = s.grounded;
    let ramped = grounded;
    if grounded {
        let (hx, hz) = ramp_horizontal(
            (vel.x, vel.z),
            (wish_x, wish_z),
            // Reference speed: the mode's full-deflection speed (for released keys: the mode the
            // held buttons would select going forward).
            if has_input {
                speed
            } else {
                select_speed(cmd, 1.0, want, exert, p).mps
            },
            p,
        );
        vel.x = hx;
        vel.z = hz;
        vel.y = 0.0;
        if cmd.buttons & buttons::JUMP != 0
            && want == Stance::Stand
            && stamina::can_afford(&s, p.jump_cost)
        {
            vel.y = p.jump_speed_mps;
            grounded = false;
            jumped = true;
        }
    } else if has_input {
        vel.x += (wish_x - vel.x) * p.air_control;
        vel.z += (wish_z - vel.z) * p.air_control;
    }
    stamina::update(
        &mut s,
        stamina::Spend {
            sprint: spend_sprint,
            prone_fast: spend_fast,
            jump: jumped,
            extra: 0.0,
        },
        want,
        p,
    );

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
    // Stored horizontal velocity. In the air it is the achieved one. After a ground ramp it is the
    // ramped (commanded) velocity, so that slopes (achieved ~ cos^2 of the slope angle) do not feed
    // back into the next tick's ramp; but a move obstructed below half of what was asked (walls,
    // too-steep slopes) stores the achieved velocity, so there is no phantom momentum.
    let (ach_x, ach_z) = (m.translation.x * p.hz, m.translation.z * p.hz);
    let want_h = libm::sqrtf(vel.x * vel.x + vel.z * vel.z);
    let got_h = libm::sqrtf(ach_x * ach_x + ach_z * ach_z);
    if !ramped || got_h < 0.5 * want_h {
        vel.x = ach_x;
        vel.z = ach_z;
    }
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
