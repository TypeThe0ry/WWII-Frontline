//! Player sessions on the sim side (M0-018): lifecycle `Connecting -> Joined -> Gone`, the
//! per-player input pipeline (window check, validation, ring buffer) and the missing-input
//! policy of ARCHITECTURE §6.3.
//!
//! Everything here runs on the sim thread; no locks.

use frontline_protocol::msg::input::{InputBundle, InputCmd};
use frontline_sim::{PlayerId, Team};

use crate::input_buffer::{InputBuffer, PushResult};
use crate::validate::{
    validate_input, Clamps, PlayerInputState, Reject, ValidateCfg, AUX_LEVEL_MASK, EDGE_BUTTONS,
    FAR_FUTURE_TICKS,
};
use frontline_protocol::msg::input::buttons;

/// A missing input is replaced by the previous one for at most this many ticks (~167 ms) ...
pub const REPEAT_MAX_GAP: u32 = 5;
/// ... and from this many ticks (>= 300 ms) the stream is considered stalled.
pub const STALL_GAP: u32 = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Handshake done on the net side, not yet in the world.
    Connecting,
    Joined,
    Gone,
}

/// What [`PlayerInput::resolve`] produced for a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// The player's own command for this tick.
    Real,
    /// Previous movement repeated (gap 1..=5), edge bits cleared.
    Repeated,
    /// Movement zeroed, view kept (gap > 5).
    Held,
}

#[derive(Debug, Clone, Copy)]
pub struct Resolved {
    pub cmd: InputCmd,
    pub kind: Resolution,
    /// First tick of a stall (`gap == 9`) for a player that had sent input before.
    pub new_stall: bool,
}

/// Outcome of ingesting one command.
#[derive(Debug, Clone, Copy)]
pub struct Ingest {
    pub result: PushResult,
    pub clamps: Clamps,
}

/// Input pipeline of one player.
#[derive(Debug, Clone)]
pub struct PlayerInput {
    pub state: PlayerInputState,
    buf: InputBuffer,
    /// Basis for repeats: the newest command actually used for a tick.
    last_cmd: InputCmd,
    stalled: bool,
    had_real: bool,
    /// Arrival lead (ticks between arrival and execution tick) of the newest accepted
    /// commands, for the snapshot header's `input_slack` (M0-019).
    leads: [u8; SLACK_WINDOW],
    lead_n: usize,
    lead_i: usize,
}

/// Accepted commands remembered for the slack estimate.
pub const SLACK_WINDOW: usize = 8;
/// `input_slack` reported before any command has been accepted (the client's target).
pub const DEFAULT_SLACK: u8 = 1;

fn neutral(yaw: u16) -> InputCmd {
    InputCmd {
        seq: 0,
        tick: 0,
        move_x: 0,
        move_z: 0,
        yaw,
        pitch: 0,
        buttons: 0,
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    }
}

impl PlayerInput {
    /// `yaw` is the wire code of the spawn heading (kept until the first real input).
    pub fn new(cfg: ValidateCfg, yaw: u16) -> Self {
        PlayerInput {
            state: PlayerInputState::new(cfg),
            buf: InputBuffer::new(),
            last_cmd: neutral(yaw),
            stalled: false,
            had_real: false,
            leads: [0; SLACK_WINDOW],
            lead_n: 0,
            lead_i: 0,
        }
    }

    /// Sequence number of the command the sim used for the latest resolved tick (the
    /// snapshot's `ack_input_seq`; a repeated command keeps the sequence of the real one).
    pub fn last_applied_seq(&self) -> u16 {
        self.last_cmd.seq
    }

    /// `input_slack` (4 bit): how many ticks before its execution tick the input arrived - the
    /// minimum over the last 8 accepted commands (the late ones decide what the client must do).
    pub fn input_slack(&self) -> u8 {
        if self.lead_n == 0 {
            return DEFAULT_SLACK;
        }
        self.leads[..self.lead_n].iter().copied().min().unwrap_or(0)
    }

    fn note_lead(&mut self, cmd_tick: u32, now: u32) {
        let lead = (cmd_tick as i64 - now as i64).clamp(0, 15) as u8;
        self.leads[self.lead_i] = lead;
        self.lead_i = (self.lead_i + 1) % SLACK_WINDOW;
        self.lead_n = (self.lead_n + 1).min(SLACK_WINDOW);
    }

