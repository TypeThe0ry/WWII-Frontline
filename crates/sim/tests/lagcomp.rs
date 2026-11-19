//! M1-025: lag compensation. A target strafes across the shooter's line of fire; the shooter
//! fires at the pose the client saw `r` ticks ago. With rewind the shot lands; without it the
//! bullet passes behind the target. Also: clamp, determinism of the hash with rewound shots.
mod common;

use common::*;
use frontline_sim::weapon::ballistics::BallisticsParams;
use frontline_sim::{SimEvent, Team, World};

const US: u8 = 0;
const GER: u8 = 1;
const GAP: f32 = 20.0;

/// Whether the shot fired at the pose seen `rewind` ticks ago hits.
fn trial(seed: u64, rewind: u32, enabled: bool) -> bool {
    let data = no_spread();
    let mut w: World = duel_world(
        data,
        &flat(4, 5.0),
        seed,
        &[
            (US, Team::Us, 100.0, 100.0, 16384),
            (GER, Team::Ger, 100.0 + GAP, 80.0, 0),
        ],
    );
    w.set_lagcomp_enabled(enabled);
    let bp = BallisticsParams::from_def(data.weapon("us_m1_garand").unwrap());
    let pitch = solve_pitch(&bp, 1.65, GAP, 1.235);
    let mut zs: Vec<f32> = Vec::new();
    let mut hit = false;
    let mut fired = false;
    for _ in 0..200 {
        let now = w.tick().0 + 1;
        let mut shooter = fire_cmd(pitch, 0);
        // Fire when the pose of tick (now - rewind) has just crossed the line of fire z = 100.
        if !fired && now > 40 && zs.len() as u32 >= rewind {
            let seen_idx = zs.len() - rewind as usize;
            if zs[seen_idx - 1] < 100.0 && zs[seen_idx] >= 100.0 {
                shooter.buttons = BTN_FIRE;
                // zs[i] is the pose after the step of tick base+i.
                shooter.view_tick = rewind as u16;
                fired = true;
            }
        }
        let target = cmd(0, 127, 0, 0, 0);
        let ev = w.step(&[(US, shooter), (GER, target)]).events;
        zs.push(w.player_state(GER).unwrap().pos.z);
        for e in &ev {
            match e {
                SimEvent::Hit { shooter, .. } if *shooter < 100 => hit = true,
                _ => {}
            }
        }
    }
    hit
}

#[test]
fn rewind_makes_the_shot_land_and_control_misses() {
    let (mut on, mut off) = (0, 0);
    for seed in 0..12 {
        on += u32::from(trial(seed, 6, true));
        off += u32::from(trial(seed, 6, false));
    }
    eprintln!("hit rate with rewind {on}/12, without {off}/12");
    assert!(on >= 10, "compensated hits {on}");
    assert!(off <= 2, "uncompensated hits {off}");
}

#[test]
fn rewind_is_clamped_to_eight_ticks() {
    // Seen 8 ticks ago: compensated in full. Seen 12 ticks ago: only 8 are rewound, so the
    // target (4 ticks further along) is missed.
    let hits8 = (0..8).filter(|&s| trial(s, 8, true)).count();
    let hits12 = (0..8).filter(|&s| trial(s, 12, true)).count();
    eprintln!("8 ticks {hits8}/8, 12 ticks {hits12}/8");
    assert!(hits8 >= 7 && hits12 <= 1);
}

fn hit_target(events: &[SimEvent]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, SimEvent::Hit { .. }))
}

#[test]
fn projectile_switches_to_live_pose_after_catching_up_to_fire_tick() {
    use frontline_sim::Vec3;

    let data = no_spread();
    let mut w = duel_world(
        data,
        &flat(2, 5.0),
        1,
        &[
            (US, Team::Us, 40.0, 40.0, 16384),
            (GER, Team::Ger, 80.0, 42.0, 0),
        ],
    );
    let bp = BallisticsParams::from_def(data.weapon("us_m1_garand").unwrap());
    let pitch = solve_pitch(&bp, 1.65, 40.0, 1.235);
    let idle = fire_cmd(pitch, 0);
    w.step(&[(US, idle)]);
    let rewind_shot = frontline_protocol::msg::input::InputCmd {
        view_tick: 1,
        ..fire_cmd(pitch, BTN_FIRE)
    };
    assert!(!hit_target(&w.step(&[(US, rewind_shot)]).events));
    // The last historical frame is still off the line of fire. Only the live pose can hit.
    w.debug_teleport(GER, Vec3::new(80.0, 5.0, 40.0));
    assert!(hit_target(&w.step(&[(US, idle)]).events));
    assert_eq!(w.lagcomp_stats().history_clamped, 0);
}

