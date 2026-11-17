//! M0-018 acceptance 1, 2, 3 and 6 plus the golden-trajectory regression: the input buffer
//! against a reference model, the missing-input policy driving a real `World`, one counter per
//! rejection reason, the 30-violation kick, and a 10^5-case fuzz run. (The unit tests next to the
//! code cover the pieces; this file drives the public API the way the sim thread does.)

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::sim_harness::{gen_cmd, terrain};
use frontline_protocol::msg::input::{buttons, decode_input_bundle, InputBundle, InputCmd};
use frontline_server::input_buffer::{InputBuffer, PushResult, CAPACITY, MAX_FUTURE, MAX_PAST};
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::Sessions;
use frontline_server::net::transport::ClientHandle;
use frontline_server::sim_bridge::{NetLink, SimBridge};
use frontline_server::validate::{
    Reject, ValidateCfg, KICK_STREAK, MAX_MOVE_LEN, MAX_WEAPON_SLOT, RESERVED_BUTTONS,
};
use frontline_server::world_loader::LoadedMap;
use frontline_sim::{
    CollisionData, GameData, GameMode, MapId, PlayerId, SimConfig, SpawnReq, Team, Vec3, World,
};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn cmd(tick: u32, seq: u16) -> InputCmd {
    InputCmd {
        seq,
        tick,
        move_x: 0,
        move_z: 0,
        yaw: 0,
        pitch: 0,
        buttons: 0,
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    }
}

fn world_with(collision: &CollisionData) -> World {
    World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 7,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        collision,
    )
}

fn bridge(m: &Arc<Metrics>, map: Option<Arc<LoadedMap>>) -> SimBridge {
    SimBridge::new(m.clone(), None, map, ValidateCfg::default())
}

fn rejected(m: &Metrics, reason: &str) -> u64 {
    m.input_rejected_total.with_label_values(&[reason]).get()
}

// ------------------------------------------------------------------ acceptance 1: the buffer

#[test]
fn window_classification_is_exact_for_every_now_and_tick() {
    for now in 0..40u32 {
        for tick in 0..80u32 {
            let b = InputBuffer::new();
            let expect = if tick + MAX_PAST < now {
                Err(PushResult::TooOld)
            } else if tick > now + MAX_FUTURE {
                Err(PushResult::TooFar)
            } else {
                Ok(())
            };
            assert_eq!(b.classify(tick, now), expect, "now {now} tick {tick}");
        }
    }
}

/// Reference model: `take(t)` returns the command of tick `t` iff it was accepted and not
/// displaced; the ring displaces only by a newer tick of the same slot.
#[test]
fn buffer_matches_a_reference_model_under_random_traffic() {
    let mut rng = Rng(0xDEADBEEF);
    for round in 0..300 {
        let mut b = InputBuffer::new();
        let mut model: [Option<u32>; CAPACITY] = [None; CAPACITY];
        let mut now = 10 + round;
        for step in 0..200u32 {
            if rng.below(4) == 0 {
                // The sim consumes tick `now`, then moves on.
                let got = b.take(now).map(|c| c.tick);
                let slot = now as usize % CAPACITY;
                let want = (model[slot] == Some(now)).then_some(now);
                assert_eq!(got, want, "round {round} step {step}");
                if want.is_some() {
                    model[slot] = None;
                }
                now += 1;
                continue;
            }
            let tick = (now + rng.below(16) as u32).saturating_sub(4);
            let r = b.push(cmd(tick, step as u16), now);
            let slot = tick as usize % CAPACITY;
            let expect = if tick + MAX_PAST < now {
                PushResult::TooOld
            } else if tick > now + MAX_FUTURE {
                PushResult::TooFar
            } else {
                match model[slot] {
                    Some(t) if t == tick => PushResult::Duplicate,
                    Some(t) if t > tick => PushResult::TooOld,
                    Some(t) if t >= now => {
                        model[slot] = Some(tick);
                        PushResult::AcceptedOverwrote
                    }
                    _ => {
                        model[slot] = Some(tick);
                        PushResult::Accepted
                    }
                }
            };
            assert_eq!(r, expect, "round {round} step {step} tick {tick} now {now}");
        }
    }
}

