//! M0-013: damage -> death -> respawn, end to end through `World` (kill events, shots to kill from
//! the data, respawn timing, dead players inert, disconnect cleanup, determinism).
mod common;

use common::*;
use frontline_protocol::msg::input::InputCmd;
use frontline_protocol::quant::{quant_yaw, rad_to_pitch_i16};
use frontline_sim::projectile::damage;
use frontline_sim::weapon::ballistics::BallisticsParams;
use frontline_sim::{
    CollisionData, GameData, HitZone, LifeState, SimEvent, Team, Tick, Vec3, World,
};

const SHOOTER: u8 = 0;
const VICTIM: u8 = 1;
const EYE: f32 = 1.65;

/// Weapons without spread and recoil: every shot flies the same line (damage tests only).
fn exact() -> &'static GameData {
    data_with(|w| {
        w.gameplay.spread.hip_deg = 0.0;
        w.gameplay.spread.ads_deg = 0.0;
        w.gameplay.recoil.vkick_deg = 0.0;
        w.gameplay.recoil.hsigma_deg = 0.0;
        w.gameplay.recoil.bloom_per_shot_deg = 0.0;
    })
}

fn arena() -> CollisionData {
    synth_data(14, 2, &|_, _| 5.0)
}

fn idle(yaw: u16) -> InputCmd {
    cmd(0, 0, yaw, 0, 0)
}

/// Shooter (id 0, faces +x) and victim (id 1, faces -x) `dist` metres apart on flat ground.
fn pair(data: &'static GameData, shooter_team: Team, dist: f32, seed: u64) -> World {
    let victim_team = if shooter_team == Team::Us {
        Team::Ger
    } else {
        Team::Us
    };
    duel_world(
        data,
        &arena(),
        seed,
        &[
            (SHOOTER, shooter_team, 20.0, 60.0, 16384),
            (VICTIM, victim_team, 20.0 + dist, 60.0, 49152),
        ],
    )
}

/// Steps both players with the given shooter command.
fn step2(w: &mut World, shooter: InputCmd) -> Vec<SimEvent> {
    w.step(&[(SHOOTER, shooter), (VICTIM, idle(49152))]).events
}

/// Shots needed to kill a standing victim at `dist` by aiming at `aim_y`, and the zone of the
/// lethal hit.
fn shots_to_kill(team: Team, weapon: &str, dist: f32, aim_y: f32) -> (u32, HitZone) {
    let data = exact();
    let mut w = pair(data, team, dist, 5);
    let bp = BallisticsParams::from_def(data.weapon(weapon).unwrap());
    let pitch = solve_pitch(&bp, EYE, dist, aim_y);
    let (mut hits, mut zone) = (0, None);
    for i in 0..2400u32 {
        let b = if i.is_multiple_of(2) { BTN_FIRE } else { 0 };
        for e in step2(&mut w, fire_cmd(pitch, b)) {
            match e {
                SimEvent::Hit { zone: z, .. } => {
                    hits += 1;
                    zone = Some(z);
                }
                // The hit that takes a victim down (or kills outright) ends the count (M1-016).
                SimEvent::Downed { victim, killer, .. } | SimEvent::Kill { victim, killer, .. } => {
                    assert_eq!((victim, killer), (VICTIM, Some(SHOOTER)));
                    return (hits, zone.unwrap());
                }
                _ => {}
            }
        }
        // Keep the shooter supplied: this test is about damage, not ammunition.
        if w.weapon_state(SHOOTER).unwrap().mag == 0 {
            break;
        }
    }
    panic!("{weapon} @ {dist} m: no kill after {hits} hits");
}

/// Torso shots to kill derived from the data (`ceil(max_hp / damage)`).
fn stk_from_data(weapon: &str, dist: f32) -> u32 {
    let def = GameData::embedded().weapon(weapon).unwrap();
    (100.0 / damage(def, HitZone::Torso, dist)).ceil() as u32
}

