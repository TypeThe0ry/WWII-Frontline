//! Isolated M2-018 cost with 100 online humans (`cargo bench -p frontline-sim --bench serverrules_tick`).
use std::hint::black_box;
use std::time::Instant;

use frontline_protocol::msg::InputCmd;
use frontline_sim::serverrules::{Activity, PlayerFacts, RuleContext, ServerRules};
use frontline_sim::{Team, Tick};

fn measure(ingest_input: bool) {
    let roster: Vec<_> = (0..100)
        .map(|id| PlayerFacts {
            id,
            team: if id < 50 { Team::Us } else { Team::Ger },
            is_bot: false,
            alive: id % 10 != 0,
            in_vehicle: false,
            driving: None,
            joined_tick: Tick(0),
            last_switch_tick: None,
            squad_size: 5,
            is_squad_leader: id % 5 == 0,
            commander_since: None,
            is_officer: false,
            score: 0,
            is_admin: false,
            party: None,
        })
        .collect();
    let mut samples = Vec::new();
    for sample in 0..=50 {
        let mut rules = ServerRules::default();
        let mut run = |start: u32, end: u32| {
            for tick in start..end {
                let ctx = RuleContext {
                    now: Tick(tick),
                    match_started: Tick(0),
                    tick_hz: 30,
                    match_seed: 5,
                    roster: &roster,
                };
                if ingest_input {
                    for p in &roster {
                        // A mix of held movement, changing input every tick, and idle packets.
                        let phase = tick.wrapping_mul(101).wrapping_add(u32::from(p.id) * 199);
                        let cmd = InputCmd {
                            seq: tick as u16,
                            tick,
                            move_x: if p.id % 3 == 0 {
                                (phase & 127) as i8
                            } else {
                                0
                            },
                            move_z: if p.id % 3 == 1 { 127 } else { 0 },
                            yaw: if p.id % 3 == 2 { 0 } else { phase as u16 },
                            pitch: 0,
                            buttons: 0,
                            weapon_slot: 0,
                            aux: 0,
                            view_tick: 0,
                        };
                        rules.record_activity(
                            black_box(&ctx),
                            p.id,
                            black_box(Activity::Input(cmd)),
                        );
                    }
                }
                black_box(rules.step(black_box(&ctx)));
            }
        };
        run(0, 400);
        let start = Instant::now();
        run(400, 1400);
        if sample != 0 {
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    samples.sort_by(f64::total_cmp);
    let label = if ingest_input {
        "input + step"
    } else {
        "step only"
    };
    println!(
        "{label:12} ({} samples x 1000 ticks): min {:.2}, p50 {:.2}, p95 {:.2}, max {:.2} us/tick",
        samples.len(),
        samples[0],
        (samples[24] + samples[25]) / 2.0,
        samples[47],
        samples[49]
    );
}

fn main() {
    measure(false);
    measure(true);
}