#[test]
fn five_commands_in_every_order_come_out_in_tick_order() {
    // All 120 permutations of 5 commands.
    fn perms(items: &mut Vec<u32>, k: usize, out: &mut Vec<Vec<u32>>) {
        if k == items.len() {
            out.push(items.clone());
            return;
        }
        for i in k..items.len() {
            items.swap(k, i);
            perms(items, k + 1, out);
            items.swap(k, i);
        }
    }
    let mut all = Vec::new();
    perms(&mut vec![0, 1, 2, 3, 4], 0, &mut all);
    assert_eq!(all.len(), 120);
    for p in all {
        let mut b = InputBuffer::new();
        for i in &p {
            assert_eq!(b.push(cmd(200 + i, *i as u16), 200), PushResult::Accepted);
        }
        for t in 200..205 {
            assert_eq!(b.take(t).unwrap().tick, t);
        }
    }
}

// ------------------------------------------------ acceptance 2: missing input over a real World

fn speed(w: &World, p: PlayerId) -> f32 {
    let v = w.player_state(p).unwrap().vel;
    (v.x * v.x + v.z * v.z).sqrt()
}

#[test]
fn ten_silent_ticks_repeat_five_then_hold_still_and_never_replay_fire() {
    let Some(map) = terrain() else {
        return;
    };
    let m = Metrics::new();
    let mut world = world_with(&map.collision);
    map.apply_to(&mut world);
    let mut br = bridge(&m, Some(map.clone()));
    br.join(&mut world, 0, Team::Us).unwrap();

    // 20 ticks of walking forward while pulsing the trigger (a rising edge every other tick),
    // then silence.
    let mut t = 0u32;
    for _ in 0..20 {
        t += 1;
        let mut c = cmd(t, t as u16);
        c.move_z = 127;
        c.yaw = 4000;
        c.pitch = 100;
        c.buttons = if t.is_multiple_of(2) {
            buttons::FIRE
        } else {
            0
        } | buttons::AIM;
        br.ingest_cmd(0, &c, t);
        let inputs = br.resolve(t).to_vec();
        world.step(&inputs);
    }
    let v_walk = speed(&world, 0);
    assert!(v_walk > 1.0, "player must be walking, speed {v_walk}");
    let mag_before = world.weapon_state(0).unwrap().mag;
    assert!(mag_before < 8, "the pulsing trigger fired");
    let yaw_before = world.player_state(0).unwrap().yaw;

    let mut speeds = vec![];
    let mut resolved = vec![];
    let mut positions = vec![];
    for i in 1..=20u32 {
        t += 1;
        let inputs = br.resolve(t).to_vec();
        resolved.push(inputs[0].1);
        world.step(&inputs);
        speeds.push(speed(&world, 0));
        positions.push(world.player_state(0).unwrap().pos);
        // The trigger is never held by a repeated command.
        assert!(!world.weapon_state(0).unwrap().trigger_held, "tick +{i}");
    }
    // Ticks +1..+5: the previous move command is repeated (edge bits stripped).
    for (i, c) in resolved.iter().take(5).enumerate() {
        assert_eq!(c.move_z, 127, "silent tick {}", i + 1);
        assert_eq!(c.buttons & buttons::FIRE, 0);
        assert_ne!(c.buttons & buttons::AIM, 0, "level bits stay");
    }
    // Ticks +6..: movement zeroed, view kept.
    for (i, c) in resolved.iter().enumerate().skip(5) {
        assert_eq!((c.move_x, c.move_z), (0, 0), "silent tick {}", i + 1);
        assert_eq!((c.yaw, c.pitch), (4000, 100), "view kept at tick {}", i + 1);
    }
    // The player keeps its speed for 5 ticks, then decelerates (PI-015: decel_time_s) to exactly
    // zero and stays put.
    assert!(speeds[..5].iter().all(|&s| s > 0.9 * v_walk), "{speeds:?}");
    assert!(
        speeds[5] < speeds[4],
        "deceleration starts at tick 6: {speeds:?}"
    );
    let stop = speeds
        .iter()
        .position(|&s| s == 0.0)
        .expect("comes to rest");
    eprintln!(
        "stops {} ticks after the last repeated input; speeds {speeds:?}",
        stop as i32 - 4
    );
    assert!(stop <= 5 + 8, "rest within the decel window: {speeds:?}");
    for i in stop..positions.len() {
        assert_eq!(
            positions[i], positions[stop],
            "no displacement once stopped"
        );
    }
    assert_eq!(world.player_state(0).unwrap().yaw, yaw_before);
    assert_eq!(
        world.weapon_state(0).unwrap().mag,
        mag_before,
        "no shots while silent"
    );
    // Gap >= 9: one stall counted, nothing else.
    assert_eq!(m.session_stall_total.get(), 1);
    assert_eq!(m.input_missing_ticks_total.get(), 20);
}

