//! The M0 soldier brain: a small hand-written state machine (no behaviour tree before M1-038).
//!
//! ```text
//! Patrol --(enemy recognised)--> Engage --(target lost)--> chase last known position for
//!   ^                              |                         `memory_s`, then back to Patrol
//!   '------------------------------'
//! Dead: the bot is not in `players()`; it outputs a neutral command until the sim respawns it.
//! Reload: an overlay of the other modes (RELOAD pulse, no shooting, no sprint).
//! ```
//!
//! Cheap per-tick work (view smoothing, button building, fire gating) runs every tick; the
//! decision step (perception, mode, path) runs once per `think_interval_ms` (3 ticks) on the
//! shard `tick % 3 == bot_id % 3` (ARCHITECTURE 16.5). Randomness only at decision steps, from
//! `Rng::for_(seed, tick, bot_id, Purpose::Bot)`.

use std::sync::Arc;
use std::time::Instant;

use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_protocol::quant::{quant_yaw, rad_to_pitch_i16};
use frontline_sim::data::defs::{FireMode, Hist};
use frontline_sim::rng::{unit_f32, Pcg64Mcg, Purpose, Rng};
use frontline_sim::view::PlayerView;
use frontline_sim::weapon::{Fsm, TICK_UNITS};
use frontline_sim::{GameData, PlayerId, Tick, Vec3, WeaponState, WorldView};

use crate::aim::{aim_error_deg, angles_to, drop_m, step_angle, wrap_pi};
use crate::brain::{BotBrain, BrainStats};
use crate::ctx::BotCtx;
use crate::difficulty::{Tuning, TICK_HZ};
use crate::grid_nav::{NavGrid, PathFinder};
use crate::perception::{can_see, scan, Contact, EYE_CROUCH_M, EYE_STAND_M, TORSO_Y};

/// `InputCmd::aux` bit 0 (walk modifier); mirrors `frontline_sim::movement::AUX_WALK`.
const AUX_WALK: u8 = 1;
/// Ticks a stance change blocks shooting (sim: 250 ms transition, plus a margin).
const STANCE_BLOCK_TICKS: u32 = 10;
/// Ticks without own movement before a shot (the sim ramps velocity down over 0.2 s).
const SETTLE_TICKS: u32 = 5;
/// A target seen again within this many ticks keeps its reaction timer.
const REACQUIRE_GRACE_TICKS: u32 = 30;
/// Re-plan when the requested goal moved more than this (m).
const GOAL_MOVED_M: f32 = 2.0;
/// Limit of the view pitch the brain asks for (rad).
const PITCH_LIMIT: f32 = 1.45;

/// Static knowledge shared by all bots of a match: tuning, the nav grid, patrol points.
#[derive(Debug)]
pub struct BotMap {
    pub tuning: Tuning,
    /// `None`: no terrain (tests without collision); bots then stand still.
    pub nav: Option<NavGrid>,
    /// Patrol points (x, z) from `spawns.json`.
    pub patrol: Vec<[f32; 2]>,
}

impl BotMap {
    /// Builds the nav grid by sampling `view` over `[0, size] x [0, size]`.
    pub fn build(
        view: &dyn WorldView,
        size_m: (f32, f32),
        patrol: Vec<[f32; 2]>,
        tuning: Tuning,
    ) -> Self {
        let nav = NavGrid::build(
            view,
            size_m.0,
            size_m.1,
            tuning.cell_m,
            tuning.max_slope_deg,
        );
        BotMap {
            tuning,
            nav: (nav.walkable_count() > 0).then_some(nav),
            patrol,
        }
    }

