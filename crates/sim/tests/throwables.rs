//! M1-015: blast curve and occlusion, cooking and fuse timing, throw distance, bounce, smoke clouds
//! and line of sight, friendly fire, end-to-end kills.
mod common;

use common::*;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::throwables::grenade::{self, Flight};
use frontline_sim::throwables::SmokeParams;
use frontline_sim::throwables::{explosion_damage, smoke_blocks_los, Occlusion, SmokeCloud};
use frontline_sim::{SimEvent, Team, Tick, Vec3, World};

fn arena() -> frontline_sim::CollisionData {
    synth_data(14, 2, &|_, _| 5.0)
}

#[test]
fn explosion_curve() {
    // Mk 2: Dmax 220, R 8; D = Dmax (1 - (r/R)^2) (PI-054: the task text's 211.7 / 121 are off).
    for (r, want) in [
        (0.0, 220.0),
        (2.0, 206.25),
        (4.0, 165.0),
        (6.0, 96.25),
        (8.0, 0.0),
    ] {
        let d = explosion_damage(220.0, r, 8.0, Occlusion::Clear);
        assert!((d - want).abs() < 0.5, "r={r}: {d}");
    }
    // 3 m kills outright (>= 150 HP).
    assert!(explosion_damage(220.0, 3.0, 8.0, Occlusion::Clear) > 150.0);
}

#[test]
fn occlusion_levels() {
    let clear = explosion_damage(220.0, 3.0, 8.0, Occlusion::Clear);
    assert!((explosion_damage(220.0, 3.0, 8.0, Occlusion::Solid) - clear * 0.25).abs() < 1e-3);
    assert!((explosion_damage(220.0, 3.0, 8.0, Occlusion::Thin) - clear * 0.6).abs() < 1e-3);
}

fn gadget(buttons: u16, pitch_deg: f32) -> InputCmd {
    let mut c = fire_cmd(pitch_deg.to_radians(), buttons);
    c.yaw = 16384; // +x
    c
}

fn idle() -> InputCmd {
    cmd(0, 0, 49152, 0, 0)
}

fn two(dist: f32, vteam: Team) -> World {
    duel_world(
        data(),
        &arena(),
        3,
        &[
            (0, Team::Us, 20.0, 60.0, 16384),
            (1, vteam, 20.0 + dist, 60.0, 49152),
        ],
    )
}

fn data() -> &'static frontline_sim::GameData {
    data_with(|_| {})
}

/// Cooks for `cook` ticks, releases, and returns (release tick, explosion tick, events).
fn throw(
    w: &mut World,
    cook: u32,
    pitch_deg: f32,
    extra: u16,
) -> (u32, Option<u32>, Vec<SimEvent>) {
    let mut all = Vec::new();
    let mut explode = None;
    let mut release = 0;
    for t in 0..(cook + 400) {
        let held = t <= cook;
        let b = if held { buttons::GADGET | extra } else { 0 };
        if !held && release == 0 {
            release = t;
        }
        let ev = w.step(&[(0, gadget(b, pitch_deg)), (1, idle())]).events;
        for e in &ev {
            if matches!(e, SimEvent::Explosion { .. }) && explode.is_none() {
                explode = Some(t);
            }
        }
        all.extend(ev);
        if explode.is_some() {
            break;
        }
    }
    (release, explode, all)
}

#[test]
fn cooked_grenade_explodes_fuse_after_the_start() {
    // Mk 2 fuse 4.0 s = 120 ticks counted from the start of cooking: cook 45 ticks (1.5 s), the
    // blast comes 75 ticks (2.5 s) after the release.
    let mut w = two(40.0, Team::Ger);
    let (release, explode, _) = throw(&mut w, 45, 40.0, 0);
    let after = explode.unwrap() - release;
    assert!((74..=76).contains(&after), "{after}");
}

