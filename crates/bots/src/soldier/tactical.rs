//! M1 decision step. Only visual contacts carry an enemy identity or authorize a shot.

use frontline_sim::medical::REVIVE_RANGE_M;
use frontline_sim::supply::ammo::RESUPPLY_RANGE_M;
use frontline_sim::view::ObjectiveView;

use super::*;
use crate::bt::{Blackboard, Leaves, Status, Tree};
use crate::perception::{in_view_cone, recognition_probability, view_distance};
use crate::squad_leader::SquadOrder;
use crate::tactics::{self, leaf, Situation};
use crate::utility::{Goal, GoalSelector, HYSTERESIS};

fn xz(pos: Vec3) -> [f32; 2] {
    [pos.x, pos.z]
}

pub(super) fn see(
    view: &dyn WorldView,
    me: &PlayerView,
    eye: Vec3,
    pos: Vec3,
    tuning: &Tuning,
    difficulty: &crate::Difficulty,
    rays: &mut u8,
) -> bool {
    if *rays == 0
        || eye.distance(pos + Vec3::Y * TORSO_Y) > view_distance(tuning, difficulty)
        || !in_view_cone(eye, me.yaw, pos, tuning.fov_deg)
    {
        return false;
    }
    *rays -= 1;
    view.los(eye, pos + Vec3::Y * TORSO_Y)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn scan(
    view: &dyn WorldView,
    me: &PlayerView,
    eye: Vec3,
    tuning: &Tuning,
    diff: &crate::Difficulty,
    rng: &mut Pcg64Mcg,
    out: &mut Vec<Contact>,
    rays: &mut u8,
) {
    let range = view_distance(tuning, diff);
    out.clear();
    out.extend(view.players().iter().filter_map(|p| {
        let dist = eye.distance(p.pos + Vec3::Y * TORSO_Y);
        (p.team != me.team && dist <= range && in_view_cone(eye, me.yaw, p.pos, tuning.fov_deg))
            .then_some(Contact {
                player: p.player,
                pos: p.pos,
                dist,
            })
    }));
    out.sort_by(|a, b| a.dist.total_cmp(&b.dist).then(a.player.cmp(&b.player)));
    out.retain(|c| {
        see(view, me, eye, c.pos, tuning, diff, rays)
            && unit_f32(rng) < recognition_probability(c.dist, range, tuning.min_recognition)
    });
}

pub(super) struct Tactical {
    tree: Tree,
    bb: Blackboard,
    goals: GoalSelector,
    scores: Vec<(Goal, f32)>,
    heard: Option<(u32, Vec3)>,
}

impl Tactical {
    pub(super) fn new() -> Self {
        let tree = tactics::combat_tree().expect("built-in combat tree");
        let bb = Blackboard::for_tree(&tree);
        Self {
            tree,
            bb,
            goals: GoalSelector::new(HYSTERESIS),
            scores: Vec::with_capacity(8),
            heard: None,
        }
    }

    pub(super) fn reset(&mut self) {
        self.bb.reset();
        self.goals = GoalSelector::new(HYSTERESIS);
        self.heard = None;
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn decide(
        &mut self,
        brain: &mut SoldierBrain,
        view: &dyn WorldView,
        ctx: &BotCtx,
        me: &PlayerView,
        eye: Vec3,
        weapon: Option<WeaponInfo>,
        visible: bool,
        rng: &mut Pcg64Mcg,
    ) {
        let tick = ctx.tick;
        let memory = (ctx.difficulty.memory_s * TICK_HZ) as u32;
        if let Some(sound) = ctx.hearing.iter().rev().find(|s| {
            tick.saturating_sub(s.tick) <= memory
                && (4.0..=s.range_m).contains(&me.pos.distance(s.pos))
        }) {
            self.heard = Some((sound.tick, sound.pos));
        }
        self.heard = self
            .heard
            .filter(|(t, _)| tick.saturating_sub(*t) <= memory);

        let objective = select_objective(view, ctx, me);
        let leader = ctx.squad_leader.and_then(|id| {
            view.players()
                .iter()
                .find(|p| p.player == id && id != me.player && p.team == me.team)
        });
        let follower = leader.map(|p| {
            let side = if ctx.bot_id.is_multiple_of(2) {
                -1.0
            } else {
                1.0
            };
            let lane = f32::from(ctx.bot_id % 3 + 1);
            let forward = Vec3::new(libm::sinf(p.yaw), 0.0, libm::cosf(p.yaw));
            p.pos - 3.0 * lane * forward + side * 3.0 * lane * Vec3::new(forward.z, 0.0, -forward.x)
        });
        let target = brain.target;
        let enemy = target.map(|g| (g.player, me.pos.distance(g.last_pos)));
        let aimed_at = visible
            && target.is_some_and(|g| {
                view.players()
                    .iter()
                    .find(|p| p.player == g.player)
                    .is_some_and(|p| in_view_cone(p.pos, p.yaw, me.pos, 18.0))
            });
        let casualty = ctx.survival.downed_ally;
        let crate_pos = ctx.survival.ammo_crate;
        let order = ctx.squad_order;
        let cap = weapon.map_or(1, |w| w.mag_capacity.max(1));
        let ammo = ctx
            .self_weapon
            .as_ref()
            .map_or(0.0, |w| {
                (f32::from(w.mag) + f32::from(w.reserve)) / (f32::from(cap) * 6.0)
            })
            .clamp(0.0, 1.0);
        let hp = view.health(me.entity).unwrap_or(100.0) / 100.0;
        let crowd = objective.map_or(0, |o| {
            view.players()
                .iter()
                .filter(|p| {
                    p.team == me.team
                        && p.player != me.player
                        && p.pos.distance(o.pos) <= o.radius_m
                })
                .count()
                .min(255) as u8
        });
        tactics::candidates(
            &Situation {
                hp,
                ammo,
                enemy,
                aimed_at,
                downed_ally: casualty.map(|(id, pos)| (id, me.pos.distance(pos))),
                ammo_crate_m: crate_pos.map(|p| me.pos.distance(p)),
                objective: objective.map(|o| {
                    let defending = matches!(order, Some(SquadOrder::Defend(id)) if id == o.id)
                        || o.owner == Some(me.team);
                    (o.id, me.pos.distance(o.pos), defending)
                }),
                leader_m: follower
                    .filter(|_| order.is_none())
                    .map(|p| me.pos.distance(p)),
                crowd,
            },
            &mut self.scores,
        );
        let goal = self.goals.select(&self.scores);
        let mut dec = Decision {
            jump: brain.dec.jump,
            ..Decision::IDLE
        };
        brain.mode = if matches!(goal, Some(Goal::Engage(_))) {
            Mode::Engage
        } else {
            Mode::Patrol
        };
        let reload = ctx.self_weapon.as_ref().is_some_and(|w| {
            (w.mag == 0 || (!visible && w.mag <= 1 && w.mag < cap))
                && w.reserve > 0
                && !matches!(w.fsm, Fsm::Reloading(_))
        });
        let mut leaves = DecisionLeaves {
            brain,
            view,
            ctx,
            me,
            eye,
            weapon,
            visible,
            rng,
            goal,
            objective,
            follower,
            heard: self.heard.map(|(_, pos)| pos),
            reload,
            aimed_at,
            dec: &mut dec,
        };
        // Recheck priority conditions on every decision; keep grenade cooldown deadlines.
        self.bb.reset();
        tactics::run(&self.tree, &mut self.bb, &mut leaves, tick);
        brain.dec = dec;
    }
}

fn select_objective<'a>(
    view: &'a dyn WorldView,
    ctx: &BotCtx,
    me: &PlayerView,
) -> Option<&'a ObjectiveView> {
    match ctx.squad_order {
        Some(SquadOrder::Attack(id) | SquadOrder::Defend(id)) => {
            view.objectives().iter().find(|o| o.id == id)
        }
        Some(SquadOrder::Hold | SquadOrder::Move(_)) => None,
        None => view
            .objectives()
            .iter()
            .filter(|o| o.owner != Some(me.team) || o.contested)
            .min_by(|a, b| {
                a.pos
                    .distance_squared(me.pos)
                    .total_cmp(&b.pos.distance_squared(me.pos))
            }),
    }
}

