//! `weapon_step` (M1-013 acceptance 8): 100 players all holding the trigger of a full-auto weapon,
//! one tick of `step_weapon` each. Budget 0.3 ms per tick (ARCHITECTURE 6.10 stage 4 keeps 2.5 ms).
//! Run: `cargo bench -p frontline-sim --bench weapon_step`.
use std::hint::black_box;
use std::time::Instant;

use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::weapon::{step_weapon, Fsm, ShotCtx, WeaponParams, WeaponState};
use frontline_sim::{GameData, MoveParams, PlayerAuthState, Tick, Vec3};

fn main() {
    let data = GameData::embedded();
    let mp = MoveParams::from_role(data.role("rifleman").expect("rifleman"), 30).expect("params");
    let def = data.weapon("us_thompson_m1a1").expect("weapon");
    let wp = WeaponParams::from_def(def, 30).expect("params");
    let idx = data
        .weapons
        .iter()
        .position(|w| w.id == def.id)
        .expect("index") as u16;
    let mut auth = PlayerAuthState::spawn(Vec3::new(10.0, 5.0, 10.0), 0.0);
    auth.grounded = true;
    let cmd = InputCmd {
        seq: 0,
        tick: 0,
        move_x: 0,
        move_z: 0,
        yaw: 0,
        pitch: 0,
        buttons: buttons::FIRE,
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    };
    let mut ws: Vec<WeaponState> = (0..100)
        .map(|_| WeaponState::new(idx, wp.mag_capacity, 300))
        .collect();
    let mut samples = Vec::new();
    let mut shots = 0u64;
    for t in 0..3000u32 {
        let t0 = Instant::now();
        for (i, w) in ws.iter_mut().enumerate() {
            if w.mag == 0 {
                w.mag = wp.mag_capacity;
                w.fsm = Fsm::Ready;
            }
            let ctx = ShotCtx {
                match_seed: 1,
                tick: Tick(t + 1),
                entity: i as u16,
            };
            shots += u64::from(
                step_weapon(black_box(w), &wp, black_box(&cmd), &auth, &mp, ctx)
                    .shot
                    .is_some(),
            );
        }
        samples.push(t0.elapsed().as_nanos() as f64 / 1e6);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "100 players firing a Thompson: median {:.4} ms/tick, p99 {:.4} ms (budget 0.3 ms); {shots} shots",
        samples[samples.len() / 2],
        samples[samples.len() * 99 / 100]
    );
}