    /// Window check, validation, buffering - in that order. `now` is the tick about to run.
    /// The single door every command (human or bot) goes through.
    pub fn ingest(&mut self, cmd: &InputCmd, now: u32) -> Ingest {
        let none = Clamps::default();
        if let Err(r) = self.buf.classify(cmd.tick, now) {
            // A tick a little beyond the window is clock skew (dropped, no strike); one absurdly
            // far ahead is forged and counts like any other malformed value.
            if r == PushResult::TooFar && cmd.tick as u64 > now as u64 + FAR_FUTURE_TICKS as u64 {
                self.state.note(Reject::FarFuture);
                return Ingest {
                    result: PushResult::Invalid(Reject::FarFuture),
                    clamps: none,
                };
            }
            if let Some(reason) = r.reject() {
                self.state.note(reason);
            }
            return Ingest {
                result: r,
                clamps: none,
            };
        }
        match validate_input(&mut self.state, cmd) {
            Ok(v) => {
                let result = self.buf.push(v.cmd, now);
                if result.is_accepted() {
                    self.note_lead(v.cmd.tick, now);
                }
                Ingest {
                    result,
                    clamps: v.clamps,
                }
            }
            Err(Reject::Duplicate) => Ingest {
                result: PushResult::Duplicate,
                clamps: none,
            },
            Err(r) => Ingest {
                result: PushResult::Invalid(r),
                clamps: none,
            },
        }
    }

    /// Records the per-bundle metadata (ack, prediction hash, redundant fire commands). Returns
    /// the rejection reasons of fire commands that were refused.
    pub fn note_bundle(&mut self, b: &InputBundle) -> Vec<Reject> {
        self.state.last_ack_snapshot_tick = b.ack_snapshot_tick;
        self.state.last_pred_hash = b.pred_hash;
        let mut out = Vec::new();
        for f in &b.fire {
            if let Err(r) = self.state.record_fire(f) {
                self.state.note(r);
                out.push(r);
            }
        }
        out
    }

    /// The command to simulate at `tick` (the missing-input state machine).
    pub fn resolve(&mut self, tick: u32) -> Resolved {
        if let Some(mut c) = self.buf.take(tick) {
            c.tick = tick;
            self.state.gap = 0;
            self.stalled = false;
            self.had_real = true;
            self.last_cmd = c;
            return Resolved {
                cmd: c,
                kind: Resolution::Real,
                new_stall: false,
            };
        }
        self.state.gap = self.state.gap.saturating_add(1);
        let gap = self.state.gap;
        let mut c = self.last_cmd;
        c.tick = tick;
        // One-shot actions are never replayed; level bits (aim, stance, sprint, lean) stay.
        c.buttons &= !EDGE_BUTTONS;
        c.aux &= AUX_LEVEL_MASK;
        let kind = if gap > REPEAT_MAX_GAP {
            c.move_x = 0;
            c.move_z = 0;
            c.buttons &= !buttons::SPRINT;
            Resolution::Held
        } else {
            Resolution::Repeated
        };
        let new_stall = gap >= STALL_GAP && !self.stalled;
        if gap >= STALL_GAP {
            self.stalled = true;
        }
        Resolved {
            cmd: c,
            kind,
            new_stall: new_stall && self.had_real,
        }
    }

    /// `gap >= 9` (and still no input): the connection is probably broken.
    pub fn suspected_stall(&self) -> bool {
        self.stalled
    }

    pub fn buffered(&self) -> usize {
        self.buf.len()
    }
}

/// One player's server-side session.
#[derive(Debug, Clone)]
pub struct PlayerSession {
    pub player: PlayerId,
    pub team: Team,
    pub state: SessionState,
    pub input: PlayerInput,
}

impl PlayerSession {
    pub fn new(player: PlayerId, team: Team, cfg: ValidateCfg, spawn_yaw_code: u16) -> Self {
        PlayerSession {
            player,
            team,
            state: SessionState::Connecting,
            input: PlayerInput::new(cfg, spawn_yaw_code),
        }
    }

    pub fn joined(&mut self) {
        self.state = SessionState::Joined;
    }

    pub fn gone(&mut self) {
        self.state = SessionState::Gone;
        self.input.buf.clear();
    }
}

