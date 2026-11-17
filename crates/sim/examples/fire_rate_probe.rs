//! Measures the delivered cyclic rate of the M1 automatic and semi-automatic weapons over 30 s of
//! simulated trigger time (M1-013 acceptance 1). Run: `cargo run -p frontline-sim --example fire_rate_probe`.
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::weapon::{step_weapon, Fsm, ShotCtx, WeaponParams, WeaponState};
use frontline_sim::{GameData, MoveParams, PlayerAuthState, Tick, Vec3};

fn cmd(b: u16) -> InputCmd {
    InputCmd {
        seq: 0,
        tick: 0,
        move_x: 0,
        move_z: 0,
        yaw: 0,
        pitch: 0,
        buttons: b,
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    }
}

fn main() {
    let data = GameData::embedded();
    let mp = MoveParams::from_role(data.role("rifleman").expect("rifleman"), 30).expect("params");
    let mut auth = PlayerAuthState::spawn(Vec3::new(10.0, 5.0, 10.0), 0.0);
    auth.grounded = true;
    println!("weapon             data rpm   measured rpm   error");
    for (id, rpm, auto) in [
        ("us_thompson_m1a1", 700.0, true),
        ("us_m3_grease", 450.0, true),
        ("ger_mp40", 500.0, true),
        ("us_m1_carbine", 200.0, false),
        ("us_m1911a1", 300.0, false),
        ("ger_p38", 300.0, false),
    ] {
        let def = data.weapon(id).expect("weapon");
        let wp = WeaponParams::from_def(def, 30).expect("params");
        let idx = data.weapons.iter().position(|w| w.id == id).expect("index") as u16;
        let mut w = WeaponState::new(idx, wp.mag_capacity, 0);
        let mut shots = 0u32;
        for t in 0..900u32 {
            if w.mag == 0 {
                w.mag = wp.mag_capacity;
                w.fsm = Fsm::Ready;
                w.trigger_held = false;
            }
            // Auto: hold. Semi: the fastest clicker (release for the single tick before ready).
            let release =
                !auto && w.fsm == Fsm::Cooldown && w.timer_units > 1000 && w.timer_units <= 2000;
            let b = if release { 0 } else { buttons::FIRE };
            let ctx = ShotCtx {
                match_seed: 1,
                tick: Tick(t + 1),
                entity: 1,
            };
            if step_weapon(&mut w, &wp, &cmd(b), &auth, &mp, ctx)
                .shot
                .is_some()
            {
                shots += 1;
            }
        }
        let measured = f64::from(shots) * 2.0;
        println!(
            "{id:<18} {rpm:>8.0} {measured:>14.0} {:>7.2} %",
            (measured / rpm - 1.0) * 100.0
        );
    }
}