#[test]
fn golden_input_sequence_gives_the_same_trajectory_as_feeding_the_world_directly() {
    let Some(map) = terrain() else {
        return;
    };
    let m = Metrics::new();
    let mut a = world_with(&map.collision);
    let mut b = world_with(&map.collision);
    map.apply_to(&mut a);
    map.apply_to(&mut b);
    let mut br = bridge(&m, Some(map.clone()));
    for p in [0u8, 1] {
        let team = if p == 0 { Team::Us } else { Team::Ger };
        br.join(&mut a, p, team).unwrap();
        let (pos, yaw) = map.join_spawn(p, team);
        b.spawn_player(SpawnReq {
            player: p,
            team,
            pos,
            yaw,
        })
        .unwrap();
    }
    let mut hash = 0xcbf29ce484222325u64;
    for t in 1..=150u32 {
        // Each player sends its newest command three ticks ahead plus the two before it, as the
        // client does.
        for p in [0u8, 1] {
            for k in t.saturating_sub(2).max(1)..=t + 3 {
                // Only the first time a tick is sent matters; the rest are duplicates.
                let _ = br.ingest_cmd(p, &gen_cmd(p, k), t);
            }
        }
        let inputs = br.resolve(t).to_vec();
        a.step(&inputs);
        let direct: Vec<_> = [0u8, 1].iter().map(|&p| (p, gen_cmd(p, t))).collect();
        b.step(&direct);
        for p in [0u8, 1] {
            let (sa, sb) = (a.player_state(p).unwrap(), b.player_state(p).unwrap());
            assert_eq!(sa.to_bytes(), sb.to_bytes(), "tick {t} player {p}");
            for byte in sa.to_bytes() {
                hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
            }
        }
    }
    eprintln!("golden trajectory hash (150 ticks, 2 players): {hash:016x}");
    assert_eq!(rejected(&m, "move_clamped"), 0);
    assert_eq!(rejected(&m, "yaw_rate"), 0);
    assert_eq!(m.input_missing_ticks_total.get(), 0);
}

// ------------------------------------------------ acceptance 3: validation counters and the kick

/// Runs `f` on a fresh bridge with one joined player and returns the metrics.
fn with_player(cfg: ValidateCfg, f: impl FnOnce(&mut SimBridge, &mut World)) -> Arc<Metrics> {
    let m = Metrics::new();
    let mut world = world_with(&CollisionData::empty());
    let mut br = SimBridge::new(m.clone(), None, None, cfg);
    br.join(&mut world, 3, Team::Ger).unwrap();
    f(&mut br, &mut world);
    m
}

fn only(m: &Metrics, reason: &str, n: u64) {
    for l in Reject::LABELS {
        let want = if *l == reason { n } else { 0 };
        assert_eq!(rejected(m, l), want, "{l} (expected only {reason} = {n})");
    }
}

