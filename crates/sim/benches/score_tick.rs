//! Isolated scoring-stage workload: 100 observed participants, 50 mixed scoring events per tick.
use std::{hint::black_box, time::Instant};

use frontline_sim::{
    score::{Action, Actor, ScoreEngine},
    GameData, SimEvent, Team,
};

const TICKS: u32 = 36_000;
const COUNT: u8 = 100;
const EVENTS: usize = 50;

fn main() {
    let mut engine = ScoreEngine::new(30, GameData::embedded().roles);
    let actors: Vec<_> = (0..COUNT)
        .map(|p| Actor {
            role: engine.role(["rifleman", "medic", "officer", "support"][p as usize % 4]),
            team: if p < 50 { Team::Us } else { Team::Ger },
            squad: Some(u16::from(p % 10)),
            alive: true,
            afk: false,
        })
        .collect();
    let actions = [
        Action::Heal(80),
        Action::GarrisonSpawn { forward: true },
        Action::AmmoCrateUse,
        Action::Build(80),
        Action::Repair(60),
        Action::ModuleRepair,
        Action::Resupply,
    ];
    let mut ns = Vec::with_capacity(TICKS as usize);
    let roster: Vec<_> = (0..COUNT).map(|p| (p, actors[p as usize].team)).collect();
    let mut score = 0i64;
    for tick in 1..=TICKS + 600 {
        let start = Instant::now();
        engine.advance(tick);
        for (p, actor) in actors.iter().copied().enumerate() {
            engine.observe(p as u8, actor);
        }
        for i in 0..EVENTS {
            let index = tick as usize + i;
            let out = engine.on_event(black_box(&SimEvent::ScoreAction {
                player: (index % usize::from(COUNT)) as u8,
                action: actions[index % actions.len()],
            }));
            score += out.iter().map(|d| i64::from(d.value)).sum::<i64>();
        }
        for (point, present) in roster.chunks(34).enumerate() {
            let out = engine.objective(
                point as u16,
                Some(Team::Ger),
                Some(Team::Us),
                0.0,
                None,
                black_box(present),
            );
            score += out.iter().map(|d| i64::from(d.value)).sum::<i64>();
        }
        if tick > 600 {
            ns.push(start.elapsed().as_nanos());
        }
    }
    ns.sort_unstable();
    let expected: i64 = engine
        .summary(0, "frontline", None, None)
        .players
        .iter()
        .map(|p| p.total_score)
        .sum();
    assert_eq!(score, expected);
    let pct = |p| ns[ns.len() * p / 100 - 1] as f64 / 1000.0;
    println!("score_tick: {COUNT} players, {EVENTS} events/tick, {TICKS} samples, p50={:.3} us p95={:.3} us p99={:.3} us max={:.3} us, audited score={score}", pct(50), pct(95), pct(99), pct(100));
}
