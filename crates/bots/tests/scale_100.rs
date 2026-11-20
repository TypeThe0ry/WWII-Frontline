//! M2-029 acceptance. Run the opt-in match-length cases with:
//! `cargo test --release -p frontline-bots --test scale_100 -- --ignored --nocapture --test-threads=1`.

mod scale_support;

use std::sync::Arc;

use frontline_bots::perception::{EYE_CROUCH_M, EYE_STAND_M, TORSO_Y};
use frontline_bots::scheduler::BotRuntime;
use frontline_bots::{BotMap, SoldierBrain, Tuning};
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::view::PlayerView;
use frontline_sim::{PlayerId, Team, Vec3, World};
use scale_support::{context, spawn, Battle, Terrain, HZ, SEED};

const BOTS: u8 = 100;

// Reject malformed input here and model accepted turn clamps (e.g. spawn-facing changes) before
// the command reaches sim. The exact human/bot ingest path is exercised by server/bots_validation;
// depending on server here would create a cycle.
fn check_input(
    id: PlayerId,
    tick: u32,
    cmd: &mut InputCmd,
    prev: &mut Option<InputCmd>,
    cap: f32,
) -> bool {
    let at = || format!("bot {id}, tick {tick}, {cmd:?}");
    assert_eq!(cmd.tick, tick, "{}", at());
    assert_eq!(cmd.buttons & 0xC000, 0, "{}", at());
    assert!(cmd.weapon_slot <= 4, "{}", at());
    assert_ne!(cmd.move_x, i8::MIN, "{}", at());
    assert_ne!(cmd.move_z, i8::MIN, "{}", at());
    let x = i32::from(cmd.move_x);
    let z = i32::from(cmd.move_z);
    assert!(x * x + z * z <= 128 * 128, "{}", at());
    let mut clamped = false;
    if let Some(p) = *prev {
        assert_eq!(cmd.seq, p.seq.wrapping_add(1), "{}", at());
        let yaw = cmd.yaw.wrapping_sub(p.yaw) as i16;
        let pitch = i32::from(cmd.pitch) - i32::from(p.pitch);
        if f32::from(yaw.unsigned_abs()) * (360.0 / 65536.0) > cap {
            let allowed = (cap * (65536.0 / 360.0)) as i16;
            cmd.yaw = p.yaw.wrapping_add((allowed * yaw.signum()) as u16);
            clamped = true;
        }
        if pitch.unsigned_abs() as f32 * (180.0 / 65535.0) > cap {
            let allowed = (cap * (65535.0 / 180.0)) as i32;
            cmd.pitch = (i32::from(p.pitch) + allowed * pitch.signum()) as i16;
            clamped = true;
        }
    }
    *prev = Some(*cmd);
    clamped
}

#[test]
fn scale_100_deterministic_inputs_and_world_hashes() {
    const TICKS: u32 = 3_000;
    let terrain = Terrain::load();
    let rules: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/rules/anticheat.json")).unwrap();
    let cap = rules["view_max_deg_s"].as_f64().unwrap() as f32 / f32::from(HZ);
    let mut hashes = Vec::with_capacity(TICKS as usize);
    eprintln!(
        "PI-109 workstation/non-reference: {} {BOTS} bots, seed {SEED:#x}",
        terrain.name
    );
    for run in 0..2 {
        let mut battle = Battle::new(&terrain, BOTS);
        let mut runtime = BotRuntime::deterministic();
        let mut prev = [None; BOTS as usize];
        let mut clamps = 0;
        for tick in 1..=TICKS {
            battle.step(Some(&mut runtime), |_, _, commands| {
                assert_eq!(commands.len(), BOTS as usize);
                for (i, (id, cmd)) in commands.iter_mut().enumerate() {
                    assert_eq!(usize::from(*id), i, "unexpected slot at tick {tick}");
                    clamps += usize::from(check_input(*id, tick, cmd, &mut prev[i], cap));
                }
            });
            let hash = battle.world.state_hash();
            if run == 0 {
                hashes.push(hash);
            } else {
                assert_eq!(
                    hash,
                    hashes[tick as usize - 1],
                    "state diverged at tick {tick}"
                );
            }
        }
        assert!(battle.shots > 0, "no combat in replay {run}");
        eprintln!(
            "replay {}: {TICKS} ticks / {} legal inputs ({clamps} with accepted turn clamps); {} shots, {} kills, {} spawns, final state {:016x}",
            run + 1,
            u32::from(BOTS) * TICKS,
            battle.shots,
            battle.kills,
            battle.spawns,
            hashes.last().unwrap()
        );
    }
}

