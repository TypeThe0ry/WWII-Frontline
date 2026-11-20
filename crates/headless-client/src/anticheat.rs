//! Input fixtures for a loopback-only mixed validation run.

use frontline_protocol::msg::input::{buttons, InputCmd};

pub(crate) fn inject(id: u32, team: u8, cmd: &mut InputCmd) {
    let step = u32::from(cmd.seq);
    match id {
        // 1.30 times the legal vector length. Sample every fifth input to sustain the run
        // below the rolling disconnect threshold, then exercise that threshold with client 3.
        0 if step.is_multiple_of(5) => speed(cmd),
        1 => {
            cmd.weapon_slot = 0;
            cmd.buttons &= !(buttons::FIRE | buttons::RELOAD);
            // Twice the 120 rpm Garand cap (15 ticks), or twice the Kar98k's 45-tick cycle.
            let fire = if team == 0 {
                (2 * step % 15) < 2
            } else {
                (2 * step % 45) < 2
            };
            if fire {
                cmd.buttons |= buttons::FIRE;
            } else if step.is_multiple_of(149) {
                cmd.buttons |= buttons::RELOAD;
            }
        }
        // Snap every five ticks: the first tick is clamped and the rest hold the view,
        // keeping this observation client below the rolling disconnect threshold.
        2 => {
            cmd.yaw = if (step / 5).is_multiple_of(2) {
                0
            } else {
                32768
            }
        }
        3 => speed(cmd),
        // An encodable weapon selector that the authoritative validator rejects.
        4 => cmd.weapon_slot = 7,
        _ => {}
    }
}

fn speed(cmd: &mut InputCmd) {
    cmd.move_x = 117;
    cmd.move_z = 117;
    cmd.buttons |= buttons::SPRINT;
}
