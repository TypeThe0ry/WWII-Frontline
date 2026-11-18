//! Soldier tactics on top of the engine (M1-038b): the goal scoring that feeds
//! [`GoalSelector`](crate::utility::GoalSelector), the threat score, and the combat behaviour tree.
//!
//! Everything here is pure: inputs are a [`Situation`] the brain assembles from the (no-wallhack)
//! perception, outputs are goal scores and leaf ids. Wiring into `SoldierBrain` is M1-039,
//! together with the squad-order interface, so the M0 golden replays stay untouched.

use frontline_sim::PlayerId;

use crate::bt::{Builder, Leaves, Status, Tree, TreeError};
use crate::utility::{score, Consideration, Curve, Goal};

/// What the bot knows this think step. Distances are metres; `None` = nothing perceived.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Situation {
    /// Own health 0..=1.
    pub hp: f32,
    /// Magazine plus reserve as a fraction of the full load, 0..=1.
    pub ammo: f32,
    pub enemy: Option<(PlayerId, f32)>,
    pub aimed_at: bool,
    pub downed_ally: Option<(PlayerId, f32)>,
    pub ammo_crate_m: Option<f32>,
    /// `(objective, distance, defending)`.
    pub objective: Option<(u16, f32, bool)>,
    pub leader_m: Option<f32>,
    /// Allies already heading for the same objective (crowding).
    pub crowd: u8,
}

fn near(d: f32, range: f32) -> f32 {
    1.0 - (d / range).clamp(0.0, 1.0)
}

/// Threat of one enemy: closer and aiming-at-me are worse (GAMEDESIGN 20.3 B-01).
pub fn threat(dist_m: f32, aimed_at_me: bool) -> f32 {
    let t = near(dist_m, 150.0);
    if aimed_at_me {
        (t + 0.35).min(1.0)
    } else {
        t
    }
}

/// Scores every applicable goal into `out` (cleared first). Scores are comparable across goals.
pub fn candidates(s: &Situation, out: &mut Vec<(Goal, f32)>) {
    out.clear();
    let lin = |x: f32| Consideration {
        input: x,
        curve: Curve::Linear,
        weight: 1.0,
    };
    let hurt = Consideration {
        input: 1.0 - s.hp,
        curve: Curve::Logistic { k: 10.0, mid: 0.6 },
        weight: 1.0,
    };
    if let Some((id, d)) = s.enemy {
        let th = threat(d, s.aimed_at);
        // Healthy and armed bots fight; the veto on ammo keeps an empty bot from engaging.
        out.push((
            Goal::Engage(id),
            score(0.9, &[lin(th), lin(s.hp.max(0.05)), lin(s.ammo)]),
        ));
    }
    if s.hp < 0.6 || (s.enemy.is_some() && s.ammo < 0.05) {
        let danger = s.enemy.map_or(0.3, |(_, d)| threat(d, s.aimed_at).max(0.3));
        out.push((Goal::Retreat, score(1.0, &[hurt, lin(danger)])));
    }
    if let Some((id, d)) = s.downed_ally {
        // B-08: medics revive; nearer is better, and never under direct aim.
        let safe = if s.aimed_at { 0.2 } else { 1.0 };
        out.push((
            Goal::Revive(id),
            score(0.8, &[lin(near(d, 40.0)), lin(safe)]),
        ));
    }
    if let Some(d) = s.ammo_crate_m {
        out.push((
            Goal::Resupply,
            score(
                0.9,
                &[
                    Consideration {
                        input: 1.0 - s.ammo,
                        curve: Curve::Quadratic,
                        weight: 1.0,
                    },
                    lin(near(d, 120.0)),
                ],
            ),
        ));
    }
    if let Some((obj, d, defending)) = s.objective {
        let crowd = 1.0 - (f32::from(s.crowd) / 6.0).min(1.0);
        let base = score(0.5, &[lin(near(d, 400.0).max(0.1)), lin(crowd.max(0.1))]);
        out.push((
            if defending {
                Goal::Defend(obj)
            } else {
                Goal::CapturePoint(obj)
            },
            base,
        ));
    }
    if let Some(d) = s.leader_m {
        out.push((
            Goal::FollowLeader,
            score(0.35, &[lin(near(d, 60.0).max(0.3))]),
        ));
    }
}

