//! M0 navigation: a walkability grid over the (small) map and a deterministic A\* on it.
//!
//! No navmesh before M1-036: the 256 m M0 map is cut into 4 m cells (data: `nav.cell_size_m`).
//! A cell is walkable when every sample inside it (1 m spacing) lies in loaded terrain with a
//! slope of at most `nav.max_slope_deg` (30 deg, a little more careful than the sim's 35 deg).
//! The search is 8-neighbour A\* with integer costs (10 / 14) and the octile heuristic; ties are
//! broken by cell index, so the result is a pure function of the inputs. Paths are smoothed by
//! greedy line-of-walk and returned as waypoints.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use frontline_sim::WorldView;

const ORTHO: u32 = 10;
const DIAG: u32 = 14;
/// Longest shortcut (in path cells) tried by the smoother.
const SMOOTH_REACH: usize = 48;

/// Walkability of the map in square cells; cell `(ix, iz)` covers `[ix*cell, (ix+1)*cell)` east
/// and north.
#[derive(Debug, Clone)]
pub struct NavGrid {
    nx: usize,
    nz: usize,
    cell_m: f32,
    walk: Vec<bool>,
}

impl NavGrid {
    /// Samples the terrain of `[0, size_x] x [0, size_z]` through the read-only view.
    pub fn build(
        view: &dyn WorldView,
        size_x_m: f32,
        size_z_m: f32,
        cell_m: f32,
        max_slope_deg: f32,
    ) -> Self {
        let nx = ((size_x_m / cell_m).floor() as usize).max(1);
        let nz = ((size_z_m / cell_m).floor() as usize).max(1);
        let sub = (cell_m.round() as usize).clamp(1, 8);
        let step = cell_m / sub as f32;
        let (max_x, max_z) = (size_x_m - 0.01, size_z_m - 0.01);
        let mut walk = vec![false; nx * nz];
        for iz in 0..nz {
            for ix in 0..nx {
                let mut ok = true;
                'cell: for sz in 0..=sub {
                    for sx in 0..=sub {
                        let x = (ix as f32 * cell_m + sx as f32 * step).min(max_x);
                        let z = (iz as f32 * cell_m + sz as f32 * step).min(max_z);
                        match view.slope_deg_at(x, z) {
                            Some(s) if s <= max_slope_deg => {}
                            _ => {
                                ok = false;
                                break 'cell;
                            }
                        }
                    }
                }
                walk[iz * nx + ix] = ok;
            }
        }
        NavGrid {
            nx,
            nz,
            cell_m,
            walk,
        }
    }

    /// A grid from explicit flags (row-major, `iz * nx + ix`); for tests.
    pub fn from_walkable(nx: usize, nz: usize, cell_m: f32, walk: Vec<bool>) -> Self {
        assert_eq!(walk.len(), nx * nz);
        NavGrid {
            nx,
            nz,
            cell_m,
            walk,
        }
    }

    pub fn size(&self) -> (usize, usize) {
        (self.nx, self.nz)
    }

    pub fn cell_m(&self) -> f32 {
        self.cell_m
    }

    pub fn walkable_count(&self) -> usize {
        self.walk.iter().filter(|w| **w).count()
    }

    /// The cell containing `(x, z)`, `None` outside the grid.
    pub fn cell_of(&self, x: f32, z: f32) -> Option<(usize, usize)> {
        if !(x.is_finite() && z.is_finite()) || x < 0.0 || z < 0.0 {
            return None;
        }
        let (ix, iz) = ((x / self.cell_m) as usize, (z / self.cell_m) as usize);
        (ix < self.nx && iz < self.nz).then_some((ix, iz))
    }

    pub fn walkable(&self, ix: usize, iz: usize) -> bool {
        ix < self.nx && iz < self.nz && self.walk[iz * self.nx + ix]
    }

    pub fn walkable_at(&self, x: f32, z: f32) -> bool {
        self.cell_of(x, z)
            .is_some_and(|(ix, iz)| self.walkable(ix, iz))
    }

    pub fn center(&self, ix: usize, iz: usize) -> [f32; 2] {
        [
            (ix as f32 + 0.5) * self.cell_m,
            (iz as f32 + 0.5) * self.cell_m,
        ]
    }

    /// The walkable cell closest to `(x, z)` within `max_ring` cells (the cell itself first).
    pub fn snap(&self, x: f32, z: f32, max_ring: usize) -> Option<(usize, usize)> {
        let (cx, cz) = self.cell_of(x, z)?;
        if self.walkable(cx, cz) {
            return Some((cx, cz));
        }
        let mut best: Option<(f32, usize, usize)> = None;
        for r in 1..=max_ring as i32 {
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dz.abs()) != r {
                        continue;
                    }
                    let (ix, iz) = (cx as i32 + dx, cz as i32 + dz);
                    if ix < 0 || iz < 0 || !self.walkable(ix as usize, iz as usize) {
                        continue;
                    }
                    let c = self.center(ix as usize, iz as usize);
                    let d = (c[0] - x) * (c[0] - x) + (c[1] - z) * (c[1] - z);
                    if best.is_none_or(|b| d < b.0) {
                        best = Some((d, ix as usize, iz as usize));
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        best.map(|(_, ix, iz)| (ix, iz))
    }

    /// Whether the straight segment between two points stays on walkable cells.
    pub fn line_walkable(&self, a: [f32; 2], b: [f32; 2]) -> bool {
        let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
        let len = (dx * dx + dz * dz).sqrt();
        let n = ((len / (self.cell_m * 0.25)).ceil() as usize).max(1);
        (0..=n).all(|i| {
            let t = i as f32 / n as f32;
            self.walkable_at(a[0] + dx * t, a[1] + dz * t)
        })
    }
}