#[test]
fn earliest_retained_frame_is_used_and_counted_during_startup() {
    let data = no_spread();
    let mut w = World::new(sim_cfg(data, 1), &flat(2, 5.0));
    for (player, team, x) in [(US, Team::Us, 40.0), (GER, Team::Ger, 60.0)] {
        w.spawn_player(frontline_sim::SpawnReq {
            player,
            team,
            pos: frontline_sim::Vec3::new(x, 5.02, 40.0),
            yaw: 0.0,
        })
        .unwrap();
    }
    let bp = BallisticsParams::from_def(data.weapon("us_m1_garand").unwrap());
    let pitch = solve_pitch(&bp, 1.65, 20.0, 1.235);
    w.step(&[(US, fire_cmd(pitch, 0))]);
    w.debug_teleport(GER, frontline_sim::Vec3::new(60.0, 5.0, 42.0));
    let fire = frontline_protocol::msg::input::InputCmd {
        view_tick: 8,
        ..fire_cmd(pitch, BTN_FIRE)
    };
    assert!(hit_target(&w.step(&[(US, fire)]).events));
    assert_eq!(w.lagcomp_stats().history_clamped, 1);
}

#[test]
fn thick_cover_rejects_rewound_hit_only_after_the_required_duration() {
    use frontline_sim::Vec3;

    let data = no_spread();
    // The ridge occludes the target's torso while a head-height ray can pass above it.
    let col = synth_data(1, 1, &|x, _| {
        if (21.0..=23.0).contains(&x) {
            6.45
        } else {
            5.0
        }
    });
    let bp = BallisticsParams::from_def(data.weapon("us_m1_garand").unwrap());
    let pitch = solve_pitch(&bp, 1.65, 20.0, 1.66);
    for (cover_ticks, rewind, expect_hit) in
        [(3, 2, true), (9, 2, true), (15, 2, false), (15, 0, true)]
    {
        let mut w = duel_world(
            data,
            &col,
            1,
            &[
                (US, Team::Us, 10.0, 30.0, 16384),
                (GER, Team::Ger, 15.0, 30.0, 0),
            ],
        );
        let idle = fire_cmd(pitch, 0);
        for _ in 0..12 {
            w.step(&[(US, idle)]);
        }
        w.debug_teleport(GER, Vec3::new(30.0, 5.0, 30.0));
        for _ in 0..cover_ticks {
            w.step(&[(US, idle)]);
        }
        let shot = frontline_protocol::msg::input::InputCmd {
            view_tick: rewind,
            ..fire_cmd(pitch, BTN_FIRE)
        };
        let events = w.step(&[(US, shot)]).events;
        assert_eq!(
            hit_target(&events),
            expect_hit,
            "cover_ticks={cover_ticks}, rewind={rewind}: {events:?}"
        );
        assert_eq!(w.lagcomp_stats().wall_rejected, u64::from(!expect_hit));
    }
}

#[test]
fn world_history_keeps_both_vehicle_boxes_in_their_old_pose() {
    use frontline_sim::Vec3;

    let mut w = duel_world(no_spread(), &flat(2, 5.0), 1, &[]);
    let id = w
        .spawn_vehicle("us_m4a1_75", Team::Us, Vec3::new(20.0, 6.0, 20.0), 0.0)
        .unwrap();
    w.step(&[]);
    let first_tick = w.tick();
    let old = w.lagcomp_history().frame(first_tick).unwrap().vehicles[0];
    assert_eq!(old.id, id);
    assert!(old.turret.center.y > old.hull.center.y);
    assert!(old.hull.half_extents.z > old.turret.half_extents.z);
    w.debug_place_vehicle(
        id,
        Vec3::new(30.0, 6.0, 20.0),
        glam::Quat::from_rotation_y(0.6),
    );
    w.step(&[]);
    let new = w.lagcomp_history().frame(w.tick()).unwrap().vehicles[0];
    assert!(new.hull.center.x - old.hull.center.x > 5.0);
    assert_ne!(new.turret.rotation, old.turret.rotation);
    assert_eq!(
        w.lagcomp_history().frame(first_tick).unwrap().vehicles[0],
        old
    );
}
