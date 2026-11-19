//! M1-014b: single-hit previews and authoritative life transitions through World::step.
#![cfg(feature = "test-util")]

mod common;

use common::{cmd, data_with, duel_world, fire_cmd, flat, solve_pitch, BTN_FIRE};
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::combat::{
    compute_damage, DamageKind, DamageOutcome, HitInfo, KillReason, TargetCtx,
};
use frontline_sim::medical::{CreditKind, BLEED_HPS, DOWNED_HP};
use frontline_sim::weapon::ballistics::BallisticsParams;
use frontline_sim::weapon::HitZone;
use frontline_sim::{EntityId, GameData, PlayerId, SimEvent, Team, Vec3, World};

const A: u8 = 0;
const B: u8 = 1;
const C: u8 = 2;
const RIFLE: &str = "ger_kar98k";
const RANGE: f32 = 6.0;
const FIRST_DOWNED_HP: f32 = DOWNED_HP - DOWNED_HP / 2400.0;

fn rifle_data(damage: f64) -> &'static GameData {
    data_with(|w| {
        if w.id == RIFLE {
            w.gameplay.damage_torso_hp = damage;
            w.gameplay.spread.hip_deg = 0.0;
            w.gameplay.spread.ads_deg = 0.0;
        }
    })
}

fn duel(data: &'static GameData, second_shooter: bool) -> World {
    let players = [
        (A, Team::Ger, 20.0, 60.0, 16384),
        (B, Team::Us, 26.0, 60.0, 49152),
        (C, Team::Ger, 26.0, 54.0, 0),
    ];
    duel_world(
        data,
        &flat(2, 5.0),
        5,
        &players[..if second_shooter { 3 } else { 2 }],
    )
}

fn shot(data: &GameData, height: f32, yaw: u16) -> InputCmd {
    let bp = BallisticsParams::from_def(data.weapon(RIFLE).unwrap());
    let mut input = fire_cmd(solve_pitch(&bp, 1.65, RANGE, height), BTN_FIRE);
    input.yaw = yaw;
    input
}

fn entity(world: &World, player: PlayerId) -> EntityId {
    world
        .view()
        .players()
        .iter()
        .find(|p| p.player == player)
        .unwrap()
        .entity
}

fn credits(events: &[SimEvent]) -> Vec<(CreditKind, Option<PlayerId>)> {
    events
        .iter()
        .filter_map(|e| match e {
            SimEvent::KillCredit {
                victim: B,
                kind,
                killer,
                ..
            } => Some((*kind, *killer)),
            _ => None,
        })
        .collect()
}

#[test]
fn single_bullet_preview_and_world_agree_at_hp_boundaries() {
    let heavy = rifle_data(100.0);
    let light = rifle_data(30.0);
    for (data, hp, zone, height, expected) in [
        (heavy, 100.0, HitZone::Torso, 1.33, DamageOutcome::Downed),
        (heavy, 40.1, HitZone::Torso, 1.33, DamageOutcome::Downed),
        (
            heavy,
            40.0,
            HitZone::Torso,
            1.33,
            DamageOutcome::Killed(KillReason::Overkill),
        ),
        (light, 100.0, HitZone::Head, 1.66, DamageOutcome::Hit),
        (
            light,
            90.0,
            HitZone::Head,
            1.66,
            DamageOutcome::Killed(KillReason::Headshot),
        ),
    ] {
        let mut world = duel(data, false);
        let victim = entity(&world, B);
        world.debug_set_hp(B, hp);
        let events = world.step(&[(A, shot(data, height, 16384))]).events;
        let (actual_zone, damage, dist) = events
            .iter()
            .find_map(|e| match e {
                SimEvent::Hit {
                    target,
                    zone,
                    damage_hp,
                    dist_m,
                    ..
                } if *target == victim => Some((*zone, *damage_hp, *dist_m)),
                _ => None,
            })
            .expect("bullet hit the victim this tick");
        assert_eq!(actual_zone, zone);
        let preview = compute_damage(
            data.weapon(RIFLE).unwrap(),
            &HitInfo {
                zone,
                travelled_m: dist,
                pen_mult: 1.0,
                kind: DamageKind::Bullet,
            },
            &TargetCtx {
                hp,
                same_team: false,
                helmet: true,
                friendly_fire: true,
            },
        );
        assert_eq!(preview.damage, damage);
        assert_eq!(preview.outcome, expected, "hp={hp}, zone={zone:?}");

        let state = world.player_life(B).unwrap();
        let credit = match expected {
            DamageOutcome::Hit => {
                assert!(state.is_alive());
                // The nonlethal head hit starts bleeding in the same tick.
                assert_eq!(world.health(B), Some(preview.hp_after - BLEED_HPS / 30.0));
                vec![]
            }
            DamageOutcome::Downed => {
                assert!(state.is_downed());
                assert_eq!(world.health(B), Some(FIRST_DOWNED_HP));
                vec![(CreditKind::Down, Some(A))]
            }
            DamageOutcome::Killed(_) => {
                assert!(state.is_dead());
                assert_eq!(world.health(B), Some(0.0));
                vec![(CreditKind::Direct, Some(A))]
            }
        };
        assert_eq!(credits(&events), credit);
    }
}