#[test]
fn each_rejection_reason_has_its_own_counter() {
    let d = ValidateCfg::default();
    // seq replay
    let m = with_player(d, |b, _| {
        b.ingest_cmd(3, &cmd(10, 5), 10);
        b.ingest_cmd(3, &cmd(11, 5), 10);
        b.ingest_cmd(3, &cmd(11, 4), 10);
    });
    only(&m, "duplicate", 2);
    assert_eq!(m.input_accepted_total.get(), 1);
    // seq wrap is monotonic
    let m = with_player(d, |b, _| {
        for (i, s) in [65534u16, 65535, 0, 1].into_iter().enumerate() {
            b.ingest_cmd(3, &cmd(10 + i as u32, s), 10);
        }
    });
    only(&m, "duplicate", 0);
    assert_eq!(m.input_accepted_total.get(), 4);
    // same tick again with a newer seq: the slot is taken
    let m = with_player(d, |b, _| {
        b.ingest_cmd(3, &cmd(10, 1), 10);
        b.ingest_cmd(3, &cmd(10, 2), 10);
    });
    only(&m, "duplicate", 1);
    // tick not advancing with seq
    let m = with_player(d, |b, _| {
        b.ingest_cmd(3, &cmd(12, 1), 10);
        b.ingest_cmd(3, &cmd(11, 2), 10);
    });
    only(&m, "tick_regress", 1);
    // window
    let m = with_player(d, |b, _| {
        b.ingest_cmd(3, &cmd(7, 1), 10);
        b.ingest_cmd(3, &cmd(8, 2), 10); // exactly now - 2: fine
    });
    only(&m, "too_old", 1);
    let m = with_player(d, |b, _| {
        b.ingest_cmd(3, &cmd(21, 1), 10);
        b.ingest_cmd(3, &cmd(20, 2), 10); // exactly now + 10: fine
    });
    only(&m, "too_far", 1);
    // ring overwrite: 10 and 18 share a slot
    let m = with_player(d, |b, _| {
        b.ingest_cmd(3, &cmd(10, 1), 10);
        b.ingest_cmd(3, &cmd(18, 2), 10);
    });
    only(&m, "ring_overwrite", 1);
    assert_eq!(m.input_accepted_total.get(), 2);
    // reserved bits, weapon slot, bad move axis
    let m = with_player(d, |b, _| {
        let mut c = cmd(10, 1);
        c.buttons = RESERVED_BUTTONS;
        b.ingest_cmd(3, &c, 10);
    });
    only(&m, "reserved_bits", 1);
    let m = with_player(d, |b, _| {
        let mut c = cmd(10, 1);
        c.weapon_slot = MAX_WEAPON_SLOT + 1;
        b.ingest_cmd(3, &c, 10);
    });
    only(&m, "weapon_slot", 1);
    let m = with_player(d, |b, _| {
        let mut c = cmd(10, 1);
        c.move_x = i8::MIN;
        b.ingest_cmd(3, &c, 10);
    });
    only(&m, "bad_move", 1);
    // move length 1.2: clamped, accepted
    let m = with_player(d, |b, w| {
        let mut c = cmd(1, 1);
        c.move_x = 127;
        c.move_z = 84;
        b.ingest_cmd(3, &c, 1);
        let r = b.resolve(1).to_vec();
        let len = (r[0].1.move_x as f32).hypot(r[0].1.move_z as f32) / 127.0;
        assert!(len <= 1.0 + 1e-6, "{len}");
        w.step(&r);
    });
    only(&m, "move_clamped", 1);
    assert_eq!(m.input_accepted_total.get(), 1);
    // turn rate: with the shipped 8000 deg/s nothing a 16-bit angle can express between two
    // ticks exceeds the limit (PI-022); with a lower cap the clamp and its counter work.
    let m = with_player(d, |b, _| {
        b.ingest_cmd(3, &cmd(10, 1), 10);
        let mut c = cmd(11, 2);
        c.yaw = 0x8000;
        c.pitch = i16::MAX;
        b.ingest_cmd(3, &c, 10);
    });
    only(&m, "yaw_rate", 0);
    let low = ValidateCfg {
        max_turn_dps: 2000.0, // 66.7 deg per tick
        ..d
    };
    let m = with_player(low, |b, _| {
        let mut first = cmd(10, 1);
        first.pitch = -20000;
        b.ingest_cmd(3, &first, 10);
        let mut c = cmd(11, 2);
        c.yaw = 0x4000; // +90 deg in one tick (> 266 deg/s * ... = 66.7)
        c.pitch = 20000; // +110 deg
        b.ingest_cmd(3, &c, 10);
    });
    assert_eq!(rejected(&m, "yaw_rate"), 1);
    assert_eq!(rejected(&m, "pitch_rate"), 1);
    // input for a player that never joined
    let m = with_player(d, |b, _| {
        b.ingest_cmd(99, &cmd(10, 1), 10);
    });
    only(&m, "no_session", 1);
}

