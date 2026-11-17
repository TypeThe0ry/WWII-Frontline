//! The brain interface (ARCHITECTURE 4.3.6).

use frontline_protocol::msg::input::InputCmd;
use frontline_sim::WorldView;

use crate::ctx::BotCtx;

/// Per-tick statistics a brain reports to its host (drained by [`BotBrain::take_stats`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BrainStats {
    /// Nanoseconds spent in path queries since the last drain.
    pub path_ns: u64,
    pub path_queries: u32,
    /// The brain ran a full decision step (not only the cheap per-tick part).
    pub thought: bool,
    /// Stuck detections since the last drain.
    pub stuck_events: u32,
}

/// One bot. Called once per tick for the tick `ctx.tick`; it returns the command of that tick.
pub trait BotBrain: Send {
    /// Reads the world (read-only) and returns this tick's input.
    fn think(&mut self, view: &dyn WorldView, ctx: &BotCtx) -> InputCmd;

    /// Statistics since the previous call.
    fn take_stats(&mut self) -> BrainStats {
        BrainStats::default()
    }
}