#[test]
fn stk_matches_data() {
    // (weapon, team, expected shots at 50 / 300 / 600 / 800 m) from the acceptance criteria.
    let cases = [
        ("us_m1_garand", Team::Us, [2u32, 2, 2, 2]),
        ("ger_kar98k", Team::Ger, [1, 2, 2, 2]),
    ];
    for (id, team, expect) in cases {
        for (d, want) in [50.0f32, 300.0, 600.0, 800.0].into_iter().zip(expect) {
            let from_data = stk_from_data(id, d);
            let (sim, zone) = shots_to_kill(team, id, d, 1.33);
            println!(
                "{id} @ {d} m: data {from_data} shots, simulated {sim} (lethal zone {zone:?})"
            );
            assert_eq!(zone, HitZone::Torso, "{id} {d}: aim must land in the torso");
            assert_eq!(from_data, want, "{id} {d}: data-derived shots");
            assert_eq!(sim, want, "{id} {d}: simulated shots");
        }
    }
}

#[test]
fn headshot_kills_with_one_shot() {
    for (id, team, want_dmg) in [
        ("us_m1_garand", Team::Us, 210.0f32),
        ("ger_kar98k", Team::Ger, 300.0),
    ] {
        let (n, zone) = shots_to_kill(team, id, 50.0, 1.66);
        assert_eq!((n, zone), (1, HitZone::Head), "{id}");
        let def = GameData::embedded().weapon(id).unwrap();
        let d = damage(def, HitZone::Head, 50.0);
        println!("{id} head @ 50 m: {d:.1} HP");
        assert!((d - want_dmg).abs() < 0.5, "{d} vs {want_dmg}");
    }
}

#[test]
fn kill_event_carries_killer_weapon_headshot_distance() {
    let mut w = pair(exact(), Team::Us, 10.0, 3);
    let bp = BallisticsParams::from_def(GameData::embedded().weapon("us_m1_garand").unwrap());
    let pitch = solve_pitch(&bp, EYE, 10.0, 1.66);
    let (e_s, e_v) = {
        let p = w.view().players();
        (p[0].entity, p[1].entity)
    };
    let ev = step2(&mut w, fire_cmd(pitch, BTN_FIRE));
    let kill = ev
        .iter()
        .find_map(|e| match e {
            SimEvent::Kill {
                victim,
                killer,
                weapon,
                headshot,
                dist_m,
            } => Some((*victim, *killer, *weapon, *headshot, *dist_m)),
            _ => None,
        })
        .expect("kill");
    assert_eq!(kill.0, VICTIM);
    assert_eq!(kill.1, Some(SHOOTER));
    assert_eq!(kill.2 as u16, weapon_index("us_m1_garand"));
    assert!(kill.3, "head hit");
    assert!(kill.4 > 8.0 && kill.4 < 10.0, "{}", kill.4);
    // The Hit event precedes the Kill in the same step.
    let hit_pos = ev.iter().position(|e| matches!(e, SimEvent::Hit { .. }));
    let kill_pos = ev.iter().position(|e| matches!(e, SimEvent::Kill { .. }));
    assert!(hit_pos < kill_pos);
    // State: dead, hp 0, gone from the living list, entity id kept.
    assert_eq!(w.health(VICTIM), Some(0.0));
    assert!(matches!(
        w.player_life(VICTIM),
        Some(LifeState::Dead {
            killer: Some(SHOOTER),
            headshot: true,
            ..
        })
    ));
    assert!(!w.view().alive(e_v));
    assert!(w.view().alive(e_s));
    assert_eq!(w.view().players().len(), 1);
    assert_eq!(w.view().entity_state(e_v).unwrap().health_q, 0);
    assert_eq!(w.view().entity_state(e_s).unwrap().health_q, 255);
    // A second kill event is never produced for the same death.
    for _ in 0..30 {
        assert!(!step2(&mut w, idle(16384))
            .iter()
            .any(|e| matches!(e, SimEvent::Kill { .. })));
    }
}

/// Fires at the victim's head at tick `kill_tick` (the projectile lands in the same tick at
/// 10 m) after idling up to it; returns the events of that step.
fn kill_at_tick(w: &mut World, kill_tick: u32) -> Vec<SimEvent> {
    let bp = BallisticsParams::from_def(GameData::embedded().weapon("us_m1_garand").unwrap());
    let pitch = solve_pitch(&bp, EYE, 10.0, 1.66);
    while w.tick().0 + 1 < kill_tick {
        step2(w, idle(16384));
    }
    let ev = step2(w, fire_cmd(pitch, BTN_FIRE));
    assert_eq!(w.tick().0, kill_tick);
    assert!(
        ev.iter().any(|e| matches!(e, SimEvent::Kill { .. })),
        "{ev:?}"
    );
    ev
}

