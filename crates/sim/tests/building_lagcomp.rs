//! A rewound head hit can pass above a wall that has continuously hidden the victim's torso.
#![cfg(feature = "test-util")]

mod common;

use common::*;
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::ballistics::materials::MaterialTable;
use frontline_sim::collision::CoverSlab;
use frontline_sim::throwables::Occlusion;
use frontline_sim::weapon::ballistics::BallisticsParams;
use frontline_sim::{SimEvent, Team, Vec3};

#[test]
fn building_wall_guard_requires_sustained_thick_cover_and_a_rewound_hit() {
    let data = no_spread();
    let brick = MaterialTable::embedded().id_of("brick").unwrap();
    let bp = BallisticsParams::from_def(data.weapon("us_m1_garand").unwrap());
    let pitch = solve_pitch(&bp, 1.65, 20.0, 1.66);
    for (cover_ticks, rewind, oblique, expect_reject) in [
        (3, 2, true, false),
        (9, 2, true, false),
        (15, 2, true, true),
        (15, 0, true, false),
        (15, 2, false, false),
    ] {
        let mut col = flat(1, 5.0);
        // Both walls have the loader's 0.30 m masonry width. Only the oblique wall
        // exceeds 0.50 m along the chest ray; both leave the head-height shot clear.
        let (a, b) = if oblique {
            ([17.0, 27.5], [27.0, 32.5])
        } else {
            ([22.0, 25.0], [22.0, 35.0])
        };
        col.install_cover(vec![CoverSlab::new(
            a,
            b,
            4.6,
            6.45,
            0.30,
            brick,
            Occlusion::Solid,
        )
        .unwrap()]);
        let (eye, chest) = (Vec3::new(10.0, 6.65, 30.0), Vec3::new(30.0, 6.22, 30.0));
        assert!(col.terrain_los(eye, chest));
        assert!(col.raycast_cover_segment(eye, chest).is_some());

        let mut w = duel_world(
            data,
            &col,
            1,
            &[
                (0, Team::Us, 10.0, 30.0, 16384),
                (1, Team::Ger, 15.0, 30.0, 0),
            ],
        );
        let idle = fire_cmd(pitch, 0);
        w.debug_teleport(1, Vec3::new(30.0, 5.0, 30.0));
        for _ in 0..cover_ticks {
            w.step(&[(0, idle)]);
        }
        let events = w
            .step(&[(
                0,
                InputCmd {
                    view_tick: rewind,
                    ..fire_cmd(pitch, BTN_FIRE)
                },
            )])
            .events;
        assert_eq!(
            events.iter().any(|e| matches!(e, SimEvent::Hit { .. })),
            !expect_reject,
            "cover_ticks={cover_ticks}, rewind={rewind}, oblique={oblique}: {events:?}"
        );
        assert_eq!(w.lagcomp_stats().wall_rejected, u64::from(expect_reject));
    }
}
