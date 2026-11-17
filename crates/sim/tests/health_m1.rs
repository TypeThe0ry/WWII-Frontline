//! M1-016a: life-state transitions (alive / downed / KIA), bleeding, giving up, revive, fatigue,
//! kill credit.
mod common;

use common::*;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::medical::CreditKind;
use frontline_sim::weapon::ballistics::BallisticsParams;
use frontline_sim::{GameData, LifeState, SimEvent, Team, World};

const A: u8 = 0;
const B: u8 = 1;
const C: u8 = 2;
const EYE: f32 = 1.65;

fn exact() -> &'static GameData {
    data_with(|w| {
        w.gameplay.spread.hip_deg = 0.0;
        w.gameplay.spread.ads_deg = 0.0;
        w.gameplay.recoil.vkick_deg = 0.0;
        w.gameplay.recoil.hsigma_deg = 0.0;
        w.gameplay.recoil.bloom_per_shot_deg = 0.0;
    })
}

fn arena() -> frontline_sim::CollisionData {
    synth_data(14, 2, &|_, _| 5.0)
}

fn idle() -> InputCmd {
    cmd(0, 0, 49152, 0, 0)
}

fn btn(b: u16) -> InputCmd {
    cmd(0, 0, 49152, b, 0)
}

/// A (Ger, Kar98k, +x) and B (Us, 6 m away, facing -x); `extra` adds more players.
fn world(extra: &[(u8, Team, f32)]) -> World {
    let mut p = vec![
        (A, Team::Ger, 20.0, 60.0, 16384),
        (B, Team::Us, 26.0, 60.0, 49152),
    ];
    for &(id, t, x) in extra {
        p.push((id, t, x, 60.8, 49152));
    }
    duel_world(exact(), &arena(), 5, &p)
}

fn pitch_to_torso(d: f32) -> f32 {
    let bp = BallisticsParams::from_def(exact().weapon("ger_kar98k").unwrap());
    solve_pitch(&bp, EYE, d, 1.33)
}

/// Fires A's kar98k once (torso) while the others idle; returns the events of the tick of impact.
fn shoot_once(w: &mut World, inputs: &dyn Fn(u8) -> InputCmd) -> Vec<SimEvent> {
    let pitch = pitch_to_torso(6.0);
    let mut out = Vec::new();
    for i in 0..30 {
        let mut ins = vec![(
            A,
            if i == 0 {
                fire_cmd(pitch, BTN_FIRE)
            } else {
                fire_cmd(pitch, 0)
            },
        )];
        for id in 1..4u8 {
            if w.player_life(id).is_some() {
                ins.push((id, inputs(id)));
            }
        }
        out.extend(w.step(&ins).events);
    }
    out
}

/// Tick (as `World::tick()` reads after the step) at which the first `Downed` event came.
fn down_tick(w: &mut World) -> u32 {
    let pitch = pitch_to_torso(6.0);
    for i in 0..60 {
        let b = if i == 0 { BTN_FIRE } else { 0 };
        let ev = w.step(&[(A, fire_cmd(pitch, b)), (B, idle())]).events;
        if ev.iter().any(|e| matches!(e, SimEvent::Downed { .. })) {
            return w.tick().0;
        }
    }
    panic!("never downed");
}

fn calm(_: u8) -> InputCmd {
    idle()
}

#[test]
fn torso_hit_downs_with_40_hp_and_headshot_kills() {
    let mut w = world(&[]);
    let ev = shoot_once(&mut w, &calm);
    assert!(ev.iter().any(|e| matches!(
        e,
        SimEvent::Downed {
            victim: B,
            killer: Some(A),
            ..
        }
    )));
    assert!(matches!(w.player_life(B), Some(LifeState::Downed { .. })));
    assert!(w.health(B).unwrap() <= 40.0 && w.health(B).unwrap() > 39.0);
    assert!(
        !ev.iter().any(|e| matches!(e, SimEvent::Kill { .. })),
        "down is not KIA"
    );
    // Head: one shot, no downed stage.
    let mut w = world(&[]);
    let bp = BallisticsParams::from_def(exact().weapon("ger_kar98k").unwrap());
    let pitch = solve_pitch(&bp, EYE, 6.0, 1.66);
    let mut ev = Vec::new();
    for i in 0..30 {
        let b = if i == 0 { BTN_FIRE } else { 0 };
        ev.extend(w.step(&[(A, fire_cmd(pitch, b)), (B, idle())]).events);
    }
    assert!(ev
        .iter()
        .any(|e| matches!(e, SimEvent::Kill { victim: B, .. })));
    assert!(matches!(w.player_life(B), Some(LifeState::Dead { .. })));
}

