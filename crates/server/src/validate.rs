//! Input validation: the **single entry point** [`validate_input`] shared by human players and
//! bots (M0-018; ARCHITECTURE §6.3, §15.1). Client input is untrusted: nothing here relies on
//! the client being well behaved, and nothing panics on any field combination.
//!
//! Outcomes of one command:
//! * accepted unchanged;
//! * accepted but **clamped** (move vector too long, yaw/pitch turning faster than the limit);
//! * **rejected** (replayed seq, reserved button bits, unknown weapon slot, ...).
//!
//! Every non-clean outcome is counted by the caller under `input_rejected_total{reason}`.
//! Replays and stale commands are normal with redundant bundles (each bundle repeats the two
//! previous commands) and never count towards the kick streak; everything else does.

use frontline_protocol::msg::input::{buttons, FireCmd, InputCmd};
use frontline_sim::{combat::lagcomp::origin_needs_correction, Vec3};

/// The client hint is telemetry only. Omit points the wire grid cannot represent instead of
/// comparing a snapped edge coordinate with an out-of-grid authoritative position.
pub(crate) fn fire_origin_hint_corrected(codes: [u16; 3], muzzle: Vec3) -> bool {
    FireCmd::decode_origin_hint(codes).is_some_and(|hint| {
        FireCmd::encode_origin_hint(muzzle.to_array()).is_some()
            && origin_needs_correction(Vec3::from_array(hint), muzzle)
    })
}

/// Consecutive violating commands after which the connection is closed.
pub const KICK_STREAK: u32 = 30;
/// Highest accepted `weapon_slot` (primary, secondary, melee, two gadgets; M0 uses slot 0).
pub const MAX_WEAPON_SLOT: u8 = 4;
/// `buttons` bits 14 and 15 are reserved and must be zero.
pub const RESERVED_BUTTONS: u16 = 0xC000;
/// Tolerated move vector length, in 1/127 units: `hypot(x, z) / 127 <= 1 + 1/127`.
pub const MAX_MOVE_LEN: f32 = 128.0;

/// A command stamped further ahead than this many ticks (10 s) is not a clock error but a forged
/// or corrupt tick: it counts towards the kick streak. Anything between `now + MAX_FUTURE` and
/// this is plain clock skew (a client that stalled or measured its RTT while its main thread was
/// blocked, e.g. a software-rendered browser at 1-5 fps): the command is dropped and counted as
/// `too_far`, but a legitimate client must not be kicked for it (M0-030).
pub const FAR_FUTURE_TICKS: u32 = 300;

/// Why an input was refused, clamped or dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    /// `seq` is not newer than the newest accepted one (also: a tick already buffered).
    Duplicate,
    /// `seq` is newer but `tick` is not.
    TickRegress,
    /// `tick` is older than the accepted window (or older than what the ring slot holds).
    TooOld,
    /// `tick` is beyond the accepted window by at most [`FAR_FUTURE_TICKS`] (clock skew; dropped,
    /// not a violation).
    TooFar,
    /// `tick` is more than [`FAR_FUTURE_TICKS`] ahead: forged or corrupt (a violation).
    FarFuture,
    /// A buffered, not yet consumed command was overwritten by a newer one (ring wrap).
    RingOverwrite,
    ReservedBits,
    WeaponSlot,
    /// A move axis was `i8::MIN` (not representable on the wire; only a bot can produce it).
    BadMove,
    /// Clamp: move vector longer than allowed (scaled down).
    MoveClamped,
    /// Clamp: yaw turned faster than the configured turn cap.
    YawRate,
    /// Clamp: pitch turned faster than the configured turn cap.
    PitchRate,
    /// C1 for a player without a joined session.
    NoSession,
    /// C0 message over its rate limit (Ping 5/s, others 10/s).
    C0Rate,
}

