//! Rifle firing (M0-012): fire state machine, spread, recoil, reload, ADS.
//!
//! [`step_weapon`] is a **pure function** of `(weapon state, weapon data, input, post-movement
//! player state)` (plus the match seed / tick / entity for the random draws). The server
//! (`World::step`, stage "weapons") and the client prediction (`PredictSim`) both call it, so
//! ammo, cooldowns, recoil and bloom are bit-identical. Projectile flight, hits and damage live
//! in [`crate::projectile`] and are server-only (the client never shows predicted damage).
//!
//! # Rules
//! * **Fire**: `buttons::FIRE`. Semi / bolt / burst / pump need a rising edge (release and press
//!   again); auto fires while held. Fire is refused while reloading, cycling (cooldown /
//!   bolt), sprinting (SPRINT held, moving forward, not crouching: the same condition that makes
//!   the movement sprint) and while a stance transition runs (GAMEDESIGN 9.2). An edge on an
//!   empty magazine is a dry click (no projectile). A trigger press that arrives while the
//!   weapon cannot fire is lost (no input buffering in M0).
//! * **Timing**: durations are `ms * tick_hz` tick-milli-units ([`state::TICK_UNITS`] = 1000 per
//!   tick). The unused fraction of an interval is carried into the next one (credit below one
//!   tick), so the average rate is exact: 120 rpm at 30 Hz is 15 ticks, a 140 rpm weapon
//!   would alternate 12 / 13 ticks.
//! * **Reload**: `buttons::RELOAD`, only with a non-full magazine and spare rounds. Duration
//!   `tactical_ms` (rounds left) or `empty_ms` (empty); at the end the magazine is filled from
//!   the reserve (whole clips; per-round loading is M1-013). Firing is impossible meanwhile.
//! * **ADS**: `buttons::AIM`; after `ads_ms` the ADS spread applies, before it the hip spread
//!   (M0 simplification). Sprinting, reloading and releasing the button reset it.
//! * **Aim**: view direction = input yaw / pitch + accumulated recoil, then a uniform draw in
//!   the cone `theta = base * stance_spread * move_spread + bloom` ([`spread`]). Pitch > 0 looks up.
//! * **Recoil**: per shot `view_pitch += vkick * stance_recoil * (1 +- 15 %)`,
//!   `view_yaw += N(0, hsigma) * stance_recoil`, `bloom += bloom_per_shot`; after
//!   [`recoil::RECOIL_DELAY_MS`] without a shot the recoil returns to zero at `recover_deg_s`
//!   and the bloom decays at `bloom_decay_deg_s`. (GAMEDESIGN 6.5 leaves 40 % of the recoil to
//!   the player's own pull-down; the simulation recovers all of it, the client camera adds the
//!   player's compensation on top.)
//! * The random draws of a shot use `Rng::for_(match_seed, tick, entity, Purpose::Spread)`:
//!   spread (2 numbers), then recoil (3 numbers).

pub mod ammo;
pub mod ballistics;
pub mod hitbox;
pub mod melee;
pub mod recoil;
pub mod spread;
pub mod state;

use frontline_protocol::msg::input::{buttons, InputCmd};
use glam::Vec3;

use crate::data::defs::{FireMode, WeaponDef};
use crate::ids::{EntityId, Tick};
use crate::movement::{decode_move, MoveParams, PlayerAuthState};
use crate::rng::{Purpose, Rng};

pub use hitbox::HitZone;
pub use melee::{MeleeParams, MeleeState};
pub use state::{Fsm, ReloadKind, WeaponState, TICK_UNITS};

/// Per-tick constants derived from a [`WeaponDef`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponParams {
    pub mode: FireMode,
    pub mag_capacity: u16,
    /// Shot-to-shot interval (semi / auto: `60000 / cap_rpm`, bolt: `bolt_cycle_ms`), units.
    pub interval_units: i32,
    pub reload_tactical_units: i32,
    pub reload_empty_units: i32,
    pub ads_units: i32,
    pub recoil_delay_units: i32,
    pub hip_deg: f32,
    pub ads_deg: f32,
    pub vkick_deg: f32,
    pub hsigma_deg: f32,
    pub recover_deg_s: f32,
    pub bloom_per_shot_deg: f32,
    pub bloom_decay_deg_s: f32,
    pub dt: f32,
}

/// Why a weapon cannot be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponParamsError {
    /// `historical.magazine_capacity` is "未核实" or zero.
    MagazineCapacity,
    /// Neither `fire.cap_rpm` (semi / auto) nor `fire.bolt_cycle_ms` (bolt) is usable.
    FireRate,
}

fn ms_to_units(ms: f64, hz: u8) -> i32 {
    libm::round(ms * f64::from(hz)) as i32
}

