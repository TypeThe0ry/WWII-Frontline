//! M0-012: `weapon_duel` (Garand vs Kar98k, event sequence and damage conservation) and
//! acceptance 7: determinism (state hash incl. projectiles) and server vs `PredictSim`
//! `WeaponState` equality.
mod common;

use common::*;
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::predict::PredictSim;
use frontline_sim::weapon::ballistics::BallisticsParams;
use frontline_sim::{GameData, SimEvent, Team, Tick, Vec3, World};

const US: u8 = 0;
const GER: u8 = 1;
const GAP_M: f32 = 15.0;

fn arena() -> frontline_sim::CollisionData {
    flat(4, 5.0)
}

/// Duel script: both hip-fire at the other player, press the trigger every other tick (the cooldown
/// decides the real rate), reload when empty, stop when the enemy is down.
fn duel_cmds(tick: u32, mags: [u16; 2], pitch: [f32; 2], enemy_down: [bool; 2]) -> [InputCmd; 2] {
    let mk = |yaw: u16, pitch: f32, mag: u16, phase: u32, down: bool| {
        let mut b = 0; // hip fire: misses and terrain impacts happen
        if mag == 0 {
            b |= BTN_RELOAD;
        } else if tick % 2 == phase && tick > 12 && !down {
            b |= BTN_FIRE;
        }
        let mut c = cmd(0, 0, yaw, b, 0);
        c.pitch = frontline_protocol::quant::rad_to_pitch_i16(pitch).code;
        c
    };
    [
        mk(16384, pitch[0], mags[0], 0, enemy_down[0]),
        mk(49152, pitch[1], mags[1], 1, enemy_down[1]),
    ]
}

fn duel(seed: u64, ticks: u32) -> (Vec<SimEvent>, World) {
    let data = GameData::embedded();
    let mut w = duel_world(
        data,
        &arena(),
        seed,
        &[
            (US, Team::Us, 100.0, 100.0, 16384),
            (GER, Team::Ger, 100.0 + GAP_M, 100.0, 49152),
        ],
    );
    let bp_us = BallisticsParams::from_def(data.weapon("us_m1_garand").unwrap());
    let bp_ger = BallisticsParams::from_def(data.weapon("ger_kar98k").unwrap());
    let pitch = [
        solve_pitch(&bp_us, 1.65, GAP_M, 1.235),
        solve_pitch(&bp_ger, 1.65, GAP_M, 1.235),
    ];
    let mut events = Vec::new();
    for t in 0..ticks {
        let mags = [
            w.weapon_state(US).unwrap().mag,
            w.weapon_state(GER).unwrap().mag,
        ];
        // A shooter stops when the enemy is dead (hp 0 until the respawn, M0-013).
        let down = [w.health(GER).unwrap() <= 0.0, w.health(US).unwrap() <= 0.0];
        let [a, b] = duel_cmds(t, mags, pitch, down);
        events.extend(w.step(&[(US, a), (GER, b)]).events);
    }
    (events, w)
}

