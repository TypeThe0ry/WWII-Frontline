//! Squad-leader bot (M1-039): picks the squad's objective every few seconds and issues one
//! [`SquadOrder`]. Hysteresis comes from the same [`GoalSelector`] the soldiers use.

use frontline_sim::Vec3;

use crate::utility::{score, Consideration, Curve, Goal, GoalSelector, HYSTERESIS};

/// What the leader tells the squad (members' `BotCtx.squad_order`, wired in M1-038c).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SquadOrder {
    Move(Vec3),
    Attack(u16),
    Defend(u16),
    Hold,
}

/// One objective as the leader sees it.
#[derive(Debug, Clone, Copy)]
pub struct ObjView {
    pub id: u16,
    pub pos: Vec3,
    pub own: bool,
    /// Capture progress of the owning side, 0..=1 (only meaningful when `own`).
    pub held: f32,
    pub friends_near: u8,
    pub enemies_near: u8,
}

/// Re-evaluation period, ticks (3 s at 30 Hz; spec says every 2-5 s).
pub const REPICK_TICKS: u32 = 90;

pub struct SquadLeader {
    sel: GoalSelector,
    last_pick: Option<u32>,
    order: SquadOrder,
}

impl Default for SquadLeader {
    fn default() -> Self {
        SquadLeader {
            sel: GoalSelector::new(HYSTERESIS),
            last_pick: None,
            order: SquadOrder::Hold,
        }
    }
}

impl SquadLeader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn order(&self) -> SquadOrder {
        self.order
    }

    pub fn switches(&self) -> u32 {
        self.sel.switches
    }

    /// Call every tick; it only re-evaluates every [`REPICK_TICKS`].
    pub fn update(&mut self, tick: u32, from: Vec3, objs: &[ObjView]) -> SquadOrder {
        if self
            .last_pick
            .is_some_and(|t| tick.saturating_sub(t) < REPICK_TICKS)
        {
            return self.order;
        }
        self.last_pick = Some(tick);
        let lin = |x: f32, w: f32| Consideration {
            input: x,
            curve: Curve::Linear,
            weight: w,
        };
        let scored: Vec<(Goal, f32)> = objs
            .iter()
            .map(|o| {
                let d = (o.pos - from).length();
                let near = 1.0 - (d / 600.0).clamp(0.0, 0.9);
                let adv = f32::from(o.friends_near) + 1.0;
                let opp = f32::from(o.enemies_near) + 1.0;
                let ratio = (adv / (adv + opp)).max(0.1);
                if o.own {
                    // Defend only when it is under threat.
                    let threat = (f32::from(o.enemies_near) / 4.0).min(1.0);
                    (
                        Goal::Defend(o.id),
                        score(0.8, &[lin(threat, 1.0), lin(near, 1.0)]),
                    )
                } else {
                    (
                        Goal::CapturePoint(o.id),
                        score(0.7, &[lin(near, 1.0), lin(ratio, 0.7)]),
                    )
                }
            })
            .collect();
        self.order = match self.sel.select(&scored) {
            Some(Goal::CapturePoint(id)) => SquadOrder::Attack(id),
            Some(Goal::Defend(id)) => SquadOrder::Defend(id),
            _ => SquadOrder::Hold,
        };
        self.order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(id: u16, x: f32, own: bool, enemies: u8) -> ObjView {
        ObjView {
            id,
            pos: Vec3::new(x, 0.0, 0.0),
            own,
            held: 1.0,
            friends_near: 0,
            enemies_near: enemies,
        }
    }

    #[test]
    fn attacks_nearest_enemy_point_and_defends_threatened_own() {
        let mut l = SquadLeader::new();
        let o = l.update(
            0,
            Vec3::ZERO,
            &[obj(1, 100.0, false, 0), obj(2, 400.0, false, 0)],
        );
        assert_eq!(o, SquadOrder::Attack(1));
        let mut l = SquadLeader::new();
        let o = l.update(
            0,
            Vec3::ZERO,
            &[obj(1, 100.0, false, 0), obj(3, 60.0, true, 4)],
        );
        assert_eq!(o, SquadOrder::Defend(3));
        // Quiet own point is not worth defending.
        let mut l = SquadLeader::new();
        let o = l.update(
            0,
            Vec3::ZERO,
            &[obj(1, 100.0, false, 0), obj(3, 60.0, true, 0)],
        );
        assert_eq!(o, SquadOrder::Attack(1));
    }

    #[test]
    fn repicks_only_on_period_and_is_stable() {
        let mut l = SquadLeader::new();
        let objs = [obj(1, 100.0, false, 0), obj(2, 105.0, false, 0)];
        let first = l.update(0, Vec3::ZERO, &objs);
        for t in 1..REPICK_TICKS {
            assert_eq!(
                l.update(t, Vec3::ZERO, &[obj(2, 1.0, false, 0)]),
                first,
                "cached until period"
            );
        }
        for k in 1..20 {
            l.update(k * REPICK_TICKS, Vec3::ZERO, &objs);
        }
        assert!(l.switches() <= 1);
    }

    #[test]
    fn empty_objectives_hold() {
        assert_eq!(
            SquadLeader::new().update(0, Vec3::ZERO, &[]),
            SquadOrder::Hold
        );
    }
}
