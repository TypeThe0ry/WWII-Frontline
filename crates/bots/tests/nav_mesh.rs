//! M1-037: navmesh loading, A* + funnel, stuck handling, replan cap and avoidance, on the real
//! Carentan 1/3 mesh (skipped when `data/maps/carentan/m1/navmesh.bin` is absent) and on a
//! hand-built L-shaped corridor.

use std::path::PathBuf;

use frontline_bots::nav::path::{PathFinder, PathResult};
use frontline_bots::nav::steer::{Status, Steering};
use frontline_bots::nav::{NavMesh, Poly, Portal, V2};

fn data_file(rel: &str) -> Option<PathBuf> {
    let mut d = std::env::current_dir().ok()?;
    loop {
        let f = d.join(rel);
        if f.is_file() {
            return Some(f);
        }
        if !d.pop() {
            return None;
        }
    }
}

fn carentan() -> Option<NavMesh> {
    let p = data_file("data/maps/carentan/m1/navmesh.bin")?;
    Some(NavMesh::load(&p).expect("navmesh.bin parses"))
}

fn quad(x0: f32, z0: f32, x1: f32, z1: f32) -> Poly {
    let v = vec![[x0, z0], [x0, z1], [x1, z1], [x1, z0]];
    Poly {
        centre: [(x0 + x1) / 2.0, (z0 + z1) / 2.0],
        verts: v,
        y: 0.0,
        area: 1,
        edges: vec![None; 4],
    }
}

/// Edge `i` of `quad` runs from vertex i to i+1: 0 west, 1 north, 2 east, 3 south.
fn link(p: &mut [Poly], a: usize, ea: usize, b: usize, eb: usize) {
    let (va, vb) = (p[a].verts[ea], p[a].verts[(ea + 1) % 4]);
    p[a].edges[ea] = Some(Portal {
        to: b as u32,
        a: va,
        b: vb,
    });
    p[b].edges[eb] = Some(Portal {
        to: a as u32,
        a: vb,
        b: va,
    });
}

/// L-corridor: A(0..10, 0..10) -> B(10..20, 0..10) -> C(10..20, 10..30).
fn l_mesh() -> NavMesh {
    let mut p = vec![
        quad(0.0, 0.0, 10.0, 10.0),
        quad(10.0, 0.0, 20.0, 10.0),
        quad(10.0, 10.0, 20.0, 30.0),
    ];
    link(&mut p, 0, 2, 1, 0);
    link(&mut p, 1, 1, 2, 3);
    NavMesh::from_polys(p)
}

fn dist(a: V2, b: V2) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

#[test]
fn funnel_cuts_the_corner_of_an_l_corridor() {
    let m = l_mesh();
    let mut pf = PathFinder::new();
    let r = pf.find(&m, [1.0, 1.0], [19.0, 29.0], !0);
    let PathResult::Found(p) = r else {
        panic!("no path")
    };
    // straight is not possible; the optimum bends at the inner corner (10, 10)
    assert!(
        p.points.iter().any(|q| dist(*q, [10.0, 10.0]) < 0.01),
        "{:?}",
        p.points
    );
    let opt = dist([1.0, 1.0], [10.0, 10.0]) + dist([10.0, 10.0], [19.0, 29.0]);
    assert!((p.length_m - opt).abs() < 0.05, "{} vs {opt}", p.length_m);
}

#[test]
fn unreachable_and_off_mesh_inputs() {
    let m = l_mesh();
    let mut pf = PathFinder::new();
    assert_eq!(
        pf.find(&m, [1.0, 1.0], [100.0, 100.0], !0),
        PathResult::NoPath
    );
    assert_eq!(pf.find(&m, [1.0, 1.0], [19.0, 29.0], 0), PathResult::NoPath); // area mask excludes ground
                                                                              // disconnected island -> Partial towards the closest reachable polygon
    let mut p = vec![
        quad(0.0, 0.0, 10.0, 10.0),
        quad(10.0, 0.0, 20.0, 10.0),
        quad(50.0, 0.0, 60.0, 10.0),
    ];
    link(&mut p, 0, 2, 1, 0);
    let m = NavMesh::from_polys(p);
    assert!(matches!(
        pf.find(&m, [1.0, 5.0], [55.0, 5.0], !0),
        PathResult::Partial(_)
    ));
}

#[test]
fn expansion_cap_is_enforced() {
    // a 1 x 20000 strip of quads needs 20000 expansions > 16384
    let mut p: Vec<Poly> = (0..20_000)
        .map(|i| quad(i as f32, 0.0, i as f32 + 1.0, 1.0))
        .collect();
    for i in 0..19_999 {
        link(&mut p, i, 2, i + 1, 0);
    }
    let m = NavMesh::from_polys(p);
    let mut pf = PathFinder::new();
    let r = pf.find(&m, [0.5, 0.5], [19_999.5, 0.5], !0);
    assert!(
        matches!(r, PathResult::Partial(_)),
        "capped search must not report Found"
    );
    assert!(pf.last_expanded <= 16_385);
}

#[test]
fn missing_navmesh_falls_back_without_panic() {
    assert!(NavMesh::load_or_log(std::path::Path::new("/nonexistent/navmesh.bin")).is_none());
    assert!(NavMesh::from_fnav(b"junk").is_err());
}

