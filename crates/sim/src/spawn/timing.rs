//! Spawn timing: base wait, wave alignment, suicide / flood penalties, warm-up halving.

use crate::ids::Tick;
use crate::respawn::{ms_to_ticks, respawn_time, RespawnPolicy};

use super::{FLOOD_COUNT, FLOOD_PENALTY_S, FLOOD_WINDOW_S, SUICIDE_PENALTY_S, SUICIDE_WINDOW_S};

/// Extra wait in whole seconds for a player killed at `t_kia`, `last_spawn` ticks after its last
/// spawn, with `recent_same_point` spawns at the point in the flood window before `t_kia`.
pub fn penalty_s(t_kia: Tick, last_spawn: Option<u32>, recent: &[u32], hz: u8) -> u32 {
    let mut extra = 0;
    if let Some(ls) = last_spawn {
        if t_kia.0.saturating_sub(ls) <= SUICIDE_WINDOW_S * u32::from(hz) {
            extra += SUICIDE_PENALTY_S;
        }
    }
    let n = recent
        .iter()
        .filter(|&&t| t <= t_kia.0 && t_kia.0 - t <= FLOOD_WINDOW_S * u32::from(hz))
        .count();
    if n >= FLOOD_COUNT {
        extra += FLOOD_PENALTY_S;
    }
    extra
}

/// HQ spawn tick: the wave-aligned `respawn_time` with `base_wait` scaled by `wait_mult` (warm-up
/// 0.5) plus `extra_s` of penalty.
pub fn hq_spawn_tick(
    t_kia: Tick,
    t_pick: Tick,
    policy: &RespawnPolicy,
    wait_mult: f32,
    extra_s: u32,
    hz: u8,
) -> Tick {
    let mut p = *policy;
    let ms = f64::from(p.base_wait_ms) * f64::from(wait_mult);
    p.base_wait_ms = libm::round(ms) as u32 + extra_s * 1000;
    respawn_time(t_kia, t_pick, &p, hz)
}

/// Ticks of `s` seconds.
pub fn secs(s: u32, hz: u8) -> u32 {
    ms_to_ticks(s * 1000, hz)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pol() -> RespawnPolicy {
        RespawnPolicy {
            death_cam_ms: 3000,
            base_wait_ms: 5000,
            wave_interval_ms: 15000,
            wave_aligned: true,
            spawn_jitter_m: 2.5,
        }
    }

    #[test]
    fn suicide_flood_and_warmup() {
        let hz = 30;
        let k = Tick(60 * 30); // 60 s, a wave boundary is 60 s
        let t_pick = pol().pick_tick(k, hz);
        let base = hq_spawn_tick(k, t_pick, &pol(), 1.0, 0, hz);
        // max(65, 63)=65 -> next wave 75 s
        assert_eq!(base, Tick(75 * 30));
        // suicide: spawned 10 s before KIA -> +5 s -> 70 s -> wave 75 s still; use a point just
        // after a wave: k=61 s -> ready 66 -> 75; +5 -> 71 -> 75: wave alignment hides it, so
        // check the penalty value and an unaligned policy.
        assert_eq!(penalty_s(k, Some(k.0 - 10 * 30), &[], hz), 5);
        assert_eq!(penalty_s(k, Some(k.0 - 21 * 30), &[], hz), 0);
        let mut un = pol();
        un.wave_aligned = false;
        let t = hq_spawn_tick(k, t_pick, &un, 1.0, 0, hz);
        assert_eq!(t, Tick(65 * 30));
        assert_eq!(hq_spawn_tick(k, t_pick, &un, 1.0, 5, hz), Tick(70 * 30));
        // flood: 8 spawns within 10 s -> +2 s
        let recent: Vec<u32> = (0..8).map(|i| k.0 - 100 + i).collect();
        assert_eq!(penalty_s(k, None, &recent, hz), 2);
        assert_eq!(penalty_s(k, None, &recent[..7], hz), 0);
        assert_eq!(hq_spawn_tick(k, t_pick, &un, 1.0, 2, hz), Tick(67 * 30));
        // warm-up x0.5: 2.5 s wait, picked at 63 s -> 63 s
        assert_eq!(hq_spawn_tick(k, t_pick, &un, 0.5, 0, hz), Tick(63 * 30));
    }
}