/// Spawn tick of the victim after dying at `kill_tick` under `data`.
fn spawn_tick(data: &'static GameData, kill_tick: u32) -> u32 {
    let mut w = pair(data, Team::Us, 10.0, 3);
    kill_at_tick(&mut w, kill_tick);
    for _ in 0..3000 {
        let ev = step2(&mut w, idle(16384));
        if ev
            .iter()
            .any(|e| matches!(e, SimEvent::Spawn { player: VICTIM, .. }))
        {
            return w.tick().0;
        }
    }
    panic!("no respawn after kill at tick {kill_tick}");
}

#[test]
fn respawn_timing_matches_the_formula() {
    let aligned = with_respawn_rules(|r| {
        r.wave_aligned = true;
        r.wave_interval_ms = 15000.0;
        r.base_wait_ms = 5000.0;
        r.death_cam_ms = 3000.0;
    });
    let free = with_respawn_rules(|r| r.wave_aligned = false);
    // (kill tick, expected spawn tick): 30 Hz, so seconds * 30.
    for (kia_s, want_s) in [(7.0f32, 15.0f32), (1.0, 15.0), (16.0, 30.0), (15.0, 30.0)] {
        let kia = (kia_s * 30.0) as u32;
        let got = spawn_tick(aligned, kia);
        println!(
            "aligned: killed at {kia_s} s -> spawned at {:.3} s",
            got as f32 / 30.0
        );
        assert!(
            (got as i64 - (want_s * 30.0) as i64).abs() <= 1,
            "kia {kia_s}: spawn tick {got}, want {}",
            want_s * 30.0
        );
    }
    // Without wave alignment: max(ready, pick) = kia + 5 s.
    for (kia_s, want_s) in [(7.0f32, 12.0f32), (1.0, 6.0)] {
        let kia = (kia_s * 30.0) as u32;
        let got = spawn_tick(free, kia);
        println!(
            "free: killed at {kia_s} s -> spawned at {:.3} s",
            got as f32 / 30.0
        );
        assert!((got as i64 - (want_s * 30.0) as i64).abs() <= 1);
    }
    // A death cam longer than the base wait decides.
    let long_cam = with_respawn_rules(|r| {
        r.wave_aligned = false;
        r.death_cam_ms = 8000.0;
    });
    assert_eq!(spawn_tick(long_cam, 210), 210 + 240);
}

