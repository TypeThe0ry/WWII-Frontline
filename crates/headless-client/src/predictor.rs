//! Client-side prediction and reconciliation (ARCHITECTURE §6.4, PI-023), the Rust twin of
//! `client/src/game/{unacked-ring,reconcile}.ts` on top of `sim::PredictSim`.
//!
//! The server sends a position (`pos_corr`) only when the client's `pred_hash` disagreed; that
//! position is the authoritative one right after the command `ack_input_seq`.
//!
//! * error `err0 = |predicted position at ack - pos_corr|`; `<= 5 cm`: keep the prediction;
//! * otherwise roll back to the authoritative position, replay every unacknowledged command and
//!   count a correction (`>= 1 m` is a "snap" in the browser client).
//!
//! The hash reported next is that of the position right after the acked command, evaluated after
//! any rollback (so a corrected client reports the matching hash and the server stops sending
//! corrections).

use std::collections::VecDeque;

use frontline_protocol::msg::input::InputCmd;
use frontline_protocol::snapshot::pred_hash::pred_hash;
use frontline_sim::{
    CollisionData, GameData, GameMode, MapId, PlayerAuthState, SimConfig, Tick, Vec3,
};

use crate::report::Corrections;

pub const RING_CAPACITY: usize = 128;
pub const RECONCILE_THRESHOLD_M: f32 = 0.05;
pub const SNAP_DISTANCE_M: f32 = 1.0;

#[derive(Clone, Copy)]
struct Entry {
    cmd: InputCmd,
    state: PlayerAuthState,
}

pub struct Predictor {
    sim: frontline_sim::predict::PredictSim,
    /// Sequence number of `base` (the newest state no longer in the ring).
    base_seq: u16,
    base: Option<PlayerAuthState>,
    ring: VecDeque<Entry>,
    spawned: bool,
    /// The server says the player is dead: its inputs are ignored there, so nothing is predicted.
    frozen: bool,
    pub stats: Corrections,
    pub overflows: u64,
    pub replayed: u64,
    /// Error of the most recent rollback (diagnostics, `HEADLESS_TRACE=1`).
    pub last_err_m: f32,
}

fn dist(a: Vec3, b: Vec3) -> f32 {
    (a - b).length()
}

pub(crate) fn new_sim(
    match_seed: u64,
    map: u8,
    player: u8,
    collision: &CollisionData,
) -> frontline_sim::predict::PredictSim {
    let cfg = SimConfig {
        tick_hz: 30,
        match_seed,
        map: MapId(u16::from(map)),
        mode: GameMode::Sandbox,
        data: GameData::embedded(),
    };
    frontline_sim::predict::PredictSim::new(cfg, collision, player)
}

impl Predictor {
    /// `first_seq` is the sequence number of the first command that will be pushed.
    pub fn new(
        match_seed: u64,
        map: u8,
        player: u8,
        collision: &CollisionData,
        first_seq: u16,
    ) -> Self {
        let sim = new_sim(match_seed, map, player, collision);
        Predictor {
            sim,
            base_seq: first_seq.wrapping_sub(1),
            base: None,
            ring: VecDeque::with_capacity(RING_CAPACITY + 1),
            spawned: false,
            frozen: false,
            stats: Corrections::default(),
            overflows: 0,
            replayed: 0,
            last_err_m: 0.0,
        }
    }

    pub fn spawned(&self) -> bool {
        self.spawned
    }

    /// Predicted position after the newest command (`None` before the first placement).
    pub fn pos(&self) -> Option<Vec3> {
        self.spawned.then(|| self.sim.state().pos)
    }

    /// Simulates `cmd` and remembers it until the server acknowledges it.
    pub fn push(&mut self, cmd: &InputCmd) {
        let state = if self.spawned && self.frozen {
            self.sim.state()
        } else if self.spawned {
            self.sim.step_player(cmd)
        } else {
            PlayerAuthState::spawn(Vec3::ZERO, 0.0)
        };
        self.ring.push_back(Entry { cmd: *cmd, state });
        if self.ring.len() > RING_CAPACITY {
            if let Some(e) = self.ring.pop_front() {
                self.base_seq = e.cmd.seq;
                self.base = self.spawned.then_some(e.state);
            }
            self.overflows += 1;
        }
    }

