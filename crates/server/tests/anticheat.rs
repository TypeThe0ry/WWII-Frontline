//! M1-027: input injection against the authoritative server. The speed hack asks for 1.3x the
//! allowed movement vector, the fire-rate hack mashes the trigger at twice the cap, the aim snap
//! turns 180 degrees in one tick. The server stays authoritative and counts each attempt.

use std::sync::Arc;

use frontline_protocol::msg::input::{buttons, FireCmd, InputCmd};
use frontline_server::anticheat::{AntiCheat, DisconnectReason, RULES};
use frontline_server::input_buffer::PushResult;
use frontline_server::metrics::Metrics;
use frontline_server::sim_bridge::SimBridge;
use frontline_server::validate::{Reject, ValidateCfg};
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, SimConfig, Team, Vec3, World,
};

const P: u8 = 3;

fn flat() -> CollisionData {
    let mut v = Vec::new();
    for bz in 0..4 {
        for bx in 0..4 {
            v.push(CollisionChunk::new(
                bx,
                bz,
                vec![5.0; 65 * 65],
                vec![0; 64 * 64],
            ));
        }
    }
    CollisionData::from_chunks(v)
}

struct Rig {
    w: World,
    br: SimBridge,
    m: Arc<Metrics>,
    seq: u16,
}

impl Rig {
    fn new() -> Rig {
        Self::with_cfg(ValidateCfg::default())
    }

    fn with_cfg(cfg: ValidateCfg) -> Rig {
        Self::with_data(cfg, GameData::embedded())
    }

    fn with_data(cfg: ValidateCfg, data: &'static GameData) -> Rig {
        let m = Metrics::new();
        let mut w = World::new(
            SimConfig {
                tick_hz: 30,
                match_seed: 9,
                map: MapId(0),
                mode: GameMode::Sandbox,
                data,
            },
            &flat(),
        );
        let mut br = SimBridge::new(m.clone(), None, None, cfg);
        br.spawn(
            &mut w,
            frontline_sim::SpawnReq {
                player: P,
                team: Team::Us,
                pos: Vec3::new(30.0, 5.02, 100.0),
                yaw: std::f32::consts::FRAC_PI_2,
            },
        )
        .unwrap();
        let mut r = Rig { w, br, m, seq: 0 };
        for _ in 0..10 {
            r.tick(|c| c);
        }
        r
    }

    /// Sends one command for the next tick (built by `f` from an idle command) and steps.
    fn tick(&mut self, f: impl FnOnce(InputCmd) -> InputCmd) -> Vec<frontline_sim::SimEvent> {
        let now = self.w.tick().0 + 1;
        self.seq = self.seq.wrapping_add(1);
        let c = f(InputCmd {
            seq: self.seq,
            tick: now,
            move_x: 0,
            move_z: 0,
            yaw: 16384,
            pitch: 0,
            buttons: 0,
            weapon_slot: 0,
            aux: 0,
            view_tick: 0,
        });
        self.br.ingest_cmd(P, &c, now);
        assert!(self.br.process_kicks(&mut self.w).is_empty());
        self.br.resolve(now);
        self.br.anticheat_before_step(&self.w);
        let ev = self.w.step(self.br.resolved()).events;
        self.br.anticheat_after_step(&ev);
        ev
    }

    fn pos(&self) -> Vec3 {
        self.w.player_state(P).unwrap().pos
    }
}

// ------------------------------------------------------------------ acceptance 1: speed hack

#[test]
fn speed_hack_is_clamped_and_counted() {
    let mut r = Rig::new();
    let mut prev = r.pos();
    let mut max_v = 0.0f32;
    for i in 0..900 {
        // 1.30x the legal input every fifth tick for 30 s. This stays below the
        // rolling disconnect limit so we can measure sustained movement.
        r.tick(|mut c| {
            (c.move_x, c.move_z) = if i % 5 == 0 { (117, 117) } else { (89, 89) };
            c.buttons = buttons::SPRINT;
            c
        });
        let p = r.pos();
        max_v = max_v.max(((p.x - prev.x).powi(2) + (p.z - prev.z).powi(2)).sqrt() * 30.0);
        prev = p;
    }
    let count = r.m.suspicion_move_total.get();
    eprintln!("30 s at 1.3x every fifth tick: max {max_v:.3} m/s, {count} clamped");
    assert!((6.0..=6.4 * 1.02).contains(&max_v));
    assert_eq!(count, 180);
    assert_eq!(r.br.anticheat.counts(P).total(), 180);
}

#[test]
fn rolling_limit_expires_old_violations_and_kicks_at_three_times_warn_count() {
    let mut ac = AntiCheat::new(std::time::Instant::now());
    let now: u32 = u32::MAX - 13;
    let limit = 3 * RULES.warn_count;
    for _ in 0..limit - 1 {
        assert_eq!(ac.input_violation(P, Reject::MoveClamped, now), None);
    }
    let boundary = now.wrapping_add(RULES.window_ticks);
    // At the exact expiry boundary the first burst stops contributing.
    for _ in 0..limit - 1 {
        assert_eq!(ac.input_violation(P, Reject::MoveClamped, boundary), None);
        assert_eq!(ac.input_violation(P, Reject::YawRate, boundary), None);
    }
    assert_eq!(
        ac.input_violation(P, Reject::MoveClamped, boundary),
        Some(DisconnectReason::RepeatedInputViolation)
    );
}

