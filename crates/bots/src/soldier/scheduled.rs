//! Transaction boundary for one cooperative decision, and handoff to shared navigation.

use super::*;
use crate::path_budget::PathBudget;

type Leg = ([f32; 2], [f32; 2]);

#[derive(Default, Clone, Copy)]
pub(super) struct QueuedNav {
    pub invalid: bool,
    pending: Option<[f32; 2]>,
    request: Option<Leg>,
    reached: [Option<usize>; 2],
}

impl QueuedNav {
    pub(super) fn invalidate(&mut self) {
        self.invalid = true;
        self.pending = None;
        self.request = None;
    }

    pub(super) fn request(&mut self, here: [f32; 2], goal: [f32; 2]) {
        self.request = Some((here, goal));
    }

    pub(super) fn record_reached(&mut self, index: usize) {
        if let Some(entry) = self.reached.iter_mut().find(|p| p.is_none()) {
            *entry = Some(index);
        }
    }

    pub(super) fn waiting(&self) -> bool {
        self.pending.is_some() || self.request.is_some()
    }
}

impl SoldierBrain {
    pub(super) fn take_path(&mut self, paths: &mut PathBudget, id: PlayerId, tick: u32) {
        let q = self.queued.as_mut().expect("scheduled navigation");
        if let Some(result) = paths.take(id) {
            if q.pending
                .is_some_and(|goal| dist2d(goal, result.goal) <= GOAL_MOVED_M)
            {
                q.pending = None;
                q.invalid = false;
                self.path = result.path;
                self.path_goal = result.goal;
                self.path_failed = result.failed;
                self.path_i = 0;
                self.path_tick = tick;
            }
        }
    }

    pub(super) fn commit_path(&mut self, paths: &mut PathBudget, id: PlayerId) {
        let q = self.queued.as_mut().expect("scheduled navigation");
        if let Some((here, goal)) = q.request.take() {
            // Keep the anchor current while a request waits, without resetting its queue age.
            paths.request(id, self.map.clone(), here, goal);
            q.pending = Some(goal);
        } else if q.pending.is_some() || q.invalid {
            paths.forget(id);
            q.pending = None;
        }
        for idx in std::mem::take(&mut q.reached).into_iter().flatten() {
            self.patrol_reached[idx] += 1;
        }
    }
}

// Scheduled decisions write no path vectors or queue entries until commit. Preserve only
// the mutable tactical state, leaving allocations and timing counters outside the rollback.
pub(super) struct DecisionCheckpoint {
    wpn: Option<(u16, WeaponInfo)>,
    want_yaw: f32,
    want_pitch: f32,
    mode: Mode,
    dec: Decision,
    patrol_goal: Option<usize>,
    patrol_last: Option<usize>,
    patrol_prev: Option<usize>,
    path_i: usize,
    path_tick: u32,
    path_failed: bool,
    target: Option<TargetMem>,
    strafe_until: u32,
    strafe_dir: i8,
    stuck_anchor: [f32; 2],
    stuck_tick: u32,
    stuck_total: u32,
    stuck_events: u32,
    tactics: Option<Tactical>,
    queued: Option<QueuedNav>,
    recognition: Option<(PlayerId, u32)>,
}

impl DecisionCheckpoint {
    pub(super) fn save(b: &SoldierBrain) -> Self {
        Self {
            wpn: b.wpn,
            want_yaw: b.want_yaw,
            want_pitch: b.want_pitch,
            mode: b.mode,
            dec: b.dec,
            patrol_goal: b.patrol_goal,
            patrol_last: b.patrol_last,
            patrol_prev: b.patrol_prev,
            path_i: b.path_i,
            path_tick: b.path_tick,
            path_failed: b.path_failed,
            target: b.target,
            strafe_until: b.strafe_until,
            strafe_dir: b.strafe_dir,
            stuck_anchor: b.stuck_anchor,
            stuck_tick: b.stuck_tick,
            stuck_total: b.stuck_total,
            stuck_events: b.stats.stuck_events,
            tactics: b.tactics.clone(),
            queued: b.queued,
            recognition: b.recognition,
        }
    }

    pub(super) fn restore(self, b: &mut SoldierBrain) {
        b.wpn = self.wpn;
        b.want_yaw = self.want_yaw;
        b.want_pitch = self.want_pitch;
        b.mode = self.mode;
        b.dec = self.dec;
        b.patrol_goal = self.patrol_goal;
        b.patrol_last = self.patrol_last;
        b.patrol_prev = self.patrol_prev;
        b.path_i = self.path_i;
        b.path_tick = self.path_tick;
        b.path_failed = self.path_failed;
        b.target = self.target;
        b.strafe_until = self.strafe_until;
        b.strafe_dir = self.strafe_dir;
        b.stuck_anchor = self.stuck_anchor;
        b.stuck_tick = self.stuck_tick;
        b.stuck_total = self.stuck_total;
        b.stats.stuck_events = self.stuck_events;
        b.tactics = self.tactics;
        b.queued = self.queued;
        b.recognition = self.recognition;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{scheduler::BotRuntime, Difficulty};
    use frontline_sim::ids::entity_id;
    use frontline_sim::view::{EntityStateView, NavData, ObjectiveView, Resources, VehicleView};
    use frontline_sim::{EntityId, Team, WorldView};
    use std::time::Duration;

    struct OnePlayer(PlayerView);

    impl WorldView for OnePlayer {
        fn players(&self) -> &[PlayerView] {
            std::slice::from_ref(&self.0)
        }
        fn vehicles(&self) -> &[VehicleView] {
            &[]
        }
        fn entity_state(&self, _: EntityId) -> Option<EntityStateView> {
            None
        }
        fn los(&self, _: Vec3, _: Vec3) -> bool {
            true
        }
        fn height_at(&self, _: f32, _: f32) -> Option<f32> {
            None
        }
        fn team_resources(&self, _: Team) -> Resources {
            Resources::default()
        }
        fn objectives(&self) -> &[ObjectiveView] {
            &[]
        }
        fn nav_hint(&self) -> &NavData {
            static NAV: NavData = NavData {};
            &NAV
        }
    }

    #[test]
    fn expired_decision_keeps_previous_intent_and_counts_timeout() {
        let view = OnePlayer(PlayerView {
            player: 0,
            entity: entity_id(0, 0),
            team: Team::Us,
            pos: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
        });
        let mut brain = SoldierBrain::with_tactics(Arc::new(BotMap::without_nav(
            Tuning::embedded(),
            Vec::new(),
        )));
        brain.on_spawn(&view.players()[0], 1);
        brain.present = true;
        brain.dec.mv = Mv::Go {
            dir: [0.0, 1.0],
            sprint: false,
        };
        let mut rt = BotRuntime::default();
        rt.set_think_limit_for_test(Duration::ZERO);
        let ctx = BotCtx {
            bot_id: 0,
            team: Team::Us,
            tick: 3,
            difficulty: Difficulty::configured_default(),
            rng_seed: 7,
            self_weapon: None,
            squad_order: None,
            squad_leader: None,
            hearing: &[],
            survival: Default::default(),
        };
        rt.begin_tick(&view, ctx.tick);
        rt.plan(&[0]);
        let cmd = rt.think(&mut brain, &view, &ctx);
        assert!(
            cmd.move_z > 0,
            "the previous movement decision must survive"
        );
        assert_eq!(cmd.tick, 3);
        assert_eq!(rt.finish_tick().timeouts, 1);
        assert_eq!(rt.finish_tick().deferred, 1);
    }
}
