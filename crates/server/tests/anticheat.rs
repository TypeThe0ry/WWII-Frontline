//! M1-027: input injection against the authoritative server. The speed hack asks for 1.3x the
//! allowed movement vector, the fire-rate hack mashes the trigger at twice the cap, the aim snap
//! turns 180 degrees in one tick. The server stays authoritative and counts each attempt.

use std::sync::Arc;

use frontline_protocol::msg::input::{buttons, FireCmd, InputCmd};
use frontline_server::metrics::Metrics;
use frontline_server::sim_bridge::SimBridge;
use frontline_server::validate::ValidateCfg;
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
        let m = Metrics::new();
        let mut w = World::new(
            SimConfig {
                tick_hz: 30,
                match_seed: 9,
                map: MapId(0),
                mode: GameMode::Sandbox,
                data: GameData::embedded(),
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
    for _ in 0..900 {
        // 1.3x the unit movement vector, sprinting.
        r.tick(|mut c| {
            c.move_x = 83;
            c.move_z = 83;
            c.buttons = buttons::SPRINT;
            c
        });
        let p = r.pos();
        max_v = max_v.max(((p.x - prev.x).powi(2) + (p.z - prev.z).powi(2)).sqrt() * 30.0);
        prev = p;
    }
    // 83*sqrt(2)/127 = 0.924 would be legal; send a real 1.3x below.
    assert!(max_v <= 6.4 * 1.02, "max speed {max_v}");
    let mut r2 = Rig::new();
    let mut prev = r2.pos();
    let mut max_v = 0.0f32;
    for _ in 0..900 {
        r2.tick(|mut c| {
            c.move_x = 117;
            c.move_z = 117; // |v| = 165 / 127 = 1.30
            c.buttons = buttons::SPRINT;
            c
        });
        let p = r2.pos();
        max_v = max_v.max(((p.x - prev.x).powi(2) + (p.z - prev.z).powi(2)).sqrt() * 30.0);
        prev = p;
    }
    eprintln!("1.3x speed hack: max speed {max_v:.2} m/s");
    assert!(max_v <= 6.4 * 1.02, "max speed {max_v}");
    let n = r2.m.suspicion_move_total.get();
    assert!(n >= 100, "suspicion_move_total {n}");
    assert!(r2.br.anticheat.counts(P).total() >= 100);
    assert_eq!(
        r.m.suspicion_move_total.get(),
        0,
        "legal movement is not suspicious"
    );
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
    // 1000 synthetic human flicks peaking at 3000 deg/s (100 deg/tick) never trip it.
    let mut r = Rig::with_cfg(cfg);
    let mut yaw = 16384f32;
    for i in 0..1000u32 {
        let step_deg = 100.0 * ((i % 7) as f32 / 6.0);
        yaw = (yaw + step_deg / 360.0 * 65536.0).rem_euclid(65536.0);
        r.tick(|mut c| {
            c.yaw = yaw as u16;
            c
        });
    }
    assert_eq!(r.m.view_rate_clamped_total.get(), 0);
}

// ------------------------------------------------------------------ acceptance 2: fire rate

#[test]
fn fire_rate_hack_is_refused_and_counted() {
    let mut r = Rig::new();
    let ticks = 300u32; // 10 s
    let mut shots = 0u32;
    let mut pulls = 0u32;
    for i in 0..ticks {
        // Rising edge every other tick: 15 pulls/s, far above any rifle's cap.
        let fire = i % 2 == 0;
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
    assert!(shots <= 10 * 12, "the cap held: {shots} shots");
    assert!(rejected > 0 && rejected + shots <= pulls);
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
