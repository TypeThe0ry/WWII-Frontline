//! M0-012 acceptance 6: damage and falloff, end to end through `World` (projectile flight,
//! hit volumes, `Hit` events, health), plus the hit / impact / filtering rules of the projectile stage.
mod common;

use common::*;
use frontline_sim::projectile::damage;
use frontline_sim::weapon::ballistics::BallisticsParams;
use frontline_sim::{GameData, HitZone, SimEvent, Team, Vec3};

const SHOOTER: u8 = 0;
const TARGET: u8 = 1;
const EYE: f32 = 1.65;

struct Outcome {
    hit: Option<(HitZone, f32, f32)>,
    events: Vec<SimEvent>,
    hp: f32,
}

/// One shot at a stationary enemy `dist` metres away along +x, aimed at height `aim_y` above
/// its feet; zero-spread data, so the first shot is exact.
fn shoot(weapon_team: Team, weapon: &str, dist: f32, aim_y: f32) -> Outcome {
    let data = no_spread();
    let col = synth_data(14, 2, &|_, _| 5.0);
    let target_team = if weapon_team == Team::Us {
        Team::Ger
    } else {
        Team::Us
    };
    let mut w = duel_world(
        data,
        &col,
        7,
        &[
            (SHOOTER, weapon_team, 20.0, 60.0, 16384),
            (TARGET, target_team, 20.0 + dist, 60.0, 49152),
        ],
    );
    let def = data.weapon(weapon).unwrap();
    let bp = BallisticsParams::from_def(def);
    let pitch = solve_pitch(&bp, EYE, dist, aim_y);
    let mut events = Vec::new();
    let mut hit = None;
    for i in 0..120 {
        let c = if i == 0 {
            fire_cmd(pitch, BTN_FIRE)
        } else {
            fire_cmd(pitch, 0)
        };
        let out = w.step(&[(SHOOTER, c), (TARGET, cmd(0, 0, 49152, 0, 0))]);
        for e in out.events {
            if let SimEvent::Hit {
                zone,
                damage_hp,
                dist_m,
                ..
            } = &e
            {
                hit = Some((*zone, *damage_hp, *dist_m));
            }
            events.push(e);
        }
        if hit.is_some() {
            break;
        }
    }
    Outcome {
        hit,
        events,
        hp: w.health(TARGET).unwrap(),
    }
}

#[test]
fn torso_damage_matches_falloff_at_50_300_600_m() {
    let data = GameData::embedded();
    let g = data.weapon("us_m1_garand").unwrap();
    let k = data.weapon("ger_kar98k").unwrap();
    // The pure formula at the nominal distances (acceptance values).
    for (def, name, expect) in [
        (
            g,
            "Garand",
            [70.0f32, 70.0 * (1.0 - 0.28 * 200.0 / 500.0), 70.0 * 0.72],
        ),
        (
            k,
            "Kar98k",
            [
                100.0,
                100.0 * (1.0 - 0.2 * 100.0 / 600.0),
                100.0 * (1.0 - 0.2 * 400.0 / 600.0),
            ],
        ),
    ] {
        for (d, want) in [50.0f32, 300.0, 600.0].into_iter().zip(expect) {
            let got = damage(def, HitZone::Torso, d);
            println!("{name} formula torso @ {d} m: {got:.3} HP (expected {want:.3})");
            assert!((got - want).abs() < 0.01, "{name} {d}: {got} vs {want}");
        }
        assert!(
            (damage(def, HitZone::Head, 50.0) - 3.0 * damage(def, HitZone::Torso, 50.0)).abs()
                < 1e-3
        );
    }
    // End to end: the Hit carries damage = formula(actual flight distance), health drops by it.
    for (team, id, nominal_torso) in [
        (Team::Us, "us_m1_garand", 70.0f32),
        (Team::Ger, "ger_kar98k", 100.0),
    ] {
        let def = data.weapon(id).unwrap();
        for dist in [50.0f32, 300.0, 600.0] {
            let o = shoot(team, id, dist, 1.235);
            let (zone, dmg, d) = o
                .hit
                .unwrap_or_else(|| panic!("{id} {dist} m: no hit; {:?}", o.events));
            println!(
                "{id} @ {dist} m: zone {zone:?}, hit distance {d:.2} m, damage {dmg:.3} HP (nominal torso {nominal_torso}), target HP {:.3}",
                o.hp
            );
            assert!(matches!(zone, HitZone::Torso | HitZone::Pelvis), "{zone:?}");
            // Muzzle is 0.5 m ahead of the eye and the torso capsule starts ~0.2 m before its axis.
            assert!(d < dist && d > dist - 1.0, "distance {d} vs {dist}");
            let mult = if zone == HitZone::Torso { 1.0 } else { 0.9 };
            let want = def.gameplay.damage_torso_hp as f32
                * frontline_sim::weapon::ballistics::falloff(&def.gameplay.falloff, d)
                * mult;
            assert!((dmg - want).abs() < 0.01, "{dmg} vs {want}");
            // A hit that takes the 100 hp to 0 downs the target with 40 downed hp (M1-016).
            let want_hp = if 100.0 - dmg < 1e-3 {
                40.0
            } else {
                100.0 - dmg
            };
            // (a downed target has already bled one tick when the test reads it)
            assert!((o.hp - want_hp).abs() < 0.05, "health conserved");
        }
    }
}