impl WeaponParams {
    pub fn from_def(def: &WeaponDef, hz: u8) -> Result<Self, WeaponParamsError> {
        use crate::data::defs::Hist;
        let cap = match def.historical.magazine_capacity {
            Hist::Known(n) if n > 0 && n <= u32::from(u16::MAX) => n as u16,
            _ => return Err(WeaponParamsError::MagazineCapacity),
        };
        let g = &def.gameplay;
        let interval_ms = match g.fire.mode {
            FireMode::Bolt | FireMode::Pump => g.fire.bolt_cycle_ms.filter(|v| *v > 0.0),
            _ => g
                .fire
                .cap_rpm
                .filter(|v| *v > 0.0)
                .map(|rpm| 60_000.0 / rpm),
        }
        .ok_or(WeaponParamsError::FireRate)?;
        Ok(Self {
            mode: g.fire.mode,
            mag_capacity: cap,
            interval_units: ms_to_units(interval_ms, hz).max(1),
            reload_tactical_units: ms_to_units(g.reload.tactical_ms, hz),
            reload_empty_units: ms_to_units(g.reload.empty_ms, hz),
            ads_units: ms_to_units(g.ads_ms, hz),
            recoil_delay_units: ms_to_units(recoil::RECOIL_DELAY_MS, hz),
            hip_deg: g.spread.hip_deg as f32,
            ads_deg: g.spread.ads_deg as f32,
            vkick_deg: g.recoil.vkick_deg as f32,
            hsigma_deg: g.recoil.hsigma_deg as f32,
            recover_deg_s: g.recoil.recover_deg_s as f32,
            bloom_per_shot_deg: g.recoil.bloom_per_shot_deg as f32,
            bloom_decay_deg_s: g.recoil.bloom_decay_deg_s as f32,
            dt: 1.0 / f32::from(hz),
        })
    }

    /// The trigger must be released between shots.
    pub fn needs_edge(&self) -> bool {
        self.mode != FireMode::Auto
    }

    fn cycling_state(&self) -> Fsm {
        match self.mode {
            FireMode::Bolt | FireMode::Pump => Fsm::BoltCycle,
            _ => Fsm::Cooldown,
        }
    }
}

/// A shot leaving the barrel this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotSpawn {
    /// First projectile position: eye + [`ballistics::MUZZLE_OFFSET_M`] along `dir`.
    pub muzzle: Vec3,
    /// Unit direction including recoil and spread.
    pub dir: Vec3,
    /// Spread seed of the shot (`crate::rng::spread_seed`), the value `FireCmd.spread_seed` carries
    /// so that the client can predict the same cone sample (ARCHITECTURE 6.8.2).
    pub spread_seed: u16,
}

/// What happened in one weapon step (besides the state change).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WeaponOut {
    pub shot: Option<ShotSpawn>,
    /// The trigger was pulled on an empty magazine.
    pub dry_fire: bool,
    pub reload_started: Option<ReloadKind>,
    pub reload_finished: bool,
    /// A running reload was cancelled (sprint, vault); nothing was consumed.
    pub reload_cancelled: bool,
}

/// Randomness context of a step.
#[derive(Debug, Clone, Copy)]
pub struct ShotCtx {
    pub match_seed: u64,
    /// Tick being simulated (the server's `CurrentTick` after the increment).
    pub tick: Tick,
    pub entity: EntityId,
}

/// Whether the command sprints (same rule as in movement: SPRINT, forward, standing).
pub fn is_sprinting(cmd: &InputCmd) -> bool {
    let (_, mz) = decode_move(cmd);
    cmd.buttons & buttons::SPRINT != 0
        && mz > 0.0
        && cmd.buttons & (buttons::CROUCH | buttons::PRONE) == 0
}

fn eye_position(auth: &PlayerAuthState, mp: &MoveParams) -> Vec3 {
    let h = crate::movement::eye_height(auth.stance_blend, mp);
    auth.pos + Vec3::new(0.0, h, 0.0) + crate::movement::lean::head_offset(auth, mp)
}

