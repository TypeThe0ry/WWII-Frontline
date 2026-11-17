//! State machine, output legality, stuck handling, determinism (M0-026).

mod common;

use common::*;
use frontline_bots::soldier::Mode;
use frontline_bots::BotBrain;
use frontline_protocol::msg::input::buttons;
use frontline_sim::weapon::Fsm;
use frontline_sim::{Team, WeaponState};

const ME: u8 = 3;

fn run(
    brain: &mut impl BotBrain,
    v: &FakeView,
    w: Option<WeaponState>,
    from: u32,
    n: u32,
) -> Vec<frontline_protocol::msg::input::InputCmd> {
    (from..from + n)
        .map(|t| brain.think(v, &ctx(ME, Team::Us, t, w)))
        .collect()
}

#[test]
fn patrol_engage_chase_patrol() {
    let mut v = FakeView::new();
    v.put(ME, Team::Us, 100.0, 100.0, 0.0);
    let mut b = fake_brain(&v, vec![[200.0, 100.0], [100.0, 200.0]]);
    let w = Some(weapon_by_id("us_m1_garand", 8, 40));
    run(&mut b, &v, w, 1, 12);
    assert_eq!(b.mode(), Mode::Patrol, "alone: patrol");
    // An enemy 40 m ahead appears.
    v.put(9, Team::Ger, 100.0, 140.0, std::f32::consts::PI);
    run(&mut b, &v, w, 13, 15);
    assert_eq!(b.mode(), Mode::Engage);
    assert_eq!(b.target_player(), Some(9));
    // It vanishes: the bot keeps chasing for the memory time (4 s = 120 ticks), then gives up.
    v.remove(9);
    run(&mut b, &v, w, 28, 90);
    assert_eq!(
        b.mode(),
        Mode::Engage,
        "still chasing the last known position after 3 s"
    );
    run(&mut b, &v, w, 118, 60);
    assert_eq!(
        b.mode(),
        Mode::Patrol,
        "back to patrol after the 4 s memory"
    );
    assert_eq!(b.target_player(), None);
}

#[test]
fn dead_bot_outputs_a_neutral_command_and_resyncs_on_respawn() {
    let mut v = FakeView::new();
    v.put(ME, Team::Us, 100.0, 100.0, 1.0);
    let mut b = fake_brain(&v, vec![[200.0, 100.0]]);
    let w = Some(weapon_by_id("us_m1_garand", 8, 40));
    run(&mut b, &v, w, 1, 20);
    v.remove(ME);
    let cmds = run(&mut b, &v, None, 21, 10);
    assert_eq!(b.mode(), Mode::Dead);
    for c in &cmds {
        assert_eq!((c.move_x, c.move_z, c.buttons, c.aux), (0, 0, 0, 0));
    }
    // Respawn facing another way: the view angle is taken over, not turned toward gradually.
    v.put(ME, Team::Us, 128.0, 56.0, 2.5);
    let c = b.think(&v, &ctx(ME, Team::Us, 31, w));
    let yaw = c.yaw as f32 / 65536.0 * std::f32::consts::TAU;
    assert!(
        (yaw - 2.5).abs() < 0.3,
        "yaw {yaw} should start near the respawn heading 2.5"
    );
    assert_ne!(b.mode(), Mode::Dead);
}

