//! Synthetic driver and same-tick vehicle reconciliation (the headless counterpart of
//! `client/src/game/vehicle-predict.ts`, without the visual blend).

use std::collections::VecDeque;
use std::time::Duration;

use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_protocol::quant::quat_unpack;
use frontline_protocol::snapshot::DecodedSnapshot;
use frontline_sim::{predict::PredictSim, Quat, Vec3};

use crate::predictor::{RECONCILE_THRESHOLD_M, RING_CAPACITY, SNAP_DISTANCE_M};
use crate::report::VehicleReport;
use crate::stats::Dist;

// The M1 snapshot does not carry a model id (same assumption as the browser's GameLoop).
const DEF_ID: &str = "us_m4a1_75";
const EXIT_WINDOW: Duration = Duration::from_secs(3);

pub(crate) struct Driver {
    sim: PredictSim,
    report: VehicleReport,
    pending: VecDeque<InputCmd>,
    positions: VecDeque<(u32, Vec3)>,
    errors: Vec<f64>,
    slot: u8,
    seat: u8,
    next_board_tick: Option<u32>,
    lane: Option<(Vec3, Vec3)>,
    throttle: i8,
}

fn after(tick: u32, base: u32) -> bool {
    (tick.wrapping_sub(base) as i32) > 0
}

impl Driver {
    pub(crate) fn new(sim: PredictSim) -> Self {
        Self {
            sim,
            report: VehicleReport::default(),
            pending: VecDeque::with_capacity(RING_CAPACITY),
            positions: VecDeque::with_capacity(RING_CAPACITY),
            errors: Vec::new(),
            slot: 0,
            seat: 0,
            next_board_tick: None,
            lane: None,
            throttle: 127,
        }
    }

    pub(crate) fn seated(&self) -> bool {
        self.slot != 0
    }

    pub(crate) fn cmd(&mut self, seq: u16, tick: u32, alive: bool, left: Duration) -> InputCmd {
        let mut cmd = InputCmd {
            seq,
            tick,
            move_x: 0,
            move_z: 0,
            yaw: 0,
            pitch: 0,
            buttons: 0,
            weapon_slot: 0,
            aux: 0,
            view_tick: 3,
        };
        if alive {
            if left <= EXIT_WINDOW {
                if self.seated() {
                    cmd.buttons = buttons::VEHICLE_SEAT | buttons::JUMP;
                }
            } else if self.seated() {
                if self.seat == 0 {
                    cmd.move_z = self.throttle;
                    self.report.driver_input_ticks += 1;
                }
            } else if self.next_board_tick.is_none_or(|t| !after(t, tick)) {
                // One command high then release; retry if it was dropped or boarding failed.
                cmd.buttons = buttons::VEHICLE_SEAT;
                self.next_board_tick = Some(tick.wrapping_add(30));
            }
        }
        if self.pending.len() == RING_CAPACITY {
            self.pending.pop_front();
        }
        self.pending.push_back(cmd);
        Self::step(&mut self.sim, &mut self.positions, &cmd);
        cmd
    }

    pub(crate) fn on_snapshot(&mut self, s: &DecodedSnapshot) {
        let Some(ss) = s.self_state else { return };
        self.report.last_vehicle_slot = ss.vehicle_slot;
        self.report.last_seat = ss.seat;
        let slot = if ss.health == 0 { 0 } else { ss.vehicle_slot };
        if slot != self.slot || ss.seat != self.seat {
            if slot != 0 && self.slot == 0 {
                self.report.entries += 1;
            } else if slot == 0 && self.slot != 0 && ss.health > 0 {
                self.report.exits += 1;
            }
            self.sim.reset_vehicle(None);
            self.positions.clear();
            self.lane = None;
            self.throttle = 127;
            self.slot = slot;
            self.seat = ss.seat;
        }
        self.pending
            .retain(|c| (c.seq.wrapping_sub(s.ack_input_seq) as i16) > 0 && after(c.tick, s.tick));
        if slot == 0 || self.seat != 0 {
            return;
        }
        let Some(i) = s.entities.ids.iter().position(|&id| id == slot) else {
            return;
        };
        let Some(row) = s.entities.state(i) else {
            return;
        };
        let (Some(vel), Some(extra)) = (row.vel, row.vehicle) else {
            return;
        };
        self.report.driver_vehicle_slot = Some(slot);
        self.report.driver_snapshots += 1;
        let pos = Vec3::from_array(row.pos.to_meters());
        let rot = Quat::from_array(quat_unpack(extra.quat));
        // Use the authoritative position to reverse: forward/reverse speeds aren't symmetric.
        let (origin, forward) = *self.lane.get_or_insert_with(|| (pos, rot * Vec3::Z));
        let d = pos - origin;
        let along = d.x * forward.x + d.z * forward.z;
        if along >= 8.0 {
            self.throttle = -127;
        } else if along <= 0.0 {
            self.throttle = 127;
        }
        self.report.max_displacement_m = self.report.max_displacement_m.max(d.x.hypot(d.z));
        if self.sim.vehicle().is_some() {
            if let Some((_, predicted)) = self.positions.iter().find(|(tick, _)| *tick == s.tick) {
                let error = predicted.distance(pos);
                self.report.corrections.comparisons += 1;
                if error < RECONCILE_THRESHOLD_M {
                    return;
                }
                self.errors.push(f64::from(error));
                self.report.corrections.total += 1;
                self.report.corrections.snaps += u64::from(error >= SNAP_DISTANCE_M);
            } else {
                self.report.corrections.missing_ticks += 1;
            }
        }

        // Like the browser, reset from this tick's vehicle row (hull yaw rate isn't on the wire).
        self.sim
            .reset_vehicle_kinematics(DEF_ID, pos, Vec3::from_array(vel.to_mps()), rot, 0.0)
            .expect("M1 vehicle exists in the embedded data");
        self.positions.clear();
        self.positions.push_back((s.tick, pos));
        for cmd in &self.pending {
            Self::step(&mut self.sim, &mut self.positions, cmd);
        }
    }

    fn step(sim: &mut PredictSim, positions: &mut VecDeque<(u32, Vec3)>, cmd: &InputCmd) {
        if positions.back().is_some_and(|(t, _)| !after(cmd.tick, *t)) {
            return;
        }
        if let Some(vehicle) = sim.step_vehicle_driver(cmd) {
            if positions.len() == RING_CAPACITY {
                positions.pop_front();
            }
            positions.push_back((cmd.tick, vehicle.pos));
        }
    }

    pub(crate) fn finish(mut self) -> VehicleReport {
        self.report.corrections.error_m = (!self.errors.is_empty()).then(|| Dist::of(&self.errors));
        self.report
    }
}