#[test]
fn replay_and_window_boundaries_do_not_trigger_enforcement() {
    let mut r = Rig::new();
    let now = r.w.tick().0 + 10;
    let mut c = r.br.resolved()[0].1;
    c.seq += 1;
    for (offset, want) in [
        (-3, PushResult::TooOld),
        (-2, PushResult::Accepted),
        (11, PushResult::TooFar),
        (10, PushResult::Accepted),
    ] {
        c.tick = now.wrapping_add_signed(offset);
        assert_eq!(r.br.ingest_cmd(P, &c, now), want);
        c.seq += 1;
    }
    c.seq -= 1;
    for _ in 0..100 {
        assert_eq!(r.br.ingest_cmd(P, &c, now), PushResult::Duplicate);
    }
    assert_eq!(
        r.m.input_rejected_total
            .with_label_values(&["duplicate"])
            .get(),
        100
    );
    assert!(r.br.process_kicks(&mut r.w).is_empty());
}

// ------------------------------------------------------------------ acceptance 3: aim snap

#[test]
fn aim_snap_is_clamped_and_legit_flicks_are_not() {
    // At 30 Hz a single tick can turn at most 180 degrees (5400 deg/s), so the spec's 8000 deg/s
    // limit can never trip (PI-074); the limit is a config value and the test uses 3600 deg/s.
    let cfg = ValidateCfg {
        max_turn_dps: 3600.0,
        ..ValidateCfg::default()
    };
    let mut r = Rig::with_cfg(cfg);
    // A 180 degree snap in one tick.
    r.tick(|mut c| {
        c.yaw = 16384 + 32768;
        c
    });
    assert_eq!(r.m.view_rate_clamped_total.get(), 1);
    let applied = r.br.resolved()[0].1.yaw;
    let turned = f32::from((applied.wrapping_sub(16384) as i16).unsigned_abs()) * 360.0 / 65536.0;
    eprintln!("180° snap clamped to {turned:.4}°; 1000 legal inputs up to 3000°/s checked");
    assert!((119.99..=120.01).contains(&turned));
    // 1000 synthetic human flicks peaking at 3000 deg/s (100 deg/tick) never trip it.
    let mut r = Rig::with_cfg(cfg);
    let mut yaw = 16384f32;
    for i in 0..1000u32 {
        let step_deg = 100.0 * ((i % 7) as f32 / 6.0);
        yaw = (yaw + step_deg / 360.0 * 65536.0).rem_euclid(65536.0);
        let previous = r.br.resolved()[0].1;
        r.tick(|mut c| {
            c.yaw = yaw as u16;
            c
        });
        if i % 3 == 0 {
            // A packet two ticks behind the next arrival is still in the window.
            // Its older sequence is dropped without a strike.
            let now = r.w.tick().0 + 1;
            assert_eq!(r.br.ingest_cmd(P, &previous, now), PushResult::Duplicate);
        }
    }
    assert_eq!(
        r.m.input_rejected_total
            .with_label_values(&["duplicate"])
            .get(),
        334
    );
    assert_eq!(r.m.session_kicked_total.get(), 0);
    assert_eq!(r.m.view_rate_clamped_total.get(), 0);
}

// ------------------------------------------------------------------ acceptance 2: fire rate

#[test]
fn fire_rate_hack_is_refused_and_counted() {
    // Use the real Garand cycle and a larger fixture magazine so dry-fire and reload
    // do not mask rate refusals in the ten-second test.
    let base = GameData::embedded();
    let mut weapons = base.weapons.to_vec();
    let garand = weapons.iter_mut().find(|w| w.id == "us_m1_garand").unwrap();
    garand.historical.magazine_capacity = frontline_sim::data::defs::Hist::Known(64);
    let data = Box::leak(Box::new(GameData {
        weapons: Box::leak(weapons.into_boxed_slice()),
        ..*base
    }));
    let mut r = Rig::with_data(ValidateCfg::default(), data);
    let ticks = 300u32; // 10 s
    let mut shots = 0u32;
    let mut pulls = 0u32;
    for i in 0..ticks {
        // Garand: 120 rpm / 2 shots/s cap. Pull the trigger at 240 rpm.
        let fire = (2 * i % 15) < 2;
        pulls += u32::from(fire);
        let ev = r.tick(|mut c| {
            if fire {
                c.buttons = buttons::FIRE;
            }
            c
        });
        shots += ev
            .iter()
            .filter(|e| matches!(e, frontline_sim::SimEvent::Shot { .. }))
            .count() as u32;
    }
    let rejected = r.m.fire_rate_rejected_total.get() as u32;
    eprintln!("pulls {pulls}, shots {shots}, rejected {rejected}");
    let cap_shots = 20;
    assert_eq!(pulls, 40);
    assert!(shots <= 21, "10 s cap plus 5%: {shots} shots");
    assert!(
        (rejected * 10) >= (pulls - cap_shots) * 9,
        "only {rejected} rate rejections"
    );
}

// ------------------------------------------------------------------ acceptance 7: audit

#[test]
fn no_client_authoritative_fields() {
    // Exhaustive destructuring: adding a field to either message breaks this test until the new
    // field is reviewed (a position, ammo or damage field must never appear).
    let InputCmd {
        seq: _,
        tick: _,
        move_x: _,
        move_z: _,
        yaw: _,
        pitch: _,
        buttons: _,
        weapon_slot: _,
        aux: _,
        view_tick: _,
    } = InputCmd {
        seq: 0,
        tick: 0,
        move_x: 0,
        move_z: 0,
        yaw: 0,
        pitch: 0,
        buttons: 0,
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    };
    let FireCmd {
        seq: _,
        view_tick: _,
        weapon: _,
        dir_yaw: _,
        dir_pitch: _,
        origin_hint: _,
        spread_seed: _,
    } = FireCmd {
        seq: 0,
        view_tick: 0,
        weapon: 0,
        dir_yaw: 0,
        dir_pitch: 0,
        origin_hint: [0; 3],
        spread_seed: 0,
    };
}