// Leaf ids of the combat tree.
pub mod leaf {
    pub const MAG_EMPTY: u16 = 0;
    pub const HAS_TARGET: u16 = 1;
    pub const HAS_LOS: u16 = 2;
    pub const GRENADE_OK: u16 = 3;
    pub const IN_COVER: u16 = 4;
    pub const DOWNED_NEARBY: u16 = 5;
    pub const RELOAD: u16 = 0;
    pub const THROW_GRENADE: u16 = 1;
    pub const SHOOT: u16 = 2;
    pub const TAKE_COVER: u16 = 3;
    pub const MOVE_TO_GOAL: u16 = 4;
    pub const REVIVE: u16 = 5;
}

/// Grenade cooldown, ticks (10 s at 30 Hz).
pub const GRENADE_COOLDOWN_TICKS: u32 = 300;

/// The soldier combat tree: reload first, then (with a target) grenade / shoot / seek cover,
/// else revive a nearby downed ally, else move toward the current goal.
pub fn combat_tree() -> Result<Tree, TreeError> {
    use leaf::*;
    let mut b = Builder::new();
    let empty = b.condition(MAG_EMPTY);
    let reload = b.action(RELOAD);
    let reload_seq = b.sequence(&[empty, reload]);

    let ok = b.condition(GRENADE_OK);
    let throw = b.action(THROW_GRENADE);
    let throw_cd = b.cooldown(throw, GRENADE_COOLDOWN_TICKS);
    let grenade_seq = b.sequence(&[ok, throw_cd]);

    let los = b.condition(HAS_LOS);
    let shoot = b.action(SHOOT);
    let shoot_seq = b.sequence(&[los, shoot]);
    let cover = b.action(TAKE_COVER);
    let fight = b.selector(&[grenade_seq, shoot_seq, cover]);
    let has = b.condition(HAS_TARGET);
    let engage = b.sequence(&[has, fight]);

    let down = b.condition(DOWNED_NEARBY);
    let revive = b.action(REVIVE);
    let revive_seq = b.sequence(&[down, revive]);

    let go = b.action(MOVE_TO_GOAL);
    let root = b.selector(&[reload_seq, engage, revive_seq, go]);
    b.finish(root)
}