    /// A map without navigation (bots stand still).
    pub fn without_nav(tuning: Tuning, patrol: Vec<[f32; 2]>) -> Self {
        BotMap {
            tuning,
            nav: None,
            patrol,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Patrol,
    Engage,
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mv {
    Stop,
    Go { dir: [f32; 2], sprint: bool },
    Strafe(i8),
}

#[derive(Debug, Clone, Copy)]
struct Decision {
    mv: Mv,
    crouch: bool,
    aim: bool,
    want_fire: bool,
    reload: bool,
    jump: bool,
}

impl Decision {
    const IDLE: Decision = Decision {
        mv: Mv::Stop,
        crouch: false,
        aim: false,
        want_fire: false,
        reload: false,
        jump: false,
    };
}

#[derive(Debug, Clone, Copy)]
struct TargetMem {
    player: PlayerId,
    last_pos: Vec3,
    /// Velocity estimated from two sightings (m/s, x/z); zero until seen twice.
    vel: [f32; 2],
    last_seen: u32,
    fire_not_before: u32,
    /// Aim error of this think step, degrees (yaw, pitch).
    aim_off: (f32, f32),
}

#[derive(Debug, Clone, Copy)]
struct WeaponInfo {
    mode: FireMode,
    muzzle_velocity: f32,
    gravity: f32,
    mag_capacity: u16,
    /// Time to settle into ADS, tick-milli-units (`ads_ms * tick_hz`).
    ads_units: i32,
}

impl WeaponInfo {
    fn of(def_id: u16) -> Option<Self> {
        let d = GameData::embedded().weapons.get(usize::from(def_id))?;
        Some(WeaponInfo {
            mode: d.gameplay.fire.mode,
            muzzle_velocity: d.gameplay.ballistics.muzzle_velocity_mps as f32,
            gravity: d.gameplay.ballistics.gravity_mps2 as f32,
            ads_units: (d.gameplay.ads_ms * f64::from(TICK_HZ)).round() as i32,
            mag_capacity: match d.historical.magazine_capacity {
                Hist::Known(n) => n as u16,
                Hist::Unverified(_) => 0,
            },
        })
    }

    fn needs_edge(&self) -> bool {
        self.mode != FireMode::Auto
    }
}

/// Whether a trigger pull on the next tick would fire (the sim advances timers before it
/// evaluates the trigger, so a cooldown with at most one tick left already counts as ready).
fn fires_next_tick(w: &WeaponState) -> bool {
    w.mag > 0
        && match w.fsm {
            Fsm::Ready => true,
            Fsm::Cooldown | Fsm::BoltCycle => w.timer_units <= TICK_UNITS,
            Fsm::Reloading(_) | Fsm::Empty => false,
        }
}

/// The M0 soldier. One instance per bot.
pub struct SoldierBrain {
    map: Arc<BotMap>,
    pf: PathFinder,
    wpn: Option<(u16, WeaponInfo)>,
    present: bool,
    seq: u16,
    yaw: f32,
    pitch: f32,
    want_yaw: f32,
    want_pitch: f32,
    mode: Mode,
    dec: Decision,
    patrol_goal: Option<usize>,
    patrol_last: Option<usize>,
    patrol_prev: Option<usize>,
    path: Vec<[f32; 2]>,
    path_i: usize,
    path_goal: [f32; 2],
    path_tick: u32,
    path_failed: bool,
    target: Option<TargetMem>,
    last_fire_down: bool,
    last_reload_down: bool,
    crouching: bool,
    stance_block_until: u32,
    last_move_tick: u32,
    strafe_until: u32,
    strafe_dir: i8,
    stuck_anchor: [f32; 2],
    stuck_tick: u32,
    stats: BrainStats,
    contacts: Vec<Contact>,
    /// How often each patrol point was reached (index = `spawns.json` order).
    pub patrol_reached: Vec<u32>,
    /// Stuck detections so far.
    pub stuck_total: u32,
}

impl SoldierBrain {
    pub fn new(map: Arc<BotMap>) -> Self {
        let n = map.patrol.len();
        SoldierBrain {
            map,
            pf: PathFinder::new(),
            wpn: None,
            present: false,
            seq: 0,
            yaw: 0.0,
            pitch: 0.0,
            want_yaw: 0.0,
            want_pitch: 0.0,
            mode: Mode::Dead,
            dec: Decision::IDLE,
            patrol_goal: None,
            patrol_last: None,
            patrol_prev: None,
            path: Vec::new(),
            path_i: 0,
            path_goal: [0.0; 2],
            path_tick: 0,
            path_failed: false,
            target: None,
            last_fire_down: false,
            last_reload_down: false,
            crouching: false,
            stance_block_until: 0,
            last_move_tick: 0,
            strafe_until: 0,
            strafe_dir: 0,
            stuck_anchor: [0.0; 2],
            stuck_tick: 0,
            stats: BrainStats::default(),
            contacts: Vec::new(),
            patrol_reached: vec![0; n],
            stuck_total: 0,
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The remembered / current target, if any.
    pub fn target_player(&self) -> Option<PlayerId> {
        self.target.map(|t| t.player)
    }

    fn on_spawn(&mut self, me: &PlayerView, tick: u32) {
        self.yaw = me.yaw;
        self.pitch = 0.0;
        self.want_yaw = me.yaw;
        self.want_pitch = 0.0;
        self.mode = Mode::Patrol;
        self.target = None;
        self.clear_path();
        self.patrol_goal = None;
        self.dec = Decision::IDLE;
        self.crouching = false;
        self.stance_block_until = 0;
        self.stuck_anchor = [me.pos.x, me.pos.z];
        self.stuck_tick = tick;
        self.last_move_tick = tick;
    }

    fn clear_path(&mut self) {
        self.path.clear();
        self.path_i = 0;
        self.path_failed = false;
        self.path_tick = 0;
    }

    fn weapon_info(&mut self, w: &WeaponState) -> Option<WeaponInfo> {
        match self.wpn {
            Some((id, info)) if id == w.def_id => Some(info),
            _ => {
                let info = WeaponInfo::of(w.def_id)?;
                self.wpn = Some((w.def_id, info));
                Some(info)
            }
        }
    }

    fn reaction_ticks(rng: &mut Pcg64Mcg, ctx: &BotCtx) -> u32 {
        let (lo, hi) = ctx.difficulty.reaction_ms;
        let ms = f32::from(lo) + unit_f32(rng) * f32::from(hi.saturating_sub(lo));
        (ms * TICK_HZ / 1000.0).ceil() as u32
    }

    // ---------------------------------------------------------------- decision step

    fn decide(&mut self, view: &dyn WorldView, ctx: &BotCtx, me: &PlayerView) {
        let map = self.map.clone();
        let t = &map.tuning;
        let diff = &ctx.difficulty;
        let tick = ctx.tick;
        let mut rng = Rng::for_(
            ctx.rng_seed,
            Tick(tick),
            u16::from(ctx.bot_id),
            Purpose::Bot,
        );
        let eye = me.pos
            + Vec3::Y
                * if self.crouching {
                    EYE_CROUCH_M
                } else {
                    EYE_STAND_M
                };
        let winfo = ctx.self_weapon.as_ref().and_then(|w| self.weapon_info(w));

        // --- perception ---------------------------------------------------------------
        let mut visible = false;
        // A target that is absent from the player list is simply "not visible": the bot cannot
        // know whether it died or hid, and keeps its last-known-position memory.
        let seen = self
            .target
            .and_then(|g| view.players().iter().find(|p| p.player == g.player))
            .filter(|p| can_see(view, eye, me.yaw, p.pos, t, diff))
            .copied();
        if let (Some(p), Some(g)) = (seen, self.target.as_mut()) {
            let dt = tick.saturating_sub(g.last_seen);
            if (1..=9).contains(&dt) {
                let k = TICK_HZ / dt as f32;
                let v = [(p.pos.x - g.last_pos.x) * k, (p.pos.z - g.last_pos.z) * k];
                g.vel = [0.5 * (g.vel[0] + v[0]), 0.5 * (g.vel[1] + v[1])];
            } else {
                g.vel = [0.0; 2];
            }
            g.last_pos = p.pos;
            g.last_seen = tick;
            visible = true;
        }
        if !visible {
            let mut contacts = std::mem::take(&mut self.contacts);
            scan(view, me, eye, t, diff, &mut rng, &mut contacts);
            if let Some(c) = contacts.first().copied() {
                let keep = self.target.filter(|g| {
                    g.player == c.player
                        || tick.saturating_sub(g.last_seen) <= REACQUIRE_GRACE_TICKS
                });
                let fire_not_before = match keep {
                    Some(g) => g.fire_not_before,
                    None => tick + Self::reaction_ticks(&mut rng, ctx),
                };
                self.target = Some(TargetMem {
                    player: c.player,
                    last_pos: c.pos,
                    vel: [0.0; 2],
                    last_seen: tick,
                    fire_not_before,
                    aim_off: (0.0, 0.0),
                });
                visible = true;
            }
            self.contacts = contacts;
        }

        // --- stuck detection (B-21) ---------------------------------------------------
        let here = [me.pos.x, me.pos.z];
        let moving_intent = matches!(self.dec.mv, Mv::Go { .. });
        if !moving_intent {
            self.stuck_anchor = here;
            self.stuck_tick = tick;
        } else {
            let moved = dist2d(here, self.stuck_anchor);
            if moved >= t.stuck_min_move_m {
                self.stuck_anchor = here;
                self.stuck_tick = tick;
            } else if tick.saturating_sub(self.stuck_tick) >= t.stuck_window_ticks {
                self.stuck_total += 1;
                self.stats.stuck_events += 1;
                self.stuck_anchor = here;
                self.stuck_tick = tick;
                self.clear_path();
                self.dec.jump = true;
                if self.mode == Mode::Patrol {
                    self.patrol_last = self.patrol_goal;
                    self.patrol_goal = None;
                }
            }
        }

        // --- mode + movement ----------------------------------------------------------
        // Nothing left to shoot with (no resupply in M0): do not stand around staring at enemies.
        let out_of_ammo = ctx
            .self_weapon
            .as_ref()
            .is_none_or(|w| w.mag == 0 && w.reserve == 0);
        if out_of_ammo {
            self.target = None;
            visible = false;
        }
        let memory_ticks = (diff.memory_s * TICK_HZ) as u32;
        let jump = self.dec.jump;
        let mut dec = Decision::IDLE;
        dec.jump = jump;
        match self.target {
            Some(g) if visible && !out_of_ammo => {
                self.mode = Mode::Engage;
                self.engage(view, ctx, me, eye, g, winfo, &mut rng, &mut dec);
            }
            Some(g) if tick.saturating_sub(g.last_seen) <= memory_ticks => {
                self.mode = Mode::Engage;
                self.chase(me, g, tick, &mut dec);
            }
            _ => {
                self.target = None;
                self.mode = Mode::Patrol;
                self.patrol(me, tick, ctx.bot_id, &mut rng, &mut dec);
            }
        }

        // --- reload overlay -------------------------------------------------------------
        if let Some(w) = ctx.self_weapon.as_ref() {
            let cap = winfo.map_or(0, |i| i.mag_capacity);
            let reloading = matches!(w.fsm, Fsm::Reloading(_));
            let empty = w.mag == 0;
            let low_and_calm = w.mag <= 1 && w.mag < cap && !visible;
            dec.reload = !reloading && w.reserve > 0 && (empty || low_and_calm);
            if reloading || empty {
                dec.want_fire = false;
            }
            if w.mag == 0 && w.reserve == 0 {
                dec.want_fire = false;
            }
        } else {
            dec.want_fire = false;
        }
        self.dec = dec;
    }

    #[allow(clippy::too_many_arguments)]
    fn engage(
        &mut self,
        _view: &dyn WorldView,
        ctx: &BotCtx,
        me: &PlayerView,
        eye: Vec3,
        mut g: TargetMem,
        winfo: Option<WeaponInfo>,
        rng: &mut Pcg64Mcg,
        dec: &mut Decision,
    ) {
        let map = self.map.clone();
        let t = &map.tuning;
        let tick = ctx.tick;
        let body = g.last_pos + Vec3::Y * TORSO_Y;
        let dist = (body - eye).length();
        // Lead a moving target by the time of flight plus the ticks until the shot lands.
        let tof = winfo.map_or(0.0, |i| dist / i.muzzle_velocity.max(1.0)) + 2.0 / TICK_HZ;
        let aim_pt = body + Vec3::new(g.vel[0] * tof, 0.0, g.vel[1] * tof);
        let hdist = dist2d([me.pos.x, me.pos.z], [g.last_pos.x, g.last_pos.z]);

        // Aim: torso centre + ballistic drop + gaussian error, minus the weapon's own recoil.
        let drop = winfo.map_or(0.0, |i| drop_m(dist, i.muzzle_velocity, i.gravity));
        let (yaw, pitch) = angles_to(eye, aim_pt + Vec3::Y * drop);
        g.aim_off = aim_error_deg(rng, dist, t, &ctx.difficulty);
        let (rec_y, rec_p) = ctx
            .self_weapon
            .as_ref()
            .map_or((0.0, 0.0), |w| (w.recoil_yaw_deg, w.recoil_pitch_deg));
        self.want_yaw = yaw + (g.aim_off.0 - rec_y).to_radians();
        self.want_pitch =
            (pitch + (g.aim_off.1 - rec_p).to_radians()).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        self.target = Some(g);

        dec.aim = dist > t.blind_fire_max_m;
        dec.crouch = hdist >= t.crouch_min_m;

        if dist > t.approach_beyond_m {
            // Too far for useful fire: close the distance first.
            dec.crouch = false;
            dec.aim = false;
            let goal = [g.last_pos.x, g.last_pos.z];
            if let Some(dir) = self.steer(me, goal, tick) {
                dec.mv = Mv::Go {
                    dir,
                    sprint: dist > t.approach_beyond_m + t.sprint_min_m,
                };
            }
            return;
        }

        // Shoot from standing still; before the reaction delay is over, shuffle a little.
        let waiting = tick < g.fire_not_before;
        dec.want_fire = !waiting;
        if waiting && !dec.crouch {
            if tick >= self.strafe_until {
                let r = unit_f32(rng);
                let dur = 10 + (unit_f32(rng) * 12.0) as u32;
                self.strafe_dir = if r < 0.30 {
                    1
                } else if r < 0.60 {
                    -1
                } else {
                    0
                };
                self.strafe_until = tick + dur;
            }
            if self.strafe_dir != 0 {
                dec.mv = Mv::Strafe(self.strafe_dir);
            }
        } else {
            self.strafe_until = 0;
        }
    }

    fn chase(&mut self, me: &PlayerView, g: TargetMem, tick: u32, dec: &mut Decision) {
        let goal = [g.last_pos.x, g.last_pos.z];
        if dist2d([me.pos.x, me.pos.z], goal) <= self.map.tuning.goal_radius_m {
            self.target = None; // reached the last known position: nothing there
            return;
        }
        match self.steer(me, goal, tick) {
            Some(dir) => {
                self.want_yaw = dir[0].atan2(dir[1]);
                self.want_pitch = 0.0;
                dec.mv = Mv::Go { dir, sprint: true };
            }
            None => self.target = None, // unreachable: give up
        }
    }

    fn patrol(
        &mut self,
        me: &PlayerView,
        tick: u32,
        bot_id: PlayerId,
        rng: &mut Pcg64Mcg,
        dec: &mut Decision,
    ) {
        let map = self.map.clone();
        if map.nav.is_none() || map.patrol.is_empty() {
            return;
        }
        let here = [me.pos.x, me.pos.z];
        for _ in 0..2 {
            if self.patrol_goal.is_none() {
                self.patrol_goal = self.pick_patrol(here, rng);
                self.clear_path();
            }
            let Some(gi) = self.patrol_goal else {
                return;
            };
            let goal = map.patrol[gi];
            if dist2d(here, goal) <= map.tuning.goal_radius_m {
                self.patrol_reached[gi] += 1;
                self.patrol_prev = self.patrol_last;
                self.patrol_last = Some(gi);
                self.patrol_goal = None;
                continue; // pick the next one in the same step
            }
            match self.steer(me, goal, tick) {
                Some(dir) => {
                    let heading = dir[0].atan2(dir[1]);
                    // Sweep the view +-35 deg around the heading so side contacts are possible.
                    let sweep = 0.6 * libm::sinf(tick as f32 * 0.07 + f32::from(bot_id));
                    self.want_yaw = heading + sweep;
                    self.want_pitch = 0.0;
                    let far = dist2d(here, goal) > map.tuning.sprint_min_m;
                    dec.mv = Mv::Go { dir, sprint: far };
                }
                None => {
                    // Unreachable (or no path yet): try another point next step.
                    self.patrol_last = self.patrol_goal;
                    self.patrol_goal = None;
                }
            }
            return;
        }
    }

    /// One of the 2 patrol points nearest to `here`, at random, never the last two visited.
    fn pick_patrol(&self, here: [f32; 2], rng: &mut Pcg64Mcg) -> Option<usize> {
        let pts = &self.map.patrol;
        let mut cand: Vec<(f32, usize)> = (0..pts.len())
            .filter(|i| Some(*i) != self.patrol_last && Some(*i) != self.patrol_prev)
            .map(|i| (dist2d(here, pts[i]), i))
            .filter(|(d, _)| *d > self.map.tuning.goal_radius_m)
            .collect();
        cand.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        cand.truncate(2);
        if cand.is_empty() {
            return None;
        }
        let k = (unit_f32(rng) * cand.len() as f32) as usize;
        Some(cand[k.min(cand.len() - 1)].1)
    }

    /// Unit direction toward the next waypoint on the way to `goal` (planning a path when
    /// needed, at most every `repath_min_ticks`); `None` when the goal cannot be reached.
    fn steer(&mut self, me: &PlayerView, goal: [f32; 2], tick: u32) -> Option<[f32; 2]> {
        let map = self.map.clone();
        let here = [me.pos.x, me.pos.z];
        let Some(nav) = map.nav.as_ref() else {
            return unit_dir(here, goal);
        };
        let goal_moved = dist2d(self.path_goal, goal) > GOAL_MOVED_M;
        let need_plan = (self.path.is_empty() && !self.path_failed) || goal_moved;
        if need_plan
            && (self.path_tick == 0
                || tick.saturating_sub(self.path_tick) >= map.tuning.repath_min_ticks)
        {
            let t0 = Instant::now();
            let ok = self.pf.find(nav, here, goal, &mut self.path);
            self.stats.path_ns += t0.elapsed().as_nanos() as u64;
            self.stats.path_queries += 1;
            self.path_tick = tick.max(1);
            self.path_goal = goal;
            self.path_i = 0;
            self.path_failed = !ok;
        }
        if self.path_failed {
            return None;
        }
        // Follow: skip reached waypoints, steer at the first one still ahead.
        while self.path_i + 1 < self.path.len()
            && dist2d(here, self.path[self.path_i]) <= map.tuning.goal_radius_m
        {
            self.path_i += 1;
        }
        let wp = *self.path.get(self.path_i)?;
        unit_dir(here, wp).or_else(|| unit_dir(here, goal))
    }

    // ---------------------------------------------------------------- per-tick output

    fn act(&mut self, ctx: &BotCtx) -> InputCmd {
        let t = &self.map.tuning;
        let tick = ctx.tick;
        let max_step = t.turn_rate_deg_s.to_radians() / TICK_HZ;
        self.yaw = step_angle(self.yaw, self.want_yaw, max_step);
        self.pitch =
            step_angle(self.pitch, self.want_pitch, max_step).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        let dec = self.dec;
        let (mut lx, mut lz) = (0.0f32, 0.0f32);
        let mut aux = 0u8;
        let mut sprint_wanted = false;
        match dec.mv {
            Mv::Stop => {}
            Mv::Go { dir, sprint } => {
                let (sy, cy) = (libm::sinf(self.yaw), libm::cosf(self.yaw));
                lz = dir[0] * sy + dir[1] * cy;
                lx = dir[0] * cy - dir[1] * sy;
                sprint_wanted = sprint && lz > 0.3;
            }
            Mv::Strafe(s) => {
                lx = f32::from(s);
                aux |= AUX_WALK;
            }
        }
        let moving = lx != 0.0 || lz != 0.0;
        if moving {
            self.last_move_tick = tick;
        }

        let mut b = 0u16;
        let crouch = dec.crouch && !sprint_wanted;
        if crouch != self.crouching {
            self.crouching = crouch;
            self.stance_block_until = tick + STANCE_BLOCK_TICKS;
        }
        if crouch {
            b |= buttons::CROUCH;
        }

        // Fire gate: decided every tick because the weapon state changes every tick.
        let mut fire = false;
        if let (true, Some(w)) = (dec.want_fire, ctx.self_weapon.as_ref()) {
            let edge = self.wpn.is_none_or(|(_, i)| i.needs_edge());
            let tol = t.fire_tolerance_deg.to_radians();
            let on_target = wrap_pi(self.want_yaw - self.yaw).abs() <= tol
                && (self.want_pitch - self.pitch).abs() <= tol;
            // With AIM held the shot only counts once the sights are up (spread blends in).
            let sights_up = !dec.aim
                || self
                    .wpn
                    .is_none_or(|(_, i)| w.ads_units + TICK_UNITS >= i.ads_units);
            fire = on_target
                && sights_up
                && fires_next_tick(w)
                && tick >= self.stance_block_until
                && tick >= self.last_move_tick + SETTLE_TICKS
                && !(edge && self.last_fire_down)
                && !sprint_wanted;
        }
        self.last_fire_down = fire;
        if fire {
            b |= buttons::FIRE;
        }
        if sprint_wanted && !fire && !crouch {
            b |= buttons::SPRINT;
        }
        if dec.aim && b & buttons::SPRINT == 0 {
            b |= buttons::AIM;
        }
        // Re-checked against the live weapon state: the decision may be up to 2 ticks old.
        let can_reload = ctx
            .self_weapon
            .as_ref()
            .is_some_and(|w| !matches!(w.fsm, Fsm::Reloading(_)) && w.reserve > 0);
        let reload = dec.reload && can_reload && !self.last_reload_down;
        self.last_reload_down = reload;
        if reload {
            b |= buttons::RELOAD;
        }
        if self.dec.jump {
            b |= buttons::JUMP;
            self.dec.jump = false;
        }

        // 127 * unit vector truncated: never longer than the validator's tolerance.
        let q = |v: f32| (v.clamp(-1.0, 1.0) * 127.0) as i8;
        InputCmd {
            seq: self.seq,
            tick,
            move_x: q(lx),
            move_z: q(lz),
            yaw: quant_yaw(self.yaw, 16).code,
            pitch: rad_to_pitch_i16(self.pitch).code,
            buttons: b,
            weapon_slot: 0,
            aux,
            view_tick: 0,
        }
    }

    fn dead_cmd(&mut self, ctx: &BotCtx) -> InputCmd {
        if self.present {
            self.present = false;
            self.mode = Mode::Dead;
            self.target = None;
            self.dec = Decision::IDLE;
            self.clear_path();
            self.last_fire_down = false;
            self.last_reload_down = false;
        }
        InputCmd {
            seq: self.seq,
            tick: ctx.tick,
            move_x: 0,
            move_z: 0,
            yaw: quant_yaw(self.yaw, 16).code,
            pitch: rad_to_pitch_i16(self.pitch).code,
            buttons: 0,
            weapon_slot: 0,
            aux: 0,
            view_tick: 0,
        }
    }
}

fn dist2d(a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
    libm::sqrtf(dx * dx + dz * dz)
}

fn unit_dir(from: [f32; 2], to: [f32; 2]) -> Option<[f32; 2]> {
    let (dx, dz) = (to[0] - from[0], to[1] - from[1]);
    let l = libm::sqrtf(dx * dx + dz * dz);
    (l > 1e-3).then(|| [dx / l, dz / l])
}

impl BotBrain for SoldierBrain {
    fn think(&mut self, view: &dyn WorldView, ctx: &BotCtx) -> InputCmd {
        self.seq = self.seq.wrapping_add(1);
        let Some(me) = view
            .players()
            .iter()
            .find(|p| p.player == ctx.bot_id)
            .copied()
        else {
            return self.dead_cmd(ctx);
        };
        if !self.present {
            self.present = true;
            self.on_spawn(&me, ctx.tick);
        }
        let every = self.map.tuning.think_every_ticks.max(1);
        let shard = u32::from(ctx.bot_id) % every;
        if ctx.tick % every == shard {
            self.stats.thought = true;
            self.decide(view, ctx, &me);
        }
        self.act(ctx)
    }

    fn take_stats(&mut self) -> BrainStats {
        std::mem::take(&mut self.stats)
    }
}
