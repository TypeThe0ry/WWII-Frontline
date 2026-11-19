//! M1 scenarios drive SoldierBrain, not stand-ins for behavior tree leaves.
mod common;

use std::sync::Arc;

use common::*;
use frontline_bots::perception::Sound;
use frontline_bots::squad_leader::SquadOrder;
use frontline_bots::{BotBrain, BotMap, Difficulty, SoldierBrain, Tuning};
use frontline_protocol::msg::input::buttons;
use frontline_sim::medical::GIVE_UP_BUTTON;
use frontline_sim::view::ObjectiveView;
use frontline_sim::{GameData, Team, Vec3};

fn brain() -> SoldierBrain {
    SoldierBrain::with_tactics(Arc::new(BotMap::without_nav(
        Tuning::embedded(),
        Vec::new(),
    )))
}

#[test]
fn b05_wall_audio_and_visual_memory_do_not_authorize_fire() {
    let mut view = FakeView::new();
    view.put(0, Team::Us, 100.0, 100.0, 0.0);
    view.put(1, Team::Ger, 100.0, 109.0, 0.0);
    let mut b = brain();
    let weapon = weapon_by_id("us_m1_garand", 8, 40);
    let sounds = [Sound {
        pos: Vec3::new(110.0, 0.0, 110.0),
        tick: 0,
        range_m: 75.0,
    }];
    view.blocked = true;
    for t in 0..1000 {
        let c = b.think(
            &view,
            &frontline_bots::BotCtx {
                hearing: &sounds,
                ..ctx(0, Team::Us, t, Some(weapon))
            },
        );
        assert_eq!(c.buttons & buttons::FIRE, 0, "tick {t}");
        assert_eq!(b.target_player(), None);
        if t == 0 {
            assert_ne!(c.move_z, 0, "the sound can prompt investigation");
        }
        if t == 150 {
            assert_eq!((c.move_x, c.move_z), (0, 0), "sound expired");
        }
    }
    view.blocked = false;
    // Close the wall between the decision and the very next off-shard tick.
    let fire = (1002..1200)
        .find(|&t| b.think(&view, &ctx(0, Team::Us, t, Some(weapon))).buttons & buttons::FIRE != 0)
        .unwrap();
    view.blocked = true;
    for t in fire + 1..fire + 151 {
        let c = b.think(&view, &ctx(0, Team::Us, t, Some(weapon)));
        assert_eq!(c.buttons & buttons::FIRE, 0, "tick {t}");
        if t == fire + 1 {
            assert_eq!(b.target_player(), Some(1));
        }
    }
    assert_eq!(b.target_player(), None);
}

#[test]
fn b02_b03_b06_soldier_follows_live_orders_and_leader() {
    let mut view = FakeView::new();
    view.put(0, Team::Us, 100.0, 100.0, 0.0);
    view.put(1, Team::Us, 100.0, 130.0, 0.0);
    view.objectives.push(ObjectiveView {
        id: 2,
        pos: Vec3::new(100., 0., 73.),
        radius_m: 15.,
        p: 0.,
        owner: None,
        contested: false,
    });
    let mut b = brain();
    let mut c = ctx(0, Team::Us, 0, None);
    c.squad_leader = Some(1);
    c.squad_order = Some(SquadOrder::Move(Vec3::new(100.0, 0.0, 130.0)));
    assert!(b.think(&view, &c).move_z > 0);
    c.tick = 3;
    c.squad_order = Some(SquadOrder::Attack(2));
    assert!(b.think(&view, &c).move_z < 0);
    c.tick = 6;
    c.squad_order = Some(SquadOrder::Hold);
    let hold = b.think(&view, &c);
    assert_eq!((hold.move_x, hold.move_z), (0, 0));
    assert_ne!(hold.buttons & buttons::CROUCH, 0);
    c.tick = 9;
    c.squad_order = Some(SquadOrder::Defend(2));
    assert!(b.think(&view, &c).move_z < 0);
    c.tick = 12;
    c.squad_order = None;
    view.objectives.clear();
    let follow = b.think(&view, &c);
    assert!(
        follow.move_x < 0,
        "move toward the formation slot, not straight at the leader"
    );
}

