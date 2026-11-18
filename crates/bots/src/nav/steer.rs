//! Path following: next waypoint -> movement intent, plus stuck detection (GAMEDESIGN B-21) and
//! the 2 Hz replanning cap (M1-037 behaviours 3-4).

use super::avoid;
use super::path::{Path, PathFinder, PathResult};
use super::{NavMesh, V2};

pub const ARRIVE_M: f32 = 0.6;
pub const REPLAN_COOLDOWN_S: f32 = 0.5;
pub const REUSE_GOAL_SHIFT_M: f32 = 3.0;
pub const STUCK_WINDOW_S: f32 = 5.0;
pub const STUCK_MIN_MOVE_M: f32 = 1.0;
/// After the first stuck replan, give up when the bot still moved < 0.5 m within this time.
pub const RECOVER_WINDOW_S: f32 = 2.5;
pub const RECOVER_MIN_MOVE_M: f32 = 0.5;
pub const WALK_AREAS: u64 = !0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stance {
    #[default]
    Stand,
    Crouch,
    Prone,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MoveIntent {
    pub dir: V2,
    pub yaw: f32,
    pub sprint: bool,
    pub stance: Stance,
    pub jump: bool,
    pub vault: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Following,
    Arrived,
    /// Stuck and replans did not help; the caller should pick another goal.
    Unreachable,
}

/// One bot's navigation state. `step` is called every tick; `find_path` runs at most every
/// `REPLAN_COOLDOWN_S` seconds.
#[derive(Default)]
pub struct Steering {
    finder: PathFinder,
    path: Option<Path>,
    idx: usize,
    goal: Option<V2>,
    since_plan: f32,
    anchor: V2,
    anchor_t: f32,
    stuck_count: u8,
    recover_jump: bool,
    anchor_valid: bool,
    pub plans: u32,
    pub status_unreachable: bool,
}

impl Steering {
    pub fn new() -> Self {
        Self {
            since_plan: 1e9,
            ..Self::default()
        }
    }

    pub fn set_goal(&mut self, goal: V2) {
        let shifted = self.goal.is_none_or(|g| {
            ((g[0] - goal[0]).powi(2) + (g[1] - goal[1]).powi(2)).sqrt() >= REUSE_GOAL_SHIFT_M
        });
        if shifted {
            self.goal = Some(goal);
            self.path = None;
            self.stuck_count = 0;
            self.recover_jump = false;
            self.anchor_t = 0.0;
            self.anchor_valid = false;
            self.status_unreachable = false;
        }
    }

    fn plan(&mut self, mesh: &NavMesh, pos: V2) {
        let Some(goal) = self.goal else { return };
        self.since_plan = 0.0;
        self.plans += 1;
        self.idx = 1;
        self.path = match self.finder.find(mesh, pos, goal, WALK_AREAS) {
            PathResult::Found(p) | PathResult::Partial(p) => Some(p),
            PathResult::NoPath => None,
        };
    }

    pub fn step(
        &mut self,
        mesh: &NavMesh,
        pos: V2,
        dt: f32,
        allies: &[V2],
    ) -> (Status, MoveIntent) {
        let Some(goal) = self.goal else {
            return (Status::Idle, MoveIntent::default());
        };
        if !self.anchor_valid {
            self.anchor = pos;
            self.anchor_t = 0.0;
            self.anchor_valid = true;
        }
        self.since_plan += dt;
        self.anchor_t += dt;
        if self.status_unreachable {
            return (Status::Unreachable, MoveIntent::default());
        }
        let window = if self.stuck_count == 0 {
            STUCK_WINDOW_S
        } else {
            RECOVER_WINDOW_S
        };
        if self.anchor_t >= window {
            let moved =
                ((pos[0] - self.anchor[0]).powi(2) + (pos[1] - self.anchor[1]).powi(2)).sqrt();
            let need = if self.stuck_count == 0 {
                STUCK_MIN_MOVE_M
            } else {
                RECOVER_MIN_MOVE_M
            };
            if moved < need {
                self.stuck_count += 1;
                self.path = None;
                self.recover_jump = true;
                if self.stuck_count >= 2 {
                    self.status_unreachable = true;
                    return (Status::Unreachable, MoveIntent::default());
                }
            } else {
                self.stuck_count = 0;
                self.recover_jump = false;
            }
            self.anchor = pos;
            self.anchor_t = 0.0;
        }
        if self.path.is_none() && self.since_plan >= REPLAN_COOLDOWN_S {
            self.plan(mesh, pos);
        }
        let Some(path) = &self.path else {
            return (Status::Following, MoveIntent::default());
        };
        while self.idx < path.points.len() {
            let w = path.points[self.idx];
            if ((w[0] - pos[0]).powi(2) + (w[1] - pos[1]).powi(2)).sqrt() < ARRIVE_M {
                self.idx += 1;
            } else {
                break;
            }
        }
        if self.idx >= path.points.len() {
            let d = ((goal[0] - pos[0]).powi(2) + (goal[1] - pos[1]).powi(2)).sqrt();
            return (
                if d < ARRIVE_M * 4.0 {
                    Status::Arrived
                } else {
                    Status::Following
                },
                MoveIntent::default(),
            );
        }
        let w = path.points[self.idx];
        let (mut dx, mut dz) = (w[0] - pos[0], w[1] - pos[1]);
        let l = (dx * dx + dz * dz).sqrt().max(1e-4);
        dx /= l;
        dz /= l;
        let sep = avoid::separation(pos, allies);
        let (mut mx, mut mz) = (dx + sep[0] * 0.8, dz + sep[1] * 0.8);
        let m = (mx * mx + mz * mz).sqrt().max(1e-4);
        mx /= m;
        mz /= m;
        let intent = MoveIntent {
            dir: [mx, mz],
            yaw: dx.atan2(dz),
            sprint: l > 6.0,
            stance: Stance::Stand,
            jump: self.recover_jump,
            vault: false,
        };
        (Status::Following, intent)
    }
}
