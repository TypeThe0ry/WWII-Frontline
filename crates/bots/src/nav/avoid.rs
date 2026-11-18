//! Local avoidance: a separation steering force from nearby allies and a detour around a vehicle
//! bounding box (M1 has one tank). Pure functions over positions.

use super::V2;

pub const ALLY_MIN_GAP_M: f32 = 0.8;
pub const VEHICLE_MARGIN_M: f32 = 3.0;

/// Sum of repulsion from allies closer than `ALLY_MIN_GAP_M * 2`, normalised to at most 1.
pub fn separation(me: V2, others: &[V2]) -> V2 {
    let (mut fx, mut fz) = (0.0f32, 0.0f32);
    for o in others {
        let (dx, dz) = (me[0] - o[0], me[1] - o[1]);
        let d = (dx * dx + dz * dz).sqrt();
        if d > 1e-4 && d < ALLY_MIN_GAP_M * 2.0 {
            let w = (ALLY_MIN_GAP_M * 2.0 - d) / (ALLY_MIN_GAP_M * 2.0);
            fx += dx / d * w;
            fz += dz / d * w;
        }
    }
    let m = (fx * fx + fz * fz).sqrt();
    if m > 1.0 {
        [fx / m, fz / m]
    } else {
        [fx, fz]
    }
}

/// Pushes the desired direction away when `me` is inside the tank box grown by the margin.
/// `half` is the box half-extent (x, z) in the tank's frame; `yaw` its heading.
pub fn vehicle_push(me: V2, tank: V2, yaw: f32, half: V2) -> V2 {
    let (s, c) = yaw.sin_cos();
    let (dx, dz) = (me[0] - tank[0], me[1] - tank[1]);
    let (lx, lz) = (dx * c - dz * s, dx * s + dz * c);
    let (hx, hz) = (half[0] + VEHICLE_MARGIN_M, half[1] + VEHICLE_MARGIN_M);
    if lx.abs() > hx || lz.abs() > hz {
        return [0.0, 0.0];
    }
    // exit through the nearest face
    let (px, pz) = (hx - lx.abs(), hz - lz.abs());
    let (ox, oz) = if px < pz {
        (lx.signum(), 0.0)
    } else {
        (0.0, lz.signum())
    };
    [ox * c + oz * s, -ox * s + oz * c]
}