#[test]
fn weapon_duel_events_and_damage_are_conserved() {
    let (events, w) = duel(11, 900);
    let data = GameData::embedded();
    let eid = |p| {
        w.view()
            .players()
            .iter()
            .find(|v| v.player == p)
            .unwrap()
            .entity
    };
    let (e_us, e_ger) = (eid(US), eid(GER));
    let mut shots = [0u32; 2];
    let mut dry = [0u32; 2];
    let mut dealt = [0.0f32; 2]; // damage dealt BY us / ger
    let mut hits = [0u32; 2];
    // M0-013: a respawn restores health and ammunition, so both are accounted per life.
    let mut lost = [0.0f32; 2]; // damage taken BY us / ger since its last spawn
    let mut shots_life = [0u32; 2];
    let mut kills = [0u32; 2];
    let mut last_shot_for_hit: [u32; 2] = [0, 0];
    for e in &events {
        match e {
            SimEvent::Shot {
                shooter, weapon, ..
            } => {
                let i = usize::from(*shooter == e_ger);
                shots[i] += 1;
                shots_life[i] += 1;
                last_shot_for_hit[i] += 1;
                let expect = if i == 0 { "us_m1_garand" } else { "ger_kar98k" };
                assert_eq!(data.weapons[usize::from(*weapon)].id, expect);
            }
            SimEvent::Hit {
                shooter,
                target,
                damage_hp,
                ..
            } => {
                let i = usize::from(*shooter == e_ger);
                assert_ne!(shooter, target);
                assert_eq!(*target, if i == 0 { e_ger } else { e_us });
                assert!(
                    last_shot_for_hit[i] > 0,
                    "every hit follows a shot of the same shooter"
                );
                last_shot_for_hit[i] -= 1;
                dealt[i] += damage_hp;
                lost[1 - i] += damage_hp;
                hits[i] += 1;
            }
            SimEvent::Kill { victim, killer, .. } => {
                assert_ne!(Some(*victim), *killer);
                kills[usize::from(*victim == US)] += 1;
            }
            SimEvent::Spawn { player, .. } => {
                let i = usize::from(*player == GER);
                lost[i] = 0.0;
                shots_life[i] = 0;
            }
            SimEvent::DryFire { shooter } => dry[usize::from(*shooter == e_ger)] += 1,
            _ => {}
        }
    }
    let hp = [w.health(US).unwrap(), w.health(GER).unwrap()];
    println!(
        "duel 30 s: Garand shots {} hits {} dealt {:.1} HP -> Kar98k HP {:.1}; Kar98k shots {} hits {} dealt {:.1} HP -> Garand HP {:.1}; dry clicks {dry:?}",
        shots[0], hits[0], dealt[0], hp[1], shots[1], hits[1], dealt[1], hp[0]
    );
    println!("duel kills (ger died, us died): {kills:?}");
    assert!(kills[0] + kills[1] >= 1, "the duel must produce a kill");
    // Damage conservation: health lost since the last spawn == damage of the Hit events (a dead
    // player is at 0 hp, not below).
    for (p, i) in [(US, 0usize), (GER, 1)] {
        let expect = (100.0 - lost[i]).max(0.0);
        assert!((w.health(p).unwrap() - expect).abs() < 1e-3, "player {p}");
    }
    // Ammunition conservation (per life).
    for (p, s) in [(US, shots_life[0]), (GER, shots_life[1])] {
        let ws = w.weapon_state(p).unwrap();
        // Loadout ammo_mags counts the clip in the gun (GAMEDESIGN 5.3): 10 x 8 and 16 x 5.
        let (cap, mags) = if p == US { (8u32, 10u32) } else { (5, 16) };
        assert_eq!(
            u32::from(ws.mag) + u32::from(ws.reserve) + s,
            cap * mags,
            "player {p}"
        );
    }
    let impacts = events
        .iter()
        .filter(|e| matches!(e, SimEvent::Impact { .. }))
        .count();
    println!("duel: {impacts} terrain impacts (misses)");
    assert!(hits[0] + hits[1] >= 1 && impacts >= 1, "{hits:?} {impacts}");
    assert!(shots[0] >= 2 && shots[1] >= 1);
    // Every shot ends as a hit, a terrain impact, or leaves the loaded map / flies on (lobbed upward).
    assert!(hits[0] + hits[1] + impacts as u32 <= shots[0] + shots[1]);
}

#[test]
fn duel_is_deterministic_and_golden() {
    let (e1, w1) = duel(11, 600);
    let (e2, w2) = duel(11, 600);
    assert_eq!(e1, e2);
    assert_eq!(w1.state_hash(), w2.state_hash());
    let (e3, _) = duel(12, 600);
    assert_ne!(
        event_hash(&e1),
        event_hash(&e3),
        "another seed changes the spread draws"
    );
    println!(
        "duel event hash {:#018x}, {} events, state hash {:#018x}",
        event_hash(&e1),
        e1.len(),
        w1.state_hash()
    );
    assert_eq!(
        event_hash(&e1),
        GOLDEN_DUEL_EVENTS,
        "golden duel event sequence changed"
    );
}

/// Regression pin of the 600-tick duel (seed 11). Update only for deliberate weapon / data
/// changes (and say why).
///
/// M0-013 re-pin (was 0x2ced_8d49_9901_efde): the duel now ends in a kill and the victim
/// respawns, so the 600-tick sequence contains `Kill` / `Spawn` events and the duel goes on after
/// the respawn (before, the loser stayed at hp <= 0 and the winner stopped shooting).
/// M1-016: re-pinned (0x3849_3e76_201d_9cd5 before): lethal hits now down first, adding events.
const GOLDEN_DUEL_EVENTS: u64 = 0xff00_043d_6c81_f8cf;

fn script(n: usize, seed: u64) -> Vec<InputCmd> {
    let mut r = Rng(seed);
    let mut cur = cmd(0, 0, 0, 0, 0);
    (0..n)
        .map(|i| {
            if i % 4 == 0 {
                let b = r.next_u64();
                let mut btn = 0;
                if b & 1 == 0 {
                    btn |= BTN_FIRE;
                }
                if b >> 1 & 3 == 0 {
                    btn |= BTN_AIM;
                }
                if b >> 3 & 31 == 0 {
                    btn |= BTN_RELOAD;
                }
                if b >> 8 & 7 == 0 {
                    btn |= BTN_SPRINT;
                }
                if b >> 11 & 7 == 0 {
                    btn |= BTN_CROUCH;
                }
                let mut c = cmd(
                    ((r.next_u64() % 255) as i32 - 127) as i8,
                    ((r.next_u64() % 255) as i32 - 127) as i8,
                    (r.next_u64() & 0xffff) as u16,
                    btn,
                    (b >> 14 & 1) as u8,
                );
                c.pitch = ((r.next_u64() % 8000) as i32 - 4000) as i16;
                cur = c;
            }
            cur
        })
        .collect()
}

