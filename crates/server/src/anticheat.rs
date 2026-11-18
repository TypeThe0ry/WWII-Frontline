//! Basic validation counters and per-player suspicion bookkeeping (M1-026/027, ARCHITECTURE
//! §15.1, §15.4). The server is authoritative, so cheats mostly fail by construction; this module
//! makes the attempts visible: every clamp, rejection and refused trigger pull is counted per
//! player and per match, and the five most suspicious players are logged every minute.
//! Statistical detection (headshot rate, aim snaps, replays) is M2.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::weapon::Fsm;
use frontline_sim::{PlayerId, SimEvent, World};

use crate::metrics::Metrics;
use crate::validate::Reject;

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