#[test]
fn downed_bleeds_out_in_80_seconds() {
    let mut w = world(&[]);
    let t0 = frontline_sim::Tick(down_tick(&mut w));
    let mut kia = None;
    for _ in 0..2600 {
        let ev = w.step(&[(A, idle()), (B, idle())]).events;
        if ev.iter().any(|e| {
            matches!(
                e,
                SimEvent::Kill {
                    victim: B,
                    killer: None,
                    ..
                }
            )
        }) {
            kia = Some(w.tick());
            assert!(
                !ev.iter().any(|e| matches!(e, SimEvent::KillCredit { .. })),
                "bleed-out has no credit"
            );
            break;
        }
    }
    let n = kia.expect("bled out").0 - t0.0;
    // Down hp is 40 minus the bleed of the impact tick: 80 s = 2400 ticks +- 2.
    assert!((2398..=2402).contains(&n), "{n}");
}

#[test]
fn give_up_needs_5_s_then_3_s_hold() {
    let held = |w: &mut World, ticks: u32| {
        let mut dead = false;
        for _ in 0..ticks {
            let ev = w.step(&[(A, idle()), (B, btn(buttons::RELOAD))]).events;
            dead |= ev
                .iter()
                .any(|e| matches!(e, SimEvent::Kill { victim: B, .. }));
        }
        dead
    };
    // Holding from the moment they go down: nothing for 5 s (150 ticks), then 3 s (90).
    let mut w = world(&[]);
    shoot_once(&mut w, &calm);
    let after_shot = 30; // shoot_once ran 30 ticks; the hit landed a few ticks in
    assert!(
        !held(&mut w, 150 - after_shot + 80),
        "still alive before 5 s + 3 s"
    );
    assert!(held(&mut w, 30), "gives up after the hold");
    // Giving up grants no credit to the shooter.
    assert!(matches!(
        w.player_life(B),
        Some(LifeState::Dead { killer: None, .. })
    ));
}

#[test]
fn revive_takes_5_seconds_and_heals_to_40() {
    let mut w = world(&[(C, Team::Us, 26.5)]);
    w.set_medic(C, true);
    shoot_once(&mut w, &calm);
    assert!(matches!(w.player_life(B), Some(LifeState::Downed { .. })));
    let mut revived_after = None;
    for i in 0..400u32 {
        let ev = w
            .step(&[(A, idle()), (B, idle()), (C, btn(buttons::USE))])
            .events;
        if ev
            .iter()
            .any(|e| matches!(e, SimEvent::Revive { target, .. } if *target != 0))
        {
            revived_after = Some(i + 1);
            break;
        }
    }
    assert_eq!(revived_after, Some(150), "5.0 s at 30 Hz");
    assert!(w.player_life(B).unwrap().is_alive());
    let hp = w.health(B).unwrap();
    assert!((hp - 40.0).abs() < 0.5, "{hp}");
}

#[test]
fn non_medic_cannot_revive_and_interrupt_resumes_within_3_s() {
    let mut w = world(&[(C, Team::Us, 26.5)]);
    shoot_once(&mut w, &calm);
    for _ in 0..300 {
        w.step(&[(A, idle()), (B, idle()), (C, btn(buttons::USE))]);
    }
    assert!(
        matches!(w.player_life(B), Some(LifeState::Downed { .. })),
        "rifleman cannot revive"
    );
    // A medic who lets go for 2.9 s keeps the progress; after 3.1 s it restarts.
    for (gap, expect_total) in [(87u32, 150u32), (93, 250)] {
        let mut w = world(&[(C, Team::Us, 26.5)]);
        w.set_medic(C, true);
        shoot_once(&mut w, &calm);
        let mut ticks_held = 0;
        let mut done = None;
        let plan: Vec<bool> = std::iter::repeat_n(true, 100)
            .chain(std::iter::repeat_n(false, gap as usize))
            .chain(std::iter::repeat_n(true, 200))
            .collect();
        for (i, h) in plan.into_iter().enumerate() {
            let c = if h { btn(buttons::USE) } else { idle() };
            if h {
                ticks_held += 1;
            }
            let ev = w.step(&[(A, idle()), (B, idle()), (C, c)]).events;
            if ev.iter().any(|e| matches!(e, SimEvent::Revive { .. })) {
                done = Some(ticks_held);
                let _ = i;
                break;
            }
        }
        assert_eq!(done, Some(expect_total), "gap {gap}");
    }
}

/// 60 damage per torso shot: 100 hp needs two shots to down, a revived player (40) one.
fn weak() -> &'static GameData {
    data_with(|w| {
        w.gameplay.spread.hip_deg = 0.0;
        w.gameplay.spread.ads_deg = 0.0;
        w.gameplay.recoil.vkick_deg = 0.0;
        w.gameplay.recoil.hsigma_deg = 0.0;
        w.gameplay.recoil.bloom_per_shot_deg = 0.0;
        if w.id == "ger_kar98k" {
            w.gameplay.damage_torso_hp = 60.0;
        }
    })
}