#[test]
fn nav_hq_to_objectives() {
    let Some(m) = carentan() else { return };
    let rep: serde_json::Value = serde_json::from_slice(
        &std::fs::read(data_file("reports/m1-navmesh.json").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        m.poly_count() as u64,
        rep["walk"]["polys"].as_u64().unwrap(),
        "polygon count of the M1-036 walk mesh"
    );
    let zones: serde_json::Value = serde_json::from_slice(
        &std::fs::read(data_file("data/maps/carentan/m1/zones.json").unwrap()).unwrap(),
    )
    .unwrap();
    let spawns: serde_json::Value = serde_json::from_slice(
        &std::fs::read(data_file("data/maps/carentan/m1/spawns.json").unwrap()).unwrap(),
    )
    .unwrap();
    let pos =
        |v: &serde_json::Value| [v[0].as_f64().unwrap() as f32, v[1].as_f64().unwrap() as f32];
    let us = pos(&spawns["hq"]["US"]["pos"]);
    let ger = pos(&spawns["hq"]["GER"]["pos"]);
    let mut targets = vec![("GER", ger)];
    for line in zones["lines"].as_array().unwrap() {
        for p in line["points"].as_array().unwrap() {
            let id = p["id"].as_str().unwrap();
            if ["L1a", "L2a", "L3a"].contains(&id) {
                targets.push((
                    if id == "L1a" {
                        "L1a"
                    } else if id == "L2a" {
                        "L2a"
                    } else {
                        "L3a"
                    },
                    pos(&p["pos"]),
                ));
            }
        }
    }
    assert_eq!(targets.len(), 4);
    let mut pf = PathFinder::new();
    // Detour (Node, M1-036, goal reached within 3 m) lengths for the same pairs
    for (name, expect) in [
        ("L1a", 628.2f32),
        ("L2a", 905.9),
        ("L3a", 1245.2),
        ("GER", 1641.5),
    ] {
        let t = targets.iter().find(|t| t.0 == name).unwrap().1;
        let PathResult::Found(p) = pf.find(&m, us, t, !0) else {
            panic!("{name}: no path")
        };
        assert!(
            p.length_m <= dist(us, t) * 2.5,
            "{name}: {} vs straight {}",
            p.length_m,
            dist(us, t)
        );
        assert!(
            (p.length_m - expect).abs() / expect < 0.05,
            "{name}: {} vs Detour {expect}",
            p.length_m
        );
    }
}

#[test]
fn replans_are_capped_at_2hz_and_goal_reuse() {
    let m = l_mesh();
    let mut s = Steering::new();
    let mut pos = [1.0, 1.0];
    for tick in 0..600 {
        // the goal jitters by 4 m every tick, which would force a replan each tick without the cap
        s.set_goal([19.0, 20.0 + (tick % 2) as f32 * 4.0]);
        let (_st, mv) = s.step(&m, pos, 1.0 / 60.0, &[]);
        pos[0] += mv.dir[0] * 0.05;
        pos[1] += mv.dir[1] * 0.05;
    }
    assert!(s.plans <= 20, "{} plans in 10 s", s.plans);
}

#[test]
fn stuck_bot_replans_and_reports_unreachable_within_8s() {
    let m = l_mesh();
    let mut s = Steering::new();
    s.set_goal([19.0, 29.0]);
    let mut t = 0.0;
    let mut reported = None;
    while t < 12.0 {
        let (st, _) = s.step(&m, [1.0, 1.0], 1.0 / 30.0, &[]); // never moves
        t += 1.0 / 30.0;
        if st == Status::Unreachable {
            reported = Some(t);
            break;
        }
    }
    let at = reported.expect("unreachable was reported");
    assert!(at <= 8.0, "reported after {at}");
    assert!(s.plans >= 2, "a replan happened before giving up");
}

#[test]
fn twenty_bots_share_a_goal_without_stacking() {
    let Some(m) = carentan() else { return };
    let us = [480.0, 1600.0];
    let goal = [905.0, 1450.0];
    let n = 20;
    let mut pos: Vec<V2> = (0..n)
        .map(|i| [us[0] + (i % 5) as f32 * 1.2, us[1] + (i / 5) as f32 * 1.2])
        .collect();
    let mut st: Vec<Steering> = (0..n).map(|_| Steering::new()).collect();
    for s in &mut st {
        s.set_goal(goal);
    }
    let (mut close, mut total) = (0u32, 0u32);
    let dt = 1.0 / 30.0;
    for tick in 0..(60 * 30) {
        let snapshot = pos.clone();
        for i in 0..n {
            let others: Vec<V2> = (0..n).filter(|j| *j != i).map(|j| snapshot[j]).collect();
            let (_s, mv) = st[i].step(&m, pos[i], dt, &others);
            pos[i][0] += mv.dir[0] * 4.0 * dt;
            pos[i][1] += mv.dir[1] * 4.0 * dt;
        }
        if tick % 30 == 0 {
            for i in 0..n {
                for j in i + 1..n {
                    total += 1;
                    if dist(pos[i], pos[j]) < 0.5 {
                        close += 1;
                    }
                }
            }
        }
    }
    assert!(
        (close as f32) / (total as f32) < 0.02,
        "{close}/{total} pairs closer than 0.5 m"
    );
}
