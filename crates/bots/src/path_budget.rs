//! Match-wide budget for bot navigation. Brains own path validity and reuse; this queue owns
//! scheduling and search scratch, not movement decisions.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use frontline_sim::PlayerId;

use crate::grid_nav;
use crate::nav::path;
use crate::{BotMap, TICK_HZ};

const MIN_REPATH_TICKS: u32 = (TICK_HZ as u32).div_ceil(2);
// Leave room for work between A* checkpoints and for publishing the result.
const GUARD_MARGIN: Duration = Duration::from_micros(25);

#[derive(Debug, PartialEq)]
pub struct PathResult {
    pub goal: [f32; 2],
    pub path: Vec<[f32; 2]>,
    pub failed: bool,
}

struct Request {
    map: Arc<BotMap>,
    from: [f32; 2],
    goal: [f32; 2],
}

pub struct PathBudget {
    pub ms_per_tick: f32,
    /// Optional cap on searches worked on in a tick, independent of the wall clock.
    pub max_queries_per_tick: Option<usize>,
    pending: BTreeMap<PlayerId, Request>,
    ready: BTreeMap<PlayerId, PathResult>,
    last_attempt: [Option<u32>; u8::MAX as usize + 1],
    order: Vec<PlayerId>,
    active: Option<(PlayerId, Request)>,
    grid: grid_nav::PathFinder,
    mesh: path::PathFinder,
    deferred: u64,
}

impl Default for PathBudget {
    fn default() -> Self {
        Self {
            ms_per_tick: 1.0,
            max_queries_per_tick: None,
            pending: BTreeMap::new(),
            ready: BTreeMap::new(),
            last_attempt: [None; u8::MAX as usize + 1],
            order: Vec::new(),
            active: None,
            grid: grid_nav::PathFinder::new(),
            mesh: path::PathFinder::new(),
            deferred: 0,
        }
    }
}

impl PathBudget {
    /// Replace the queued request for this bot without moving it back in the queue. An in-flight
    /// search keeps its original endpoints so a moving bot cannot keep restarting the search.
    pub fn request(&mut self, player: PlayerId, map: Arc<BotMap>, from: [f32; 2], goal: [f32; 2]) {
        if self.active.as_ref().is_some_and(|(id, req)| {
            *id == player && Arc::ptr_eq(&req.map, &map) && req.goal == goal
        }) {
            self.pending.remove(&player);
        } else {
            self.pending.insert(player, Request { map, from, goal });
        }
    }

    pub fn take(&mut self, player: PlayerId) -> Option<PathResult> {
        self.ready.remove(&player)
    }

    /// Discard work/results on a life or target change, without bypassing the rate limit.
    pub fn forget(&mut self, player: PlayerId) {
        self.pending.remove(&player);
        self.ready.remove(&player);
        if self.active.as_ref().is_some_and(|(id, _)| *id == player) {
            self.active = None;
        }
    }

    /// Eligible work held over after the last process call, excluding cooldowns.
    pub fn deferred(&self) -> u64 {
        self.deferred
    }

    /// Returns actual stage wall time in ns, including queue overhead. When `realtime` is false,
    /// no clock is read and the return value is zero; tick ordering and the optional query cap
    /// still apply. Checkpoints cannot prevent OS preemption or bound atomic setup/smoothing.
    pub fn process(&mut self, tick: u32, realtime: bool) -> u64 {
        if !realtime {
            self.drain(tick, || false);
            return 0;
        }
        let start = Instant::now();
        let limit = Duration::from_nanos((f64::from(self.ms_per_tick) * 1_000_000.0) as u64);
        let cutoff = limit - GUARD_MARGIN.min(limit / 2);
        self.drain(tick, || start.elapsed() >= cutoff);
        start.elapsed().as_nanos() as u64
    }

    fn drain(&mut self, tick: u32, mut should_yield: impl FnMut() -> bool) {
        self.deferred = 0;
        if self.pending.is_empty() && self.active.is_none() {
            return;
        }
        let active = self.active.as_ref().map(|(id, _)| *id);
        let age = |id: PlayerId| self.last_attempt[usize::from(id)].map(|t| tick.wrapping_sub(t));
        self.order.clear();
        self.order
            .extend(self.pending.iter().filter_map(|(&id, req)| {
                let min_ticks = MIN_REPATH_TICKS.max(req.map.tuning.repath_min_ticks);
                (Some(id) != active && age(id).is_none_or(|n| n >= min_ticks)).then_some(id)
            }));
        // Never-queried bots first; ties go to the lower player slot regardless of insertion order.
        self.order
            .sort_unstable_by_key(|&id| (age(id).map(Reverse), id));

        let mut slots = self.max_queries_per_tick.unwrap_or(usize::MAX);
        if let Some((id, req)) = self.active.take() {
            if slots == 0 || should_yield() || !self.resume(id, &req, &mut should_yield) {
                self.active = Some((id, req));
                self.deferred = self.order.len() as u64 + 1;
                return;
            }
            slots -= 1;
        }

        let mut started = 0;
        for i in 0..self.order.len().min(slots) {
            if should_yield() {
                break;
            }
            let id = self.order[i];
            let req = self.pending.remove(&id).unwrap();
            self.last_attempt[usize::from(id)] = Some(tick);
            started += 1;
            if !self.start(id, &req) && !self.resume(id, &req, &mut should_yield) {
                self.active = Some((id, req));
                break;
            }
        }
        self.deferred = (self.order.len() - started) as u64 + u64::from(self.active.is_some());
    }

