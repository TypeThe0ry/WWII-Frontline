//! Match-local, staggered soldier decisions. Cached inputs are rebuilt every tick.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::time::{Duration, Instant};

use frontline_protocol::msg::input::InputCmd;
use frontline_sim::{PlayerId, WorldView};

use crate::{path_budget::PathBudget, perception_cache::PerceptionCache, BotBrain, BotCtx};

const THINK_LIMIT_NS: u64 = 120_000;
const INITIAL_THINK_ESTIMATE_NS: u64 = 60_000;
const SMALL_TICK_LIMIT: Duration = Duration::from_millis(3);
const LARGE_TICK_LIMIT: Duration = Duration::from_millis(5);

fn estimate_fits(used_ns: u64, budget_ns: u64, estimate_ns: u64) -> bool {
    used_ns <= budget_ns && estimate_ns <= budget_ns - used_ns
}

pub fn phase_due(tick: u32, id: u32, period: u32) -> bool {
    (tick % period + id % period).is_multiple_of(period)
}

#[derive(Default)]
pub struct ThinkPlan {
    pub due: Vec<PlayerId>,
    estimated_ns: u64,
}

pub struct BotScheduler {
    pending: [Option<u32>; 256],
    estimate_ns: [u64; 256],
    heap: BinaryHeap<Candidate>,
}

#[derive(Clone, Copy, Debug, Eq)]
struct Candidate {
    estimate_ns: u64,
    id: PlayerId,
    deferred_since: u32,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        (self.estimate_ns, self.deferred_since, self.id)
            == (other.estimate_ns, other.deferred_since, other.id)
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.estimate_ns
            .cmp(&other.estimate_ns)
            .then_with(|| other.deferred_since.cmp(&self.deferred_since))
            .then_with(|| other.id.cmp(&self.id))
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Default for BotScheduler {
    fn default() -> Self {
        Self {
            pending: [None; 256],
            estimate_ns: [INITIAL_THINK_ESTIMATE_NS; 256],
            heap: BinaryHeap::with_capacity(128),
        }
    }
}

impl BotScheduler {
    pub fn plan(&mut self, tick: u32, bots: &[PlayerId], budget_ns: u64) -> ThinkPlan {
        self.heap.clear();
        for &id in bots {
            let i = usize::from(id);
            if self.due(tick, id) {
                self.heap.push(Candidate {
                    estimate_ns: self.estimate_ns[i],
                    id,
                    deferred_since: self.pending[i].unwrap_or(tick),
                });
            }
        }

        let mut admitted = Vec::with_capacity(self.heap.len());
        let mut left = budget_ns;
        while let Some(candidate) = self.heap.pop() {
            if candidate.estimate_ns > THINK_LIMIT_NS || candidate.estimate_ns > left {
                self.pending[usize::from(candidate.id)].get_or_insert(tick);
                continue;
            }
            left -= candidate.estimate_ns;
            admitted.push(candidate.id);
        }
        let estimated_ns = admitted
            .iter()
            .map(|&id| self.estimate_ns[usize::from(id)])
            .sum();
        ThinkPlan {
            due: admitted,
            estimated_ns,
        }
    }

    fn due(&self, tick: u32, id: PlayerId) -> bool {
        self.pending[usize::from(id)].is_some() || phase_due(tick, u32::from(id), 3)
    }

    fn defer(&mut self, tick: u32, id: PlayerId) {
        self.pending[usize::from(id)].get_or_insert(tick);
    }

    fn clear(&mut self, id: PlayerId) {
        self.pending[usize::from(id)] = None;
    }

    fn completed(&mut self, id: PlayerId, elapsed_ns: u64) {
        let estimate = &mut self.estimate_ns[usize::from(id)];
        // Track a decaying high-water mark with 25% headroom. It remains conservative
        // after a spike, while eventually adapting when the bot's work becomes cheaper.
        *estimate = (*estimate * 7 / 8)
            .max(elapsed_ns.saturating_mul(5) / 4)
            .min(THINK_LIMIT_NS);
    }