#[test]
fn b20_b21_downed_hold_and_stuck_jump() {
    let mut view = FakeView::new();
    view.put(0, Team::Us, 100.0, 100.0, 0.0);
    let mut b = brain();
    let mut c = ctx(0, Team::Us, 0, None);
    c.squad_order = Some(SquadOrder::Move(Vec3::new(100.0, 0.0, 130.0)));
    let mut jumps = vec![];
    for t in 0..=165 {
        c.tick = t;
        if b.think(&view, &c).buttons & buttons::JUMP != 0 {
            jumps.push(t);
        }
    }
    assert_eq!(jumps, [150]);
    assert_eq!(b.stuck_total, 1);
    c.survival.downed_since = Some(166);
    for age in 0..900 {
        c.tick = 166 + age;
        let cmd = b.think(&view, &c);
        assert_eq!(cmd.buttons, if age < 810 { 0 } else { GIVE_UP_BUTTON });
        assert_eq!((cmd.move_x, cmd.move_z), (0, 0));
    }
}

#[test]
fn b04_b08_b22_interact_reload_and_empty_magazine() {
    let mut view = FakeView::new();
    view.put(0, Team::Us, 100.0, 100.0, 0.0);
    let mut b = brain();
    let mut c = ctx(0, Team::Us, 0, Some(weapon_by_id("us_m1_garand", 0, 40)));
    assert_ne!(b.think(&view, &c).buttons & buttons::RELOAD, 0);
    c.tick = 3;
    c.self_weapon = Some(weapon_by_id("us_m1_garand", 0, 0));
    c.survival.ammo_crate = Some(Vec3::new(100.0, 0.0, 102.0));
    let cmd = b.think(&view, &c);
    assert_ne!(cmd.buttons & buttons::USE, 0);
    assert_eq!(cmd.buttons & (buttons::FIRE | buttons::RELOAD), 0);
    c.tick = 6;
    c.survival.ammo_crate = None;
    c.survival.downed_ally = Some((2, Vec3::new(100.0, 0.0, 101.0)));
    assert_ne!(b.think(&view, &c).buttons & buttons::USE, 0);
}

#[test]
fn recruit_and_veteran_recognition_to_first_shot() {
    let mut view = FakeView::new();
    view.put(0, Team::Us, 100.0, 100.0, 0.0);
    view.put(1, Team::Ger, 100.0, 109.0, 0.0);
    let rules = &GameData::embedded().rules.bots;
    let weapon = weapon_by_id("us_m1_garand", 8, 40);
    for name in ["recruit", "veteran"] {
        let diff = Difficulty::named(rules, name).unwrap();
        let mut delays = Vec::new();
        for seed in 0..200 {
            let mut b = brain();
            let mut seen = None;
            for t in 0..120 {
                let mut c = ctx(0, Team::Us, t, Some(weapon));
                c.difficulty = diff;
                c.rng_seed = seed;
                let cmd = b.think(&view, &c);
                if b.target_player().is_some() {
                    seen.get_or_insert(t);
                }
                if cmd.buttons & buttons::FIRE != 0 {
                    delays.push(t - seen.expect("visual recognition before fire"));
                    break;
                }
            }
        }
        assert_eq!(delays.len(), 200);
        let mean_ms = f64::from(delays.iter().sum::<u32>()) / 200.0 / 30.0 * 1000.0;
        println!(
            "{name}: n={} mean={mean_ms:.2} ms, min/max={} / {} ticks",
            delays.len(),
            delays.iter().min().unwrap(),
            delays.iter().max().unwrap()
        );
        assert!((f64::from(diff.reaction_ms.0)..=f64::from(diff.reaction_ms.1)).contains(&mean_ms));
    }
}