#[test]
fn headshot_is_three_times_torso() {
    for (team, id) in [(Team::Us, "us_m1_garand"), (Team::Ger, "ger_kar98k")] {
        let data = GameData::embedded();
        let def = data.weapon(id).unwrap();
        let o = shoot(team, id, 50.0, 1.66);
        let (zone, dmg, d) = o.hit.expect("hit");
        let torso = damage(def, HitZone::Torso, d);
        println!(
            "{id} headshot @ 50 m: {zone:?} {dmg:.2} HP = {:.2} x torso {torso:.2}",
            dmg / torso
        );
        assert_eq!(zone, HitZone::Head);
        assert!((dmg - 3.0 * torso).abs() < 0.01);
        assert!(dmg >= 100.0, "a rifle headshot is lethal (>= 100 HP)");
    }
    // Legs: x0.65; pelvis x0.9.
    let leg = shoot(Team::Us, "us_m1_garand", 50.0, 0.7);
    let (zone, dmg, d) = leg.hit.unwrap();
    assert_eq!(zone, HitZone::Limb);
    let def = GameData::embedded().weapon("us_m1_garand").unwrap();
    assert!((dmg - 0.65 * damage(def, HitZone::Torso, d)).abs() < 0.01);
    let pel = shoot(Team::Us, "us_m1_garand", 50.0, 0.95);
    assert_eq!(pel.hit.unwrap().0, HitZone::Pelvis);
}

fn world_for(players: &[(u8, Team, f32, f32, u16)]) -> frontline_sim::World {
    let col = synth_data(4, 4, &|_, _| 5.0);
    duel_world(no_spread(), &col, 3, players)
}

#[test]
fn teammates_take_half_damage_and_stop_the_bullet_shooter_is_never_hit() {
    // Shooter, a friend 20 m ahead, an enemy 40 m ahead on the same line: the bullet hits the friend
    // (friendly fire x0.5, M1-014) and stops there.
    let mut w = world_for(&[
        (0, Team::Us, 20.0, 60.0, 16384),
        (1, Team::Us, 40.0, 60.0, 16384),
        (2, Team::Ger, 60.0, 60.0, 16384),
    ]);
    let bp = BallisticsParams::from_def(w_data("us_m1_garand"));
    let pitch = solve_pitch(&bp, EYE, 40.0, 1.235);
    let mut hits = Vec::new();
    for i in 0..30 {
        let out = w.step(&[(0, fire_cmd(pitch, if i == 0 { BTN_FIRE } else { 0 }))]);
        for e in out.events {
            if let SimEvent::Hit { target, .. } = e {
                hits.push(target);
            }
        }
    }
    assert_eq!(hits.len(), 1);
    assert_eq!(hits, vec![frontline_sim::EntityId::from(hits[0])]);
    let friend = w.health(1).unwrap();
    assert!(
        friend < 100.0 && friend > 55.0,
        "friend took half damage: {friend}"
    );
    assert_eq!(w.health(2), Some(100.0), "the bullet stopped at the friend");
    assert_eq!(w.health(0), Some(100.0));
}