    fn timed_out(&mut self, id: PlayerId) {
        // A deadline proves that the previous estimate was too low. Keep the next
        // prediction at the admission ceiling until successful measurements lower it.
        self.estimate_ns[usize::from(id)] = THINK_LIMIT_NS;
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct BotsMetrics {
    pub total_ns: u64,
    pub perception_ns: u64,
    pub tactical_ns: u64,
    pub path_ns: u64,
    pub deferred: u64,
    pub timeouts: u64,
    pub stuck: u64,
}

#[doc(hidden)]
pub struct Scheduled<'a> {
    pub(crate) cache: &'a PerceptionCache,
    pub(crate) paths: &'a mut PathBudget,
    pub(crate) due: bool,
    pub(crate) deadline: Option<Instant>,
    pub(crate) suppression: f32,
}

#[doc(hidden)]
pub struct ScheduledStep {
    pub cmd: InputCmd,
    pub timed_out: bool,
}

/// One per match, reused by server and offline simulation. Live deadlines are cooperative,
/// not hard realtime guarantees. Replay mode only uses deterministic work limits.
pub struct BotRuntime {
    scheduler: BotScheduler,
    cache: PerceptionCache,
    paths: PathBudget,
    realtime: bool,
    metrics: BotsMetrics,
    start: Instant,
    limit: Duration,
    tick: u32,
    admitted: [bool; 256],
    think_limit: Duration,
}

impl Default for BotRuntime {
    fn default() -> Self {
        Self {
            scheduler: BotScheduler::default(),
            cache: PerceptionCache::default(),
            paths: PathBudget::default(),
            realtime: true,
            metrics: BotsMetrics::default(),
            start: Instant::now(),
            limit: LARGE_TICK_LIMIT,
            tick: 0,
            admitted: [false; 256],
            think_limit: Duration::from_nanos(THINK_LIMIT_NS),
        }
    }
}

impl BotRuntime {
    pub fn deterministic() -> Self {
        Self {
            realtime: false,
            ..Self::default()
        }
    }

    #[cfg(test)]
    pub(crate) fn set_think_limit_for_test(&mut self, limit: Duration) {
        self.think_limit = limit;
    }

    pub fn set_path_ms_per_tick(&mut self, ms: f32) {
        self.paths.ms_per_tick = ms;
    }

    pub fn forget(&mut self, id: PlayerId) {
        self.scheduler.clear(id);
        self.paths.forget(id);
    }

    pub fn begin_tick(&mut self, view: &dyn WorldView, tick: u32) {
        self.start = Instant::now();
        self.tick = tick;
        self.limit = if view.players().len() <= 60 {
            SMALL_TICK_LIMIT
        } else {
            LARGE_TICK_LIMIT
        };
        self.admitted.fill(false);
        self.cache.rebuild(view);
        self.metrics = BotsMetrics {
            perception_ns: ns(self.start.elapsed()),
            path_ns: self.paths.process(tick, self.realtime),
            ..BotsMetrics::default()
        };
    }

    /// Select due bots before invoking any brain. The candidate heap prioritizes estimated
    /// decision cost and the returned set fits both the per-bot and aggregate time budgets.
    pub fn plan(&mut self, bots: &[PlayerId]) -> ThinkPlan {
        let budget = if self.realtime {
            self.limit.saturating_sub(self.start.elapsed()).as_nanos() as u64
        } else {
            u64::MAX
        };
        let mut plan = self.scheduler.plan(self.tick, bots, budget);
        if self.realtime {
            let remaining = self.limit.saturating_sub(self.start.elapsed()).as_nanos() as u64;
            while plan.estimated_ns > remaining {
                let Some(id) = plan.due.pop() else { break };
                let estimate = self.scheduler.estimate_ns[usize::from(id)];
                plan.estimated_ns = plan.estimated_ns.saturating_sub(estimate);
                self.scheduler.pending[usize::from(id)].get_or_insert(self.tick);
            }
        }
        for &id in &plan.due {
            self.admitted[usize::from(id)] = true;
        }
        let due = bots
            .iter()
            .filter(|&&id| self.scheduler.due(self.tick, id))
            .count();
        self.metrics.deferred += due.saturating_sub(plan.due.len()) as u64;
        plan
    }

    pub fn think(
        &mut self,
        brain: &mut dyn BotBrain,
        view: &dyn WorldView,
        ctx: &BotCtx,
    ) -> InputCmd {
        self.think_with_suppression(brain, view, ctx, 0.0)
    }