impl Reject {
    /// Label values of `input_rejected_total{reason}`.
    pub const LABELS: &'static [&'static str] = &[
        "duplicate",
        "tick_regress",
        "too_old",
        "too_far",
        "far_future",
        "ring_overwrite",
        "reserved_bits",
        "weapon_slot",
        "bad_move",
        "move_clamped",
        "yaw_rate",
        "pitch_rate",
        "no_session",
        "c0_rate",
    ];

    pub fn label(self) -> &'static str {
        match self {
            Reject::Duplicate => "duplicate",
            Reject::TickRegress => "tick_regress",
            Reject::TooOld => "too_old",
            Reject::TooFar => "too_far",
            Reject::FarFuture => "far_future",
            Reject::RingOverwrite => "ring_overwrite",
            Reject::ReservedBits => "reserved_bits",
            Reject::WeaponSlot => "weapon_slot",
            Reject::BadMove => "bad_move",
            Reject::MoveClamped => "move_clamped",
            Reject::YawRate => "yaw_rate",
            Reject::PitchRate => "pitch_rate",
            Reject::NoSession => "no_session",
            Reject::C0Rate => "c0_rate",
        }
    }

    /// Whether this outcome counts towards the 30-in-a-row kick. Network artefacts (replays,
    /// late or early packets, clock skew) and rate drops do not; malformed or abusive values do.
    pub fn is_violation(self) -> bool {
        matches!(
            self,
            Reject::FarFuture
                | Reject::ReservedBits
                | Reject::WeaponSlot
                | Reject::BadMove
                | Reject::MoveClamped
                | Reject::YawRate
                | Reject::PitchRate
        )
    }
}

/// Which clamps were applied to an accepted command.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Clamps(u8);

impl Clamps {
    const MOVE: u8 = 1;
    const YAW: u8 = 2;
    const PITCH: u8 = 4;

    pub fn any(self) -> bool {
        self.0 != 0
    }
    pub fn reasons(self) -> impl Iterator<Item = Reject> {
        [
            (Self::MOVE, Reject::MoveClamped),
            (Self::YAW, Reject::YawRate),
            (Self::PITCH, Reject::PitchRate),
        ]
        .into_iter()
        .filter(move |(bit, _)| self.0 & bit != 0)
        .map(|(_, r)| r)
    }
}

/// An accepted command (possibly clamped).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Validated {
    pub cmd: InputCmd,
    pub clamps: Clamps,
}

/// Tunables of the validator (production turn limit is loaded from the anti-cheat rules).
#[derive(Debug, Clone, Copy)]
pub struct ValidateCfg {
    pub max_weapon_slot: u8,
    pub max_turn_dps: f32,
    pub tick_hz: f32,
}

impl Default for ValidateCfg {
    fn default() -> Self {
        ValidateCfg {
            max_weapon_slot: MAX_WEAPON_SLOT,
            max_turn_dps: crate::anticheat::RULES.view_max_deg_s,
            tick_hz: 30.0,
        }
    }
}

/// Per-player input bookkeeping (ARCHITECTURE §6.3).
#[derive(Debug, Clone)]
pub struct PlayerInputState {
    /// Newest accepted seq / tick (valid when `seen`).
    pub last_seq: u16,
    pub last_tick: u32,
    /// Newest accepted view angles (raw wire codes), the reference of the turn-rate check.
    pub last_yaw: u16,
    pub last_pitch: i16,
    /// Consecutive ticks resolved without an input of the player's own.
    pub gap: u32,
    pub last_ack_snapshot_tick: u16,
    pub last_pred_hash: u8,
    /// Up to two distinct `FireCmd`s that have not yet passed their input sequence.
    pub pending_fire: [Option<FireCmd>; 2],
    /// A command has been accepted at least once.
    pub seen: bool,
    /// Consecutive violating commands (reset by a clean accept).
    pub violation_streak: u32,
    pub cfg: ValidateCfg,
}

impl PlayerInputState {
    pub fn new(cfg: ValidateCfg) -> Self {
        PlayerInputState {
            last_seq: 0,
            last_tick: 0,
            last_yaw: 0,
            last_pitch: 0,
            gap: 0,
            last_ack_snapshot_tick: 0,
            last_pred_hash: 0,
            pending_fire: [None, None],
            seen: false,
            violation_streak: 0,
            cfg,
        }
    }

    /// Whether the kick threshold has been reached.
    pub fn should_kick(&self) -> bool {
        self.violation_streak >= KICK_STREAK
    }

    /// Records an outcome produced outside [`validate_input`] (window checks) in the streak.
    pub fn note(&mut self, r: Reject) {
        if r.is_violation() {
            self.violation_streak = self.violation_streak.saturating_add(1);
        }
    }

    /// Stores a redundant fire command (dedupe by `seq`, keep the newest two).
    /// Returns `Ok(false)` for a duplicate.
    pub fn record_fire(&mut self, f: &FireCmd) -> Result<bool, Reject> {
        if f.weapon > self.cfg.max_weapon_slot {
            return Err(Reject::WeaponSlot);
        }
        if self.pending_fire.iter().flatten().any(|p| p.seq == f.seq) {
            return Ok(false);
        }
        self.pending_fire[1] = self.pending_fire[0];
        self.pending_fire[0] = Some(*f);
        Ok(true)
    }