fn w_data(id: &str) -> &'static frontline_sim::data::defs::WeaponDef {
    no_spread().weapon(id).unwrap()
}

#[test]
fn shots_into_the_ground_make_impact_events_and_remove_the_projectile() {
    let mut w = world_for(&[(0, Team::Us, 20.0, 60.0, 16384)]);
    let mut c = fire_cmd(-0.3, BTN_FIRE); // 17 degrees down
    let mut impact = None;
    for i in 0..10 {
        let out = w.step(&[(0, c)]);
        for e in out.events {
            if let SimEvent::Impact { pos, normal, .. } = e {
                impact = Some((pos, normal, i));
            }
        }
        c = fire_cmd(-0.3, 0);
    }
    let (pos, normal, _) = impact.expect("impact event");
    assert!((pos.y - 5.0).abs() < 1e-3, "{pos:?}");
    assert!(normal.y > 0.99);
    assert!(
        pos.x > 20.0 && pos.x < 26.0,
        "eye 1.65 m above ground at ~17 deg: ~5.5 m ahead: {pos:?}"
    );
    assert!(w.projectiles().is_empty());
}

#[test]
fn projectiles_leaving_the_map_or_flying_too_long_are_removed() {
    // Fired almost straight up (stays above the map): removed after 4 s (120 ticks) at the latest,
    // earlier when it falls below 150 m/s.
    let mut w = world_for(&[(0, Team::Us, 20.0, 60.0, 16384)]);
    w.step(&[(0, fire_cmd(1.5, BTN_FIRE))]);
    assert_eq!(w.projectiles().len(), 1);
    let mut gone_at = None;
    for t in 1..=130 {
        w.step(&[(0, fire_cmd(1.5, 0))]);
        if w.projectiles().is_empty() {
            gone_at = Some(t);
            break;
        }
    }
    let t = gone_at.expect("removed");
    println!("sky shot removed after {t} ticks");
    assert!((60..=121).contains(&t), "lives between 2 s and 4 s: {t}");
    // Flat fire along +x leaves the 256 m wide terrain and disappears without an impact.
    let mut w = world_for(&[(0, Team::Us, 20.0, 60.0, 16384)]);
    let mut impacts = 0;
    for i in 0..60 {
        let out = w.step(&[(0, fire_cmd(0.0, if i == 0 { BTN_FIRE } else { 0 }))]);
        impacts += out
            .events
            .iter()
            .filter(|e| matches!(e, SimEvent::Impact { .. }))
            .count();
    }
    assert!(w.projectiles().is_empty());
    let _ = impacts;
}

#[test]
fn shot_event_reports_the_muzzle_and_direction() {
    let mut w = world_for(&[(0, Team::Us, 20.0, 60.0, 16384)]);
    let out = w.step(&[(0, fire_cmd(0.0, BTN_FIRE))]);
    let shot = out
        .events
        .iter()
        .find_map(|e| match e {
            SimEvent::Shot {
                shooter,
                muzzle,
                dir,
                weapon,
            } => Some((*shooter, *muzzle, *dir, *weapon)),
            _ => None,
        })
        .expect("shot");
    let p = w.player_state(0).unwrap().pos;
    assert!((shot.2.length() - 1.0).abs() < 1e-5);
    assert!((shot.2 - Vec3::X).length() < 1e-3);
    assert!(
        (shot.1 - (p + Vec3::new(0.5, 1.65, 0.0))).length() < 1e-3,
        "{:?}",
        shot.1
    );
    assert_eq!(shot.3, weapon_index("us_m1_garand"));
    // The first Garand shot costs a round.
    assert_eq!(w.weapon_state(0).unwrap().mag, 7);
}
