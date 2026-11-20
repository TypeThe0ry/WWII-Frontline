//! Per-team C2 world-state projection (M2-009).

use frontline_protocol::msg::team::{ObjectiveState, WorldState, WorldStateC2, WORLD_OBJECTIVES};
use frontline_sim::{GameMode, Team, World};

pub(crate) fn due(world: &World, tick: u32) -> bool {
    matches!(world.config().mode, GameMode::Frontline | GameMode::Assault)
        && tick.is_multiple_of(u32::from(world.config().tick_hz))
}

pub(crate) fn project(world: &World, tick: u32) -> [WorldStateC2; 2] {
    let mut objectives = [ObjectiveState {
        owner: 0,
        capture: 0,
    }; WORLD_OBJECTIVES];
    for (out, point) in objectives.iter_mut().zip(world.view().objectives()) {
        out.owner = if point.contested {
            3
        } else {
            point.owner.map_or(0, |team| team as u8 + 1)
        };
        out.capture = point.p.abs().round().clamp(0.0, 100.0) as u8;
    }
    let remaining = world.match_state().ends_at.saturating_sub(tick);
    let timer = remaining.div_ceil(u32::from(world.config().tick_hz));
    let to_u16 = |value: i32| value.clamp(0, i32::from(u16::MAX)) as u16;
    [Team::Us, Team::Ger].map(|team| {
        let pool = world.view().team_resources(team);
        WorldStateC2 {
            tick,
            team: team as u8,
            state: WorldState {
                timer: timer.min(u32::from(u16::MAX)) as u16,
                objectives,
                manpower: to_u16(pool.manpower),
                munitions: to_u16(pool.munitions),
                fuel: to_u16(pool.fuel),
                // Reserved until the Frontline mode supplies a position code.
                front_line_pos: 0,
            },
        }
    })
}
