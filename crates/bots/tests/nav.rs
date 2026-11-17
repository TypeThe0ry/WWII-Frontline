//! Navigation on the real M0 terrain and on synthetic walls (M0-026 acceptance 5).

mod common;

use common::*;
use frontline_bots::{BotMap, NavGrid, PathFinder, Tuning};
use frontline_sim::Team;

fn grid_of(col: &frontline_sim::CollisionData) -> NavGrid {
    let w = new_world(col, 1);
    let t = Tuning::embedded();
    NavGrid::build(w.view(), MAP_SIZE, MAP_SIZE, t.cell_m, t.max_slope_deg)
}

#[test]
fn real_terrain_connects_all_patrol_points() {
    let Some(col) = terrain() else { return };
    let g = grid_of(&col);
    assert_eq!(g.size(), (64, 64));
    let pts = patrol_points();
    assert_eq!(pts.len(), 8);
    let mut pf = PathFinder::new();
    let mut out = Vec::new();
    let mut max_expanded = 0;
    for a in &pts {
        for b in &pts {
            if a == b {
                continue;
            }
            assert!(pf.find(&g, *a, *b, &mut out), "no path {a:?} -> {b:?}");
            max_expanded = max_expanded.max(pf.last_expanded);
            let mut prev = *a;
            for p in &out {
                assert!(
                    g.line_walkable(prev, *p),
                    "leg {prev:?} -> {p:?} leaves the walkable cells"
                );
                // Cross-check with the sim's own 35 deg slope rule at every waypoint.
                assert!(
                    col.walkable(p[0], p[1]),
                    "waypoint {p:?} is steeper than the sim allows"
                );
                prev = *p;
            }
        }
    }
    eprintln!("patrol-point A* on the M0 map: all 56 pairs connected, worst case {max_expanded} nodes expanded");
}

#[test]
fn detours_around_a_steep_wall_and_reports_unreachable() {
    // A 20 m high wall (far steeper than 30 deg at its edges) across x = 126..130 for z < 200.
    let col = synth(Some((126.0, 130.0, 0.0, 200.0, 20.0)));
    let g = grid_of(&col);
    let mut pf = PathFinder::new();
    let mut out = Vec::new();
    assert!(pf.find(&g, [60.0, 100.0], [200.0, 100.0], &mut out));
    let top = out.iter().map(|p| p[1]).fold(0.0f32, f32::max);
    assert!(
        top > 195.0,
        "path must go round the wall's end, highest z {top}: {out:?}"
    );
    let mut prev = [60.0, 100.0];
    for p in &out {
        assert!(g.line_walkable(prev, *p));
        prev = *p;
    }
    // Wall across the whole map: the other side is unreachable.
    let col2 = synth(Some((126.0, 130.0, 0.0, 256.0, 20.0)));
    let g2 = grid_of(&col2);
    assert!(!pf.find(&g2, [60.0, 100.0], [200.0, 100.0], &mut out));
    assert!(out.is_empty());
    // Open flat ground is a straight line.
    let g3 = grid_of(&synth(None));
    assert!(pf.find(&g3, [10.0, 10.0], [240.0, 240.0], &mut out));
    assert!(out.len() <= 2, "{out:?}");
    assert_eq!(*out.last().unwrap(), [240.0, 240.0]);
}

#[test]
fn ten_bots_reach_three_patrol_points_in_60_s() {
    let Some(col) = terrain() else { return };
    for seed in [11u64, 12, 13, 14] {
        let mut a = Arena::new(&col, seed);
        let map = a.bot_map();
        // One team only, so nobody fights: pure navigation.
        for i in 0..10u8 {
            a.add_bot(
                &map,
                i,
                Team::Us,
                110.0 + i as f32 * 3.0,
                50.0 + (i % 3) as f32 * 3.0,
                0.0,
            );
        }
        for _ in 0..1800 {
            a.step();
        }
        let mut report = Vec::new();
        for (p, _, b) in &a.brains {
            let distinct = b.patrol_reached.iter().filter(|n| **n > 0).count();
            report.push(distinct);
            assert!(
                distinct >= 3,
                "seed {seed}: bot {p} reached only {distinct} distinct patrol points in 60 s ({:?})",
                b.patrol_reached
            );
            let pos = a.pos_of(*p).expect("alive");
            assert!((0.0..=MAP_SIZE).contains(&pos.x) && (0.0..=MAP_SIZE).contains(&pos.z));
        }
        eprintln!(
            "seed {seed}: distinct patrol points reached by the 10 bots within 60 s: {report:?}"
        );
    }
}

#[test]
fn bots_without_nav_stand_still() {
    let col = synth(None);
    let mut a = Arena::new(&col, 2);
    let map = std::sync::Arc::new(BotMap::without_nav(Tuning::embedded(), vec![[10.0, 10.0]]));
    a.add_bot(&map, 0, Team::Us, 100.0, 100.0, 0.0);
    for _ in 0..120 {
        a.step();
    }
    let p = a.pos_of(0).unwrap();
    assert!((p.x - 100.0).abs() < 0.1 && (p.z - 100.0).abs() < 0.1);
}