#[test]
#[ignore = "10 simulated minutes; run in release mode"]
fn scale_100_stuck_10min() {
    const TICKS: u32 = 18_000;
    let terrain = Terrain::load();
    let mut battle = Battle::new(&terrain, BOTS);
    let mut runtime = BotRuntime::default();
    let mut stuck = 0;
    let mut timeouts = 0;
    let mut moving = [false; BOTS as usize];
    eprintln!(
        "PI-109 workstation/non-reference: {} {BOTS} bots, seed {SEED:#x}",
        terrain.name
    );
    for _ in 0..TICKS {
        let m = battle
            .step(Some(&mut runtime), |_, _, commands| {
                for &(id, c) in commands.iter() {
                    moving[usize::from(id)] |= c.move_x != 0 || c.move_z != 0;
                }
            })
            .unwrap();
        stuck += m.stuck;
        timeouts += m.timeouts;
    }
    let per_bot: Vec<_> = battle.bots.iter().map(|(_, _, b)| b.stuck_total).collect();
    let offenders: Vec<_> = per_bot
        .iter()
        .enumerate()
        .filter(|&(_, &count)| count > 2)
        .collect();
    eprintln!(
        "{TICKS} ticks; {} shots; runtime stuck {stuck}, timeouts {timeouts}; per-bot stuck counts (ID order): {per_bot:?}",
        battle.shots
    );
    assert!(
        moving.into_iter().all(|m| m),
        "some bots never requested movement"
    );
    assert!(battle.shots > 0, "no combat in soak");
    assert_eq!(
        stuck,
        per_bot.iter().map(|&n| u64::from(n)).sum::<u64>(),
        "runtime stuck telemetry disagrees with the brains"
    );
    assert!(
        offenders.is_empty(),
        "over two stuck detections: {offenders:?}"
    );
}

#[derive(Default)]
struct Recognition {
    last: Option<(PlayerId, u32)>,
    pending: Option<(PlayerId, u32)>,
    latencies: Vec<u32>,
    acquisitions: usize,
    lost: usize,
}

impl Recognition {
    fn record(&mut self, cmd: InputCmd, brain: &SoldierBrain, target: Option<PlayerId>) {
        // The marker advances only when a fresh acquisition starts a reaction timer. A persistent
        // sighting does not reset the timer, and a re-acquisition of the same ID is a new sample.
        if let Some((id, tick)) = brain.last_recognition().filter(|v| Some(*v) != self.last) {
            self.last = Some((id, tick));
            assert_eq!(tick, cmd.tick);
            self.acquisitions += 1;
            self.lost += usize::from(self.pending.replace((id, tick)).is_some());
        }
        if let Some((id, tick)) = self.pending {
            if target != Some(id) {
                self.lost += 1;
                self.pending = None;
            } else if cmd.buttons & buttons::FIRE != 0 {
                self.latencies.push(cmd.tick - tick);
                self.pending = None;
            }
        }
    }
}

fn can_fire_at(world: &World, from: &PlayerView, to: &PlayerView, cmd: InputCmd) -> bool {
    let height = if cmd.buttons & buttons::CROUCH != 0 {
        EYE_CROUCH_M
    } else {
        EYE_STAND_M
    };
    from.team != to.team
        && world
            .view()
            .los(from.pos + Vec3::Y * height, to.pos + Vec3::Y * TORSO_Y)
}

fn median(mut ticks: Vec<u32>) -> f64 {
    assert!(!ticks.is_empty(), "no acquisition-to-FIRE samples");
    ticks.sort_unstable();
    let n = ticks.len();
    (f64::from(ticks[(n - 1) / 2]) + f64::from(ticks[n / 2])) / 2.0
}

fn latency_run(terrain: &Terrain, optimized: bool) -> (f64, Vec<Option<f64>>, usize) {
    const TICKS: u32 = 9_000;
    let mut battle = Battle::new(terrain, BOTS);
    let mut runtime = optimized.then(BotRuntime::default);
    let mut recognition: Vec<_> = (0..BOTS).map(|_| Recognition::default()).collect();
    let (mut fire, mut invalid_fire, mut timeouts) = (0, 0, 0);
    let mut first_violation = None;
    for _ in 0..TICKS {
        let m = battle.step(runtime.as_mut(), |world, bots, commands| {
            let mut alive = [None; BOTS as usize];
            for p in world.view().players() {
                alive[usize::from(p.player)] = Some(p);
            }
            for ((id, _, brain), &(_, cmd)) in bots.iter().zip(commands.iter()) {
                let target = brain.target_player();
                recognition[usize::from(*id)].record(cmd, brain, target);
                if cmd.buttons & buttons::FIRE != 0 {
                    fire += 1;
                    let visible = alive[usize::from(*id)]
                        .zip(target.and_then(|p| alive[usize::from(p)]))
                        .is_some_and(|(from, to)| can_fire_at(world, from, to, cmd));
                    if !visible {
                        invalid_fire += 1;
                        first_violation.get_or_insert((cmd.tick, *id, target));
                    }
                }
            }
        });
        timeouts += m.map_or(0, |m| m.timeouts);
    }
    let all: Vec<_> = recognition
        .iter()
        .flat_map(|r| r.latencies.iter().copied())
        .collect();
    let (count, acquired, lost, pending) = (
        all.len(),
        recognition.iter().map(|r| r.acquisitions).sum::<usize>(),
        recognition.iter().map(|r| r.lost).sum::<usize>(),
        recognition.iter().filter(|r| r.pending.is_some()).count(),
    );
    assert_eq!(acquired, count + lost + pending);
    let per_bot: Vec<_> = recognition
        .into_iter()
        .map(|r| (!r.latencies.is_empty()).then(|| median(r.latencies)))
        .collect();
    let with_samples = per_bot.iter().flatten().count();
    let label = if optimized { "runtime" } else { "legacy" };
    eprintln!(
        "{label}: {TICKS} ticks, {acquired} acquisitions, {count} paired samples ({with_samples}/{BOTS} bots), {lost} canceled, {pending} pending at end; {fire} FIRE commands, {invalid_fire} without LoS (first: {first_violation:?}), {} sim shots, {timeouts} timeouts",
        battle.shots
    );
    assert!(
        fire > 0 && battle.shots > 0 && count > 0,
        "vacuous fire/latency audit"
    );
    (median(all), per_bot, invalid_fire)
}