struct DecisionLeaves<'a, 'b> {
    brain: &'a mut SoldierBrain,
    view: &'a dyn WorldView,
    ctx: &'a BotCtx<'b>,
    me: &'a PlayerView,
    eye: Vec3,
    weapon: Option<WeaponInfo>,
    visible: bool,
    rng: &'a mut Pcg64Mcg,
    goal: Option<Goal>,
    objective: Option<&'a ObjectiveView>,
    follower: Option<Vec3>,
    heard: Option<Vec3>,
    reload: bool,
    aimed_at: bool,
    dec: &'a mut Decision,
}

impl Leaves for DecisionLeaves<'_, '_> {
    fn condition(&mut self, id: u16) -> bool {
        match id {
            leaf::MAG_EMPTY => self.reload,
            leaf::HAS_TARGET => matches!(self.goal, Some(Goal::Engage(_))),
            leaf::HAS_LOS => self.visible,
            leaf::GRENADE_OK => {
                !self.visible
                    && self.ctx.survival.frag > 0
                    && self.brain.target.is_some_and(|g| {
                        let d = self.me.pos.distance(g.last_pos);
                        (12.0..=25.0).contains(&d)
                            && self.ctx.tick >= g.fire_not_before
                            && self.ctx.tick.saturating_sub(g.last_seen) <= 30
                            && !self.view.players().iter().any(|p| {
                                p.team == self.me.team && p.pos.distance(g.last_pos) < 12.0
                            })
                    })
            }
            leaf::DOWNED_NEARBY => matches!(self.goal, Some(Goal::Revive(_))),
            _ => false,
        }
    }

