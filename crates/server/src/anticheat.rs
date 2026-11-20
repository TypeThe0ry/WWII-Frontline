//! Basic validation counters and per-player suspicion bookkeeping (M1-026/027, ARCHITECTURE
//! §15.1, §15.4). The server is authoritative, so cheats mostly fail by construction; this module
//! makes the attempts visible: every clamp, rejection and refused trigger pull is counted per
//! player and per match, and the five most suspicious players are logged every minute.
//! Statistical detection (headshot rate, aim snaps, replays) is M2.

use std::collections::{BTreeMap, VecDeque};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::weapon::Fsm;
use frontline_sim::{PlayerId, SimEvent, World};

use crate::metrics::Metrics;
use crate::validate::Reject;

/// Policy at the server's 30 Hz tick rate. Weapon and movement physics stay in the sim.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub view_max_deg_s: f32,
    pub window_ticks: u32,
    pub warn_count: usize,
    pub invalid_streak_ticks: u32,
}

pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let rules: Rules = serde_json::from_str(include_str!("../../../data/rules/anticheat.json"))
        .expect("valid embedded anti-cheat rules");
    // At 30 Hz the largest per-axis change is 180 degrees. Keep the cap reachable.
    assert!(rules.view_max_deg_s > 0.0 && rules.view_max_deg_s < 5400.0);
    assert!((1..=300).contains(&rules.window_ticks));
    assert!((1..=300).contains(&rules.warn_count));
    assert!((1..=300).contains(&rules.invalid_streak_ticks));
    rules
});

/// Values for the existing C0 `Disconnect.reason` byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DisconnectReason {
    InvalidInputStreak = 1,
    RepeatedInputViolation = 2,
}