    /// Consumes the hint tied to the input just executed and retires older hints. A hint never
    /// produces its own shot or rolls over into a later input with the same sequence number.
    pub(crate) fn take_fire(&mut self, input_seq: u16) -> Option<FireCmd> {
        let mut matched = None;
        for slot in &mut self.pending_fire {
            if let Some(f) = slot {
                if f.seq == input_seq {
                    matched = slot.take();
                } else if !seq_newer(f.seq, input_seq) {
                    *slot = None;
                }
            }
        }
        if self.pending_fire[0].is_none() {
            self.pending_fire.swap(0, 1);
        }
        matched
    }
}

/// `a` is newer than `b` in 16-bit wrap-around order.
#[inline]
pub fn seq_newer(a: u16, b: u16) -> bool {
    (a.wrapping_sub(b) as i16) > 0
}

/// Validates one command against the player's state. On acceptance the state advances
/// (`last_seq/tick/yaw/pitch`) and the streak is cleared (clean) or extended (clamped); on
/// rejection only the streak may move. Never panics.
pub fn validate_input(state: &mut PlayerInputState, cmd: &InputCmd) -> Result<Validated, Reject> {
    let r = validate_inner(state, cmd);
    match &r {
        Ok(v) if v.clamps.any() => {
            state.violation_streak = state.violation_streak.saturating_add(1)
        }
        Ok(_) => state.violation_streak = 0,
        Err(e) => state.note(*e),
    }
    r
}

fn validate_inner(state: &mut PlayerInputState, cmd: &InputCmd) -> Result<Validated, Reject> {
    if state.seen {
        if !seq_newer(cmd.seq, state.last_seq) {
            return Err(Reject::Duplicate);
        }
        if cmd.tick <= state.last_tick {
            return Err(Reject::TickRegress);
        }
    }
    if cmd.buttons & RESERVED_BUTTONS != 0 {
        return Err(Reject::ReservedBits);
    }
    if cmd.weapon_slot > state.cfg.max_weapon_slot {
        return Err(Reject::WeaponSlot);
    }
    if cmd.move_x == i8::MIN || cmd.move_z == i8::MIN {
        return Err(Reject::BadMove);
    }

    let mut out = *cmd;
    let mut clamps = 0u8;

    // Move vector: length <= 1 (+ one quantisation step).
    let (mx, mz) = (cmd.move_x as f32, cmd.move_z as f32);
    let len = mx.hypot(mz);
    if len > MAX_MOVE_LEN {
        let k = 127.0 / len;
        // `as i32` truncates towards zero, so the clamped vector is never longer than 127.
        out.move_x = ((mx * k) as i32).clamp(-127, 127) as i8;
        out.move_z = ((mz * k) as i32).clamp(-127, 127) as i8;
        clamps |= Clamps::MOVE;
    }

    // Turn rate against the previous accepted command.
    if state.seen {
        let dt = cmd.tick.wrapping_sub(state.last_tick).clamp(1, 30) as f32;
        let max_deg = state.cfg.max_turn_dps * dt / state.cfg.tick_hz;
        let d_yaw = cmd.yaw.wrapping_sub(state.last_yaw) as i16 as i32;
        let yaw_deg = d_yaw.unsigned_abs() as f32 * (360.0 / 65536.0);
        if yaw_deg > max_deg {
            let allowed = (max_deg * (65536.0 / 360.0)) as i32;
            let step = if d_yaw < 0 { -allowed } else { allowed };
            out.yaw = state.last_yaw.wrapping_add(step as i16 as u16);
            clamps |= Clamps::YAW;
        }
        let d_pitch = cmd.pitch as i32 - state.last_pitch as i32;
        let pitch_deg = d_pitch.unsigned_abs() as f32 * (180.0 / 65535.0);
        if pitch_deg > max_deg {
            let allowed = (max_deg * (65535.0 / 180.0)) as i32;
            let step = if d_pitch < 0 { -allowed } else { allowed };
            out.pitch = (state.last_pitch as i32 + step).clamp(-32768, 32767) as i16;
            clamps |= Clamps::PITCH;
        }
    }

    state.seen = true;
    state.last_seq = cmd.seq;
    state.last_tick = cmd.tick;
    state.last_yaw = out.yaw;
    state.last_pitch = out.pitch;
    Ok(Validated {
        cmd: out,
        clamps: Clamps(clamps),
    })
}