/// Advances `w` by one tick. `auth` is the player's state **after** this tick's movement.
pub fn step_weapon(
    w: &mut WeaponState,
    p: &WeaponParams,
    cmd: &InputCmd,
    auth: &PlayerAuthState,
    mp: &MoveParams,
    ctx: ShotCtx,
) -> WeaponOut {
    let mut out = WeaponOut::default();
    let u = TICK_UNITS;

    // --- timers ---
    w.since_shot_units = WeaponState::saturate_since(w.since_shot_units + u);
    if w.since_shot_units >= p.recoil_delay_units {
        recoil::recover(
            &mut w.recoil_pitch_deg,
            &mut w.recoil_yaw_deg,
            &mut w.bloom_deg,
            p.recover_deg_s,
            p.bloom_decay_deg_s,
            p.dt,
        );
    }
    match w.fsm {
        Fsm::Cooldown | Fsm::BoltCycle => {
            w.timer_units -= u;
            if w.timer_units <= 0 {
                w.fsm = if w.mag == 0 { Fsm::Empty } else { Fsm::Ready };
            }
        }
        Fsm::Reloading(_) => {
            w.timer_units -= u;
            if w.timer_units <= 0 {
                ammo::complete_reload(w, p.mag_capacity);
                w.fsm = if w.mag == 0 { Fsm::Empty } else { Fsm::Ready };
                w.timer_units = 0;
                out.reload_finished = true;
            }
        }
        Fsm::Ready | Fsm::Empty => {
            // Unused time of the last interval stays as credit, below one tick.
            w.timer_units = (w.timer_units - u).max(-(u - 1));
        }
    }

    let sprinting = is_sprinting(cmd);
    // Sprinting or vaulting interrupts a reload; the rounds are only moved when it completes,
    // so cancelling costs nothing (GAMEDESIGN 6.3, M1-013).
    if w.fsm.is_reloading() && (sprinting || auth.vault.is_some()) {
        w.fsm = if w.mag == 0 { Fsm::Empty } else { Fsm::Ready };
        w.timer_units = 0;
        out.reload_cancelled = true;
    }
    let reloading = matches!(w.fsm, Fsm::Reloading(_));

    // --- reload start ---
    if cmd.buttons & buttons::RELOAD != 0 && !reloading && ammo::can_reload(w, p.mag_capacity) {
        let kind = ammo::reload_kind(w);
        w.fsm = Fsm::Reloading(kind);
        w.timer_units = match kind {
            ReloadKind::Tactical => p.reload_tactical_units,
            ReloadKind::Empty => p.reload_empty_units,
        };
        out.reload_started = Some(kind);
    }
    let reloading = matches!(w.fsm, Fsm::Reloading(_));

    // --- ADS ---
    if cmd.buttons & buttons::AIM != 0 && !sprinting && !reloading {
        w.ads_units = (w.ads_units + u).min(p.ads_units.max(u));
    } else {
        w.ads_units = 0;
    }
    let ads_active = w.ads_units > 0 && w.ads_units >= p.ads_units;

    // --- fire ---
    let fire_down = cmd.buttons & buttons::FIRE != 0;
    let pulled = if p.needs_edge() {
        fire_down && !w.trigger_held
    } else {
        fire_down
    };
    w.trigger_held = fire_down;
    let blocked = sprinting || auth.stance_timer > 0 || auth.vault.is_some();
    if pulled && !blocked {
        match w.fsm {
            Fsm::Ready if w.mag > 0 => {
                out.shot = Some(fire(w, p, auth, mp, ctx, ads_active));
            }
            Fsm::Empty => out.dry_fire = true,
            _ => {}
        }
    }
    out
}

fn fire(
    w: &mut WeaponState,
    p: &WeaponParams,
    auth: &PlayerAuthState,
    mp: &MoveParams,
    ctx: ShotCtx,
    ads_active: bool,
) -> ShotSpawn {
    let mut rng = Rng::for_(ctx.match_seed, ctx.tick, ctx.entity, Purpose::Spread);
    let speed_h = libm::sqrtf(auth.vel.x * auth.vel.x + auth.vel.z * auth.vel.z);
    let base = if ads_active { p.ads_deg } else { p.hip_deg };
    let half_deg = spread::spread_deg(base, auth.stance, speed_h, w.bloom_deg)
        * crate::movement::lean::spread_mult(auth, mp);
    let yaw = auth.yaw + w.recoil_yaw_deg.to_radians();
    let pitch = (auth.pitch + w.recoil_pitch_deg.to_radians())
        .clamp(-core::f32::consts::FRAC_PI_2, core::f32::consts::FRAC_PI_2);
    let dir = spread::sample_cone(&mut rng, spread::view_dir(yaw, pitch), half_deg);
    let muzzle = eye_position(auth, mp) + dir * ballistics::MUZZLE_OFFSET_M;

    // Recoil after the shot has left the barrel.
    let (kp, ky) = recoil::kick(&mut rng, p.vkick_deg, p.hsigma_deg, auth.stance);
    w.recoil_pitch_deg += kp;
    w.recoil_yaw_deg += ky;
    w.bloom_deg += p.bloom_per_shot_deg;
    w.since_shot_units = 0;

    w.mag -= 1;
    w.fsm = p.cycling_state();
    // Carry the credit of the previous interval (<= 0) into this one.
    w.timer_units += p.interval_units;
    ShotSpawn {
        muzzle,
        dir,
        spread_seed: crate::rng::spread_seed(ctx.match_seed, ctx.tick, ctx.entity),
    }
}