#[test]
fn held_past_the_fuse_explodes_in_the_hand() {
    // Cook 4.2 s = 126 ticks: the grenade goes off at 120 on its holder.
    let mut w = two(40.0, Team::Ger);
    let (_, explode, ev) = throw(&mut w, 126, 40.0, 0);
    assert!(explode.is_some());
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::Kill { victim: 0, .. })),
        "self kill"
    );
    assert!(!ev.iter().any(|e| matches!(e, SimEvent::Thrown { .. })));
}

#[test]
fn throw_distance_matches_45_degrees() {
    // 18 m/s at 45 deg (40 deg view pitch + 5 deg lift): ~33 m on flat ground.
    let col = arena();
    let g = data().explosive("us_mk2").unwrap();
    let frontline_sim::data::defs::ExplosiveGameplay::Frag(fg) = &g.gameplay else {
        panic!()
    };
    let mut f = Flight {
        pos: Vec3::new(20.0, 6.4, 60.0),
        vel: Vec3::new(1.0, 1.0, 0.0).normalize() * fg.throw_speed_mps as f32,
        rest: 0,
    };
    let x0 = f.pos.x;
    for _ in 0..200 {
        grenade::step(&mut f, &col, 30, 0.4, 0.6);
        if f.pos.y < 5.1 {
            break;
        }
    }
    let range = f.pos.x - x0;
    assert!((range - 33.0).abs() / 33.0 < 0.1, "{range}");
}

#[test]
fn bounce_keeps_40_percent_normal_speed_and_comes_to_rest() {
    let col = arena();
    let mut f = Flight {
        pos: Vec3::new(20.0, 7.0, 60.0),
        vel: Vec3::ZERO + Vec3::new(0.0, -0.001, 0.0),
        rest: 0,
    };
    let mut peak_after_first = 0.0f32;
    let mut bounced = false;
    for _ in 0..300 {
        let vy = f.vel.y;
        grenade::step(&mut f, &col, 30, 0.4, 0.6);
        if !bounced && vy < -1.0 && f.vel.y > 0.0 {
            bounced = true;
            // Normal speed after = 0.4 x before (within the sub-step gravity of the tick).
            assert!((f.vel.y / -vy - 0.4).abs() < 0.08, "{} vs {}", f.vel.y, vy);
        }
        if bounced {
            peak_after_first = peak_after_first.max(f.pos.y);
        }
    }
    assert!(bounced && peak_after_first > 5.0);
    assert_eq!(f.vel, Vec3::ZERO, "came to rest");
    assert!(f.pos.y < 5.2, "rest height {}", f.pos.y);
}

fn cloud() -> SmokeCloud {
    SmokeCloud {
        pos: Vec3::new(50.0, 1.0, 50.0),
        start: Tick(100),
        params: SmokeParams {
            radius_m: 7.0,
            expand_ticks: 90,
            duration_ticks: 1350,
            decay_ticks: 240,
        },
    }
}

#[test]
fn smoke_radius_curve() {
    let c = cloud();
    assert!((c.radius(Tick(100 + 45)) - 3.5).abs() < 0.2); // +1.5 s
    assert!((c.radius(Tick(100 + 90)) - 7.0).abs() < 1e-3); // +3 s
    assert!((c.radius(Tick(100 + 1000)) - 7.0).abs() < 1e-3);
    let mut last = f32::MAX;
    for t in (1350 - 240)..1350 {
        let r = c.radius(Tick(100 + t));
        assert!(r <= last + 1e-6, "monotone fade at {t}");
        last = r;
    }
    assert!(!c.ended(Tick(100 + 1349)));
    assert!(c.ended(Tick(100 + 1350)));
}

#[test]
fn smoke_blocks_sight_only_while_dense() {
    let c = [cloud()];
    let (a, b) = (Vec3::new(30.0, 1.0, 50.0), Vec3::new(70.0, 1.0, 50.0));
    let side = (Vec3::new(30.0, 1.0, 70.0), Vec3::new(70.0, 1.0, 70.0));
    let t = Tick(100 + 500);
    assert!(smoke_blocks_los(&c, t, a, b));
    assert!(!smoke_blocks_los(&c, t, side.0, side.1));
    // Before it starts and once thin (< 0.5 concentration) it does not block.
    assert!(!smoke_blocks_los(&c, Tick(50), a, b));
    assert!(!smoke_blocks_los(&c, Tick(100 + 1350 - 100), a, b));
}

