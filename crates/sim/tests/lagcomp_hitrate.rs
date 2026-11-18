//! M1-068b / E1-5: hit-rate comparison of aimed shots at 0 ms and 150 ms RTT. A target strafes (random speed, random
//! direction, random reversals) across a standing shooter; the *client* aims at the pose it is displaying (the target pose
//! `view_tick` ticks ago, no lead) and fires once. `view_tick` is the client's interpolation delay (100 ms = 3 ticks) plus half
//! the RTT in ticks (0 ms -> 3, 150 ms -> 3 + 2 = 5), all below the 8-tick clamp. With lag compensation the hit rate must not
//! depend on the RTT (acceptance E1-5: difference <= 5 percentage points); without it the 150 ms rate collapses (control).
//!
//! This is a sim-level scenario: the transport (netem loss, jitter) is exercised by `scripts/loadtest-netem.sh`; here the
//! network is represented by the rewind the server would be told, so the statistic isolates the lag-compensation logic.
mod common;

use common::*;
use frontline_sim::weapon::ballistics::BallisticsParams;
use frontline_sim::{SimEvent, Team, World};

const US: u8 = 0;
const GER: u8 = 1;
const TRIALS: u32 = 600;

fn yaw_code(yaw: f32) -> u16 {
    let t = yaw.rem_euclid(core::f32::consts::TAU) / core::f32::consts::TAU;
    ((t * 65536.0) as u32 & 0xffff) as u16
}

/// One aimed shot; true when the server reports a hit by the shooter.
fn trial(seed: u64, rewind: u16, enabled: bool, jitter_ticks: i32) -> bool {
    let mut rng = Rng(seed.wrapping_mul(0x9e3779b97f4a7c15) | 1);
    let gap = rng.range(10.0, 50.0);
    let speed_axis = (rng.range(20.0, 127.0)) as i8; // fraction of walk speed
    let dir: i8 = if rng.f32() < 0.5 { 1 } else { -1 };
    let reverse_at = 30 + (rng.f32() * 20.0) as usize; // the target may reverse at this tick (50 % of trials)
    let reverses = rng.f32() < 0.5;
    let fire_at = 40 + (rng.f32() * 30.0) as usize;
    // human aim error: up to 0.2 m sideways at the target (the hit box is about 0.5 m wide), and the RTT estimate's jitter
    let aim_err = rng.range(-0.2, 0.2);
    let tick_err = if jitter_ticks == 0 {
        0
    } else {
        (rng.f32() * (2 * jitter_ticks + 1) as f32) as i32 - jitter_ticks
    };
    let told = (i32::from(rewind) + tick_err).max(1) as u16; // what the shot reports as its view delta
    let shown = rewind; // what the client actually displayed
    let data = no_spread();
    let mut w: World = duel_world(
        data,
        &flat(4, 5.0),
        seed,
        &[
            (US, Team::Us, 100.0, 100.0, 16384),
            (GER, Team::Ger, 100.0 + gap, 100.0, 0),
        ],
    );
    w.set_lagcomp_enabled(enabled);
    let bp = BallisticsParams::from_def(data.weapon("us_m1_garand").unwrap());
    let mut poses: Vec<(f32, f32)> = Vec::new();
    let mut hit = false;
    for n in 0..140usize {
        let mut shooter = cmd(0, 0, 16384, 0, 0);
        if n == fire_at && poses.len() > shown as usize {
            // the displayed pose: the one `rewind` ticks ago
            let (sx, sz) = poses[poses.len() - shown as usize];
            let (dx0, dz0) = (sx - 100.0, sz - 100.0);
            let d0 = (dx0 * dx0 + dz0 * dz0).sqrt();
            let dx = dx0 - dz0 / d0 * aim_err;
            let dz = dz0 + dx0 / d0 * aim_err;
            let dist = (dx * dx + dz * dz).sqrt();
            let pitch = solve_pitch(&bp, 1.65, dist, 1.235);
            shooter = cmd(0, 0, yaw_code(dx.atan2(dz)), BTN_FIRE, 0);
            shooter.pitch = frontline_protocol::quant::rad_to_pitch_i16(pitch).code;
            shooter.view_tick = told;
        }
        let d = if reverses && n >= reverse_at {
            -dir
        } else {
            dir
        };
        let target = cmd(0, speed_axis.saturating_mul(d), 0, 0, 0);
        let ev = w.step(&[(US, shooter), (GER, target)]).events;
        let p = w.player_state(GER).unwrap().pos;
        poses.push((p.x, p.z));
        for e in &ev {
            if let SimEvent::Hit { shooter, .. } = e {
                if *shooter < 100 {
                    hit = true;
                }
            }
        }
    }
    hit
}

fn rate(rewind: u16, enabled: bool, jitter: i32) -> (u32, f64) {
    let hits: u32 = (0..TRIALS)
        .map(|s| u32::from(trial(1000 + u64::from(s), rewind, enabled, jitter)))
        .sum();
    (hits, f64::from(hits) / f64::from(TRIALS))
}

#[test]
fn hit_rate_at_150ms_rtt_is_within_5_points_of_0ms() {
    let (h0, r0) = rate(3, true, 0); // 0 ms RTT: 100 ms interpolation only
    let (h150, r150) = rate(5, true, 0); // 150 ms RTT: + 75 ms one way (2 ticks)
    let (hj, rj) = rate(5, true, 1); // 150 ms with the view delta off by up to +-1 tick (33 ms of RTT jitter)
    let (hc0, rc0) = rate(3, false, 0);
    let (hc150, rc150) = rate(5, false, 0);
    let pp = |a: f64, b: f64| (a - b) * 100.0;
    eprintln!("E1-5 hit rate ({TRIALS} aimed shots each): 0 ms {h0} = {:.1} %, 150 ms {h150} = {:.1} % (diff {:+.1} pp), 150 ms +-1 tick jitter {hj} = {:.1} % (diff {:+.1} pp)", r0 * 100.0, r150 * 100.0, pp(r150, r0), rj * 100.0, pp(rj, r0));
    eprintln!(
        "control without lag compensation: 0 ms {hc0} = {:.1} %, 150 ms {hc150} = {:.1} %",
        rc0 * 100.0,
        rc150 * 100.0
    );
    assert!(
        r0 > 0.3 && r0 < 0.95,
        "the scenario must have both hits and misses to be a valid test: {r0}"
    );
    // standard error of a difference of two proportions of 600 trials each is about 2 points
    assert!(
        (r150 - r0).abs() <= 0.05,
        "compensated hit rate differs by {:.1} pp",
        pp(r150, r0)
    );
    assert!(
        rc150 < r150 - 0.15,
        "control without compensation must be clearly worse: {rc150} vs {r150}"
    );
}