#[test]
fn thirty_violations_in_a_row_kick_and_one_clean_command_resets_the_count() {
    let m = Metrics::new();
    let sessions = Sessions::with_reuse_delay(m.clone(), 4, 128, Duration::ZERO);
    let handle = ClientHandle::new(m.clone());
    let (p, team) = sessions.alloc_player_team(&handle).unwrap();
    let (_tx, rx) = crossbeam_channel::bounded(4);
    let mut world = world_with(&CollisionData::empty());
    let mut br = SimBridge::new(
        m.clone(),
        Some(NetLink {
            inbound: rx,
            sessions: sessions.clone(),
        }),
        None,
        ValidateCfg::default(),
    );
    br.join(&mut world, p, team).unwrap();

    let bad = |tick: u32, seq: u16| {
        let mut c = cmd(tick, seq);
        c.buttons = 1 << 15;
        c
    };
    for i in 0..KICK_STREAK - 1 {
        br.ingest_cmd(p, &bad(5 + (i % 5), i as u16), 5);
    }
    assert!(br.process_kicks(&mut world).is_empty(), "29 are tolerated");
    br.ingest_cmd(p, &cmd(6, 1000), 5); // clean: resets
    for i in 0..KICK_STREAK - 1 {
        br.ingest_cmd(p, &bad(7 + (i % 4), 2000 + i as u16), 5);
    }
    assert!(
        br.process_kicks(&mut world).is_empty(),
        "reset by the clean one"
    );
    assert!(!handle.is_closed());
    br.ingest_cmd(p, &bad(8, 3000), 5); // the 30th in a row
    assert_eq!(br.process_kicks(&mut world), vec![p]);
    assert!(handle.is_closed(), "connection closed");
    assert!(world.player_state(p).is_none(), "player removed");
    assert!(br.session(p).is_none());
    assert_eq!(m.session_kicked_total.get(), 1);
    assert_eq!(
        rejected(&m, "reserved_bits"),
        2 * (KICK_STREAK as u64 - 1) + 1
    );
    // Later commands of the kicked connection find no session.
    br.ingest_cmd(p, &cmd(7, 1), 6);
    assert_eq!(rejected(&m, "no_session"), 1);
    // The ClientGone that follows the close is harmless.
    assert!(!br.leave(&mut world, p));
}

// ------------------------------------------------ acceptance 6: fuzz

fn random_cmd(rng: &mut Rng, now: u32) -> InputCmd {
    let pick = |rng: &mut Rng| -> u64 { rng.next() };
    let mut c = InputCmd {
        seq: pick(rng) as u16,
        tick: match rng.below(6) {
            0 => pick(rng) as u32,
            1 => u32::MAX - rng.below(3) as u32,
            2 => 0,
            _ => (now + rng.below(20) as u32).saturating_sub(5),
        },
        move_x: pick(rng) as i8,
        move_z: pick(rng) as i8,
        yaw: pick(rng) as u16,
        pitch: pick(rng) as i16,
        buttons: if rng.below(3) == 0 {
            pick(rng) as u16
        } else {
            (pick(rng) as u16) & 0x3FFF
        },
        weapon_slot: if rng.below(4) == 0 {
            pick(rng) as u8
        } else {
            rng.below(5) as u8
        },
        aux: pick(rng) as u8,
        view_tick: pick(rng) as u16,
    };
    if rng.below(2) == 0 {
        c.seq = (now as u16).wrapping_add(rng.below(4) as u16);
    }
    c
}

