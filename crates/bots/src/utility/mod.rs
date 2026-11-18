//! Utility goal selection (ARCHITECTURE 16.3, GAMEDESIGN 20.3): considerations mapped through
//! response curves and combined as a weighted geometric product, with hysteresis so equal goals
//! do not flip-flop.

use frontline_sim::PlayerId;

/// A high-level goal. `u16` ids are objective indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goal {
    CapturePoint(u16),
    Defend(u16),
    Engage(PlayerId),
    Revive(PlayerId),
    Resupply,
    Retreat,
    FollowLeader,
}

/// Maps a normalised input in `[0,1]` to a score in `[0,1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Curve {
    Linear,
    /// `x^2`
    Quadratic,
    /// `1 - x`
    Inverse,
    /// Logistic around `mid` with steepness `k`, normalised so 0 -> 0 and 1 -> 1.
    Logistic {
        k: f32,
        mid: f32,
    },
}

impl Curve {
    pub fn eval(self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        match self {
            Curve::Linear => x,
            Curve::Quadratic => x * x,
            Curve::Inverse => 1.0 - x,
            Curve::Logistic { k, mid } => {
                let f = |v: f32| 1.0 / (1.0 + libm::expf(-k * (v - mid)));
                let (lo, hi) = (f(0.0), f(1.0));
                ((f(x) - lo) / (hi - lo)).clamp(0.0, 1.0)
            }
        }
    }
}

/// One consideration: `curve(input)` raised to `weight`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Consideration {
    pub input: f32,
    pub curve: Curve,
    pub weight: f32,
}

/// `U = prod(curve_i(input_i) ^ w_i)`, scaled by `base` (the goal's designer priority).
/// A zero consideration vetoes the goal.
pub fn score(base: f32, cs: &[Consideration]) -> f32 {
    let mut u = base;
    for c in cs {
        let v = c.curve.eval(c.input);
        if v <= 0.0 {
            return 0.0;
        }
        u *= libm::powf(v, c.weight);
    }
    u
}

/// Picks the highest-scoring goal; the current goal is kept unless a rival beats it by `margin`.
#[derive(Debug, Clone)]
pub struct GoalSelector {
    current: Option<Goal>,
    margin: f32,
    pub switches: u32,
}

/// New goal must exceed the current score by 15 % (M1-038).
pub const HYSTERESIS: f32 = 1.15;

impl GoalSelector {
    pub fn new(margin: f32) -> Self {
        GoalSelector {
            current: None,
            margin,
            switches: 0,
        }
    }

    pub fn current(&self) -> Option<Goal> {
        self.current
    }

    /// `scored` is every candidate this evaluation. Returns the goal to pursue (`None` only when
    /// `scored` is empty or all scores are zero).
    pub fn select(&mut self, scored: &[(Goal, f32)]) -> Option<Goal> {
        let best = scored
            .iter()
            .filter(|(_, s)| *s > 0.0)
            .max_by(|a, b| a.1.total_cmp(&b.1))?;
        let cur_score = self
            .current
            .and_then(|g| scored.iter().find(|(x, _)| *x == g))
            .map_or(0.0, |(_, s)| *s);
        let keep = self.current.is_some() && cur_score > 0.0 && best.1 <= cur_score * self.margin;
        if !keep && self.current != Some(best.0) {
            if self.current.is_some() {
                self.switches += 1;
            }
            self.current = Some(best.0);
        }
        self.current
    }
}

/// Think-slot sharding (ARCHITECTURE 16.5): a bot thinks on ticks where `bot_id % 3 == tick % 3`.
pub fn thinks_on(bot_id: PlayerId, tick: u32, period: u32) -> bool {
    u32::from(bot_id) % period == tick % period
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontline_sim::rng::{unit_f32, Pcg64Mcg};

    #[test]
    fn curves_endpoints_and_shape() {
        assert_eq!(Curve::Linear.eval(0.3), 0.3);
        assert_eq!(Curve::Quadratic.eval(0.5), 0.25);
        assert_eq!(Curve::Inverse.eval(0.25), 0.75);
        let lg = Curve::Logistic { k: 10.0, mid: 0.5 };
        assert!(lg.eval(0.0).abs() < 1e-6 && (lg.eval(1.0) - 1.0).abs() < 1e-6);
        assert!((lg.eval(0.5) - 0.5).abs() < 1e-4);
        assert!(lg.eval(0.2) < lg.eval(0.6));
        assert_eq!(Curve::Linear.eval(7.0), 1.0, "input clamped");
    }

    #[test]
    fn product_and_veto() {
        let c = |i, w| Consideration {
            input: i,
            curve: Curve::Linear,
            weight: w,
        };
        assert!((score(2.0, &[c(0.5, 1.0), c(0.5, 2.0)]) - 0.25).abs() < 1e-6);
        assert_eq!(score(2.0, &[c(0.5, 1.0), c(0.0, 1.0)]), 0.0);
    }

    #[test]
    fn low_health_prefers_retreat() {
        let hp = |h: f32| {
            let engage = score(
                1.0,
                &[Consideration {
                    input: h,
                    curve: Curve::Linear,
                    weight: 1.0,
                }],
            );
            let retreat = score(
                1.0,
                &[Consideration {
                    input: 1.0 - h,
                    curve: Curve::Logistic { k: 12.0, mid: 0.4 },
                    weight: 1.0,
                }],
            );
            (engage, retreat)
        };
        let (e, r) = hp(0.9);
        assert!(e > r);
        let (e, r) = hp(0.1);
        assert!(r > e);
    }

    #[test]
    fn hysteresis_prevents_flip_flop() {
        // Two equal goals with +-5 % noise, 10 Hz for 30 s: at most 3 switches (M1-038 #6).
        let mut rng = Pcg64Mcg::new(7);
        let mut sel = GoalSelector::new(HYSTERESIS);
        for _ in 0..300 {
            let n = |r: &mut Pcg64Mcg| 1.0 + (unit_f32(r) - 0.5) * 0.1;
            let s = [
                (Goal::CapturePoint(1), 0.6 * n(&mut rng)),
                (Goal::Defend(2), 0.6 * n(&mut rng)),
            ];
            sel.select(&s);
        }
        assert!(sel.switches <= 3, "switches = {}", sel.switches);
    }

    #[test]
    fn clearly_better_goal_wins() {
        let mut sel = GoalSelector::new(HYSTERESIS);
        assert_eq!(
            sel.select(&[(Goal::Resupply, 0.5), (Goal::Retreat, 0.2)]),
            Some(Goal::Resupply)
        );
        // +10 % is inside the margin: keep. +30 % flips.
        assert_eq!(
            sel.select(&[(Goal::Resupply, 0.5), (Goal::Retreat, 0.55)]),
            Some(Goal::Resupply)
        );
        assert_eq!(
            sel.select(&[(Goal::Resupply, 0.5), (Goal::Retreat, 0.65)]),
            Some(Goal::Retreat)
        );
        assert_eq!(sel.switches, 1);
    }

    #[test]
    fn sharding_spreads_bots() {
        let mut thinking = [0u32; 3];
        for id in 0..30u8 {
            for t in 0..3u32 {
                if thinks_on(id, t, 3) {
                    thinking[t as usize] += 1;
                }
            }
        }
        assert_eq!(thinking, [10, 10, 10]);
        // Each bot thinks exactly once per 3 ticks.
        assert_eq!((0..3).filter(|&t| thinks_on(5, t, 3)).count(), 1);
    }
}
