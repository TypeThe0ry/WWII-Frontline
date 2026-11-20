//! Recent-damage record per victim for kill attribution (GAMEDESIGN 7.5, input of M1-016):
//! who hurt the victim in the configured assist window, and for how much.

use bevy_ecs::prelude::Component;

use crate::ids::{EntityId, Tick};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageEntry {
    pub attacker: EntityId,
    pub amount: f32,
    pub tick: Tick,
}

#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct RecentDamage {
    entries: Vec<DamageEntry>,
}

impl RecentDamage {
    /// Records a hit and drops entries older than the window.
    pub fn record(&mut self, attacker: EntityId, amount: f32, tick: Tick, window: u32) {
        self.prune(tick, window);
        self.entries.push(DamageEntry {
            attacker,
            amount,
            tick,
        });
    }

    pub fn prune(&mut self, now: Tick, window: u32) {
        self.entries
            .retain(|e| now.0.wrapping_sub(e.tick.0) <= window);
    }

    pub fn last_hit(&self) -> Option<&DamageEntry> {
        self.entries.last()
    }

    /// Total damage `attacker` dealt inside the window.
    pub fn total_from(&self, attacker: EntityId) -> f32 {
        self.entries
            .iter()
            .filter(|e| e.attacker == attacker)
            .map(|e| e.amount)
            .sum()
    }

    /// Distinct attackers in first-hit order with their totals (assist candidates).
    pub fn attackers_within(&self, now: Tick, window: u32) -> Vec<(EntityId, f32)> {
        let mut out: Vec<(EntityId, f32)> = Vec::new();
        for e in &self.entries {
            if now.0.wrapping_sub(e.tick.0) > window {
                continue;
            }
            match out.iter_mut().find(|(a, _)| *a == e.attacker) {
                Some((_, t)) => *t += e.amount,
                None => out.push((e.attacker, e.amount)),
            }
        }
        out
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