#[test]
fn respawn_state_dead_inert_then_restored() {
    let data = with_respawn_rules(|r| r.wave_aligned = false);
    let mut w = pair(data, Team::Us, 10.0, 8);
    let spawn_pt = Vec3::new(200.0, 5.0, 150.0);
    w.set_team_spawn(Team::Ger, spawn_pt);
    w.set_map_center(128.0, 128.0);
    let e_v = w.view().players()[1].entity;
    // The victim empties part of its magazine first (aiming away: yaw 0 = +z).
    for i in 0..12u32 {
        let b = if i.is_multiple_of(2) { BTN_FIRE } else { 0 };
        w.step(&[(SHOOTER, idle(16384)), (VICTIM, cmd(0, 0, 0, b, 0))]);
    }
    let before = w.weapon_state(VICTIM).unwrap();
    assert!(before.mag < 5, "victim fired: {before:?}");
    let t0 = w.tick().0 + 3;
    kill_at_tick(&mut w, t0);
    let corpse = w.player_state(VICTIM).unwrap().pos;
    let respawn_tick = match w.player_life(VICTIM).unwrap() {
        LifeState::Dead {
            t_kia,
            respawn_tick,
            ..
        } => {
            assert_eq!(t_kia, Tick(t0));
            assert_eq!(respawn_tick.0, t0 + 150, "free respawn = kia + base wait");
            respawn_tick.0
        }
        s => panic!("{s:?}"),
    };
    assert_eq!(w.respawn_pending(), 1);

    // Dead: inputs are ignored (10 ticks), no shots, no hits on the corpse.
    let mag = w.weapon_state(VICTIM).unwrap().mag;
    let bp = BallisticsParams::from_def(GameData::embedded().weapon("us_m1_garand").unwrap());
    let pitch = solve_pitch(&bp, EYE, 10.0, 1.2);
    let (mut shots_at_corpse, mut hits_on_corpse) = (0, 0);
    while w.tick().0 + 1 < respawn_tick {
        let i = w.tick().0;
        let b = if i.is_multiple_of(2) { BTN_FIRE } else { 0 };
        let ev = w
            .step(&[
                (SHOOTER, fire_cmd(pitch, b)),
                (VICTIM, cmd(127, 127, 0, BTN_FIRE | BTN_JUMP, 0)),
            ])
            .events;
        for e in &ev {
            match e {
                SimEvent::Shot { shooter, .. } if *shooter == e_v => panic!("dead player fired"),
                SimEvent::Shot { .. } => shots_at_corpse += 1,
                SimEvent::Hit { target, .. } if *target == e_v => hits_on_corpse += 1,
                _ => {}
            }
        }
        let p = w.player_state(VICTIM).unwrap();
        assert_eq!(
            p.pos,
            corpse,
            "dead player must not move (tick {})",
            w.tick().0
        );
        assert_eq!(w.weapon_state(VICTIM).unwrap().mag, mag);
        assert!(matches!(
            w.player_life(VICTIM),
            Some(LifeState::Dead { .. })
        ));
    }
    println!("shots at the corpse {shots_at_corpse}, hits on it {hits_on_corpse}");
    assert!(shots_at_corpse >= 3);
    assert_eq!(hits_on_corpse, 0, "dead players cannot be hit");

    // The respawn tick.
    let ev = step2(&mut w, idle(16384));
    assert_eq!(w.tick().0, respawn_tick);
    let spawn = ev
        .iter()
        .find_map(|e| match e {
            SimEvent::Spawn { player, pos } => Some((*player, *pos)),
            _ => None,
        })
        .expect("spawn event");
    assert_eq!(spawn.0, VICTIM);
    let st = w.player_state(VICTIM).unwrap();
    assert_eq!(st.pos, spawn.1);
    let d_h = ((st.pos.x - spawn_pt.x).powi(2) + (st.pos.z - spawn_pt.z).powi(2)).sqrt();
    assert!(d_h <= 3.0, "{d_h} m from the spawn point");
    assert_eq!(w.health(VICTIM), Some(100.0));
    assert!(matches!(w.player_life(VICTIM), Some(LifeState::Alive)));
    assert_eq!(w.respawn_pending(), 0);
    // Full magazine and reserve = loadout start values (same as a fresh player).
    let fresh = w.weapon_state(SHOOTER).unwrap();
    let ws = w.weapon_state(VICTIM).unwrap();
    assert_eq!(ws.mag, 5, "Kar98k clip");
    assert_eq!(ws.reserve, 5 * 15); // 16 clips incl. the one in the gun
    assert_eq!(ws.recoil_pitch_deg, 0.0);
    assert_ne!(fresh.def_id, ws.def_id);
    assert!(w.view().alive(e_v));
    // Facing the map centre (128, 128) from (~200, ~150): yaw = atan2(dx, dz).
    let want_yaw = (128.0 - st.pos.x)
        .atan2(128.0 - st.pos.z)
        .rem_euclid(core::f32::consts::TAU);
    assert!(
        (st.yaw - want_yaw).abs() < 2e-3,
        "{} vs {}",
        st.yaw,
        want_yaw
    );
    // Back in the game: hittable again, no spawn protection in M0.
    assert_eq!(w.view().players().len(), 2);
}

#[test]
fn team_spawn_points_and_fallback() {
    // Each team respawns at its own point; a team without one returns to where its player joined.
    let data = with_respawn_rules(|r| {
        r.wave_aligned = false;
        r.death_cam_ms = 0.0;
        r.base_wait_ms = 100.0;
    });
    for victim_team in [Team::Us, Team::Ger] {
        let mut w = pair(
            data,
            if victim_team == Team::Us {
                Team::Ger
            } else {
                Team::Us
            },
            10.0,
            1,
        );
        w.set_team_spawn(Team::Us, Vec3::new(60.0, 5.0, 200.0));
        w.set_team_spawn(Team::Ger, Vec3::new(180.0, 5.0, 40.0));
        let t = w.tick().0 + 5;
        kill_at_tick_with(&mut w, t);
        let pos = respawn_pos(&mut w);
        let want = if victim_team == Team::Us {
            Vec3::new(60.0, 5.0, 200.0)
        } else {
            Vec3::new(180.0, 5.0, 40.0)
        };
        assert!(
            (pos.x - want.x).hypot(pos.z - want.z) <= 3.0,
            "{victim_team:?}: {pos}"
        );
    }
    let mut w = pair(data, Team::Us, 10.0, 1); // no team points set: join position
    let join = w.player_state(VICTIM).unwrap().pos;
    let t = w.tick().0 + 5;
    kill_at_tick_with(&mut w, t);
    let pos = respawn_pos(&mut w);
    assert!(
        (pos.x - join.x).hypot(pos.z - join.z) <= 3.0,
        "{pos} vs {join}"
    );
}