    /// Use the grid only on maps without a loaded mesh. A failed mesh route must not bypass
    /// mesh obstacles via the terrain-only grid, and partial routes aren't used by soldier.
    fn start(&mut self, id: PlayerId, req: &Request) -> bool {
        if let Some(mesh) = &req.map.navmesh {
            if let Some(result) = self.mesh.begin(mesh, req.from, req.goal, u64::MAX) {
                self.publish_mesh(id, req.goal, result);
                return true;
            }
        } else {
            let mut points = Vec::new();
            let done = req.map.nav.as_ref().map_or(Some(false), |grid| {
                self.grid.begin(grid, req.from, req.goal, &mut points)
            });
            if let Some(ok) = done {
                self.publish(id, req.goal, points, !ok);
                return true;
            }
        }
        false
    }

    fn resume(&mut self, id: PlayerId, req: &Request, should_yield: impl FnMut() -> bool) -> bool {
        if let Some(mesh) = &req.map.navmesh {
            let Some(result) = self.mesh.step(mesh, u64::MAX, should_yield) else {
                return false;
            };
            self.publish_mesh(id, req.goal, result);
        } else {
            let mut points = Vec::new();
            let Some(ok) = self.grid.step(
                req.map.nav.as_ref().unwrap(),
                req.from,
                &mut points,
                should_yield,
            ) else {
                return false;
            };
            self.publish(id, req.goal, points, !ok);
        }
        true
    }

    fn publish_mesh(&mut self, id: PlayerId, goal: [f32; 2], result: path::PathResult) {
        match result {
            path::PathResult::Found(p) => self.publish(id, goal, p.points, false),
            _ => self.publish(id, goal, Vec::new(), true),
        }
    }

