//! M1-023b: turret slew limits and main-gun reload rules.
use frontline_sim::vehicle::gun::{FireError, Gun, GunParams, ShellSlot, NO_COMMANDER_RELOAD_MULT};
use frontline_sim::vehicle::turret::{Turret, TurretParams};

const HZ: f32 = 60.0;
const DT: f32 = 1.0 / HZ;

fn tp() -> TurretParams {
    TurretParams {
        traverse_deg_s: 24.0,
        elev_min_deg: -10.0,
        elev_max_deg: 25.0,
    }
}

fn gp() -> GunParams {
    GunParams {
        reload_s: 3.0,
        switch_reload_s: 1.0,
        ammo: [30, 24, 6],
    }
}

fn traverse_rate(mult: f32, jammed: bool) -> f32 {
    let mut t = Turret::default();
    for _ in 0..60 {
        t.step(&tp(), 3.0, 0.0, mult, jammed, DT);
    }
    t.yaw.to_degrees()
}

#[test]
fn traverse_rate_matches_data() {
    let r = traverse_rate(1.0, false);
    assert!((r - 24.0).abs() <= 24.0 * 0.02, "{r}");
}

#[test]
fn damaged_drive_halves_and_jam_is_3_deg() {
    let r = traverse_rate(0.5, false);
    assert!((r - 12.0).abs() <= 12.0 * 0.05, "{r}");
    let j = traverse_rate(1.0, true);
    assert!((j - 3.0).abs() <= 0.15, "{j}");
}

#[test]
fn elevation_is_clamped() {
    let mut t = Turret::default();
    for _ in 0..600 {
        t.step(&tp(), 0.0, 1.0, 1.0, false, DT);
    }
    assert!((t.pitch.to_degrees() - 25.0).abs() < 1e-3);
    for _ in 0..600 {
        t.step(&tp(), 0.0, -1.0, 1.0, false, DT);
    }
    assert!((t.pitch.to_degrees() + 10.0).abs() < 1e-3);
}

#[test]
fn turret_takes_the_short_way_round() {
    let mut t = Turret::default();
    for _ in 0..30 {
        t.step(&tp(), 350f32.to_radians(), 0.0, 1.0, false, DT);
    }
    assert!(t.yaw < 0.0, "went the long way: {}", t.yaw);
}

#[test]
fn reload_3s_and_commander_penalty() {
    let mut g = Gun::new(&gp());
    assert_eq!(g.fire(&gp(), 1.0, HZ), Ok(ShellSlot::Ap));
    assert_eq!(g.reload_left, 180);
    assert_eq!(g.fire(&gp(), 1.0, HZ), Err(FireError::Loading));
    for _ in 0..180 {
        g.tick();
    }
    assert!(g.ready());
    g.fire(&gp(), NO_COMMANDER_RELOAD_MULT, HZ).unwrap();
    assert_eq!(g.reload_left, 234); // 3.9 s
}

#[test]
fn shell_switch_costs_one_second_and_ammo_runs_out() {
    let mut g = Gun::new(&gp());
    g.select(&gp(), ShellSlot::Wp, 1.0, HZ);
    assert_eq!(g.reload_left, 60);
    g.select(&gp(), ShellSlot::Wp, 1.0, HZ); // same shell: no extra cost
    assert_eq!(g.reload_left, 60);
    for _ in 0..6 {
        while !g.ready() {
            g.tick();
        }
        g.fire(&gp(), 1.0, HZ).unwrap();
    }
    while !g.ready() {
        g.tick();
    }
    assert_eq!(g.fire(&gp(), 1.0, HZ), Err(FireError::OutOfAmmo));
    assert_eq!(g.ammo[ShellSlot::Wp as usize], 0);
}