/// A half-open tick window (the event exactly `width` ticks ago has expired).
pub(crate) fn expire(events: &mut VecDeque<u32>, now: u32, width: u32) {
    while events
        .front()
        .is_some_and(|&t| now.wrapping_sub(t) >= width)
    {
        events.pop_front();
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum InputOutcome {
    #[default]
    None,
    Malformed,
    Accepted,
}

#[derive(Default)]
struct InputPolicy {
    recent: BTreeMap<&'static str, VecDeque<u32>>,
    last_bad_tick: Option<u32>,
    streak: u32,
}

/// Kinds of violation kept per player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Violation {
    SeqReplay,
    TickWindow,
    ViewRate,
    MoveSpeed,
    FireRate,
    OriginHint,
    PredHash,
}

impl Violation {
    pub const COUNT: usize = 7;

    /// Violation behind an input rejection / clamp, if it is one.
    pub fn of(r: Reject) -> Option<Violation> {
        Some(match r {
            Reject::Duplicate | Reject::TickRegress => Violation::SeqReplay,
            Reject::TooOld | Reject::TooFar | Reject::FarFuture | Reject::RingOverwrite => {
                Violation::TickWindow
            }
            Reject::YawRate | Reject::PitchRate => Violation::ViewRate,
            Reject::MoveClamped | Reject::BadMove => Violation::MoveSpeed,
            _ => return None,
        })
    }
}

/// How often the top suspects are logged.
pub const REPORT_PERIOD: Duration = Duration::from_secs(60);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts(pub [u32; Violation::COUNT]);

impl Counts {
    pub fn total(&self) -> u32 {
        self.0.iter().sum()
    }
}

pub struct AntiCheat {
    per: BTreeMap<PlayerId, Counts>,
    policy: BTreeMap<PlayerId, InputPolicy>,
    tick_outcomes: [InputOutcome; 256],
    /// Players whose trigger was held on the previous command (a pull is a rising edge).
    held: BTreeMap<PlayerId, bool>,
    /// Players that pulled the trigger while the weapon was still cycling: (player) this tick.
    pending: Vec<(PlayerId, frontline_sim::EntityId)>,
    last_report: Instant,
}

impl AntiCheat {
    pub fn new(now: Instant) -> Self {
        AntiCheat {
            per: BTreeMap::new(),
            policy: BTreeMap::new(),
            tick_outcomes: [InputOutcome::None; 256],
            held: BTreeMap::new(),
            pending: Vec::new(),
            last_report: now,
        }
    }

    pub fn note(&mut self, p: PlayerId, v: Violation) {
        self.per.entry(p).or_default().0[v as usize] += 1;
    }

    pub fn counts(&self, p: PlayerId) -> Counts {
        self.per.get(&p).copied().unwrap_or_default()
    }

    pub fn forget(&mut self, p: PlayerId) {
        self.held.remove(&p);
        self.policy.remove(&p);
        self.tick_outcomes[p as usize] = InputOutcome::None;
    }

    /// Counts only enforceable outcomes here. Network jitter, redundant commands and
    /// prediction drift use their own telemetry without contributing to disconnection.
    pub fn input_violation(
        &mut self,
        p: PlayerId,
        r: Reject,
        now: u32,
    ) -> Option<DisconnectReason> {
        if !r.is_violation() {
            return None;
        }
        let recent = self
            .policy
            .entry(p)
            .or_default()
            .recent
            .entry(r.label())
            .or_default();
        expire(recent, now, RULES.window_ticks);
        let limit = 3 * RULES.warn_count;
        if recent.len() >= limit {
            return Some(DisconnectReason::RepeatedInputViolation);
        }
        recent.push_back(now);
        if recent.len() == RULES.warn_count {
            tracing::warn!(
                event = "input_violation_warning",
                player_id = p,
                tick = now,
                reason = r.label(),
                count = recent.len(),
            );
        }
        (recent.len() == limit).then_some(DisconnectReason::RepeatedInputViolation)
    }

    /// Record the strongest input outcome seen for this player during this server tick.
    /// One accepted command (including clamped) keeps the tick from being invalid-only.
    pub fn input_outcome(&mut self, p: PlayerId, accepted: bool, invalid: bool) {
        let outcome = &mut self.tick_outcomes[p as usize];
        if accepted {
            *outcome = InputOutcome::Accepted;
        } else if invalid && *outcome == InputOutcome::None {
            *outcome = InputOutcome::Malformed;
        }
    }

    pub fn finish_input_tick(
        &mut self,
        now: u32,
        mut kick: impl FnMut(PlayerId, DisconnectReason),
    ) {
        for (&p, policy) in &mut self.policy {
            match self.tick_outcomes[p as usize] {
                InputOutcome::Accepted => {
                    policy.last_bad_tick = None;
                    policy.streak = 0;
                }
                InputOutcome::Malformed if policy.last_bad_tick != Some(now) => {
                    policy.streak = if policy.last_bad_tick == Some(now.wrapping_sub(1)) {
                        policy.streak + 1
                    } else {
                        1
                    };
                    policy.last_bad_tick = Some(now);
                    if policy.streak >= RULES.invalid_streak_ticks {
                        kick(p, DisconnectReason::InvalidInputStreak);
                    }
                }
                _ => {}
            }
        }
        self.tick_outcomes.fill(InputOutcome::None);
    }

    /// Before the step: a rising trigger edge while the weapon is cycling is a pull the sim will
    /// refuse (faster than the weapon's cap).
    pub fn before_step(&mut self, world: &World, inputs: &[(PlayerId, InputCmd)]) {
        self.pending.clear();
        for (p, c) in inputs {
            let fire = c.buttons & buttons::FIRE != 0;
            let was = self.held.insert(*p, fire).unwrap_or(false);
            if fire && !was {
                if let Some(w) = world.weapon_state(*p) {
                    if matches!(w.fsm, Fsm::Cooldown | Fsm::BoltCycle) {
                        if let Some(v) = world.view().players().iter().find(|v| v.player == *p) {
                            self.pending.push((*p, v.entity));
                        }
                    }
                }
            }
        }
    }

    /// After the step: the pulls that produced no `Shot` were refused.
    pub fn after_step(&mut self, events: &[SimEvent], metrics: &Metrics) {
        let pending = std::mem::take(&mut self.pending);
        for (p, ent) in pending {
            let shot = events
                .iter()
                .any(|e| matches!(e, SimEvent::Shot { shooter, .. } if *shooter == ent));
            if !shot {
                self.note(p, Violation::FireRate);
                metrics.fire_rate_rejected_total.inc();
            }
        }
    }

    /// The `n` players with the most violations, worst first.
    pub fn top(&self, n: usize) -> Vec<(PlayerId, Counts)> {
        let mut v: Vec<_> = self
            .per
            .iter()
            .filter(|(_, c)| c.total() > 0)
            .map(|(p, c)| (*p, *c))
            .collect();
        v.sort_by(|a, b| b.1.total().cmp(&a.1.total()).then(a.0.cmp(&b.0)));
        v.truncate(n);
        v
    }

    /// Logs the top five suspects once per [`REPORT_PERIOD`].
    pub fn report_if_due(&mut self, now: Instant) {
        if now.saturating_duration_since(self.last_report) < REPORT_PERIOD {
            return;
        }
        self.last_report = now;
        for (p, c) in self.top(5) {
            tracing::info!(
                event = "suspicion_top",
                player_id = p as u64,
                replay = c.0[Violation::SeqReplay as usize],
                tick_window = c.0[Violation::TickWindow as usize],
                view_rate = c.0[Violation::ViewRate as usize],
                move_speed = c.0[Violation::MoveSpeed as usize],
                fire_rate = c.0[Violation::FireRate as usize],
            );
        }
    }
}
