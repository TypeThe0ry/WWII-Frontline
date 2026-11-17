//! M0-012 acceptance 1: `ballistics_vs_reference`. The 4-sub-steps-per-tick integrator of
//! GAMEDESIGN 6.4 against an RK4 reference (step 0.1 ms, f64) of the same ODE
//! `a = (0, -g) - k |v| v`, horizontal launch.
mod common;

use frontline_sim::weapon::ballistics::{substep, BallisticsParams, SUBSTEPS};
use frontline_sim::{GameData, Vec3};

#[derive(Clone, Copy)]
struct S {
    t: f64,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

fn deriv(s: &S, k: f64, g: f64) -> [f64; 4] {
    let sp = (s.vx * s.vx + s.vy * s.vy).sqrt();
    [s.vx, s.vy, -k * sp * s.vx, -g - k * sp * s.vy]
}

fn axpy(s: &S, d: &[f64; 4], h: f64) -> S {
    S {
        t: s.t + h,
        x: s.x + d[0] * h,
        y: s.y + d[1] * h,
        vx: s.vx + d[2] * h,
        vy: s.vy + d[3] * h,
    }
}

/// RK4 reference state at horizontal distance `range` (linear interpolation inside the last step).
fn rk4_at(v0: f64, k: f64, g: f64, range: f64) -> S {
    let h = 1e-4;
    let mut s = S {
        t: 0.0,
        x: 0.0,
        y: 0.0,
        vx: v0,
        vy: 0.0,
    };
    loop {
        let k1 = deriv(&s, k, g);
        let k2 = deriv(&axpy(&s, &k1, h / 2.0), k, g);
        let k3 = deriv(&axpy(&s, &k2, h / 2.0), k, g);
        let k4 = deriv(&axpy(&s, &k3, h), k, g);
        let n = S {
            t: s.t + h,
            x: s.x + h / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]),
            y: s.y + h / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]),
            vx: s.vx + h / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]),
            vy: s.vy + h / 6.0 * (k1[3] + 2.0 * k2[3] + 2.0 * k3[3] + k4[3]),
        };
        if n.x >= range {
            let f = (range - s.x) / (n.x - s.x);
            let l = |a: f64, b: f64| a + f * (b - a);
            return S {
                t: l(s.t, n.t),
                x: range,
                y: l(s.y, n.y),
                vx: l(s.vx, n.vx),
                vy: l(s.vy, n.vy),
            };
        }
        s = n;
    }
}

/// The sim integrator at 30 Hz x 4 sub-steps: state at horizontal distance `range`.
fn sim_at(bp: &BallisticsParams, range: f32) -> (f32, f32, f32, f32) {
    let h = 1.0 / (30.0 * SUBSTEPS as f32);
    let (mut p, mut v) = (Vec3::ZERO, Vec3::new(bp.muzzle_velocity_mps, 0.0, 0.0));
    let mut t = 0.0f32;
    loop {
        let (p0, v0) = (p, v);
        substep(&mut p, &mut v, bp.drag_k_per_m, bp.gravity_mps2, h);
        t += h;
        if p.x >= range {
            let f = (range - p0.x) / (p.x - p0.x);
            let l = |a: f32, b: f32| a + f * (b - a);
            let speed0 = (v0.x * v0.x + v0.y * v0.y).sqrt();
            let speed1 = (v.x * v.x + v.y * v.y).sqrt();
            return (l(p0.y, p.y), l(speed0, speed1), t - h * (1.0 - f), 0.0);
        }
    }
}

#[test]
fn ballistics_vs_reference() {
    for id in ["us_m1_garand", "ger_kar98k"] {
        let def = GameData::embedded().weapon(id).unwrap();
        let bp = BallisticsParams::from_def(def);
        println!(
            "{id}: v0 {} m/s, k {}, g {}",
            bp.muzzle_velocity_mps, bp.drag_k_per_m, bp.gravity_mps2
        );
        for range in [100.0f32, 200.0, 300.0, 500.0] {
            let r = rk4_at(
                f64::from(bp.muzzle_velocity_mps),
                f64::from(bp.drag_k_per_m),
                f64::from(bp.gravity_mps2),
                f64::from(range),
            );
            let (drop, speed, tof, _) = sim_at(&bp, range);
            let r_drop = -r.y;
            let r_speed = (r.vx * r.vx + r.vy * r.vy).sqrt();
            let e_drop = (f64::from(-drop) - r_drop).abs() / r_drop;
            let e_speed = (f64::from(speed) - r_speed).abs() / r_speed;
            println!(
                "  {range:>4} m: drop sim {:.4} m / RK4 {r_drop:.4} m (err {:.3} %); speed sim {speed:.2} / RK4 {r_speed:.2} m/s (err {:.4} %); TOF sim {tof:.4} / RK4 {:.4} s",
                -drop,
                e_drop * 100.0,
                e_speed * 100.0,
                r.t
            );
            // Acceptance: < 3 % at 500 m. The semi-implicit Euler of GAMEDESIGN 6.4 overestimates the
            // drop by ~ 1/2 g h t (h = 1/120 s): 5 mm at 100 m, which is a large *relative* error of
            // a tiny drop, so the nearer ranges are checked in absolute terms (< 3 cm).
            if range >= 500.0 {
                assert!(e_drop < 0.03, "{id} {range} m drop error {e_drop}");
            }
            if range < 500.0 {
                assert!(
                    (f64::from(-drop) - r_drop).abs() < 0.03,
                    "{id} {range} m drop {drop} vs {r_drop}"
                );
            }
            assert!(e_speed < 0.03, "{id} {range} m speed error {e_speed}");
        }
    }
}

#[test]
fn garand_500m_against_the_design_document_example() {
    // GAMEDESIGN 6.4: 853 m/s, k = 0.00075: 500 m -> ~586 m/s, TOF ~0.72 s, drop ~2.5 m (the
    // text uses 1/2 g t^2 with the time of flight; the coupled drag makes the true drop smaller).
    let def = GameData::embedded().weapon("us_m1_garand").unwrap();
    let bp = BallisticsParams::from_def(def);
    let (drop, speed, tof, _) = sim_at(&bp, 500.0);
    let doc_speed = 853.0 * (-0.00075f32 * 500.0).exp();
    let doc_drop = 0.5 * 9.81 * 0.72f32 * 0.72;
    println!(
        "Garand 500 m: speed {speed:.1} m/s (doc {doc_speed:.1} m/s, {:+.2} %), TOF {tof:.3} s (doc 0.72, {:+.2} %), drop {:.3} m (doc {doc_drop:.2} m, {:+.2} %)",
        (speed / doc_speed - 1.0) * 100.0,
        (tof / 0.72 - 1.0) * 100.0,
        -drop,
        (-drop / doc_drop - 1.0) * 100.0
    );
    assert!((speed - 586.0).abs() < 4.0);
    assert!((tof - 0.72).abs() < 0.02);
    assert!(
        -drop < doc_drop,
        "drag coupling: drop is a bit smaller than 1/2 g t^2"
    );
    assert!(-drop > 2.2);
}