fn kill_at_tick_with(w: &mut World, t: u32) {
    kill_at_tick(w, t);
}

fn respawn_pos(w: &mut World) -> Vec3 {
    for _ in 0..600 {
        for e in step2(w, idle(16384)) {
            if let SimEvent::Spawn {
                player: VICTIM,
                pos,
            } = e
            {
                return pos;
            }
        }
    }
    panic!("no respawn");
}

#[test]
fn disconnect_cleans_up_death_state() {
    let mut w = pair(exact(), Team::Us, 10.0, 4);
    let t = w.tick().0 + 5;
    kill_at_tick(&mut w, t);
    assert_eq!(w.respawn_pending(), 1);
    w.remove_player(VICTIM);
    assert_eq!(w.respawn_pending(), 0);
    assert!(w.player_life(VICTIM).is_none());
    for _ in 0..1500 {
        let ev = w.step(&[(SHOOTER, idle(16384))]).events;
        assert!(!ev.iter().any(|e| matches!(e, SimEvent::Spawn { .. })));
    }
    // The slot can be joined again and is alive.
    w.spawn_player(frontline_sim::SpawnReq {
        player: VICTIM,
        team: Team::Ger,
        pos: Vec3::new(60.0, 5.02, 60.0),
        yaw: 0.0,
    })
    .unwrap();
    assert!(matches!(w.player_life(VICTIM), Some(LifeState::Alive)));
    assert_eq!(w.health(VICTIM), Some(100.0));
}

#[test]
fn killer_who_left_is_none() {
    // A bullet in flight kills after its owner disconnected: the kill has no killer.
    let mut w = pair(exact(), Team::Us, 400.0, 4);
    let bp = BallisticsParams::from_def(GameData::embedded().weapon("us_m1_garand").unwrap());
    let pitch = solve_pitch(&bp, EYE, 400.0, 1.66);
    step2(&mut w, fire_cmd(pitch, BTN_FIRE));
    w.remove_player(SHOOTER);
    let mut killer = None;
    for _ in 0..90 {
        for e in w.step(&[(VICTIM, idle(49152))]).events {
            if let SimEvent::Kill {
                killer: k, victim, ..
            } = e
            {
                assert_eq!(victim, VICTIM);
                killer = Some(k);
            }
        }
    }
    assert_eq!(killer, Some(None), "kill with unknown killer expected");
}

// ---- skirmish: random fire fight, determinism ----

struct Skirmish {
    hashes: Vec<u64>,
    events: Vec<SimEvent>,
    world: World,
    deaths_seen: u32,
}