/// Convenience for tests and the future brain: ticks `tree` once.
pub fn run(tree: &Tree, bb: &mut crate::bt::Blackboard, l: &mut dyn Leaves, tick: u32) -> Status {
    tree.tick(bb, l, tick)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bt::{Blackboard, MAX_NODES};
    use crate::utility::{GoalSelector, HYSTERESIS};

    fn best(s: &Situation) -> Option<Goal> {
        let mut v = Vec::new();
        candidates(s, &mut v);
        GoalSelector::new(HYSTERESIS).select(&v)
    }

    fn healthy() -> Situation {
        Situation {
            hp: 1.0,
            ammo: 1.0,
            ..Default::default()
        }
    }

    #[test]
    fn tree_within_node_cap() {
        let t = combat_tree().unwrap();
        assert!(t.len() <= MAX_NODES, "{}", t.len());
    }

    #[test]
    fn b01_engage_nearest_threat_when_healthy() {
        let s = Situation {
            enemy: Some((9, 40.0)),
            objective: Some((1, 100.0, false)),
            ..healthy()
        };
        assert_eq!(best(&s), Some(Goal::Engage(9)));
    }

    #[test]
    fn b02_capture_when_quiet() {
        let s = Situation {
            objective: Some((3, 80.0, false)),
            leader_m: Some(30.0),
            ..healthy()
        };
        assert_eq!(best(&s), Some(Goal::CapturePoint(3)));
        let d = Situation {
            objective: Some((3, 80.0, true)),
            ..healthy()
        };
        assert_eq!(best(&d), Some(Goal::Defend(3)));
    }

    #[test]
    fn b03_retreat_when_badly_hurt_under_threat() {
        let s = Situation {
            hp: 0.15,
            enemy: Some((9, 30.0)),
            aimed_at: true,
            ..healthy()
        };
        assert_eq!(best(&s), Some(Goal::Retreat));
    }

    #[test]
    fn b04_resupply_when_dry_near_crate() {
        let s = Situation {
            ammo: 0.05,
            ammo_crate_m: Some(20.0),
            objective: Some((1, 200.0, false)),
            ..healthy()
        };
        assert_eq!(best(&s), Some(Goal::Resupply));
    }

    #[test]
    fn b08_revive_nearby_downed_ally_when_safe() {
        let s = Situation {
            downed_ally: Some((4, 8.0)),
            objective: Some((1, 150.0, false)),
            ..healthy()
        };
        assert_eq!(best(&s), Some(Goal::Revive(4)));
        let aimed = Situation {
            aimed_at: true,
            enemy: Some((9, 25.0)),
            ..s
        };
        assert!(
            matches!(best(&aimed), Some(Goal::Engage(_))),
            "no revive under aim"
        );
    }

    #[test]
    fn b17_crowding_pushes_bots_elsewhere() {
        let lone = Situation {
            objective: Some((1, 100.0, false)),
            leader_m: Some(10.0),
            crowd: 0,
            ..healthy()
        };
        let crowded = Situation { crowd: 6, ..lone };
        assert_eq!(best(&lone), Some(Goal::CapturePoint(1)));
        assert_eq!(best(&crowded), Some(Goal::FollowLeader));
    }

    #[test]
    fn threat_orders_by_distance_and_aim() {
        assert!(threat(20.0, false) > threat(100.0, false));
        assert!(threat(100.0, true) > threat(100.0, false));
    }

    struct Stub {
        conds: [bool; 6],
        log: Vec<u16>,
    }
    impl Leaves for Stub {
        fn condition(&mut self, id: u16) -> bool {
            self.conds[id as usize]
        }
        fn action(&mut self, id: u16) -> Status {
            self.log.push(id);
            Status::Success
        }
    }
    fn act(conds: [bool; 6], tick: u32, bb: &mut Blackboard, t: &Tree) -> Vec<u16> {
        let mut s = Stub { conds, log: vec![] };
        run(t, bb, &mut s, tick);
        s.log
    }

    #[test]
    fn tree_priorities_and_grenade_cooldown() {
        use leaf::*;
        let t = combat_tree().unwrap();
        let mut bb = Blackboard::for_tree(&t);
        // [mag_empty, target, los, grenade_ok, in_cover, downed]
        assert_eq!(
            act([true, true, true, false, false, false], 0, &mut bb, &t),
            [RELOAD]
        );
        assert_eq!(
            act([false, true, true, false, false, false], 1, &mut bb, &t),
            [SHOOT]
        );
        assert_eq!(
            act([false, true, false, false, false, false], 2, &mut bb, &t),
            [TAKE_COVER]
        );
        assert_eq!(
            act([false, false, false, false, false, true], 3, &mut bb, &t),
            [REVIVE]
        );
        assert_eq!(
            act([false, false, false, false, false, false], 4, &mut bb, &t),
            [MOVE_TO_GOAL]
        );
        // Grenade first, then the cooldown makes it shoot instead.
        let g = [false, true, true, true, false, false];
        assert_eq!(act(g, 10, &mut bb, &t), [THROW_GRENADE]);
        assert_eq!(act(g, 11, &mut bb, &t), [SHOOT]);
        assert_eq!(
            act(g, 10 + GRENADE_COOLDOWN_TICKS, &mut bb, &t),
            [THROW_GRENADE]
        );
    }

    /// B-05 / acceptance 4: with no line of sight the tree never reaches the shoot leaf.
    #[test]
    fn no_los_never_shoots() {
        use leaf::*;
        let t = combat_tree().unwrap();
        let mut bb = Blackboard::for_tree(&t);
        for tick in 0..1000 {
            let log = act([false, true, false, false, false, false], tick, &mut bb, &t);
            assert!(!log.contains(&SHOOT));
        }
    }
}