#[test]
fn a_hundred_thousand_random_inputs_never_panic_and_resolved_input_is_always_legal() {
    let m = Metrics::new();
    let mut world = world_with(&CollisionData::empty());
    let mut br = bridge(&m, None);
    let mut rng = Rng(0x1234_5678_9ABC_DEF1);
    let ids: Vec<PlayerId> = (0..8).collect();
    for &p in &ids {
        br.join(&mut world, p, if p % 2 == 0 { Team::Us } else { Team::Ger })
            .unwrap();
    }
    let mut now = 1u32;
    let mut cases = 0u64;
    let mut decoded_bundles = 0u64;
    while cases < 100_000 {
        for _ in 0..40 {
            let p = ids[rng.below(8) as usize];
            match rng.below(5) {
                // Random bytes through the real decoder, then the bridge.
                0 => {
                    let len = rng.below(40) as usize;
                    let bytes: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
                    if let Ok(b) = decode_input_bundle(&bytes) {
                        decoded_bundles += 1;
                        br.ingest_bundle(p, &b, now);
                    }
                }
                // A structurally valid bundle with hostile field values.
                1 => {
                    let b = InputBundle {
                        ack_snapshot_tick: rng.next() as u16,
                        pred_hash: rng.next() as u8,
                        cmds: (0..1 + rng.below(3))
                            .map(|_| random_cmd(&mut rng, now))
                            .collect(),
                        fire: (0..rng.below(3))
                            .map(|_| frontline_protocol::msg::input::FireCmd {
                                seq: rng.next() as u16,
                                view_tick: rng.next() as u16,
                                weapon: rng.next() as u8,
                                dir_yaw: rng.next() as u16,
                                dir_pitch: rng.next() as u16,
                                origin_hint: [rng.below(0x4000) as u16; 3],
                                spread_seed: rng.next() as u16,
                            })
                            .collect(),
                    };
                    br.ingest_bundle(p, &b, now);
                }
                _ => {
                    br.ingest_cmd(p, &random_cmd(&mut rng, now), now);
                }
            }
            cases += 1;
        }
        // The sim thread's per-tick sequence.
        for k in br.process_kicks(&mut world) {
            br.join(&mut world, k, Team::Us).unwrap();
        }
        let inputs = br.resolve(now).to_vec();
        for (_, c) in &inputs {
            assert_eq!(c.buttons & RESERVED_BUTTONS, 0);
            assert!(c.weapon_slot <= MAX_WEAPON_SLOT);
            assert_ne!(c.move_x, i8::MIN);
            assert_ne!(c.move_z, i8::MIN);
            assert!(
                (c.move_x as f32).hypot(c.move_z as f32) <= MAX_MOVE_LEN + 1e-3,
                "{c:?}"
            );
        }
        world.step(&inputs);
        now += 1;
    }
    let total: u64 = Reject::LABELS.iter().map(|l| rejected(&m, l)).sum();
    eprintln!(
        "fuzz: {cases} inputs, {decoded_bundles} random byte strings decoded, {} accepted, {total} rejected/clamped, {} kicks",
        m.input_accepted_total.get(),
        m.session_kicked_total.get()
    );
    assert!(
        m.input_accepted_total.get() > 1000,
        "some commands must get through"
    );
    assert!(total > 1000, "hostile commands were counted");
}

#[test]
fn undecodable_bytes_are_not_this_layers_business_but_never_reach_it_as_panics() {
    // Exhaustive-ish: every 1- and 2-byte string plus random longer ones through the decoder.
    let mut ok = 0;
    for a in 0..=255u8 {
        let _ = decode_input_bundle(&[a]);
        for b in (0..=255u8).step_by(5) {
            if decode_input_bundle(&[a, b]).is_ok() {
                ok += 1;
            }
        }
    }
    let mut rng = Rng(77);
    for _ in 0..20_000 {
        let n = rng.below(64) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        let _ = decode_input_bundle(&bytes);
    }
    let _ = (ok, Vec3::ZERO);
}