    pub fn think_with_suppression(
        &mut self,
        brain: &mut dyn BotBrain,
        view: &dyn WorldView,
        ctx: &BotCtx,
        suppression: f32,
    ) -> InputCmd {
        debug_assert_eq!(self.tick, ctx.tick);
        let id = ctx.bot_id;
        let wanted = self.cache.player(id).is_some() && self.scheduler.due(ctx.tick, id);
        let now = Instant::now();
        let admitted = self.admitted[usize::from(id)];
        let estimate_ns = self.scheduler.estimate_ns[usize::from(id)];
        let fits_budget = !self.realtime
            || estimate_fits(
                self.start.elapsed().as_nanos() as u64,
                self.limit.as_nanos() as u64,
                estimate_ns,
            );
        let due = wanted && admitted && fits_budget;
        let step = brain.think_scheduled(
            view,
            ctx,
            Scheduled {
                cache: &self.cache,
                paths: &mut self.paths,
                due,
                deadline: (self.realtime && due)
                    .then_some((now + self.think_limit).min(self.start + self.limit)),
                suppression,
            },
        );
        if step.timed_out {
            self.scheduler.defer(ctx.tick, id);
            self.scheduler.timed_out(id);
            self.metrics.deferred += 1;
        } else if due {
            self.scheduler.clear(id);
            let elapsed = now.elapsed().as_nanos() as u64;
            self.scheduler.completed(id, elapsed);
        } else if wanted && admitted && !fits_budget {
            self.scheduler.defer(ctx.tick, id);
            self.metrics.deferred += 1;
        }
        self.metrics.timeouts += u64::from(step.timed_out);
        let stats = brain.take_stats();
        self.metrics.perception_ns += stats.perception_ns;
        self.metrics.path_ns += stats.path_ns;
        self.metrics.tactical_ns += stats.tactical_ns;
        self.metrics.stuck += u64::from(stats.stuck_events);
        step.cmd
    }

    pub fn finish_tick(&self) -> BotsMetrics {
        BotsMetrics {
            total_ns: ns(self.start.elapsed()),
            ..self.metrics
        }
    }
}

pub(crate) fn ns(d: Duration) -> u64 {
    d.as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_id_is_due_once_in_every_three_ticks() {
        let ids: Vec<_> = (28..128).collect();
        let mut scheduler = BotScheduler::default();
        let mut observed = (7..10)
            .flat_map(|t| scheduler.plan(t, &ids, u64::MAX).due)
            .collect::<Vec<_>>();
        observed.sort_unstable();
        assert_eq!(observed, ids);
    }

    #[test]
    fn skipped_decisions_are_due_early_next_tick() {
        let mut scheduler = BotScheduler::default();
        scheduler.defer(2, 1);
        assert_eq!(scheduler.plan(3, &[0, 1, 2], 120_000).due, [1, 0]);
        scheduler.clear(1);
        assert_eq!(scheduler.plan(3, &[0, 1, 2], 120_000).due, [0]);
    }

    #[test]
    fn largest_predicted_cost_is_admitted_first_with_id_ties_and_both_caps() {
        let mut scheduler = BotScheduler::default();
        scheduler.completed(0, 80_000); // predicted 100 us
        scheduler.completed(3, 40_000); // predicted 52.5 us
                                        // IDs 3 and 6 compete for the remainder; the more expensive estimate wins.
        assert_eq!(scheduler.plan(0, &[6, 3, 0], 160_000).due, [0, 6]);

        scheduler.estimate_ns[0] = 120_001;
        scheduler.estimate_ns[3] = 80_000;
        scheduler.estimate_ns[6] = 50_000;
        let plan = scheduler.plan(0, &[0, 3, 6], 120_000);
        assert_eq!(plan.due, [3]);
        assert_eq!(
            scheduler.pending[0],
            Some(0),
            "over-limit work is not started"
        );
        assert_eq!(
            scheduler.pending[6],
            Some(0),
            "work that cannot fit is deferred"
        );
    }

    #[test]
    fn deferred_age_breaks_equal_cost_ties_before_player_id() {
        let mut scheduler = BotScheduler::default();
        scheduler.defer(1, 3);
        scheduler.defer(2, 6);
        assert_eq!(scheduler.plan(3, &[6, 3], 60_000).due, [3]);
        assert_eq!(scheduler.pending[6], Some(2));
    }

    #[test]
    fn timeout_raises_the_next_prediction_to_the_single_bot_ceiling() {
        let mut scheduler = BotScheduler::default();
        scheduler.timed_out(7);
        assert_eq!(scheduler.estimate_ns[7], THINK_LIMIT_NS);
    }

    #[test]
    fn candidate_only_starts_when_its_estimate_fits_the_live_remaining_budget() {
        assert!(estimate_fits(2_880_000, 3_000_000, 120_000));
        assert!(!estimate_fits(2_880_001, 3_000_000, 120_000));
        assert!(!estimate_fits(u64::MAX - 10, u64::MAX, 120_000));
    }
}