/// A\* with reusable scratch memory (no allocation per query after warm-up).
#[derive(Debug, Default)]
pub struct PathFinder {
    stamp: Vec<u32>,
    closed: Vec<u32>,
    g: Vec<u32>,
    parent: Vec<u32>,
    epoch: u32,
    heap: BinaryHeap<Reverse<(u32, u32, u32)>>,
    cells: Vec<u32>,
    goal_cell: (usize, usize),
    goal_point: [f32; 2],
    /// Nodes expanded by the last query (diagnostics).
    pub last_expanded: u32,
}

impl PathFinder {
    pub fn new() -> Self {
        Self::default()
    }

    fn prepare(&mut self, n: usize) {
        if self.stamp.len() != n {
            self.stamp = vec![0; n];
            self.closed = vec![0; n];
            self.g = vec![0; n];
            self.parent = vec![u32::MAX; n];
            self.epoch = 0;
        }
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.stamp.fill(0);
            self.closed.fill(0);
            self.epoch = 1;
        }
        self.heap.clear();
    }

    /// Octile distance, inflated by 3/2 (weighted A\*): paths are at most 1.5x the optimum
    /// before smoothing - in practice almost optimal on open terrain - and far fewer nodes
    /// are expanded, which bounds the worst-case query time (ARCHITECTURE 16.2: <= 1 ms/tick).
    fn heuristic(ax: i32, az: i32, bx: i32, bz: i32) -> u32 {
        let (dx, dz) = ((ax - bx).unsigned_abs(), (az - bz).unsigned_abs());
        (ORTHO * (dx + dz) - (2 * ORTHO - DIAG) * dx.min(dz)) * 3 / 2
    }

    /// Finds a path from `from` to `to` (world x/z). `out` receives the waypoints after the
    /// start, the last one being the goal (snapped to a walkable cell when it is not on one).
    /// Returns `false` (and leaves `out` empty) when no path exists.
    pub fn find(
        &mut self,
        grid: &NavGrid,
        from: [f32; 2],
        to: [f32; 2],
        out: &mut Vec<[f32; 2]>,
    ) -> bool {
        self.begin(grid, from, to, out)
            .unwrap_or_else(|| self.step(grid, from, out, || false).unwrap())
    }

    /// `None` means the initialised search needs `step` with the same grid and start.
    pub(crate) fn begin(
        &mut self,
        grid: &NavGrid,
        from: [f32; 2],
        to: [f32; 2],
        out: &mut Vec<[f32; 2]>,
    ) -> Option<bool> {
        out.clear();
        self.last_expanded = 0;
        let (Some((sx, sz)), Some((gx, gz))) =
            (grid.snap(from[0], from[1], 3), grid.snap(to[0], to[1], 4))
        else {
            return Some(false);
        };
        let goal_point = if grid.walkable_at(to[0], to[1]) {
            to
        } else {
            grid.center(gx, gz)
        };
        if (sx, sz) == (gx, gz) {
            out.push(goal_point);
            return Some(true);
        }
        let (nx, nz) = grid.size();
        self.prepare(nx * nz);
        let start = (sz * nx + sx) as u32;
        self.stamp[start as usize] = self.epoch;
        self.g[start as usize] = 0;
        self.parent[start as usize] = u32::MAX;
        let h0 = Self::heuristic(sx as i32, sz as i32, gx as i32, gz as i32);
        self.heap.push(Reverse((h0, u32::MAX, start)));
        self.goal_cell = (gx, gz);
        self.goal_point = goal_point;
        None
    }

    /// `None` means budget exhausted, not that the path is unreachable.
    pub(crate) fn step(
        &mut self,
        grid: &NavGrid,
        from: [f32; 2],
        out: &mut Vec<[f32; 2]>,
        mut should_yield: impl FnMut() -> bool,
    ) -> Option<bool> {
        let (nx, nz) = grid.size();
        let idx = |x: usize, z: usize| (z * nx + x) as u32;
        let (gx, gz) = self.goal_cell;
        let goal = idx(gx, gz);

        const DIRS: [(i32, i32, u32); 8] = [
            (1, 0, ORTHO),
            (-1, 0, ORTHO),
            (0, 1, ORTHO),
            (0, -1, ORTHO),
            (1, 1, DIAG),
            (1, -1, DIAG),
            (-1, 1, DIAG),
            (-1, -1, DIAG),
        ];
        let mut found = false;
        let mut pops = 0usize;
        while !self.heap.is_empty() {
            if pops.is_multiple_of(32) && should_yield() {
                return None;
            }
            pops += 1;
            let Reverse((_, _, cur)) = self.heap.pop().unwrap();
            let ci = cur as usize;
            if self.closed[ci] == self.epoch {
                continue;
            }
            self.closed[ci] = self.epoch;
            self.last_expanded += 1;
            if cur == goal {
                found = true;
                break;
            }
            let (cx, cz) = ((ci % nx) as i32, (ci / nx) as i32);
            let cg = self.g[ci];
            for (dx, dz, cost) in DIRS {
                let (x, z) = (cx + dx, cz + dz);
                if x < 0 || z < 0 || x >= nx as i32 || z >= nz as i32 {
                    continue;
                }
                if !grid.walkable(x as usize, z as usize) {
                    continue;
                }
                // No corner cutting: both orthogonal neighbours of a diagonal step must be free.
                if dx != 0
                    && dz != 0
                    && !(grid.walkable(x as usize, cz as usize)
                        && grid.walkable(cx as usize, z as usize))
                {
                    continue;
                }
                let ni = idx(x as usize, z as usize);
                if self.closed[ni as usize] == self.epoch {
                    continue;
                }
                let ng = cg + cost;
                if self.stamp[ni as usize] != self.epoch || ng < self.g[ni as usize] {
                    self.stamp[ni as usize] = self.epoch;
                    self.g[ni as usize] = ng;
                    self.parent[ni as usize] = cur;
                    let f = ng + Self::heuristic(x, z, gx as i32, gz as i32);
                    self.heap.push(Reverse((f, u32::MAX - ng, ni)));
                }
            }
        }
        if !found {
            return Some(false);
        }
        // Reconstruct goal -> start, then reverse.
        self.cells.clear();
        let mut c = goal;
        while c != u32::MAX {
            self.cells.push(c);
            c = self.parent[c as usize];
        }
        self.cells.reverse();
        self.smooth(grid, from, self.goal_point, out);
        Some(true)
    }

    /// Greedy line-of-walk smoothing over `self.cells` (start cell first).
    fn smooth(
        &self,
        grid: &NavGrid,
        from: [f32; 2],
        goal_point: [f32; 2],
        out: &mut Vec<[f32; 2]>,
    ) {
        let nx = grid.size().0;
        let pt = |k: usize| {
            let c = self.cells[k] as usize;
            grid.center(c % nx, c / nx)
        };
        let n = self.cells.len();
        let mut anchor_pt = from;
        let mut anchor = 0usize;
        while anchor + 1 < n {
            let far = (anchor + SMOOTH_REACH).min(n - 1);
            let mut next = anchor + 1;
            for j in (anchor + 1..=far).rev() {
                let p = if j == n - 1 { goal_point } else { pt(j) };
                if grid.line_walkable(anchor_pt, p) {
                    next = j;
                    break;
                }
            }
            let p = if next == n - 1 { goal_point } else { pt(next) };
            out.push(p);
            anchor_pt = p;
            anchor = next;
        }
        if out.is_empty() {
            out.push(goal_point);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(n: usize) -> NavGrid {
        NavGrid::from_walkable(n, n, 4.0, vec![true; n * n])
    }

    fn path_len(from: [f32; 2], p: &[[f32; 2]]) -> f32 {
        let mut a = from;
        let mut l = 0.0;
        for b in p {
            l += ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
            a = *b;
        }
        l
    }

    #[test]
    fn flat_ground_gives_a_straight_line() {
        let g = open(64);
        let mut pf = PathFinder::new();
        let mut out = Vec::new();
        assert!(pf.find(&g, [10.0, 10.0], [200.0, 200.0], &mut out));
        assert_eq!(
            out,
            vec![[200.0, 200.0]],
            "smoothing collapses an open diagonal to the goal"
        );
        let mut out2 = Vec::new();
        assert!(pf.find(&g, [10.0, 10.0], [200.0, 10.0], &mut out2));
        assert_eq!(out2, vec![[200.0, 10.0]]);
    }

    #[test]
    fn detours_around_a_blocked_band() {
        // A wall of unwalkable cells at ix = 30..34 spanning iz 0..50 (open above).
        let mut w = vec![true; 64 * 64];
        for iz in 0..50 {
            for ix in 30..34 {
                w[iz * 64 + ix] = false;
            }
        }
        let g = NavGrid::from_walkable(64, 64, 4.0, w);
        let mut pf = PathFinder::new();
        let mut out = Vec::new();
        let (from, to) = ([40.0, 40.0], [220.0, 40.0]);
        assert!(pf.find(&g, from, to, &mut out));
        assert!(out.len() >= 2, "needs at least one corner: {out:?}");
        // Every leg stays on walkable cells and the route is not absurdly long.
        let mut a = from;
        for b in &out {
            assert!(g.line_walkable(a, *b), "{a:?} -> {b:?} crosses the wall");
            a = *b;
        }
        let direct = 180.0;
        assert!(
            path_len(from, &out) < direct * 2.2,
            "{}",
            path_len(from, &out)
        );
        assert_eq!(*out.last().unwrap(), to);
    }

    #[test]
    fn enclosed_goal_is_unreachable() {
        // A ring of blocked cells around (ix, iz) = (50..53, 50..53) with the inside free.
        let mut w = vec![true; 64 * 64];
        for iz in 48..55 {
            for ix in 48..55 {
                let inside = (50..53).contains(&ix) && (50..53).contains(&iz);
                w[iz * 64 + ix] = inside;
            }
        }
        let g = NavGrid::from_walkable(64, 64, 4.0, w);
        let mut pf = PathFinder::new();
        let mut out = Vec::new();
        assert!(!pf.find(&g, [10.0, 10.0], [206.0, 206.0], &mut out));
        assert!(out.is_empty());
        // Off-grid goals have no path either.
        assert!(!pf.find(&g, [10.0, 10.0], [-5.0, 10.0], &mut out));
    }

    #[test]
    fn unwalkable_goal_snaps_to_the_nearest_free_cell() {
        let mut w = vec![true; 16 * 16];
        w[8 * 16 + 8] = false;
        let g = NavGrid::from_walkable(16, 16, 4.0, w);
        let mut pf = PathFinder::new();
        let mut out = Vec::new();
        assert!(pf.find(&g, [2.0, 2.0], [34.0, 34.0], &mut out));
        let last = *out.last().unwrap();
        assert!(g.walkable_at(last[0], last[1]));
    }

    #[test]
    fn same_inputs_same_path() {
        let mut w = vec![true; 64 * 64];
        for iz in 10..60 {
            w[iz * 64 + 20] = false;
            w[iz * 64 + 40] = iz < 55;
        }
        let g = NavGrid::from_walkable(64, 64, 4.0, w);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        assert!(PathFinder::new().find(&g, [8.0, 8.0], [240.0, 200.0], &mut a));
        let mut pf = PathFinder::new();
        assert!(pf.find(&g, [8.0, 8.0], [240.0, 200.0], &mut b));
        assert!(pf.find(&g, [8.0, 8.0], [240.0, 200.0], &mut b));
        assert_eq!(a, b, "scratch reuse must not change results");
    }
}