#[test]
fn same_tick_hits_use_phase_hp_and_do_not_replay_into_the_downed_pool() {
    let data = rifle_data(30.0);
    let mut world = duel(data, true);
    let (first, victim, second) = (entity(&world, A), entity(&world, B), entity(&world, C));
    world.debug_set_hp(B, 50.0);
    let events = world
        .step(&[(A, shot(data, 1.33, 16384)), (C, shot(data, 1.33, 0))])
        .events;
    let hits: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            SimEvent::Hit {
                shooter,
                target,
                damage_hp,
                ..
            } if *target == victim => Some((*shooter, *damage_hp)),
            _ => None,
        })
        .collect();
    assert_eq!(hits, vec![(first, 30.0), (second, 30.0)]);
    // Neither shot alone was lethal. The first log record sees the phase's final -10 HP
    // and downs B; the second does not deduct HP again or claim the down.
    assert_eq!(credits(&events), vec![(CreditKind::Down, Some(A))]);
    assert!(world.player_life(B).unwrap().is_downed());
    assert_eq!(world.health(B), Some(FIRST_DOWNED_HP));

    let wait = |world: &mut World| {
        for _ in 0..90 {
            world.step(&[(C, cmd(0, 0, 0, 0, 0))]);
        }
    };
    wait(&mut world);
    let events = world.step(&[(C, shot(data, 0.35, 0))]).events;
    assert!(credits(&events).is_empty());
    assert!(world.player_life(B).unwrap().is_downed());
    assert!((5.0..10.0).contains(&world.health(B).unwrap()));

    wait(&mut world);
    let events = world.step(&[(C, shot(data, 0.35, 0))]).events;
    assert_eq!(credits(&events), vec![(CreditKind::Finish, Some(C))]);
    assert!(world.player_life(B).unwrap().is_dead());
    assert_eq!(world.health(B), Some(0.0));
}

#[test]
fn lethal_falls_respect_the_producer_override_without_overkill() {
    for (drop, direct) in [(11.5, false), (12.5, true)] {
        let mut world = duel_world(
            GameData::embedded(),
            &flat(2, 5.0),
            5,
            &[(B, Team::Us, 26.0, 60.0, 0)],
        );
        world.debug_teleport(B, Vec3::new(26.0, 5.02 + drop, 60.0));
        let events = (0..180)
            .find_map(|_| {
                let events = world.step(&[]).events;
                (!world.player_life(B).unwrap().is_alive()).then_some(events)
            })
            .expect("lethal landing");
        // Both falls take between 100 and 160 HP, so only the height override causes KIA.
        assert_eq!(world.player_life(B).unwrap().is_dead(), direct);
        let kind = if direct {
            CreditKind::Direct
        } else {
            CreditKind::Down
        };
        assert_eq!(credits(&events), vec![(kind, None)]);
    }
}
