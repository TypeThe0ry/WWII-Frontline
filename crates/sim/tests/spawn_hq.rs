//! M1-019a: HQ spawn rules: timing, penalties, protection, placement, HQ protection zone.
mod common;

use common::*;
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::spawn::place::pick_spawn;
use frontline_sim::{LifeStateView, SimEvent, Team, World};
use glam::Vec3;

const HZ: u32 = 30;

fn idle() -> InputCmd {
    cmd(0, 0, 49152, 0, 0)
}

fn world() -> World {
    let players = [
        (0u8, Team::Us, 300.0, 60.0, 16384u16),
        (1, Team::Ger, 600.0, 60.0, 16384),
    ];
    let mut w = duel_world(
        data_with(|_| {}),
        &synth_data(14, 2, &|_, _| 5.0),
        5,
        &players,
    );
    w.set_hq(Team::Us, Vec3::new(60.0, 5.0, 60.0));
    w.set_hq(Team::Ger, Vec3::new(800.0, 5.0, 60.0));
    w
}

fn step(w: &mut World) -> Vec<SimEvent> {
    let ins: Vec<_> = w
        .view()
        .players()
        .iter()
        .map(|p| (p.player, idle()))
        .collect();
    w.step(&ins).events
}

fn tick(w: &World) -> u32 {
    w.tick().0
}

fn run_to(w: &mut World, t: u32) {
    while tick(w) < t {
        step(w);
    }
}

fn pos(w: &World, p: u8) -> Vec3 {
    w.view()
        .players()
        .iter()
        .find(|v| v.player == p)
        .unwrap()
        .pos
}

/// Kills player 0 and returns the tick it spawns again.
fn kill_and_wait(w: &mut World) -> u32 {
    w.debug_kill(0);
    step(w);
    let LifeStateView::Dead { respawn_tick, .. } = w.player_life(0).unwrap() else {
        panic!("not dead");
    };
    while !matches!(w.player_life(0).unwrap(), LifeStateView::Alive) {
        step(w);
        assert!(tick(w) < respawn_tick.0 + 5, "never respawned");
    }
    assert_eq!(tick(w), respawn_tick.0);
    tick(w)
}

#[test]
fn hq_spawn_is_wave_aligned_and_near_the_hq() {
    let mut w = world();
    run_to(&mut w, 20 * HZ);
    let k = tick(&w) + 1;
    let t = kill_and_wait(&mut w);
    assert_eq!(t % (15 * HZ), 0, "wave aligned");
    let wait = (t - k) as f32 / HZ as f32;
    assert!((5.0..=20.0).contains(&wait), "{wait}");
    let d = pos(&w, 0) - Vec3::new(60.0, 5.0, 60.0);
    assert!(d.x.hypot(d.z) <= 8.0 + 1e-3, "{d:?}");
}

#[test]
fn suicide_penalty_adds_five_seconds() {
    let mut w = world();
    run_to(&mut w, 20 * HZ);
    let first = kill_and_wait(&mut w); // a wave tick W
    run_to(&mut w, first + 10 * HZ - 1);
    // Died 10 s after spawning: ready W+15 s -> W+15; with +5 s -> W+20 s -> next wave W+30 s.
    w.debug_kill(0);
    step(&mut w);
    let LifeStateView::Dead { respawn_tick, .. } = w.player_life(0).unwrap() else {
        panic!()
    };
    // The queue entry is the unpenalised wave; the rules stage re-queues it.
    assert_eq!(respawn_tick.0, first + 15 * HZ);
    run_to(&mut w, first + 15 * HZ);
    assert!(matches!(
        w.player_life(0).unwrap(),
        LifeStateView::Dead { .. }
    ));
    run_to(&mut w, first + 30 * HZ);
    assert!(matches!(w.player_life(0).unwrap(), LifeStateView::Alive));
}

#[test]
fn warmup_halves_the_base_wait() {
    let mut w = world();
    w.begin_warmup();
    run_to(&mut w, 26 * HZ);
    // Killed at 26 s: halved wait ready 28.5 s -> wave 30 s; full wait ready 31 s -> wave 45 s.
    let t = kill_and_wait(&mut w);
    assert_eq!(t, 30 * HZ, "warm-up: ready 28.5 s -> wave 30 s");
    let mut live = world();
    run_to(&mut live, 26 * HZ);
    let t = kill_and_wait(&mut live);
    assert_eq!(t, 45 * HZ, "no warm-up: ready 31 s -> wave 45 s");
}