    /// Processes the `self_state` of one snapshot. `corr` is the metric `pos_corr`; `death_window`
    /// says whether a rollback now is explained by a death/respawn teleport. Returns the
    /// `pred_hash` to report together with this snapshot's ack.
    pub fn on_self(
        &mut self,
        ack_seq: u16,
        corr: Option<[f32; 3]>,
        alive: bool,
        death_window: bool,
    ) -> u8 {
        self.frozen = !alive;
        let newest = self.ring.back().map_or(self.base_seq, |e| e.cmd.seq);
        let behind = ack_seq.wrapping_sub(self.base_seq) as i16;
        let ahead = ack_seq.wrapping_sub(newest) as i16;
        if ahead > 0 {
            self.stats.unknown_acks += 1;
            return self.report_hash();
        }
        if behind < 0 {
            return self.report_hash(); // reordered / older than what we already know
        }
        // Known ack: drop everything up to and including it.
        let mut reference = self.base;
        while self.ring.front().is_some_and(|e| {
            (e.cmd.seq.wrapping_sub(self.base_seq) as i16) >= 1
                && (ack_seq.wrapping_sub(e.cmd.seq) as i16) >= 0
        }) {
            if let Some(e) = self.ring.pop_front() {
                self.base_seq = e.cmd.seq;
                reference = self.spawned.then_some(e.state);
            }
        }
        self.base = reference;
        self.base_seq = ack_seq;

        if let Some(c) = corr {
            let auth = Vec3::new(c[0], c[1], c[2]);
            match (self.spawned, reference) {
                (true, Some(r)) => {
                    let err0 = dist(r.pos, auth);
                    if err0 <= RECONCILE_THRESHOLD_M {
                        self.stats.tolerance_kept += 1;
                        self.stats.tolerance_max_err_m =
                            self.stats.tolerance_max_err_m.max(f64::from(err0));
                    } else {
                        self.rollback(r, auth, death_window);
                    }
                }
                _ => {
                    // First placement: authoritative position, everything else defaults.
                    self.stats.initial_syncs += 1;
                    self.spawned = true;
                    let r = PlayerAuthState::spawn(auth, 0.0);
                    self.base = Some(r);
                    self.replay_from(r);
                }
            }
        }
        self.report_hash()
    }

    fn rollback(&mut self, mut r: PlayerAuthState, auth: Vec3, death_window: bool) {
        let old = self.sim.state().pos;
        if dist(r.pos, auth) >= SNAP_DISTANCE_M && !self.frozen {
            // A teleport (respawn): the server starts the new life from a fresh state, so the
            // old velocity / stance must not leak into the replay.
            r = PlayerAuthState::spawn(auth, r.yaw);
        }
        r.pos = auth;
        self.base = Some(r);
        self.replay_from(r);
        let err = dist(old, self.sim.state().pos);
        self.last_err_m = err;
        let s = &mut self.stats;
        s.total += 1;
        if err >= SNAP_DISTANCE_M {
            s.snaps += 1;
        }
        if death_window {
            s.around_death += 1;
        }
        s.max_err_m = s.max_err_m.max(f64::from(err));
    }

    /// Resets the simulation to `base` and re-simulates every command still in the ring.
    fn replay_from(&mut self, base: PlayerAuthState) {
        self.sim.reset_to(&base, Tick(0));
        for e in self.ring.iter_mut() {
            e.state = if self.frozen {
                base
            } else {
                self.sim.step_player(&e.cmd)
            };
        }
        self.replayed += self.ring.len() as u64;
    }

