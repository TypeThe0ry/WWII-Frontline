//! Smoke cloud state machine and the line-of-sight test (GAMEDESIGN 6.9.2). The cloud is a sphere
//! (ground truncation is not modelled, PI-054) whose radius grows linearly over `expand_s`, holds,
//! and shrinks linearly over the last `decay_s`; concentration is 1 until the decay starts, then
//! falls linearly with the radius. A cloud blocks sight while its concentration exceeds 0.5.

use glam::Vec3;

use crate::ids::Tick;

/// Concentration above which a cloud blocks line of sight.
pub const BLOCK_CONCENTRATION: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmokeParams {
    pub radius_m: f32,
    pub expand_ticks: u32,
    pub duration_ticks: u32,
    pub decay_ticks: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmokeCloud {
    pub pos: Vec3,
    pub start: Tick,
    pub params: SmokeParams,
}

impl SmokeCloud {
    /// Ticks since the cloud began; `None` before it started.
    fn age(&self, now: Tick) -> Option<u32> {
        now.0.checked_sub(self.start.0)
    }

    pub fn ended(&self, now: Tick) -> bool {
        self.age(now)
            .is_some_and(|a| a >= self.params.duration_ticks)
    }

    /// Concentration in `0..=1`.
    pub fn concentration(&self, now: Tick) -> f32 {
        let Some(age) = self.age(now) else {
            return 0.0;
        };
        let p = &self.params;
        if age >= p.duration_ticks {
            return 0.0;
        }
        let decay_start = p.duration_ticks.saturating_sub(p.decay_ticks);
        if age >= decay_start && p.decay_ticks > 0 {
            (p.duration_ticks - age) as f32 / p.decay_ticks as f32
        } else {
            1.0
        }
    }

    /// Sphere radius in metres.
    pub fn radius(&self, now: Tick) -> f32 {
        let Some(age) = self.age(now) else {
            return 0.0;
        };
        let p = &self.params;
        let grow = if p.expand_ticks == 0 {
            1.0
        } else {
            (age as f32 / p.expand_ticks as f32).min(1.0)
        };
        p.radius_m * grow * self.concentration(now)
    }

    /// Whether the segment `a..b` passes through the opaque part of the cloud.
    pub fn blocks(&self, now: Tick, a: Vec3, b: Vec3) -> bool {
        if self.concentration(now) <= BLOCK_CONCENTRATION {
            return false;
        }
        let r = self.radius(now);
        let d = b - a;
        let len2 = d.length_squared();
        let t = if len2 < 1e-12 {
            0.0
        } else {
            ((self.pos - a).dot(d) / len2).clamp(0.0, 1.0)
        };
        (a + d * t - self.pos).length_squared() <= r * r
    }
}

/// Whether any cloud blocks the segment `a..b`.
pub fn smoke_blocks_los(clouds: &[SmokeCloud], now: Tick, a: Vec3, b: Vec3) -> bool {
    clouds.iter().any(|c| c.blocks(now, a, b))
}