#[test]
fn spawn_protection_blocks_damage_until_moving() {
    let mut w = world();
    run_to(&mut w, 20 * HZ);
    kill_and_wait(&mut w);
    let left = w.spawn_protection_left(0);
    assert!((88..=90).contains(&left), "{left}");
    w.debug_kill(0);
    step(&mut w);
    assert!(matches!(w.player_life(0).unwrap(), LifeStateView::Alive));
    assert_eq!(w.health(0).unwrap(), 100.0);
    // Moving cancels the protection at once.
    let ins = [(0u8, cmd(0, 100, 49152, 0, 0)), (1u8, idle())];
    w.step(&ins);
    assert_eq!(w.spawn_protection_left(0), 0);
    w.debug_kill(0);
    step(&mut w);
    assert!(matches!(
        w.player_life(0).unwrap(),
        LifeStateView::Dead { .. }
    ));
}

#[test]
fn protection_expires_after_three_seconds() {
    let mut w = world();
    run_to(&mut w, 20 * HZ);
    let t = kill_and_wait(&mut w);
    run_to(&mut w, t + 3 * HZ + 1);
    assert_eq!(w.spawn_protection_left(0), 0);
    w.debug_kill(0);
    step(&mut w);
    assert!(matches!(
        w.player_life(0).unwrap(),
        LifeStateView::Dead { .. }
    ));
}

#[test]
fn hq_zone_warns_then_hurts_enemies_only() {
    let mut w = world();
    // Player 0 (US) walks into the German HQ zone (HQ x=800, r=150): teleport to x=700.
    w.debug_teleport(0, Vec3::new(700.0, 5.02, 60.0));
    let mut warned = None;
    let hp0 = w.health(0).unwrap();
    for _ in 0..(2 * HZ) {
        for e in step(&mut w) {
            if let SimEvent::HqWarning { player: 0 } = e {
                warned = Some(tick(&w));
            }
        }
    }
    assert!(warned.is_some(), "warning on entry");
    assert_eq!(
        w.health(0).unwrap(),
        hp0,
        "no damage during the 3 s warning"
    );
    for _ in 0..(3 * HZ) {
        step(&mut w);
    }
    // 2 s in + 3 s = 5 s since entry: 2 s of 40 HP/s = 80.
    let hp = w.health(0).unwrap();
    // Explosion damage also starts bleeding (about 1 HP/s), hence the tolerance.
    assert!((hp0 - 80.0 - hp).abs() < 3.0, "{hp}");
    // The German in his own HQ zone is unharmed.
    w.debug_teleport(1, Vec3::new(780.0, 5.02, 60.0));
    for _ in 0..(5 * HZ) {
        step(&mut w);
    }
    assert_eq!(w.health(1).unwrap(), 100.0);
}

#[test]
fn spawn_moves_away_from_a_visible_enemy() {
    let col = synth_data(14, 2, &|_, _| 5.0);
    let base = Vec3::new(60.0, 5.0, 60.0);
    let enemy = [base + Vec3::new(2.0, 0.0, 0.0)];
    let mut seq = 0u32;
    let mut next = move || {
        seq += 1;
        ((seq * 37) % 101) as f32 / 101.0
    };
    let pick = pick_spawn(&col, base, &enemy, &enemy, &mut next);
    // Re-draw the same candidates and check none is farther from the enemy.
    let mut seq = 0u32;
    let mut next = move || {
        seq += 1;
        ((seq * 37) % 101) as f32 / 101.0
    };
    let dist = |p: Vec3| ((p.x - enemy[0].x).powi(2) + (p.z - enemy[0].z).powi(2)).sqrt();
    let mut best = 0.0f32;
    for _ in 0..8 {
        let a = next() * core::f32::consts::TAU;
        let r = 8.0 * next().sqrt();
        let p = Vec3::new(base.x + r * a.cos(), 5.0, base.z + r * a.sin());
        best = best.max(dist(p));
    }
    assert!(pick.threatened);
    assert!(
        (dist(pick.pos) - best).abs() < 1e-3,
        "{} {}",
        dist(pick.pos),
        best
    );
    // No enemy near: first candidate is used, not threatened.
    let p2 = pick_spawn(&col, base, &[], &[], || 0.5);
    assert!(!p2.threatened);
}

#[test]
fn deterministic() {
    let run = || {
        let mut w = world();
        run_to(&mut w, 20 * HZ);
        kill_and_wait(&mut w);
        for _ in 0..200 {
            step(&mut w);
        }
        w.state_hash()
    };
    assert_eq!(run(), run());
}