/// Edge-triggered button bits: never repeated when an input is re-used for a missing tick.
pub const EDGE_BUTTONS: u16 = buttons::FIRE
    | buttons::RELOAD
    | buttons::JUMP
    | buttons::USE
    | buttons::MELEE
    | buttons::GADGET
    | buttons::SWAP;
/// `aux` bit 0 is the walk modifier (level); the other bits belong to fire/one-shot actions.
pub const AUX_LEVEL_MASK: u8 = 0x01;

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(seq: u16, tick: u32) -> InputCmd {
        InputCmd {
            seq,
            tick,
            move_x: 0,
            move_z: 0,
            yaw: 0,
            pitch: 0,
            buttons: 0,
            weapon_slot: 0,
            aux: 0,
            view_tick: 0,
        }
    }

    fn st() -> PlayerInputState {
        PlayerInputState::new(ValidateCfg::default())
    }

    #[test]
    fn seq_comparison_wraps() {
        assert!(seq_newer(1, 0));
        assert!(seq_newer(0, 65535));
        assert!(seq_newer(5, 65530));
        assert!(!seq_newer(65535, 0));
        assert!(!seq_newer(7, 7));
        assert!(!seq_newer(100, 101));
        // Exactly half the range apart is "older" (i16 sign).
        assert!(!seq_newer(0x8000, 0));
    }

    #[test]
    fn monotonic_seq_across_the_wrap() {
        let mut s = st();
        for (t, seq) in (100..).zip([65533u16, 65534, 65535, 0, 1, 2]) {
            assert!(validate_input(&mut s, &cmd(seq, t)).is_ok(), "seq {seq}");
        }
        assert_eq!(
            validate_input(&mut s, &cmd(0, 200)).unwrap_err(),
            Reject::Duplicate
        );
        assert_eq!(
            validate_input(&mut s, &cmd(2, 200)).unwrap_err(),
            Reject::Duplicate
        );
    }

    #[test]
    fn tick_must_advance_with_seq() {
        let mut s = st();
        validate_input(&mut s, &cmd(1, 50)).unwrap();
        assert_eq!(
            validate_input(&mut s, &cmd(2, 50)).unwrap_err(),
            Reject::TickRegress
        );
        assert_eq!(
            validate_input(&mut s, &cmd(3, 49)).unwrap_err(),
            Reject::TickRegress
        );
        assert!(validate_input(&mut s, &cmd(4, 51)).is_ok());
    }

    #[test]
    fn move_vector_longer_than_one_is_clamped() {
        let mut s = st();
        let mut c = cmd(1, 1);
        // 1.2 * 127 = 152 along x.
        c.move_x = 127;
        c.move_z = 84; // hypot = 152.3 -> 1.2
        let v = validate_input(&mut s, &c).unwrap();
        assert!(v.clamps.reasons().any(|r| r == Reject::MoveClamped));
        let len = (v.cmd.move_x as f32).hypot(v.cmd.move_z as f32) / 127.0;
        assert!(len <= 1.0 + 1e-6, "len {len}");
        assert!(len > 0.97, "clamped too hard: {len}");
        // Direction preserved.
        let ratio_in = 84.0 / 127.0;
        let ratio_out = v.cmd.move_z as f32 / v.cmd.move_x as f32;
        assert!((ratio_in - ratio_out).abs() < 0.02);
        // Exactly 1.0 and the 1/127 tolerance pass untouched.
        for (x, z) in [(127, 0), (0, -127), (90, 90), (110, 110), (64, 110)] {
            let mut s2 = st();
            let mut c = cmd(1, 1);
            c.move_x = x;
            c.move_z = z;
            let v = validate_input(&mut s2, &c).unwrap();
            assert_eq!(
                v.clamps.any(),
                (x as f32).hypot(z as f32) > MAX_MOVE_LEN,
                "{x},{z}"
            );
        }
    }

    #[test]
    fn extreme_move_values_never_panic() {
        for x in [i8::MIN, -127, 0, 127, i8::MAX] {
            for z in [i8::MIN, -127, 0, 127, i8::MAX] {
                let mut s = st();
                let mut c = cmd(1, 1);
                c.move_x = x;
                c.move_z = z;
                match validate_input(&mut s, &c) {
                    Ok(v) => {
                        let len = (v.cmd.move_x as f32).hypot(v.cmd.move_z as f32);
                        assert!(len <= MAX_MOVE_LEN + 1e-3);
                    }
                    Err(e) => assert_eq!(e, Reject::BadMove),
                }
            }
        }
    }

    #[test]
    fn reserved_bits_weapon_slot_and_fire_weapon_are_rejected() {
        let mut s = st();
        let mut c = cmd(1, 1);
        c.buttons = 1 << 14;
        assert_eq!(
            validate_input(&mut s, &c).unwrap_err(),
            Reject::ReservedBits
        );
        c.buttons = 1 << 15;
        assert_eq!(
            validate_input(&mut s, &c).unwrap_err(),
            Reject::ReservedBits
        );
        c.buttons = 0x3FFF;
        c.weapon_slot = MAX_WEAPON_SLOT + 1;
        assert_eq!(validate_input(&mut s, &c).unwrap_err(), Reject::WeaponSlot);
        c.weapon_slot = MAX_WEAPON_SLOT;
        assert!(validate_input(&mut s, &c).is_ok());
        let f = FireCmd {
            seq: 1,
            view_tick: 0,
            weapon: 200,
            dir_yaw: 0,
            dir_pitch: 0,
            origin_hint: [0; 3],
            spread_seed: 0,
        };
        assert_eq!(s.record_fire(&f).unwrap_err(), Reject::WeaponSlot);
    }

    #[test]
    fn rate_limit_with_default_cfg_is_unreachable_for_one_tick() {
        // 8000 deg/s * 1/30 s = 266 deg > 180 deg, the largest change a wrapped 16-bit yaw (or a
        // 16-bit pitch spanning 180 deg) can express between two ticks: nothing can trip the
        // default limit at 1-tick spacing (PLAN_ISSUES PI-022).
        let mut s = st();
        validate_input(&mut s, &cmd(1, 1)).unwrap();
        let mut c = cmd(2, 2);
        c.yaw = 0x8000; // 180 deg
        c.pitch = i16::MAX;
        let v = validate_input(&mut s, &c).unwrap();
        assert!(!v.clamps.any());
    }

    #[test]
    fn turn_rate_over_the_limit_is_clamped_and_counted_with_a_lower_cap() {
        let cfg = ValidateCfg {
            max_turn_dps: 3000.0, // 100 deg per tick
            ..ValidateCfg::default()
        };
        let mut s = PlayerInputState::new(cfg);
        let mut first = cmd(1, 1);
        first.pitch = -30000;
        validate_input(&mut s, &first).unwrap();
        let mut c = cmd(2, 2);
        c.yaw = (150.0 / 360.0 * 65536.0) as u16; // +150 deg in one tick
        c.pitch = 30000; // +165 deg
        let v = validate_input(&mut s, &c).unwrap();
        let reasons: Vec<_> = v.clamps.reasons().collect();
        assert_eq!(reasons, vec![Reject::YawRate, Reject::PitchRate]);
        let yaw_deg = v.cmd.yaw as f32 * 360.0 / 65536.0;
        assert!((yaw_deg - 100.0).abs() < 0.1, "{yaw_deg}");
        let moved = (v.cmd.pitch as i32 + 30000) as f32 * 180.0 / 65535.0;
        assert!((moved - 100.0).abs() < 0.1, "{moved}");
        // The reference moves with the clamped value, not the claimed one.
        assert_eq!(s.last_yaw, v.cmd.yaw);
        assert_eq!(s.last_pitch, v.cmd.pitch);
        // Two ticks later the allowance doubles (200 deg): +150 deg is fine.
        let mut c = cmd(3, 4);
        c.yaw = v.cmd.yaw.wrapping_add((150.0 / 360.0 * 65536.0) as u16);
        c.pitch = v.cmd.pitch;
        let v2 = validate_input(&mut s, &c).unwrap();
        assert!(!v2.clamps.any());
        // Negative direction clamps towards the old value too.
        let mut s = PlayerInputState::new(cfg);
        let mut a = cmd(1, 1);
        a.yaw = 0x8000;
        validate_input(&mut s, &a).unwrap();
        let mut c = cmd(2, 2);
        c.yaw = (0x8000u32 - 20000) as u16; // -110 deg
        let v = validate_input(&mut s, &c).unwrap();
        assert!(v.clamps.reasons().any(|r| r == Reject::YawRate));
    }

    #[test]
    fn streak_counts_violations_resets_on_clean_and_ignores_replays() {
        fn bad(s: &mut PlayerInputState, seq: u16) -> Reject {
            let mut c = cmd(seq, seq as u32);
            c.buttons = 1 << 15;
            validate_input(s, &c).unwrap_err()
        }
        let mut s = st();
        // Violating commands still advance nothing, so any seq is fine for them.
        for i in 0..29 {
            assert_eq!(bad(&mut s, i), Reject::ReservedBits);
        }
        assert!(!s.should_kick());
        // A clean command resets.
        validate_input(&mut s, &cmd(100, 100)).unwrap();
        assert_eq!(s.violation_streak, 0);
        for i in 0..29 {
            bad(&mut s, 101 + i);
        }
        // Replays are neutral.
        assert_eq!(
            validate_input(&mut s, &cmd(100, 100)).unwrap_err(),
            Reject::Duplicate
        );
        assert!(!s.should_kick());
        bad(&mut s, 200);
        assert!(s.should_kick());
        assert_eq!(s.violation_streak, KICK_STREAK);
    }

    #[test]
    fn fire_cmds_are_deduped_and_keep_the_newest_two() {
        let mut s = st();
        let f = |seq| FireCmd {
            seq,
            view_tick: 0,
            weapon: 0,
            dir_yaw: 0,
            dir_pitch: 0,
            origin_hint: [0; 3],
            spread_seed: 0,
        };
        assert_eq!(s.record_fire(&f(1)), Ok(true));
        assert_eq!(s.record_fire(&f(1)), Ok(false));
        assert_eq!(s.record_fire(&f(2)), Ok(true));
        assert_eq!(s.record_fire(&f(2)), Ok(false));
        assert_eq!(s.record_fire(&f(1)), Ok(false));
        assert_eq!(s.record_fire(&f(3)), Ok(true));
        assert_eq!(s.pending_fire[0].unwrap().seq, 3);
        assert_eq!(s.pending_fire[1].unwrap().seq, 2);
    }

    #[test]
    fn fire_metadata_consumes_exact_seq_and_retires_stale_hints_across_wrap() {
        let mut s = PlayerInputState::new(ValidateCfg::default());
        let f = |seq| FireCmd {
            seq,
            view_tick: 0,
            weapon: 0,
            dir_yaw: 0,
            dir_pitch: 0,
            origin_hint: [0; 3],
            spread_seed: 0,
        };
        assert_eq!(s.record_fire(&f(u16::MAX - 1)), Ok(true));
        assert_eq!(s.record_fire(&f(0)), Ok(true));
        assert_eq!(s.take_fire(u16::MAX), None);
        assert_eq!(s.pending_fire, [Some(f(0)), None]);
        assert_eq!(s.take_fire(0), Some(f(0)));
        assert_eq!(s.take_fire(0), None);
    }

    #[test]
    fn fire_origin_wire_grid_boundary_and_range() {
        let muzzle = Vec3::new(2048.0, -7.0, 1024.0);
        let codes =
            |dx: f32| FireCmd::encode_origin_hint((muzzle + Vec3::X * dx).to_array()).unwrap();
        assert!(!fire_origin_hint_corrected(codes(0.5), muzzle));
        assert!(fire_origin_hint_corrected(codes(0.75), muzzle));
        assert!(!fire_origin_hint_corrected([0; 3], muzzle));
        assert!(!fire_origin_hint_corrected([0x4000, 128, 640], muzzle));
        for x in [-1.0, 4096.0, f32::NAN] {
            assert!(!fire_origin_hint_corrected(
                codes(0.75),
                Vec3::new(x, -7.0, 1024.0)
            ));
        }
    }

    #[test]
    fn labels_cover_every_variant() {
        use std::collections::HashSet;
        let all = [
            Reject::Duplicate,
            Reject::TickRegress,
            Reject::TooOld,
            Reject::TooFar,
            Reject::FarFuture,
            Reject::RingOverwrite,
            Reject::ReservedBits,
            Reject::WeaponSlot,
            Reject::BadMove,
            Reject::MoveClamped,
            Reject::YawRate,
            Reject::PitchRate,
            Reject::NoSession,
            Reject::C0Rate,
        ];
        let labels: HashSet<_> = all.iter().map(|r| r.label()).collect();
        assert_eq!(labels.len(), all.len());
        assert_eq!(Reject::LABELS.len(), all.len());
        for l in &labels {
            assert!(Reject::LABELS.contains(l));
        }
    }
}