    fn publish(&mut self, id: PlayerId, goal: [f32; 2], path: Vec<[f32; 2]>, failed: bool) {
        self.ready.insert(id, PathResult { goal, path, failed });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::{NavMesh, Poly};
    use crate::Tuning;

    fn grid_map(n: usize, walk: Vec<bool>) -> BotMap {
        let mut map = BotMap::without_nav(Tuning::embedded(), Vec::new());
        map.nav = Some(grid_nav::NavGrid::from_walkable(n, n, 4.0, walk));
        map
    }

    #[test]
    fn oldest_attempt_first_then_player_id_and_latest_request_wins() {
        let map = Arc::new(grid_map(16, vec![true; 16 * 16]));
        let mut stage = PathBudget {
            max_queries_per_tick: Some(1),
            ..PathBudget::default()
        };
        let request = |stage: &mut PathBudget, id, goal| {
            stage.request(id, Arc::clone(&map), [2.0, 2.0], goal);
        };

        request(&mut stage, 9, [30.0, 30.0]);
        stage.process(0, false);
        assert!(!stage.take(9).unwrap().failed);
        request(&mut stage, 9, [31.0, 31.0]);
        request(&mut stage, 3, [30.0, 30.0]);
        request(&mut stage, 2, [30.0, 30.0]);
        request(&mut stage, 2, [25.0, 33.0]);
        stage.process(1, false);
        assert_eq!(stage.take(2).unwrap().goal, [25.0, 33.0]);
        assert_eq!(stage.deferred(), 1); // #9 is in cooldown, not deferred.
        stage.process(2, false);
        assert!(!stage.take(3).unwrap().failed);

        request(&mut stage, 2, [31.0, 31.0]);
        request(&mut stage, 6, [31.0, 31.0]);
        stage.process(17, false);
        assert!(!stage.take(6).unwrap().failed); // Never attempted beats every retry.
        stage.process(18, false);
        assert_eq!(stage.take(9).unwrap().goal, [31.0, 31.0]);
        assert!(stage.take(2).is_none());
        stage.process(19, false);
        assert!(!stage.take(2).unwrap().failed);
    }

    #[test]
    fn forget_clears_pending_and_ready_but_keeps_failed_attempt_cooldown() {
        let map = Arc::new(grid_map(8, vec![true; 8 * 8]));
        let mut stage = PathBudget::default();
        let tick = u32::MAX - 4;
        stage.request(1, Arc::clone(&map), [2.0, 2.0], [-1.0, 2.0]);
        stage.process(tick, false);
        assert!(stage.take(1).unwrap().failed);
        stage.request(1, Arc::clone(&map), [2.0, 2.0], [22.0, 2.0]);
        stage.forget(1);
        stage.process(tick.wrapping_add(14), false);
        assert!(stage.take(1).is_none());

        stage.request(1, map, [2.0, 2.0], [22.0, 2.0]);
        stage.process(tick.wrapping_add(14), false);
        assert!(stage.take(1).is_none());
        stage.process(tick.wrapping_add(15), false);
        stage.forget(1);
        assert!(stage.take(1).is_none());
        assert_eq!(stage.last_attempt[1], Some(tick.wrapping_add(15)));
    }

    #[test]
    fn unreachable_path_finishes_after_checkpoints_and_unblocks_queue() {
        let n = 64;
        let mut walk = vec![true; n * n];
        for row in walk.chunks_exact_mut(n) {
            row[32] = false;
        }
        let map = Arc::new(grid_map(n, walk));
        let mut stage = PathBudget::default();
        stage.request(1, Arc::clone(&map), [2.0, 2.0], [200.0, 200.0]);
        stage.request(2, Arc::clone(&map), [2.0, 2.0], [22.0, 22.0]);

        let mut failed = false;
        // At most eight pushes per expanded cell plus the initial start entry. Stale heap
        // entries also count towards the checkpoint interval.
        for tick in 0..(n * n * 8 + 1).div_ceil(32) as u32 + 2 {
            // Give the stage one short expansion batch per tick, without wall-clock flakiness.
            let mut checks = 0;
            stage.drain(tick, || {
                checks += 1;
                checks > 2
            });
            if tick == 0 {
                assert_eq!(stage.deferred(), 2);
                assert!(stage.take(1).is_none(), "yield is not a failed route");
                assert!(stage.grid.last_expanded > 0);
                // Updating position for the same goal must not discard the frontier.
                stage.request(1, Arc::clone(&map), [6.0, 2.0], [200.0, 200.0]);
            }
            if let Some(result) = stage.take(1) {
                assert!(result.failed);
                failed = true;
            }
            if let Some(result) = stage.take(2) {
                assert!(!result.failed);
                assert!(failed);
                return;
            }
        }
        panic!("the unreachable search must finish so that the next bot gets a route");
    }

    #[test]
    fn mesh_points_take_priority_and_disconnected_mesh_does_not_use_grid() {
        let poly = |x| Poly {
            verts: vec![[x, 0.0], [x + 8.0, 0.0], [x + 8.0, 8.0], [x, 8.0]],
            y: 0.0,
            centre: [x + 4.0, 4.0],
            area: 0,
            edges: vec![None; 4],
        };
        let mut stage = PathBudget::default();
        let map = Arc::new(
            grid_map(8, vec![true; 8 * 8])
                .with_navmesh(NavMesh::from_polys(vec![poly(0.0), poly(20.0)])),
        );
        stage.request(1, Arc::clone(&map), [2.0, 2.0], [6.0, 6.0]);
        stage.request(2, map, [2.0, 2.0], [24.0, 2.0]);
        stage.process(0, false);
        let route = stage.take(1).unwrap();
        assert!(!route.failed);
        assert_eq!(route.path, [[2.0, 2.0], [6.0, 6.0]]);
        let route = stage.take(2).unwrap();
        assert!(route.failed);
        assert!(route.path.is_empty());
    }

    fn drain_burst(stage: &mut PathBudget, realtime: bool) -> (u32, u64) {
        let n = 64;
        let mut walk = vec![true; n * n];
        for row in walk.chunks_exact_mut(n).take(24) {
            row[27] = false;
        }
        let map = Arc::new(grid_map(n, walk));
        // Give each bot a prior route before all goals change on the same tick.
        let cap = stage.max_queries_per_tick.take();
        for id in 0..100 {
            let start = [42.0, 42.0 + f32::from(id) * 0.5];
            stage.request(id, Arc::clone(&map), start, start);
        }
        stage.process(0, false);
        for id in 0..100 {
            assert!(!stage.take(id).unwrap().failed);
        }
        stage.max_queries_per_tick = cap;
        for id in (0..100).rev() {
            let z = f32::from(id) * 0.5;
            stage.request(id, Arc::clone(&map), [42.0, 42.0 + z], [222.0, 42.0 + z]);
        }
        let mut received = 0;
        let mut max_ns = 0;
        for tick in 0..60 {
            max_ns = max_ns.max(stage.process(MIN_REPATH_TICKS + tick, realtime));
            for id in 0..100 {
                if let Some(route) = stage.take(id) {
                    assert!(!route.failed);
                    assert_eq!(route.path.last(), Some(&route.goal));
                    received += 1;
                }
            }
            if received == 100 {
                return (tick + 1, max_ns);
            }
        }
        panic!("only {received}/100 requests finished in 60 ticks");
    }

    #[test]
    fn hundred_real_searches_finish_under_a_fixed_query_cap() {
        let mut stage = PathBudget {
            max_queries_per_tick: Some(2),
            ..PathBudget::default()
        };
        assert_eq!(drain_burst(&mut stage, false), (50, 0));
    }

    #[test]
    #[ignore = "wall-clock measurement; run in release with --ignored --nocapture"]
    fn measure_realtime_hundred_request_burst() {
        let (ticks, max_ns) = drain_burst(&mut PathBudget::default(), true);
        println!("100/100 routes, {ticks} ticks, max stage {max_ns} ns");
    }
}