#[test]
fn empty_magazine_triggers_one_reload_pulse() {
    let mut v = FakeView::new();
    v.put(ME, Team::Us, 100.0, 100.0, 0.0);
    let mut b = fake_brain(&v, vec![]);
    let w = Some(weapon_by_id("us_m1_garand", 0, 40));
    let cmds = run(&mut b, &v, w, 1, 30);
    let pulses: Vec<usize> = cmds
        .iter()
        .enumerate()
        .filter(|(_, c)| c.buttons & buttons::RELOAD != 0)
        .map(|(i, _)| i)
        .collect();
    assert!(
        !pulses.is_empty(),
        "no reload requested with an empty magazine"
    );
    assert!(
        pulses.windows(2).all(|p| p[1] > p[0] + 1),
        "RELOAD held on consecutive ticks: {pulses:?}"
    );
    // Already reloading: no further requests.
    let mut w2 = weapon_by_id("us_m1_garand", 0, 40);
    w2.fsm = Fsm::Reloading(frontline_sim::weapon::ReloadKind::Empty);
    w2.timer_units = 90_000;
    let cmds = run(&mut b, &v, Some(w2), 31, 30);
    assert!(cmds.iter().all(|c| c.buttons & buttons::RELOAD == 0));
    // No ammo at all: no reload (nothing to load) and no shooting even with an enemy ahead.
    v.put(9, Team::Ger, 100.0, 130.0, 0.0);
    let w3 = weapon_by_id("us_m1_garand", 0, 0);
    let cmds = run(&mut b, &v, Some(w3), 61, 60);
    assert!(cmds
        .iter()
        .all(|c| c.buttons & (buttons::RELOAD | buttons::FIRE) == 0));
}

#[test]
fn semi_auto_fires_on_edges_and_waits_for_the_reaction_delay() {
    let mut v = FakeView::new();
    v.put(ME, Team::Us, 100.0, 100.0, 0.0);
    v.put(9, Team::Ger, 100.0, 125.0, std::f32::consts::PI);
    let mut b = fake_brain(&v, vec![]);
    // The fake world never advances the weapon, so it is always ready: the trigger must still be
    // released between pulls (semi-auto needs an edge).
    let w = Some(weapon_by_id("us_m1_garand", 8, 40));
    let cmds = run(&mut b, &v, w, 1, 150);
    let fire: Vec<bool> = cmds
        .iter()
        .map(|c| c.buttons & buttons::FIRE != 0)
        .collect();
    let first = fire.iter().position(|f| *f).expect("it never fired");
    assert!(
        (first as f32 + 1.0) / 30.0 >= 0.35,
        "fired after only {} ticks",
        first + 1
    );
    assert!(
        fire.windows(2).all(|p| !(p[0] && p[1])),
        "FIRE held on two consecutive ticks"
    );
    assert!(fire.iter().filter(|f| **f).count() >= 3);
    assert!(cmds
        .iter()
        .all(|c| !(c.buttons & buttons::FIRE != 0 && c.buttons & buttons::SPRINT != 0)));
}

#[test]
fn bolt_action_does_not_pull_the_trigger_while_cycling() {
    let mut v = FakeView::new();
    v.put(ME, Team::Ger, 100.0, 100.0, 0.0);
    v.put(9, Team::Us, 100.0, 125.0, std::f32::consts::PI);
    let mut b = fake_brain(&v, vec![]);
    let mut w = weapon_by_id("ger_kar98k", 4, 40);
    w.fsm = Fsm::BoltCycle;
    w.timer_units = 600_000; // far from done
    let cmds: Vec<_> = (1..150)
        .map(|t| b.think(&v, &ctx(ME, Team::Ger, t, Some(w))))
        .collect();
    assert!(
        cmds.iter().all(|c| c.buttons & buttons::FIRE == 0),
        "fired during the bolt cycle"
    );
    // Once ready, it shoots.
    let w = weapon_by_id("ger_kar98k", 4, 40);
    let cmds: Vec<_> = (150..200)
        .map(|t| b.think(&v, &ctx(ME, Team::Ger, t, Some(w))))
        .collect();
    assert!(cmds.iter().any(|c| c.buttons & buttons::FIRE != 0));
}

#[test]
fn stuck_detection_replans_and_jumps_after_five_seconds() {
    let mut v = FakeView::new();
    v.put(ME, Team::Us, 100.0, 100.0, 0.0); // never moves: the fake world ignores commands
    let mut b = fake_brain(&v, vec![[200.0, 100.0], [100.0, 200.0], [30.0, 100.0]]);
    let w = Some(weapon_by_id("us_m1_garand", 8, 40));
    let cmds = run(&mut b, &v, w, 1, 140);
    assert_eq!(b.stuck_total, 0, "5 s have not passed yet");
    assert!(cmds.iter().all(|c| c.buttons & buttons::JUMP == 0));
    let cmds = run(&mut b, &v, w, 141, 40);
    assert_eq!(
        b.stuck_total, 1,
        "stuck must be detected once after 5 s without 1 m of movement"
    );
    assert!(
        cmds.iter().any(|c| c.buttons & buttons::JUMP != 0),
        "no jump on stuck"
    );
    assert_eq!(
        cmds.iter()
            .filter(|c| c.buttons & buttons::JUMP != 0)
            .count(),
        1,
        "jump is a single pulse"
    );
}