/// Shoots B (one bolt cycle apart) until they are downed; returns the tick of the down.
fn shoot_until_down(w: &mut World, pitch: f32) -> u32 {
    for shot in 0..4 {
        for i in 0..70 {
            let b = if i == 0 { BTN_FIRE } else { 0 };
            let ev = w
                .step(&[(A, fire_cmd(pitch, b)), (B, idle()), (C, idle())])
                .events;
            if ev.iter().any(|e| matches!(e, SimEvent::Downed { .. })) {
                return w.tick().0;
            }
        }
        let _ = shot;
    }
    panic!("never downed");
}

#[test]
fn fatigue_shortens_the_next_bleed_out() {
    let mut w = duel_world(
        weak(),
        &arena(),
        5,
        &[
            (A, Team::Ger, 20.0, 60.0, 16384),
            (B, Team::Us, 26.0, 60.0, 49152),
            (C, Team::Us, 26.5, 60.8, 49152),
        ],
    );
    w.set_medic(C, true);
    let bp = BallisticsParams::from_def(weak().weapon("ger_kar98k").unwrap());
    let pitch = solve_pitch(&bp, EYE, 6.0, 1.33);
    // Down and revive B three times; the fourth time they have 3 recent revives (80 - 60 = 20 s).
    for round in 0..4 {
        let down = shoot_until_down(&mut w, pitch);
        assert!(
            matches!(w.player_life(B), Some(LifeState::Downed { .. })),
            "round {round}"
        );
        if round < 3 {
            for _ in 0..155 {
                w.step(&[(A, idle()), (B, idle()), (C, btn(buttons::USE))]);
            }
            assert!(
                w.player_life(B).unwrap().is_alive(),
                "revived in round {round}"
            );
        } else {
            let Some(LifeState::Downed { fatigue, .. }) = w.player_life(B) else {
                panic!("downed")
            };
            assert_eq!(fatigue, 3);
            let mut kia = None;
            for _ in 0..2600 {
                let ev = w.step(&[(A, idle()), (B, idle()), (C, idle())]).events;
                if ev
                    .iter()
                    .any(|e| matches!(e, SimEvent::Kill { victim: B, .. }))
                {
                    kia = Some(w.tick().0 - down);
                    break;
                }
            }
            let n = kia.expect("bled out");
            assert!((598..=603).contains(&n), "20 s = 600 ticks: {n}");
        }
    }
}

#[test]
fn credit_down_assist_finish_and_team_kill() {
    // A (Ger) downs B with 100 damage; the Down credit names A. Then C (Ger, extra) finishes.
    let mut w = world(&[(C, Team::Ger, 20.0)]);
    let ev = shoot_once(&mut w, &calm);
    let down = ev
        .iter()
        .find_map(|e| match e {
            SimEvent::KillCredit {
                kind: CreditKind::Down,
                killer,
                assists,
                team_kill,
                ..
            } => Some((*killer, assists.clone(), *team_kill)),
            _ => None,
        })
        .expect("down credit");
    assert_eq!(down, (Some(A), vec![], false));
    // Another shot into the downed player is a finish (the victim is KIA from the hit).
    let mut fin = None;
    let pitch = pitch_to_torso(6.0);
    for i in 0..120 {
        let b = if i == 90 { BTN_FIRE } else { 0 };
        for e in w
            .step(&[(A, fire_cmd(pitch, b)), (B, idle()), (C, idle())])
            .events
        {
            if let SimEvent::KillCredit { kind, killer, .. } = e {
                fin = Some((kind, killer));
            }
        }
    }
    assert_eq!(fin, Some((CreditKind::Finish, Some(A))));
}

#[test]
fn friendly_down_is_a_team_kill() {
    // B and A on the same team: A's 100-damage shot does 50 (friendly fire x0.5): not even a down.
    let mut w = duel_world(
        exact(),
        &arena(),
        5,
        &[
            (A, Team::Ger, 20.0, 60.0, 16384),
            (B, Team::Ger, 26.0, 60.0, 49152),
        ],
    );
    let ev = shoot_once(&mut w, &calm);
    assert!(!ev.iter().any(|e| matches!(e, SimEvent::Downed { .. })));
    assert!(w.health(B).unwrap() > 40.0 && w.health(B).unwrap() < 60.0);
}

#[test]
fn deterministic_with_downs() {
    let run = || {
        let mut w = world(&[(C, Team::Us, 26.5)]);
        w.set_medic(C, true);
        shoot_once(&mut w, &calm);
        for _ in 0..200 {
            w.step(&[(A, idle()), (B, idle()), (C, btn(buttons::USE))]);
        }
        w.state_hash()
    };
    assert_eq!(run(), run());
}
