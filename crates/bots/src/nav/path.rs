//! Polygon A* (portal-midpoint costs) and string-pulling (simple stupid funnel).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::{NavMesh, V2};

/// Hard limit on expanded polygons per query. The spec's 4,096 is too low for cross-map routes
/// through the old town (the GER HQ path needs ~5,000+ on the 17.8k-polygon mesh), so the cap is
/// 16,384 (PI-086); it still guarantees termination.
pub const MAX_EXPANDED: usize = 16_384;
/// Distance tolerance when snapping the endpoints to the mesh.
pub const SNAP_M: f32 = 2.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub points: Vec<V2>,
    pub length_m: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathResult {
    Found(Path),
    /// The goal polygon was not reached; the path leads to the closest polygon explored.
    Partial(Path),
    NoPath,
}

fn dist(a: V2, b: V2) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn cross(o: V2, a: V2, b: V2) -> f32 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

fn mid(a: V2, b: V2) -> V2 {
    [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5]
}

fn mm(v: f32) -> u32 {
    (v * 1000.0).round().clamp(0.0, u32::MAX as f32) as u32
}

/// Reusable search buffers (a bot owns one; nothing is allocated per query after warm-up).
#[derive(Default)]
pub struct PathFinder {
    g: Vec<u32>,
    parent: Vec<(u32, u8)>,
    entry: Vec<V2>,
    stamp: Vec<u32>,
    now: u32,
    pub last_expanded: usize,
}

impl PathFinder {
    pub fn new() -> Self {
        Self::default()
    }

    /// `area_mask`: bit `a` set means polygons of area `a` are walkable.
    pub fn find(&mut self, mesh: &NavMesh, from: V2, to: V2, area_mask: u64) -> PathResult {
        let n = mesh.polys.len();
        if self.stamp.len() != n {
            self.g = vec![0; n];
            self.parent = vec![(super::NO_POLY, 0); n];
            self.entry = vec![[0.0; 2]; n];
            self.stamp = vec![0; n];
            self.now = 0;
        }
        self.now += 1;
        self.last_expanded = 0;
        let (Some((sp, sq)), Some((gp, gq))) =
            (mesh.nearest(from, SNAP_M), mesh.nearest(to, SNAP_M))
        else {
            return PathResult::NoPath;
        };
        if area_mask >> mesh.polys[sp as usize].area & 1 == 0
            || area_mask >> mesh.polys[gp as usize].area & 1 == 0
        {
            return PathResult::NoPath;
        }
        let mut open: BinaryHeap<Reverse<(u32, u32, u32)>> = BinaryHeap::new();
        self.visit(sp, 0, (super::NO_POLY, 0), sq);
        open.push(Reverse((mm(dist(sq, gq)), sp, 0)));
        let mut best = (f32::MAX, sp);
        let mut found = false;
        while let Some(Reverse((_, id, gpush))) = open.pop() {
            if gpush > self.g[id as usize] {
                continue; // a cheaper route to this polygon was found after this entry was queued
            }
            if id == gp {
                found = true;
                break;
            }
            self.last_expanded += 1;
            if self.last_expanded > MAX_EXPANDED {
                break;
            }
            let p = &mesh.polys[id as usize];
            let h = dist(self.entry[id as usize], gq);
            if h < best.0 {
                best = (h, id);
            }
            for (e, portal) in p.edges.iter().enumerate() {
                let Some(pt) = portal else { continue };
                if area_mask >> mesh.polys[pt.to as usize].area & 1 == 0 {
                    continue;
                }
                let m = mid(pt.a, pt.b);
                let cost = self.g[id as usize] + mm(dist(self.entry[id as usize], m));
                if self.stamp[pt.to as usize] == self.now && self.g[pt.to as usize] <= cost {
                    continue;
                }
                self.visit(pt.to, cost, (id, e as u8), m);
                let hh = mm(dist(m, gq) * 0.999);
                open.push(Reverse((cost + hh, pt.to, cost)));
            }
        }
        let end = if found { gp } else { best.1 };
        let mut corridor = vec![end];
        let mut cur = end;
        while self.parent[cur as usize].0 != super::NO_POLY {
            cur = self.parent[cur as usize].0;
            corridor.push(cur);
        }
        corridor.reverse();
        let goal = if found { gq } else { self.entry[end as usize] };
        let points = funnel(mesh, &corridor, sq, goal);
        let length_m = points.windows(2).map(|w| dist(w[0], w[1])).sum();
        let path = Path { points, length_m };
        if found {
            PathResult::Found(path)
        } else if corridor.len() > 1 {
            PathResult::Partial(path)
        } else {
            PathResult::NoPath
        }
    }

    fn visit(&mut self, id: u32, g: u32, parent: (u32, u8), entry: V2) {
        let i = id as usize;
        self.stamp[i] = self.now;
        self.g[i] = g;
        self.parent[i] = parent;
        self.entry[i] = entry;
    }
}

/// Portal (left, right) between consecutive corridor polygons, oriented for travel.
fn portals(mesh: &NavMesh, corridor: &[u32]) -> Vec<(V2, V2)> {
    let mut out = Vec::with_capacity(corridor.len());
    for w in corridor.windows(2) {
        let (a, b) = (&mesh.polys[w[0] as usize], w[1]);
        let Some(pt) = a.edges.iter().flatten().find(|p| p.to == b) else {
            continue;
        };
        let (c0, c1) = (a.centre, mesh.polys[b as usize].centre);
        // left of the travel direction c0 -> c1 is where cross(c0, c1, p) > 0
        if cross(c0, c1, pt.a) > 0.0 {
            out.push((pt.a, pt.b));
        } else {
            out.push((pt.b, pt.a));
        }
    }
    out
}

pub fn funnel(mesh: &NavMesh, corridor: &[u32], start: V2, goal: V2) -> Vec<V2> {
    let mut pl: Vec<(V2, V2)> = vec![(start, start)];
    pl.extend(portals(mesh, corridor));
    pl.push((goal, goal));
    let mut out = vec![start];
    let (mut apex, mut left, mut right) = (start, start, start);
    let (mut li, mut ri) = (0usize, 0usize);
    let mut i = 1;
    let mut guard = 0;
    while i < pl.len() && guard < 100_000 {
        guard += 1;
        let (nl, nr) = pl[i];
        // tighten right
        if cross(apex, right, nr) >= 0.0 {
            if apex == right || cross(apex, left, nr) < 0.0 {
                right = nr;
                ri = i;
            } else {
                out.push(left);
                apex = left;
                let ai = li;
                left = apex;
                right = apex;
                li = ai;
                ri = ai;
                i = ai + 1;
                continue;
            }
        }
        // tighten left
        if cross(apex, left, nl) <= 0.0 {
            if apex == left || cross(apex, right, nl) > 0.0 {
                left = nl;
                li = i;
            } else {
                out.push(right);
                apex = right;
                let ai = ri;
                left = apex;
                right = apex;
                li = ai;
                ri = ai;
                i = ai + 1;
                continue;
            }
        }
        i += 1;
    }
    if out.last() != Some(&goal) {
        out.push(goal);
    }
    out
}