    fn action(&mut self, id: u16) -> Status {
        match id {
            leaf::RELOAD => self.dec.reload = true,
            leaf::SHOOT => {
                if let Some(g) = self.brain.target {
                    self.brain.engage(
                        self.ctx,
                        self.me,
                        self.eye,
                        g,
                        self.weapon,
                        self.rng,
                        self.dec,
                        true,
                    );
                }
            }
            leaf::THROW_GRENADE => {
                if let Some(g) = self.brain.target {
                    if !self.throw(g.last_pos, false) {
                        return Status::Running;
                    }
                }
            }
            leaf::TAKE_COVER => {
                if let Some(g) = self.brain.target {
                    let here = xz(self.me.pos);
                    if let Some(cover) = self
                        .brain
                        .map
                        .cover
                        .iter()
                        .copied()
                        .filter(|p| dist2d(*p, here) <= 20.0)
                        .min_by(|a, b| dist2d(*a, here).total_cmp(&dist2d(*b, here)))
                    {
                        self.move_to(cover, 1.5, false);
                        self.dec.crouch = self.dec.mv == Mv::Stop;
                        if self.dec.crouch {
                            self.brain.want_yaw = angles_to(self.eye, g.last_pos).0;
                        }
                    } else {
                        self.brain.chase(self.me, g, self.ctx.tick, self.dec);
                    }
                }
            }
            leaf::REVIVE => {
                if let Some((_, pos)) = self.ctx.survival.downed_ally {
                    let close = self.me.pos.distance(pos) <= REVIVE_RANGE_M;
                    if self.ctx.survival.smoke > 0
                        && self.me.pos.distance(pos) <= 15.0
                        && self.brain.target.is_some()
                    {
                        self.throw(pos, true);
                    } else if close {
                        self.dec.interact = true;
                    } else if !self.aimed_at {
                        self.move_to(xz(pos), 0.6, false);
                    }
                }
            }
            leaf::MOVE_TO_GOAL => self.move_to_goal(),
            _ => return Status::Failure,
        }
        Status::Success
    }
}

