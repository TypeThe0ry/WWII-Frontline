use std::collections::BTreeMap;

use frontline_sim::rules::ObjectiveSpec;
use frontline_sim::{CollisionChunk, LifeState, MatchPhaseKind, Vec3};

use super::*;
use crate::clock::fake::FakeClock;
use crate::gateway_link::result::MatchResult;
use crate::world_loader::{HqSpawn, MapSpawns};

const TICKS: u64 = 36_000;
const SCRIPTED_PER_TEAM: u8 = 5;
const BOTS: u16 = 2;

fn arena() -> Arc<LoadedMap> {
    let hq = [8.0, 56.0].map(|x| HqSpawn {
        x,
        z: 32.0,
        y: 5.02,
        radius_m: 10.0,
    });
    Arc::new(LoadedMap {
        name: "score-audit".into(),
        collision: CollisionData::from_chunks(vec![CollisionChunk::new(
            0,
            0,
            vec![5.0; 65 * 65],
            vec![0; 64 * 64],
        )]),
        spawns: MapSpawns {
            hq,
            center: (32.0, 32.0),
        },
        chunk_count: 1,
        patrol_points: vec![[32.0, 32.0]],
        size_m: 64.0,
        dir: Default::default(),
        objectives: [Team::Us, Team::Ger]
            .into_iter()
            .enumerate()
            .map(|(i, team)| ObjectiveSpec {
                id: i as u16,
                pos: Vec3::new(8.0 + i as f32 * 48.0, 5.02, 56.0),
                radius_m: 2.0,
                owner: Some(team),
            })
            .collect(),
        no_build_zones: Default::default(),
        resource_sites: Vec::new(),
        garrison_sites: [None; 2],
    })
}

#[test]
fn tactical_warmup_counts_connections_not_bots() {
    let mut cfg = SimThreadConfig::bare(17, MapId(0), GameMode::Assault);
    cfg.map_data = Some(arena());
    let (mut lp, _, commands) = build_loop_with_bots(
        FakeClock::new(),
        &cfg,
        BOTS,
        Metrics::new(),
        StopFlag::new(),
    );
    lp.tick();
    assert_eq!(lp.world.match_state().phase, MatchPhaseKind::Warmup);
    assert_eq!(lp.world.match_state().ends_at, 1800);
    commands
        .send(Command::Join {
            player: 0,
            team: Team::Us,
        })
        .unwrap();
    lp.tick();
    assert_eq!(lp.world.match_state().humans, 1);
    assert_eq!(lp.world.match_state().phase, MatchPhaseKind::Warmup);
    commands.send(Command::ClientGone(0)).unwrap();
    commands
        .send(Command::Join {
            player: 1,
            team: Team::Ger,
        })
        .unwrap();
    lp.tick();
    assert_eq!(lp.world.match_state().humans, 1);
    assert_eq!(lp.world.match_state().phase, MatchPhaseKind::Warmup);
    commands
        .send(Command::Join {
            player: 2,
            team: Team::Us,
        })
        .unwrap();
    lp.tick();
    assert_eq!(lp.world.match_state().phase, MatchPhaseKind::Live);
    lp.tick();
    assert_eq!(lp.world.score_summary().duration_ticks, 1);

    cfg.mode = GameMode::Sandbox;
    let (mut sandbox, _, _) = build_loop(FakeClock::new(), &cfg, Metrics::new(), StopFlag::new());
    sandbox.tick();
    assert_eq!(sandbox.world.match_state().phase, MatchPhaseKind::Idle);
}

#[test]
#[ignore = "20 simulated minutes; run explicitly as a scoring audit"]
fn score_audit_5v5_plus_bots_36000_ticks() {
    let mut cfg = SimThreadConfig::bare(17, MapId(0), GameMode::Assault);
    cfg.map_data = Some(arena());
    let (mut lp, _, commands) = build_loop_with_bots(
        FakeClock::new(),
        &cfg,
        BOTS,
        Metrics::new(),
        StopFlag::new(),
    );
    for player in 0..2 * SCRIPTED_PER_TEAM {
        let team = if player < SCRIPTED_PER_TEAM {
            Team::Us
        } else {
            Team::Ger
        };
        commands
            .send(Command::SpawnPlayer {
                req: SpawnReq {
                    player,
                    team,
                    pos: Vec3::new(
                        24.0 + 6.0 * f32::from(team as u8),
                        5.02,
                        10.0 + 4.0 * f32::from(player % SCRIPTED_PER_TEAM),
                    ),
                    yaw: 0.0,
                },
            })
            .unwrap();
    }
    // The transition step starts in Warmup, so it does not count toward scoring time.
    lp.tick();
    assert_eq!(lp.world.match_state().phase, MatchPhaseKind::Live);
    let weapon = lp
        .world
        .config()
        .data
        .weapons
        .iter()
        .position(|w| w.id == "us_m1_garand")
        .unwrap() as u16;
    let mut observed = BTreeMap::<PlayerId, [i64; 4]>::new();
    let mut count = 0;
    for tick in 1..=TICKS {
        // Exercise the real damage and credit systems with predictable bursts of combat.
        if tick % 900 == 30 {
            let shooter = (tick / 900 % u64::from(SCRIPTED_PER_TEAM)) as u8;
            let target = shooter + SCRIPTED_PER_TEAM;
            if [shooter, target]
                .into_iter()
                .all(|p| matches!(lp.world.player_life(p), Some(LifeState::Alive)))
            {
                let z = 10.0 + 4.0 * f32::from(shooter);
                let source = Vec3::new(24.0, 5.02, z);
                lp.world.debug_teleport(shooter, source);
                lp.world.debug_teleport(target, source + Vec3::X * 6.0);
                lp.world.debug_set_hp(target, 10.0);
                lp.world
                    .spawn_projectile(shooter, weapon, source + Vec3::Y * 1.65, Vec3::X * 850.0)
                    .unwrap();
            }
        }
        lp.tick();
        for event in &lp.step_events {
            if let SimEvent::ScoreDelta(delta) = event {
                observed.entry(delta.player).or_default()[delta.category as usize] +=
                    i64::from(delta.value);
                count += 1;
            }
        }
    }
    assert!(
        observed
            .iter()
            .any(|(&p, c)| p < SCRIPTED_PER_TEAM && c[0] > 0),
        "scripted combat did not produce positive score"
    );
    assert_eq!(lp.bot_host().unwrap().len(), usize::from(BOTS));
    assert_eq!(lp.world.match_state().phase, MatchPhaseKind::Ended);
    let report = lp.report();
    assert_eq!(report.ticks, TICKS + 1);
    let result = MatchResult::from_report(u16::from(2 * SCRIPTED_PER_TEAM), BOTS, 0, &report);
    let summary = result.score_summary.unwrap();
    assert_eq!(u64::from(summary.duration_ticks), TICKS);
    assert_eq!(
        summary.players.len(),
        usize::from(2 * SCRIPTED_PER_TEAM) + usize::from(BOTS)
    );
    for player in &summary.players {
        let categories = observed.remove(&player.player).unwrap_or_default();
        assert_eq!(player.categories, categories, "player {}", player.player);
        assert_eq!(
            player.total_score,
            categories.into_iter().sum::<i64>(),
            "player {}",
            player.player
        );
    }
    assert!(
        observed.is_empty(),
        "a scoring player was missing from the summary"
    );
    let total: i64 = summary.players.iter().map(|p| p.total_score).sum();
    eprintln!(
        "score audit: {TICKS} ticks, {} players, {count} deltas, {total} net points",
        summary.players.len()
    );
}
