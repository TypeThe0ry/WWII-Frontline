//! M1-013b: melee windup / cooldown, range and sector boundaries, backstab, friendly fire,
//! lunge, fire blocking.
mod common;

use common::*;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::{GameData, SimEvent, Team, World};

const A: u8 = 0;
const B: u8 = 1;

fn arena() -> frontline_sim::CollisionData {
    synth_data(14, 2, &|_, _| 5.0)
}

fn idle(yaw: u16) -> InputCmd {
    cmd(0, 0, yaw, 0, 0)
}

/// Attacker at (20, 60) facing +x; target `d` m away at `deg` degrees off the facing, looking along
/// `tyaw`.
fn setup(d: f32, deg: f32, tyaw: u16, tteam: Team) -> World {
    let r = deg.to_radians();
    let (dx, dz) = (d * r.cos(), d * r.sin());
    duel_world(
        data(),
        &arena(),
        3,
        &[
            (A, Team::Us, 20.0, 60.0, 16384),
            (B, tteam, 20.0 + dx, 60.0 + dz, tyaw),
        ],
    )
}

fn data() -> &'static GameData {
    data_with(|_| {})
}

/// Holds MELEE for `ticks` ticks; returns the melee-hit events (damage, backstab).
fn swing(w: &mut World, ticks: u32, extra: u16, tyaw: u16) -> Vec<(f32, bool)> {
    let mut out = Vec::new();
    for _ in 0..ticks {
        let ev = w
            .step(&[
                (A, cmd(0, 0, 16384, buttons::MELEE | extra, 0)),
                (B, idle(tyaw)),
            ])
            .events;
        for e in ev {
            if let SimEvent::MeleeHit {
                damage_hp,
                backstab,
                ..
            } = e
            {
                out.push((damage_hp, backstab));
            }
        }
    }
    out
}

#[test]
fn range_boundary_is_1_7_m() {
    // The US bayonet reaches 1.7 m (data), not 1.8 m.
    let mut near = setup(1.7, 0.0, 49152, Team::Ger);
    assert_eq!(swing(&mut near, 14, 0, 49152), vec![(90.0, false)]);
    let mut far = setup(1.8, 0.0, 49152, Team::Ger);
    assert!(swing(&mut far, 14, 0, 49152).is_empty());
}

#[test]
fn sector_boundary_is_35_degrees() {
    let mut inside = setup(1.5, 35.0, 49152, Team::Ger);
    assert_eq!(swing(&mut inside, 14, 0, 49152).len(), 1);
    let mut outside = setup(1.5, 36.0, 49152, Team::Ger);
    assert!(swing(&mut outside, 14, 0, 49152).is_empty());
    let mut other_side = setup(1.5, -36.0, 49152, Team::Ger);
    assert!(swing(&mut other_side, 14, 0, 49152).is_empty());
}

#[test]
fn backstab_kills_from_120_degrees_not_119() {
    // Target facing +x (away from the attacker, who is at -x of it): attacker is directly behind.
    let mut w = setup(1.5, 0.0, 16384, Team::Ger);
    let hits = swing(&mut w, 14, 0, 16384);
    assert_eq!(hits.len(), 1);
    assert!(hits[0].1, "rear strike is a backstab");
    assert_eq!(w.health(B), Some(0.0));
    // Front strike is not.
    let mut f = setup(1.5, 0.0, 49152, Team::Ger);
    assert_eq!(swing(&mut f, 14, 0, 49152), vec![(90.0, false)]);
}

#[test]
fn backstab_angle_boundary() {
    // Direction target->attacker is -x. Target yaw t faces (sin t, cos t); angle to -x is
    // 180 - t' where the target is turned t' degrees away from facing the attacker.
    // Facing the attacker (yaw 49152 = -x) is 0 deg; turning 60 deg gives a 60 deg angle.
    let q = |deg: f32| {
        (49152.0 - deg / 360.0 * 65536.0)
            .round()
            .rem_euclid(65536.0) as u16
    };
    let mut at120 = setup(1.5, 0.0, q(-120.0), Team::Ger);
    assert!(
        swing(&mut at120, 14, 0, q(-120.0))[0].1,
        "120 deg is a backstab"
    );
    let mut at119 = setup(1.5, 0.0, q(-119.0), Team::Ger);
    assert!(!swing(&mut at119, 14, 0, q(-119.0))[0].1, "119 deg is not");
}

#[test]
fn friendly_takes_no_damage() {
    let mut w = setup(1.2, 0.0, 49152, Team::Us);
    assert!(swing(&mut w, 60, 0, 49152).is_empty());
    assert_eq!(w.health(B), Some(100.0));
}

#[test]
fn windup_and_cooldown_timing() {
    let mut w = setup(1.2, 0.0, 49152, Team::Ger);
    // 0.35 s windup = 11 ticks: no hit before, hit at the 11th tick after the press tick.
    assert!(swing(&mut w, 10, 0, 49152).is_empty());
    assert_eq!(swing(&mut w, 2, 0, 49152).len(), 1);
    // Cooldown 1.2 s = 36 ticks: no second strike inside it, one right after.
    assert!(swing(&mut w, 34, 0, 49152).is_empty());
    let second = swing(&mut w, 14, 0, 49152);
    assert_eq!(second.len(), 1);
    // 90 + 90 hp of damage: the second strike kills.
    assert_eq!(w.health(B), Some(0.0));
}

#[test]
fn melee_kill_is_attributed() {
    let mut w = setup(1.2, 0.0, 49152, Team::Ger);
    let mut kill = None;
    for _ in 0..120 {
        for e in w
            .step(&[(A, cmd(0, 0, 16384, buttons::MELEE, 0)), (B, idle(49152))])
            .events
        {
            if let SimEvent::Kill { victim, killer, .. } = e {
                kill = Some((victim, killer));
            }
        }
    }
    assert_eq!(kill, Some((B, Some(A))));
}

#[test]
fn melee_blocks_firing() {
    let mut w = setup(1.2, 0.0, 49152, Team::Ger);
    let mut shots = 0;
    for _ in 0..6 {
        for e in w
            .step(&[
                (A, cmd(0, 0, 16384, buttons::MELEE | buttons::FIRE, 0)),
                (B, idle(49152)),
            ])
            .events
        {
            if matches!(e, SimEvent::Shot { .. }) {
                shots += 1;
            }
        }
    }
    // The tick of the press itself may still shoot; once windup runs the trigger is dead.
    assert!(shots <= 1, "{shots}");
}

#[test]
fn deterministic() {
    let run = || {
        let mut w = setup(1.5, 10.0, 49152, Team::Ger);
        swing(&mut w, 100, 0, 49152);
        w.state_hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn lunge_extends_bayonet_range_by_20_percent() {
    use frontline_sim::weapon::melee::{melee_trace, step_melee};
    use frontline_sim::weapon::{MeleeParams, MeleeState};
    use frontline_sim::Vec3;
    let g = data();
    let p = MeleeParams::from_def(g.melee_weapon("us_m1_bayonet").unwrap(), 30);
    let (a, t) = (Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0));
    assert!(melee_trace(&p, false, a, 0.0, t, std::f32::consts::PI).is_none());
    assert!(melee_trace(&p, true, a, 0.0, t, std::f32::consts::PI).is_some());
    let mut s = MeleeState::default();
    step_melee(&mut s, &p, true, true, true);
    assert!(s.lunging);
    let k = g.melee_weapon("us_m3_knife").unwrap();
    let mut s = MeleeState::default();
    step_melee(&mut s, &MeleeParams::from_def(k, 30), true, true, true);
    assert!(!s.lunging, "knives do not lunge");
}
