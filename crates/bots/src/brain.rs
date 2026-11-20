//! The brain interface (ARCHITECTURE 4.3.6).

use frontline_protocol::msg::input::InputCmd;
use frontline_sim::WorldView;

use crate::ctx::BotCtx;
use crate::scheduler::{Scheduled, ScheduledStep};

/// Per-tick statistics a brain reports to its host (drained by [`BotBrain::take_stats`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BrainStats {
    /// Nanoseconds spent in path queries since the last drain.
    pub path_ns: u64,
    pub path_queries: u32,
    pub perception_ns: u64,
    pub tactical_ns: u64,
    /// The brain ran a full decision step (not only the cheap per-tick part).
    pub thought: bool,
    /// Stuck detections since the last drain.
    pub stuck_events: u32,
}

/// One bot. Called once per tick for the tick `ctx.tick`; it returns the command of that tick.
pub trait BotBrain: Send {
    /// Reads the world (read-only) and returns this tick's input.
    fn think(&mut self, view: &dyn WorldView, ctx: &BotCtx) -> InputCmd;

    /// Live/replay runtime entry. The default is for host-injected fixture brains;
    /// soldier implementations apply the provided scheduling and deadline.
    #[doc(hidden)]
    fn think_scheduled(
        &mut self,
        view: &dyn WorldView,
        ctx: &BotCtx,
        _: Scheduled<'_>,
    ) -> ScheduledStep {
        ScheduledStep {
            cmd: self.think(view, ctx),
            timed_out: false,
        }
    }

    /// Statistics since the previous call.
    fn take_stats(&mut self) -> BrainStats {
        BrainStats::default()
    }
}