/// Wire code of a yaw in radians (`[0, 2pi) -> 0..65535`).
pub fn yaw_code(yaw: f32) -> u16 {
    let t = yaw.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
    ((t * 65536.0).round() as u32 & 0xFFFF) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::validate::KICK_STREAK;

    fn cmd(tick: u32) -> InputCmd {
        InputCmd {
            seq: tick as u16,
            tick,
            move_x: 0,
            move_z: 127,
            yaw: 1000,
            pitch: 50,
            buttons: buttons::FIRE | buttons::AIM | buttons::CROUCH | buttons::JUMP,
            weapon_slot: 1,
            aux: 0x03,
            view_tick: 4,
        }
    }

    fn pi() -> PlayerInput {
        PlayerInput::new(ValidateCfg::default(), 77)
    }

    #[test]
    fn real_input_is_used_and_resets_the_gap() {
        let mut p = pi();
        assert!(p.ingest(&cmd(5), 5).result.is_accepted());
        let r = p.resolve(5);
        assert_eq!(r.kind, Resolution::Real);
        assert_eq!(r.cmd.move_z, 127);
        assert_eq!(p.state.gap, 0);
    }

    #[test]
    fn missing_input_repeats_five_ticks_then_zeroes_movement() {
        let mut p = pi();
        p.ingest(&cmd(10), 10);
        assert_eq!(p.resolve(10).kind, Resolution::Real);
        for i in 1..=5 {
            let r = p.resolve(10 + i);
            assert_eq!(r.kind, Resolution::Repeated, "gap {i}");
            assert_eq!(r.cmd.move_z, 127, "gap {i}");
            assert_eq!(r.cmd.tick, 10 + i);
            assert!(!r.new_stall);
        }
        for i in 6..=12 {
            let r = p.resolve(10 + i);
            assert_eq!(r.kind, Resolution::Held, "gap {i}");
            assert_eq!((r.cmd.move_x, r.cmd.move_z), (0, 0), "gap {i}");
            // View and level bits are kept.
            assert_eq!((r.cmd.yaw, r.cmd.pitch), (1000, 50));
            assert_ne!(r.cmd.buttons & buttons::AIM, 0);
            assert_ne!(r.cmd.buttons & buttons::CROUCH, 0);
        }
    }

    #[test]
    fn edge_bits_are_cleared_in_repeats() {
        let mut p = pi();
        p.ingest(&cmd(10), 10);
        assert_ne!(p.resolve(10).cmd.buttons & buttons::FIRE, 0);
        let r = p.resolve(11);
        assert_eq!(r.cmd.buttons & EDGE_BUTTONS, 0);
        assert_eq!(
            r.cmd.aux, 0x01,
            "walk modifier kept, one-shot aux bits cleared"
        );
    }

    #[test]
    fn stall_is_flagged_once_at_gap_nine_and_cleared_by_input() {
        let mut p = pi();
        p.ingest(&cmd(10), 10);
        p.resolve(10);
        let mut stalls = 0;
        for t in 11..=30 {
            if p.resolve(t).new_stall {
                stalls += 1;
                assert_eq!(p.state.gap, STALL_GAP);
            }
        }
        assert_eq!(stalls, 1);
        assert!(p.suspected_stall());
        p.ingest(&cmd(31), 31);
        p.resolve(31);
        assert!(!p.suspected_stall());
        for t in 32..=45 {
            if p.resolve(t).new_stall {
                stalls += 1;
            }
        }
        assert_eq!(stalls, 2);
    }

    #[test]
    fn a_player_that_never_sent_anything_is_neutral_without_stall_count() {
        let mut p = pi();
        for t in 1..=20 {
            let r = p.resolve(t);
            assert!(!r.new_stall);
            assert_eq!((r.cmd.move_x, r.cmd.move_z, r.cmd.yaw), (0, 0, 77));
        }
    }

    #[test]
    fn ingest_reports_window_and_validation_outcomes() {
        let mut p = pi();
        assert_eq!(p.ingest(&cmd(100), 50).result, PushResult::TooFar);
        assert_eq!(p.ingest(&cmd(10), 50).result, PushResult::TooOld);
        assert!(p.ingest(&cmd(50), 50).result.is_accepted());
        assert_eq!(p.ingest(&cmd(50), 50).result, PushResult::Duplicate);
        let mut bad = cmd(51);
        bad.buttons |= 1 << 14;
        assert_eq!(
            p.ingest(&bad, 50).result,
            PushResult::Invalid(Reject::ReservedBits)
        );
        // Reserved bits are a violation (TooFar/TooOld/Duplicate are not); the clean accept
        // of tick 50 in between reset the streak, so only the reserved-bits one remains.
        assert_eq!(p.state.violation_streak, 1);
    }

    /// M0-030: a client whose tick estimate runs ahead (a 1-5 fps browser measuring its RTT
    /// with a blocked main thread) is dropped command by command but never kicked; forged
    /// ticks and every other malformed value still count.
    #[test]
    fn clock_skew_is_not_a_violation_but_forged_ticks_are() {
        let mut p = pi();
        let now = 1000;
        // 200 commands, every one 11..=60 ticks ahead: all refused, no strike.
        for i in 0..200u32 {
            let r = p.ingest(&cmd(now + 11 + i % 50), now);
            assert_eq!(r.result, PushResult::TooFar);
        }
        assert_eq!(p.state.violation_streak, 0);
        assert!(!p.state.should_kick());
        // The edge: now + FAR_FUTURE_TICKS is still skew, one more tick is forged.
        assert_eq!(
            p.ingest(&cmd(now + FAR_FUTURE_TICKS), now).result,
            PushResult::TooFar
        );
        assert_eq!(p.state.violation_streak, 0);
        assert_eq!(
            p.ingest(&cmd(now + FAR_FUTURE_TICKS + 1), now).result,
            PushResult::Invalid(Reject::FarFuture)
        );
        assert_eq!(p.state.violation_streak, 1);
        for _ in 0..KICK_STREAK - 1 {
            p.ingest(&cmd(u32::MAX), now);
        }
        assert!(p.state.should_kick(), "30 forged ticks in a row kick");
    }

    /// M0-030: the same client at 3 fps (5 commands per frame, one frame per second, RTT
    /// inflated so the lead is 12-14 ticks) keeps getting its in-window commands accepted
    /// and is never kicked over 60 s, while 30 reserved-bit commands still are.
    #[test]
    fn low_fps_burst_pattern_is_never_kicked_for_60_s() {
        let mut p = pi();
        let mut seq = 0u16;
        let mut accepted = 0u32;
        for sec in 0..60u32 {
            let now = 1 + sec * 30;
            // lead 8 + a 5-command burst: the last two are beyond now + 10, the first three fit
            for k in 0..5u32 {
                seq = seq.wrapping_add(1);
                let mut c = cmd(now + 8 + k);
                c.seq = seq;
                if p.ingest(&c, now).result.is_accepted() {
                    accepted += 1;
                }
            }
            assert!(!p.state.should_kick());
        }
        assert_eq!(accepted, 60 * 3, "in-window commands still get through");
        // Everything out of window, 60 s long: still no kick.
        for sec in 0..60u32 {
            let now = 5000 + sec * 30;
            for k in 0..5u32 {
                seq = seq.wrapping_add(1);
                let mut c = cmd(now + 15 + k);
                c.seq = seq;
                p.ingest(&c, now);
            }
        }
        assert!(!p.state.should_kick());
        // Real violations still kick.
        for _ in 0..KICK_STREAK {
            seq = seq.wrapping_add(1);
            let mut c = cmd(9000);
            c.seq = seq;
            c.buttons |= 1 << 14;
            p.ingest(&c, 9000);
        }
        assert!(p.state.should_kick());
    }

    #[test]
    fn clamped_commands_are_stored_clamped() {
        let mut p = pi();
        let mut c = cmd(20);
        c.move_x = 127;
        c.move_z = 127;
        let i = p.ingest(&c, 20);
        assert!(i.result.is_accepted());
        assert!(i.clamps.any());
        let r = p.resolve(20);
        let len = (r.cmd.move_x as f32).hypot(r.cmd.move_z as f32) / 127.0;
        assert!(len <= 1.0 + 1e-6);
    }

    #[test]
    fn yaw_code_roundtrip() {
        assert_eq!(yaw_code(0.0), 0);
        assert_eq!(yaw_code(std::f32::consts::PI), 32768);
        assert_eq!(yaw_code(-std::f32::consts::FRAC_PI_2), 49152);
        assert_eq!(yaw_code(std::f32::consts::TAU), 0);
    }

    #[test]
    fn session_lifecycle() {
        let mut s = PlayerSession::new(3, Team::Ger, ValidateCfg::default(), 0);
        assert_eq!(s.state, SessionState::Connecting);
        s.joined();
        s.input.ingest(&cmd(5), 5);
        assert_eq!(s.input.buffered(), 1);
        s.gone();
        assert_eq!(s.state, SessionState::Gone);
        assert_eq!(s.input.buffered(), 0);
    }
}
