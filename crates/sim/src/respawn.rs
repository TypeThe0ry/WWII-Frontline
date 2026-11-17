//! Respawn timing and queue (GAMEDESIGN 10.6, M0-013).
//!
//! `ready_time = t_kia + base_wait`; the spawn point is picked automatically at
//! `t_pick = t_kia + death_cam`; for wave-aligned (HQ) points
//! `spawn_time = ceil_to_next_wave_tick(max(ready_time, t_pick))`, waves being multiples of
//! `wave_interval` from match start (tick 0). All durations are converted to whole ticks with
//! `ceil`, so the result is exact for the tick rates in use (30 Hz: 5 s = 150 ticks).

use std::collections::BTreeSet;

use bevy_ecs::prelude::Resource;
use glam::Vec3;

use crate::components::Team;
use crate::data::defs::RespawnRulesDef;
use crate::ids::{PlayerId, Tick};

/// Respawn rule values (from `data/rules/respawn.json`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RespawnPolicy {
    pub death_cam_ms: u32,
    pub base_wait_ms: u32,
    pub wave_interval_ms: u32,
    pub wave_aligned: bool,
    /// Maximum horizontal offset around the spawn point, metres.
    pub spawn_jitter_m: f32,
}

impl RespawnPolicy {
    pub fn from_def(d: &RespawnRulesDef) -> Self {
        let ms = |v: f64| libm::round(v.max(0.0)) as u32;
        Self {
            death_cam_ms: ms(d.death_cam_ms),
            base_wait_ms: ms(d.base_wait_ms),
            wave_interval_ms: ms(d.wave_interval_ms).max(1),
            wave_aligned: d.wave_aligned,
            spawn_jitter_m: d.spawn_jitter_m.unwrap_or(0.0).clamp(0.0, 3.0) as f32,
        }
    }

    /// Tick at which the spawn point is picked after dying at `t_kia`.
    pub fn pick_tick(&self, t_kia: Tick, hz: u8) -> Tick {
        Tick(t_kia.0.wrapping_add(ms_to_ticks(self.death_cam_ms, hz)))
    }
}

/// `ceil(ms * hz / 1000)`.
pub fn ms_to_ticks(ms: u32, hz: u8) -> u32 {
    (u64::from(ms) * u64::from(hz)).div_ceil(1000) as u32
}

/// Spawn tick for a player killed at `t_kia` who picked a point at `t_pick` (HQ rules).
pub fn respawn_time(t_kia: Tick, t_pick: Tick, policy: &RespawnPolicy, hz: u8) -> Tick {
    let ready = t_kia.0.wrapping_add(ms_to_ticks(policy.base_wait_ms, hz));
    let start = ready.max(t_pick.0);
    if !policy.wave_aligned {
        return Tick(start);
    }
    let wave = ms_to_ticks(policy.wave_interval_ms, hz).max(1);
    Tick(start.div_ceil(wave).wrapping_mul(wave))
}

/// Players waiting to respawn, ordered by `(spawn tick, player)`.
#[derive(Resource, Debug, Default)]
pub struct RespawnQueue {
    set: BTreeSet<(u32, PlayerId)>,
}

impl RespawnQueue {
    pub fn schedule(&mut self, player: PlayerId, at: Tick) {
        self.set.insert((at.0, player));
    }

    /// Drops every entry of `player` (disconnect).
    pub fn remove_player(&mut self, player: PlayerId) {
        self.set.retain(|&(_, p)| p != player);
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    /// Removes and returns the players due at `now`, in `(tick, player)` order.
    pub fn pop_due(&mut self, now: Tick) -> Vec<PlayerId> {
        let mut out = Vec::new();
        while let Some(&(t, p)) = self.set.first() {
            if t > now.0 {
                break;
            }
            self.set.pop_first();
            out.push(p);
        }
        out
    }
}

/// Fixed spawn points per team (from `maps/*/spawns.json` HQ, set by the server) and the map
/// centre the respawned players face. Without a team point, a player respawns where it joined.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct SpawnPoints {
    pub pos: [Option<Vec3>; 2],
    /// Map centre `(x, z)`.
    pub center: Option<(f32, f32)>,
}

impl SpawnPoints {
    pub fn team(&self, team: Team) -> Option<Vec3> {
        self.pos[team as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(aligned: bool) -> RespawnPolicy {
        RespawnPolicy {
            death_cam_ms: 3000,
            base_wait_ms: 5000,
            wave_interval_ms: 15000,
            wave_aligned: aligned,
            spawn_jitter_m: 2.5,
        }
    }

    fn secs(s: f64) -> Tick {
        Tick((s * 30.0).round() as u32)
    }

    fn spawn_s(kia_s: f64, p: &RespawnPolicy) -> f64 {
        let k = secs(kia_s);
        f64::from(respawn_time(k, p.pick_tick(k, 30), p, 30).0) / 30.0
    }

    #[test]
    fn acceptance_examples() {
        let p = policy(true);
        assert_eq!(spawn_s(7.0, &p), 15.0);
        assert_eq!(spawn_s(1.0, &p), 15.0);
        assert_eq!(spawn_s(16.0, &p), 30.0);
        let q = policy(false);
        assert_eq!(spawn_s(7.0, &q), 12.0);
        assert_eq!(spawn_s(1.0, &q), 6.0);
    }

    #[test]
    fn wave_boundaries() {
        let p = policy(true);
        // Ready exactly on a wave tick spawns on it; one tick later waits a full wave.
        assert_eq!(spawn_s(10.0, &p), 15.0);
        let k = Tick(301); // ready = 451 ticks, one past the 450-tick wave
        assert_eq!(respawn_time(k, p.pick_tick(k, 30), &p, 30), Tick(900));
        let k = Tick(300);
        assert_eq!(respawn_time(k, p.pick_tick(k, 30), &p, 30), Tick(450));
        // Dying exactly on a wave tick.
        assert_eq!(spawn_s(15.0, &p), 30.0);
        // A long death cam dominates the base wait.
        let mut long = policy(true);
        long.death_cam_ms = 9000;
        assert_eq!(spawn_s(7.0, &long), 30.0);
        let mut no_wait = policy(false);
        no_wait.death_cam_ms = 0;
        no_wait.base_wait_ms = 0;
        assert_eq!(spawn_s(7.0, &no_wait), 7.0);
    }

    #[test]
    fn ms_to_ticks_rounds_up() {
        assert_eq!(ms_to_ticks(5000, 30), 150);
        assert_eq!(ms_to_ticks(1, 30), 1);
        assert_eq!(ms_to_ticks(0, 30), 0);
        assert_eq!(ms_to_ticks(34, 30), 2);
    }

    #[test]
    fn queue_order_and_cleanup() {
        let mut q = RespawnQueue::default();
        q.schedule(5, Tick(100));
        q.schedule(2, Tick(100));
        q.schedule(9, Tick(50));
        assert_eq!(q.pop_due(Tick(49)), Vec::<PlayerId>::new());
        assert_eq!(q.pop_due(Tick(100)), vec![9, 2, 5]);
        q.schedule(3, Tick(10));
        q.schedule(4, Tick(10));
        q.remove_player(3);
        assert_eq!(q.len(), 1);
        assert_eq!(q.pop_due(Tick(10)), vec![4]);
        assert!(q.is_empty());
    }
}
