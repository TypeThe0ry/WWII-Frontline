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
                let view = now - rewind;
                shooter.view_tick = view as u16;
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