/// `n` players per team 40 m apart shoot at the nearest living enemy with random trigger
/// discipline; dead players keep sending (ignored) input.
fn skirmish(seed: u64, n: u8, ticks: u32) -> Skirmish {
    let data = GameData::embedded();
    let col = flat(4, 5.0);
    let mut w = World::new(sim_cfg(data, seed), &col);
    w.set_team_spawn(Team::Us, Vec3::new(128.0, 5.0, 50.0));
    w.set_team_spawn(Team::Ger, Vec3::new(128.0, 5.0, 110.0));
    for i in 0..n {
        for (id, team, z) in [(i, Team::Us, 60.0), (n + i, Team::Ger, 100.0)] {
            w.spawn_player(frontline_sim::SpawnReq {
                player: id,
                team,
                pos: Vec3::new(40.0 + 5.0 * f32::from(i), 5.02, z),
                yaw: 0.0,
            })
            .unwrap();
        }
    }
    let mut rng = Rng(seed | 1);
    let mut out = Skirmish {
        hashes: Vec::new(),
        events: Vec::new(),
        world: w,
        deaths_seen: 0,
    };
    let mut was_alive = vec![true; usize::from(2 * n)];
    for _ in 0..ticks {
        let living: Vec<_> = out.world.view().players().to_vec();
        let mut inputs = Vec::new();
        for id in 0..2 * n {
            let Some(me) = living.iter().find(|p| p.player == id) else {
                inputs.push((id, idle(0)));
                continue;
            };
            let enemy = living.iter().filter(|p| p.team != me.team).min_by(|a, b| {
                let da = (a.pos - me.pos).length_squared();
                let db = (b.pos - me.pos).length_squared();
                da.total_cmp(&db)
            });
            let mut c = idle(0);
            if let Some(e) = enemy {
                let d = e.pos - me.pos;
                c.yaw = quant_yaw(libm::atan2f(d.x, d.z), 16).code;
                c.pitch = rad_to_pitch_i16(0.01).code;
                let mag = out.world.weapon_state(id).map_or(1, |s| s.mag);
                c.buttons = if mag == 0 {
                    BTN_RELOAD
                } else if rng.f32() < 0.6 {
                    BTN_FIRE
                } else {
                    0
                };
                c.move_z = if rng.f32() < 0.2 { 60 } else { 0 };
            }
            inputs.push((id, c));
        }
        let ev = out.world.step(&inputs).events;
        for id in 0..2 * n {
            let alive = !matches!(out.world.player_life(id), Some(LifeState::Dead { .. }));
            if was_alive[usize::from(id)] && !alive {
                out.deaths_seen += 1;
            }
            was_alive[usize::from(id)] = alive;
        }
        out.events.extend(ev);
        out.hashes.push(out.world.state_hash());
    }
    out
}

#[test]
fn twenty_players_fight_without_panic_and_counts_balance() {
    let s = skirmish(21, 10, 3000);
    let kills = s
        .events
        .iter()
        .filter(|e| matches!(e, SimEvent::Kill { .. }))
        .count() as u32;
    let spawns = s
        .events
        .iter()
        .filter(|e| matches!(e, SimEvent::Spawn { .. }))
        .count() as u32;
    let pending = s.world.respawn_pending() as u32;
    println!("20 players, 3000 ticks: {kills} kills, {} deaths observed, {spawns} spawns, {pending} pending", s.deaths_seen);
    // Downed players bleed for 80 s before they count as KIA (M1-016), so fewer deaths land inside
    // the 3000 ticks than in M0.
    assert!(kills >= 5, "a real fight: {kills}");
    assert_eq!(kills, s.deaths_seen, "kills == deaths");
    assert!(spawns + pending >= kills);
    assert_eq!(
        spawns + pending,
        kills,
        "every kill is either respawned or pending"
    );
    // Everybody is either alive or waiting in the queue.
    let dead = (0..20u8)
        .filter(|&p| matches!(s.world.player_life(p), Some(LifeState::Dead { .. })))
        .count() as u32;
    assert_eq!(dead, pending);
}

/// M1-016: re-pinned (0x90cd_4193_862c_a94c before): lethal hits now down players first.
const GOLDEN_SKIRMISH_FINAL: u64 = 0x5eeb_1d5c_73c0_7f81;

#[test]
fn deaths_and_respawns_are_deterministic_5000_ticks() {
    let a = skirmish(7, 3, 5000);
    let b = skirmish(7, 3, 5000);
    assert_eq!(a.hashes, b.hashes, "state_hash differs between runs");
    assert_eq!(a.events, b.events);
    let kills = a
        .events
        .iter()
        .filter(|e| matches!(e, SimEvent::Kill { .. }))
        .count();
    let spawns = a
        .events
        .iter()
        .filter(|e| matches!(e, SimEvent::Spawn { .. }))
        .count();
    println!(
        "5000 ticks: {kills} kills, {spawns} spawns, final hash {:#018x}",
        a.hashes[4999]
    );
    assert!(
        kills >= 3 && spawns >= 1,
        "scenario must include deaths and respawns"
    );
    let c = skirmish(8, 3, 1000);
    let d = skirmish(7, 3, 1000);
    assert_ne!(c.hashes[999], d.hashes[999], "another seed diverges");
    assert_eq!(
        a.hashes[4999], GOLDEN_SKIRMISH_FINAL,
        "golden skirmish hash changed"
    );
}