#[test]
#[allow(clippy::needless_range_loop)]
fn world_state_hash_incl_projectiles_is_reproducible_5000_ticks() {
    let run = || {
        let col = flat(4, 5.0);
        let mut w = World::new(sim_cfg(GameData::embedded(), 42), &col);
        for (p, team) in [
            (0u8, Team::Us),
            (1, Team::Ger),
            (2, Team::Us),
            (3, Team::Ger),
        ] {
            w.spawn_player(frontline_sim::SpawnReq {
                player: p,
                team,
                pos: Vec3::new(100.0 + 8.0 * f32::from(p), 5.02, 120.0),
                yaw: 0.0,
            })
            .unwrap();
        }
        let scripts: Vec<Vec<InputCmd>> = (0..4).map(|p| script(5000, 0x1234 + p)).collect();
        let mut hashes = Vec::new();
        let mut max_proj = 0;
        let mut events = Vec::new();
        for t in 0..5000 {
            let inputs: Vec<_> = (0..4u8).map(|p| (p, scripts[p as usize][t])).collect();
            events.extend(w.step(&inputs).events);
            max_proj = max_proj.max(w.projectiles().len());
            hashes.push(w.state_hash());
        }
        (hashes, max_proj, events)
    };
    let a = std::thread::spawn(run).join().unwrap();
    let b = run();
    let same = a.0.iter().zip(&b.0).filter(|(x, y)| x == y).count();
    let shots =
        a.2.iter()
            .filter(|e| matches!(e, SimEvent::Shot { .. }))
            .count();
    let hits =
        a.2.iter()
            .filter(|e| matches!(e, SimEvent::Hit { .. }))
            .count();
    println!(
        "5000 ticks x 2 runs: {same}/5000 identical state hashes; {} projectiles max in flight, {shots} shots, {hits} hits, final hash {:#018x}",
        a.1,
        a.0[4999]
    );
    assert_eq!(same, 5000);
    assert_eq!(a.2, b.2, "identical event streams");
    assert!(shots > 100 && a.1 > 0, "script must shoot");
}

#[test]
fn server_and_predict_weapon_state_are_bit_identical() {
    let col = flat(4, 5.0);
    let cfg = sim_cfg(GameData::embedded(), 77);
    let start = Vec3::new(120.0, 5.02, 120.0);
    let mut w = World::new(cfg, &col);
    let eid = w
        .spawn_player(frontline_sim::SpawnReq {
            player: 5,
            team: Team::Ger,
            pos: start,
            yaw: 0.0,
        })
        .unwrap();
    // A second player so that the world has company (not shot at: same script area).
    w.spawn_player(frontline_sim::SpawnReq {
        player: 6,
        team: Team::Us,
        pos: start + Vec3::new(4.0, 0.0, 0.0),
        yaw: 0.0,
    })
    .unwrap();
    let mut pr = PredictSim::new(cfg, &col, 5);
    pr.set_entity(eid);
    pr.reset_to(&w.player_state(5).unwrap(), Tick(0));
    pr.reset_weapon(w.weapon_state(5));
    assert!(w.weapon_state(5).unwrap().bit_eq(&pr.weapon().unwrap()));
    let cmds = script(5000, 0xfeed);
    let mut shots = 0;
    let mut reloads = 0;
    let mut kinds = std::collections::BTreeSet::new();
    for (i, c) in cmds.iter().enumerate() {
        w.step(&[(5, *c)]);
        let a = w.player_state(5).unwrap();
        let b = pr.step_player(c);
        assert!(a.bit_eq(&b), "tick {i}: movement");
        let (ws, ps) = (w.weapon_state(5).unwrap(), pr.weapon().unwrap());
        assert!(ws.bit_eq(&ps), "tick {i}: server {ws:?} vs predict {ps:?}");
        let out = pr.last_weapon_out();
        shots += usize::from(out.shot.is_some());
        reloads += usize::from(out.reload_started.is_some());
        kinds.insert(ws.to_bytes()[6]);
    }
    println!("5000 ticks: WeaponState bit-identical every tick; {shots} predicted shots, {reloads} reloads, fsm codes seen {kinds:?}");
    assert!(shots > 20 && reloads > 3);
    // The state survives its bytes (self_state / Predictor.reset).
    let st = w.weapon_state(5).unwrap();
    assert_eq!(
        frontline_sim::WeaponState::from_bytes(&st.to_bytes()),
        Some(st)
    );
    // A reset to the authoritative state continues identically.
    let mut pr2 = PredictSim::new(cfg, &col, 5);
    pr2.set_entity(eid);
    pr2.reset_to(&w.player_state(5).unwrap(), w.tick());
    pr2.reset_weapon(Some(st));
    let c = cmd(0, 0, 0, BTN_FIRE, 0);
    w.step(&[(5, c)]);
    pr2.step_player(&c);
    assert!(w.weapon_state(5).unwrap().bit_eq(&pr2.weapon().unwrap()));
}