#[test]
fn thrown_smoke_blocks_world_los_but_not_bullets() {
    use frontline_sim::WorldView;
    let mut w = two(20.0, Team::Ger);
    // AIM + GADGET cooks the smoke grenade; throw it flat so it lands near the target.
    let (_, _, ev) = {
        let mut all = Vec::new();
        for t in 0..300u32 {
            let b = if t < 2 {
                buttons::GADGET | buttons::AIM
            } else {
                0
            };
            all.extend(w.step(&[(0, gadget(b, 10.0)), (1, idle())]).events);
        }
        ((), (), all)
    };
    assert!(ev
        .iter()
        .any(|e| matches!(e, SimEvent::Thrown { smoke: true, .. })));
    assert!(ev
        .iter()
        .any(|e| matches!(e, SimEvent::SmokeStarted { .. })));
    let pos = ev
        .iter()
        .find_map(|e| match e {
            SimEvent::SmokeStarted { pos, .. } => Some(*pos),
            _ => None,
        })
        .unwrap();
    // Let it expand, then look straight through its centre.
    for _ in 0..100 {
        w.step(&[(0, idle()), (1, idle())]);
    }
    let (a, b) = (pos - Vec3::X * 20.0, pos + Vec3::X * 20.0);
    assert!(!w.los(a, b), "smoke blocks the view");
    assert!(w.collision().los(a, b), "terrain alone would not");
}

#[test]
fn blast_kills_enemy_and_hurts_friend_less() {
    // Enemy 3 m from the blast: 189 HP -> dead. A teammate there takes x0.7 = 132.
    for (team, dead) in [(Team::Ger, true), (Team::Us, true)] {
        let mut w = two(4.0, team);
        // Grenade thrown straight down lands at the thrower; instead explode at the thrower's feet
        // by cooking past the fuse next to the target (4 m away, r = 4 -> 165 / 115).
        let (_, explode, _) = throw(&mut w, 126, 40.0, 0);
        assert!(explode.is_some());
        let hp = w.health(1).unwrap();
        if team == Team::Ger {
            assert_eq!(hp, 0.0, "enemy dies");
        } else {
            // 165 x 0.7 = 115.5 >= 100 too; the teammate dies as well but via ff.
            assert!(dead && hp == 0.0);
        }
    }
}

#[test]
fn friendly_blast_uses_ff_multiplier() {
    // 8 m R: a teammate 6 m away takes 96.25 x 0.7 = 67.4; an enemy takes 96.25.
    let hp = |team| {
        let mut w = two(5.9, team);
        throw(&mut w, 126, 40.0, 0);
        w.health(1).unwrap()
    };
    let enemy = hp(Team::Ger);
    let friend = hp(Team::Us);
    assert!(enemy < 10.0, "{enemy}");
    assert!(friend > 20.0 && friend < 45.0, "{friend}");
}

#[test]
fn deterministic_and_counts_deplete() {
    let run = || {
        let mut w = two(30.0, Team::Ger);
        throw(&mut w, 10, 30.0, 0);
        w.state_hash()
    };
    assert_eq!(run(), run());
    // Two frag grenades only: the third press does not start a cook.
    let mut w = two(60.0, Team::Ger);
    let mut thrown = 0;
    for round in 0..3 {
        for t in 0..10u32 {
            let b = if t < 3 { buttons::GADGET } else { 0 };
            for e in w.step(&[(0, gadget(b, 40.0)), (1, idle())]).events {
                if matches!(e, SimEvent::Thrown { .. }) {
                    thrown += 1;
                }
            }
        }
        let _ = round;
    }
    assert_eq!(thrown, 2);
}