impl DecisionLeaves<'_, '_> {
    fn move_to(&mut self, pos: [f32; 2], radius: f32, sprint: bool) {
        let here = xz(self.me.pos);
        if dist2d(here, pos) > radius {
            if let Some(dir) = self.brain.steer(self.me, pos, self.ctx.tick) {
                self.dec.mv = Mv::Go { dir, sprint };
                self.brain.want_yaw = libm::atan2f(dir[0], dir[1]);
                self.brain.want_pitch = 0.0;
            }
        }
    }

    fn throw(&mut self, pos: Vec3, smoke: bool) -> bool {
        let (yaw, _) = angles_to(self.eye, pos);
        self.brain.want_yaw = yaw;
        self.brain.want_pitch = if smoke { -0.2 } else { 0.1 };
        self.dec.aim = smoke;
        self.dec.gadget = wrap_pi(yaw - self.brain.yaw).abs() < 0.15;
        self.dec.gadget
    }

    fn move_to_goal(&mut self) {
        let here = xz(self.me.pos);
        match self.goal {
            Some(Goal::CapturePoint(_) | Goal::Defend(_)) => {
                if let Some(o) = self.objective {
                    let radius = 4.0_f32.min(o.radius_m * 0.45);
                    let phi = f32::from(self.ctx.bot_id) * 2.399_963_1;
                    let spread = [
                        o.pos.x + libm::sinf(phi) * radius,
                        o.pos.z + libm::cosf(phi) * radius,
                    ];
                    let defend = matches!(self.goal, Some(Goal::Defend(_)));
                    let cover = defend
                        .then(|| {
                            self.brain
                                .map
                                .cover
                                .iter()
                                .copied()
                                .filter(|p| dist2d(*p, xz(o.pos)) <= o.radius_m)
                                .min_by(|a, b| dist2d(*a, here).total_cmp(&dist2d(*b, here)))
                        })
                        .flatten();
                    self.move_to(cover.unwrap_or(spread), 1.5, !defend);
                    if defend && self.dec.mv == Mv::Stop {
                        self.dec.crouch = true;
                        if let Some(pos) = self.heard {
                            self.brain.want_yaw = angles_to(self.eye, pos).0;
                        }
                    }
                }
            }
            Some(Goal::Retreat) => {
                if let Some(g) = self.brain.target {
                    if let Some(away) = unit_dir(xz(g.last_pos), here) {
                        self.move_to(
                            [here[0] + 12.0 * away[0], here[1] + 12.0 * away[1]],
                            1.5,
                            true,
                        );
                    }
                } else {
                    self.patrol();
                }
            }
            Some(Goal::Resupply) => {
                if let Some(pos) = self.ctx.survival.ammo_crate {
                    if dist2d(here, xz(pos)) <= RESUPPLY_RANGE_M {
                        self.dec.interact = true;
                    } else {
                        self.move_to(xz(pos), 2.0, true);
                    }
                }
            }
            Some(Goal::FollowLeader) if self.ctx.squad_order.is_none() => {
                if let Some(pos) = self.follower {
                    self.move_to(xz(pos), 2.0, true);
                }
            }
            _ => match self.ctx.squad_order {
                Some(SquadOrder::Move(pos)) => self.move_to(xz(pos), 2.0, true),
                Some(SquadOrder::Hold) => self.dec.crouch = true,
                _ => {
                    if let Some(pos) = self.heard.filter(|p| dist2d(here, xz(*p)) > 4.0) {
                        self.move_to(xz(pos), 4.0, false);
                    } else {
                        self.patrol();
                    }
                }
            },
        }
    }

    fn patrol(&mut self) {
        self.brain
            .patrol(self.me, self.ctx.tick, self.ctx.bot_id, self.rng, self.dec);
    }
}