    fn report_hash(&self) -> u8 {
        match self.base {
            Some(b) if self.spawned => pred_hash(b.pos.x, b.pos.y, b.pos.z),
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(seq: u16, mz: i8) -> InputCmd {
        InputCmd {
            seq,
            tick: u32::from(seq),
            move_x: 0,
            move_z: mz,
            yaw: 0,
            pitch: 0,
            buttons: 0,
            weapon_slot: 0,
            aux: 0,
            view_tick: 0,
        }
    }

    fn p() -> Predictor {
        Predictor::new(1, 0, 3, &CollisionData::empty(), 1)
    }

    #[test]
    fn first_corr_places_player_and_is_not_a_correction() {
        let mut p = p();
        for s in 1..=3 {
            p.push(&cmd(s, 0));
        }
        assert!(p.pos().is_none());
        let h = p.on_self(0, Some([100.0, 5.0, 50.0]), true, false);
        assert!(p.spawned());
        assert_eq!(p.stats.initial_syncs, 1);
        assert_eq!(p.stats.total, 0);
        let pos = p.pos().unwrap();
        assert!((pos.x - 100.0).abs() < 1.0 && (pos.z - 50.0).abs() < 1.0);
        assert_eq!(h, pred_hash(100.0, 5.0, 50.0));
    }

    #[test]
    fn matching_corr_within_tolerance_keeps_prediction() {
        let mut p = p();
        p.on_self(0, Some([10.0, 2.0, 10.0]), true, false);
        for s in 1..=4 {
            p.push(&cmd(s, 0));
        }
        let at2 = p.ring[1].state.pos;
        p.on_self(2, Some([at2.x + 0.01, at2.y, at2.z]), true, false);
        assert_eq!(p.stats.tolerance_kept, 1);
        assert_eq!(p.stats.total, 0);
        assert_eq!(p.ring.len(), 2); // 3 and 4 remain
    }

    #[test]
    fn far_corr_rolls_back_counts_and_snaps() {
        let mut p = p();
        p.on_self(0, Some([10.0, 2.0, 10.0]), true, false);
        for s in 1..=4 {
            p.push(&cmd(s, 0));
        }
        let h = p.on_self(2, Some([20.0, 2.0, 10.0]), true, true);
        assert_eq!(p.stats.total, 1);
        assert_eq!(p.stats.snaps, 1);
        assert_eq!(p.stats.around_death, 1);
        assert!(p.stats.max_err_m > 9.0);
        let pos = p.pos().unwrap();
        assert!((pos.x - 20.0).abs() < 1.0, "{pos:?}");
        assert_eq!(h, pred_hash(20.0, 2.0, 10.0));
    }

    #[test]
    fn small_rollback_is_not_a_snap() {
        let mut p = p();
        p.on_self(0, Some([10.0, 2.0, 10.0]), true, false);
        p.push(&cmd(1, 0));
        let r = p.ring[0].state.pos;
        p.on_self(1, Some([r.x + 0.3, r.y, r.z]), true, false);
        assert_eq!((p.stats.total, p.stats.snaps), (1, 0));
    }

    #[test]
    fn unknown_future_and_stale_acks() {
        let mut p = p();
        p.on_self(0, Some([10.0, 2.0, 10.0]), true, false);
        p.push(&cmd(1, 0));
        p.push(&cmd(2, 0));
        p.on_self(9, None, true, false); // never sent
        assert_eq!(p.stats.unknown_acks, 1);
        p.on_self(2, None, true, false);
        assert_eq!(p.ring.len(), 0);
        p.on_self(1, Some([500.0, 2.0, 10.0]), true, false); // older than base: ignored
        assert_eq!(p.stats.total, 0);
    }

    #[test]
    fn ring_overflow_advances_base() {
        let mut p = p();
        p.on_self(0, Some([10.0, 2.0, 10.0]), true, false);
        for s in 1..=(RING_CAPACITY as u16 + 10) {
            p.push(&cmd(s, 0));
        }
        assert_eq!(p.ring.len(), RING_CAPACITY);
        assert_eq!(p.overflows, 10);
        // acking a seq that fell off the ring's tail is "stale", not a crash
        p.on_self(5, None, true, false);
    }

    #[test]
    fn dead_player_is_frozen_after_one_rollback() {
        let mut p = p();
        p.on_self(0, Some([10.0, 2.0, 10.0]), true, false);
        for s in 1..=3 {
            p.push(&cmd(s, 127));
        }
        // the server says: dead, standing where it died (differs from the prediction)
        p.on_self(1, Some([10.0, 2.0, 10.0]), false, true);
        let after = p.pos().unwrap();
        for s in 4..=8 {
            p.push(&cmd(s, 127)); // neutral on a real client; even full forward must not move it
        }
        assert_eq!(p.pos().unwrap(), after);
        // the same authoritative position again: within tolerance, no second rollback
        p.on_self(5, Some([after.x, after.y, after.z]), false, true);
        assert!(p.stats.total <= 1, "{:?}", p.stats);
        assert_eq!(p.stats.tolerance_kept + p.stats.total, 2);
    }

    #[test]
    fn seq_wraparound() {
        let mut p = Predictor::new(1, 0, 3, &CollisionData::empty(), 65534);
        p.on_self(65533, Some([10.0, 2.0, 10.0]), true, false);
        for s in [65534u16, 65535, 0, 1] {
            p.push(&cmd(s, 0));
        }
        p.on_self(0, None, true, false);
        assert_eq!(p.ring.len(), 1);
        assert_eq!(p.stats.unknown_acks, 0);
    }

    #[test]
    fn scripted_walk_sprint_and_vault_match_authority() {
        use frontline_protocol::msg::input::buttons;
        use frontline_sim::{CollisionChunk, SpawnReq, Team, World};

        let heights = (0..65 * 65)
            .map(|i| if i % 65 >= 10 { 1.29 } else { 0.0 })
            .collect();
        let col = CollisionData::from_chunks([CollisionChunk::new(0, 0, heights, vec![0; 4096])]);
        let mut world = World::new(
            SimConfig {
                tick_hz: 30,
                match_seed: 1,
                map: MapId(0),
                mode: GameMode::Sandbox,
                data: GameData::embedded(),
            },
            &col,
        );
        let pos = Vec3::new(5.0, 0.3, 20.0);
        world
            .spawn_player(SpawnReq {
                player: 0,
                team: Team::Us,
                pos,
                yaw: 0.0,
            })
            .unwrap();
        let mut pred = Predictor::new(1, 0, 0, &col, 1);
        pred.on_self(0, Some(pos.to_array()), true, false);
        let mut max_err = 0.0_f32;
        let mut vault_ticks = 0;
        let mut max_speed = [0.0_f32; 2];
        for seq in 1..=150 {
            let mut c = cmd(seq, if seq <= 30 || seq >= 115 { 0 } else { 127 });
            c.yaw = 16384;
            if (31..=50).contains(&seq) {
                c.aux = frontline_sim::movement::AUX_WALK;
            }
            c.buttons = match seq {
                51..=80 => buttons::SPRINT,
                81..=114 => buttons::USE,
                _ => 0,
            };
            pred.push(&c);
            world.step(&[(0, c)]);
            let auth = world.player_state(0).unwrap();
            max_err = max_err.max((pred.pos().unwrap() - auth.pos).length());
            vault_ticks += usize::from(auth.vault.is_some());
            if (31..=80).contains(&seq) {
                let i = usize::from(seq > 50);
                max_speed[i] = max_speed[i].max(auth.vel.x);
            }
            if seq % 3 == 0 {
                pred.on_self(seq, Some(auth.pos.to_array()), true, false);
            }
        }
        println!("150 ticks: headless max {max_err:.6} m, {vault_ticks} active vault ticks, walk/sprint maxima {max_speed:?} m/s, {} corrections", pred.stats.total);
        assert!(max_speed[0] > 1.9 && max_speed[1] > 5.9);
        assert!((22..=24).contains(&vault_ticks));
        assert!(world.player_state(0).unwrap().pos.x > 10.0);
        assert!(max_err < 0.05 && pred.stats.total == 0);
    }
}