#[test]
fn identical_inputs_give_identical_commands() {
    let mut v = FakeView::new();
    v.put(ME, Team::Us, 100.0, 100.0, 0.0);
    v.put(9, Team::Ger, 90.0, 140.0, std::f32::consts::PI);
    let w = Some(weapon_by_id("us_m1_garand", 8, 40));
    let mut a = fake_brain(&v, vec![[200.0, 100.0], [100.0, 200.0]]);
    let mut b = fake_brain(&v, vec![[200.0, 100.0], [100.0, 200.0]]);
    let ca = run(&mut a, &v, w, 1, 600);
    let cb = run(&mut b, &v, w, 1, 600);
    assert_eq!(ca, cb);
    // Skipping thinking ticks of other bots does not matter: a different bot id differs.
    let mut c = fake_brain(&v, vec![[200.0, 100.0], [100.0, 200.0]]);
    let cc: Vec<_> = (1..=600)
        .map(|t| c.think(&v, &ctx(ME + 1, Team::Us, t, w)))
        .collect();
    // ME+1 is not in the view: dead output only.
    assert!(cc.iter().all(|c| c.buttons == 0));
}

/// Every command of a long ten-bot fight is a legal input: sequence numbers advance by one,
/// no reserved bits, no sprint together with fire, move vectors within the validator's limit,
/// view turns within the bots' own 400 deg/s limit.
#[test]
fn commands_are_always_legal_in_a_long_fight() {
    let Some(col) = terrain() else { return };
    let mut a = Arena::new(&col, 21);
    let map = a.bot_map();
    for i in 0..10u8 {
        let (team, x, z) = if i % 2 == 0 {
            (Team::Us, 110.0 + i as f32 * 2.0, 60.0)
        } else {
            (Team::Ger, 110.0 + i as f32 * 2.0, 196.0)
        };
        a.add_bot(
            &map,
            i,
            team,
            x,
            z,
            if team == Team::Us {
                0.0
            } else {
                std::f32::consts::PI
            },
        );
    }
    let mut last: std::collections::BTreeMap<u8, (u16, u16, bool)> = Default::default();
    for _ in 0..4500 {
        let alive_before: Vec<u8> = a.world.view().players().iter().map(|p| p.player).collect();
        let tick = a.world.tick().0 + 1;
        let cmds = a.step();
        for (p, c) in cmds {
            assert_eq!(c.tick, tick);
            assert_eq!(c.buttons & 0xC000, 0, "reserved bits");
            assert!(
                !(c.buttons & buttons::FIRE != 0 && c.buttons & buttons::SPRINT != 0),
                "sprint + fire"
            );
            assert!(c.move_x != i8::MIN && c.move_z != i8::MIN);
            let len = (c.move_x as f32).hypot(c.move_z as f32);
            assert!(len <= 127.5, "move length {len}");
            assert_eq!(c.weapon_slot, 0);
            let was_alive = alive_before.contains(&p);
            if let Some((seq, yaw, prev_alive)) = last.get(&p) {
                assert_eq!(
                    c.seq,
                    seq.wrapping_add(1),
                    "seq must advance by one per tick"
                );
                if was_alive && *prev_alive {
                    let d = (c.yaw.wrapping_sub(*yaw) as i16 as i32).unsigned_abs() as f32 * 360.0
                        / 65536.0;
                    assert!(
                        d <= 400.0 / 30.0 + 0.1,
                        "bot {p} turned {d} deg in one tick"
                    );
                }
            }
            last.insert(p, (c.seq, c.yaw, was_alive));
        }
    }
}