#[test]
#[ignore = "two full five-minute matches; run in release mode"]
fn scale_100_recognition_to_fire_5min() {
    let terrain = Terrain::load();
    eprintln!(
        "PI-109 workstation/non-reference: {} {BOTS} bots, seed {SEED:#x}",
        terrain.name
    );
    let (pre, pre_bots, pre_violations) = latency_run(&terrain, false);
    let (post, post_bots, post_violations) = latency_run(&terrain, true);
    assert!(pre > 0.0, "legacy median must be positive");
    let delta = (post - pre).abs() / pre;
    let paired: Vec<_> = pre_bots
        .into_iter()
        .zip(post_bots)
        .enumerate()
        .filter_map(|(id, (a, b))| a.zip(b).map(|(a, b)| (id, a, b)))
        .collect();
    eprintln!(
        "recognition-to-FIRE: median {pre:.1} -> {post:.1} ticks ({:.2}% absolute change), {} paired per-bot medians (id, legacy ticks, runtime ticks): {paired:?}",
        delta * 100.0,
        paired.len()
    );
    assert_eq!(
        (pre_violations, post_violations),
        (0, 0),
        "FIRE with no visible current enemy"
    );
    assert!(
        delta <= 0.10,
        "aggregate latency median changed by more than 10%"
    );
}

#[test]
fn scale_los_fire_gate_with_live_control() {
    let terrain = Terrain::graybox();
    let mut open = terrain.world();
    let mut blocked = terrain.world();
    for (w, target_z) in [(&mut open, 122.0), (&mut blocked, 135.0)] {
        spawn(w, 0, Team::Us, 92.0, 115.0);
        spawn(w, 1, Team::Ger, 92.0, target_z);
        // Publish the initial positions through the same WorldView as the match harness.
        w.step(&[]);
    }
    let sight = |w: &World| {
        let p = w.view().players();
        w.view().los(
            p[0].pos + Vec3::Y * EYE_STAND_M,
            p[1].pos + Vec3::Y * TORSO_Y,
        )
    };
    assert!(sight(&open));
    assert!(!sight(&blocked));
    let map = Arc::new(BotMap::without_nav(Tuning::embedded(), Vec::new()));
    let mut brain = SoldierBrain::with_tactics(map.clone());
    let mut control_brain = SoldierBrain::with_tactics(map);
    let mut runtime = BotRuntime::deterministic();
    let mut control = BotRuntime::deterministic();
    // Freeze the sim/weapon snapshots; only the command clock advances. Both see the same
    // target until the first FIRE, then a real terrain barrier hides it for the test bot only.
    let drive = |r: &mut BotRuntime, b: &mut SoldierBrain, w: &World, tick| {
        let mut ctx = context(w, 0, Team::Us, tick, &[]);
        ctx.self_weapon.as_mut().unwrap().ads_units = 1_000_000;
        r.begin_tick(w.view(), tick);
        r.plan(&[0]);
        let cmd = r.think(b, w.view(), &ctx);
        r.finish_tick();
        cmd.buttons & buttons::FIRE != 0
    };
    let first = (1..=180)
        .find(|&tick| {
            let a = drive(&mut runtime, &mut brain, &open, tick);
            assert_eq!(a, drive(&mut control, &mut control_brain, &open, tick));
            a
        })
        .expect("open LoS must produce FIRE before the occlusion phase");
    assert_eq!(brain.target_player(), Some(1));
    let mut opportunities = 0;
    for tick in first + 1..=first + 12 {
        opportunities += usize::from(drive(&mut control, &mut control_brain, &open, tick));
        assert!(
            !drive(&mut runtime, &mut brain, &blocked, tick),
            "FIRE into occlusion on tick {tick}"
        );
        if tick == first + 1 {
            assert_eq!(
                brain.target_player(),
                Some(1),
                "occlusion must exercise a remembered target"
            );
        }
    }
    assert!(
        opportunities > 0,
        "control must still be firing during the occlusion phase"
    );
    eprintln!(
        "graybox LoS: first FIRE {first}; control {opportunities} vs occluded 0 subsequent FIRE commands"
    );
}
