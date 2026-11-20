use super::*;
use proptest::prelude::*;

fn actor(team: Team, role: u8) -> Actor {
    Actor {
        role,
        team,
        squad: Some(1),
        alive: true,
        afk: false,
    }
}

fn engine() -> ScoreEngine {
    let mut e = ScoreEngine::new(30, crate::GameData::embedded().roles);
    e.advance(1);
    for p in 0..4 {
        e.observe(
            p,
            actor(
                if p == 1 { Team::Ger } else { Team::Us },
                e.role("rifleman"),
            ),
        );
    }
    e
}

fn action(e: &mut ScoreEngine, p: u8, action: Action) -> Deltas {
    e.on_event(&SimEvent::ScoreAction { player: p, action })
}

fn kill(killer: u8, victim: u8, assists: &[u8]) -> Kill<'_> {
    Kill {
        killer,
        victim,
        assists,
        kind: CreditKind::Direct,
        team_kill: false,
        headshot: true,
        dist_mm: 42_001,
        defense: true,
    }
}

fn points(out: &[ScoreDelta], kind: K) -> i32 {
    out.iter()
        .filter(|d| d.kind == kind)
        .map(|d| i32::from(d.value))
        .sum()
}

macro_rules! table_tests {
    ($($kind:ident => $value:literal),+ $(,)?) => { $(
        #[allow(non_snake_case)]
        #[test]
        fn $kind() {
            let mut e = engine();
            let rule = e.rules.awards[&K::$kind];
            assert_eq!(rule.points, $value);
            let mut d = Deltas::new();
            e.award(0, K::$kind, rule.units, &mut d);
            if rule.enabled {
                assert_eq!(&d[..], &[ScoreDelta { player: 0, kind: K::$kind, category: K::$kind.category(), value: $value, role_at_time: e.role("rifleman") }]);
            } else { assert!(d.is_empty()); }
        }
    )+ };
}

mod score_table {
    use super::{engine, Deltas, ScoreDelta, K};
    table_tests! {
        Kill => 10, Headshot => 5, Assist => 5, Finish => 3, LightVehicle => 40,
        MediumVehicle => 60, HeavyVehicle => 80, SoftVehicle => 20, CrewMember => 15,
        GunDestroyed => 30, KillStreak => 10, SuppressionAssist => 3, TeamKill => -30,
        Revenge => 5, CaptureAdvance => 2, Capture => 50, AssaultCapture => 80,
        ForwardGarrisonSpawn => 2, GarrisonDestroyed => 40, NodeDestroyed => 30,
        ContestedDefense => 2, DefenseKill => 5, Repel => 30, GarrisonDefense => 10,
        Revive => 20, RepeatRevive => 10, Heal => 1, Resupply => 3, AmmoCrateUse => 3,
        Build => 1, NodeBuilt => 30, GarrisonBuild => 1, GarrisonBuilt => 20,
        GarrisonSpawn => 1, OutpostSpawn => 1, SupplyDelivery => 10, Repair => 1,
        ModuleRepair => 5, SpotAssist => 5, SpotterAssist => 5, ReconFlight => 3,
        ArtilleryKill => 10, ArtilleryLoad => 5, ArtilleryObserve => 5, CommanderStrike => 5,
        CommanderRally => 1, CommanderSupply => 10,
    }
}

#[test]
fn combat_windows_and_kda() {
    let mut e = engine();
    for n in 1..=4 {
        e.advance(n * 30);
        let mut out = Deltas::new();
        e.death(1);
        e.credit(kill(0, 1, &[2]), &mut out);
        let combat: i32 = out
            .iter()
            .filter(|d| d.category == Category::Combat)
            .map(|d| i32::from(d.value))
            .sum();
        assert_eq!(combat, [20, 20, 30, 0][n as usize - 1]);
        assert_eq!(points(&out, K::DefenseKill), 5);
    }
    let mut out = Deltas::new();
    e.credit(kill(1, 0, &[]), &mut out);
    assert_eq!(points(&out, K::Revenge), 5);
    out.clear();
    e.advance(1800 + 120);
    e.credit(kill(0, 1, &[]), &mut out);
    assert_eq!(points(&out, K::Kill), 10);
    out.clear();
    e.credit(
        Kill {
            kind: CreditKind::Finish,
            ..kill(0, 1, &[2])
        },
        &mut out,
    );
    assert_eq!(out.iter().map(|d| d.value).collect::<Vec<_>>(), [3]);
    out.clear();
    e.credit(
        Kill {
            team_kill: true,
            ..kill(0, 2, &[1])
        },
        &mut out,
    );
    assert_eq!(out.iter().map(|d| d.value).collect::<Vec<_>>(), [-30]);
    let s = e.summary(0, "frontline", None, None);
    assert_eq!(
        (
            s.players[0].kills,
            s.players[1].deaths,
            s.players[2].assists
        ),
        (5, 4, 4)
    );
    assert_eq!(
        (s.players[0].headshot_bps, s.players[0].farthest_kill_mm),
        (10_000, 42_001)
    );
}

#[test]
fn capture_and_defense_tick_boundaries() {
    let mut e = engine();
    let participants = [(0, Team::Us), (1, Team::Ger)];
    let mut out = Deltas::new();
    for t in 1..=600 {
        e.advance(t);
        out.extend(e.objective(
            10,
            Some(Team::Ger),
            Some(Team::Us),
            50.0,
            (t == 600).then_some(Team::Us),
            &participants,
        ));
    }
    assert_eq!(points(&out, K::CaptureAdvance), 8);
    assert_eq!(points(&out, K::ContestedDefense), 8);
    assert_eq!(points(&out, K::Capture), 50);
    out.clear();
    for t in 601..=1200 {
        e.advance(t);
        out.extend(e.objective(
            10,
            Some(Team::Us),
            Some(Team::Ger),
            -100.0,
            (t == 1200).then_some(Team::Ger),
            &participants,
        ));
    }
    assert_eq!(
        points(&out, K::CaptureAdvance) + points(&out, K::Capture),
        0
    );
    e.advance(1201);
    assert!(e
        .objective(
            10,
            Some(Team::Ger),
            Some(Team::Us),
            51.0,
            None,
            &participants
        )
        .is_empty());
    e.advance(1202);
    assert_eq!(
        points(
            &e.objective(
                10,
                Some(Team::Ger),
                Some(Team::Ger),
                0.0,
                None,
                &participants
            ),
            K::Repel
        ),
        30
    );
}

#[test]
fn caps_repeat_afk_and_role_at_time() {
    let mut e = engine();
    let officer = e.role("officer");
    let medic = e.role("medic");
    e.observe(0, actor(Team::Us, officer));
    let mut out = Deltas::new();
    for _ in 0..25 {
        for a in [
            Action::GarrisonSpawn { forward: true },
            Action::OutpostSpawn,
            Action::AmmoCrateUse,
        ] {
            out.extend(action(&mut e, 0, a));
        }
    }
    for (k, v) in [
        (K::ForwardGarrisonSpawn, 20),
        (K::GarrisonSpawn, 15),
        (K::OutpostSpawn, 10),
        (K::AmmoCrateUse, 30),
    ] {
        assert_eq!(points(&out, k), v);
    }
    e.advance(1800);
    assert!(action(&mut e, 0, Action::GarrisonSpawn { forward: true }).is_empty());
    e.advance(1801);
    assert_eq!(
        action(&mut e, 0, Action::GarrisonSpawn { forward: true }).len(),
        2
    );
    e.observe(0, actor(Team::Us, medic));
    assert_eq!(action(&mut e, 0, Action::Heal(500))[0].value, 10);
    let mut revives = Deltas::new();
    e.revive(0, 2, &mut revives);
    e.advance(5401);
    e.revive(0, 2, &mut revives);
    assert_eq!(
        revives
            .iter()
            .map(|d| (d.role_at_time, d.value))
            .collect::<Vec<_>>(),
        [(medic, 20), (medic, 10)]
    );
    e.observe(
        0,
        Actor {
            afk: true,
            ..actor(Team::Us, medic)
        },
    );
    assert!(action(&mut e, 0, Action::Heal(40)).is_empty());
    let row = &e.summary(1, "frontline", None, None).players[0];
    assert_eq!(
        row.role_xp
            .iter()
            .find(|r| r.role == medic)
            .unwrap()
            .xp_hundredths,
        6000
    );
    assert_eq!(row.total_score, row.categories.iter().sum::<i64>());
}

#[test]
fn summary_schema_and_session_isolation() {
    let mut e = engine();
    action(
        &mut e,
        0,
        Action::GarrisonPlaced {
            id: 10,
            supplies: 50,
            forward: false,
        },
    );
    action(&mut e, 0, Action::Repair(21));
    assert_eq!(action(&mut e, 0, Action::Repair(9))[0].value, 1);
    e.disconnect(0);
    e.advance(2);
    e.observe(0, actor(Team::Us, e.role("support")));
    assert!(action(&mut e, 2, Action::SpawnedAtGarrison(10)).is_empty());
    let summary = e.summary(
        1,
        "frontline",
        Some(Team::Us),
        Some("time_limit_lead".into()),
    );
    let serialized = serde_json::to_value(&summary).unwrap();
    let row = &serialized["players"][0];
    for k in "player session team squad online_ticks kills deaths assists kd_milli categories total_score shots hits headshots hit_bps headshot_bps farthest_kill_mm longest_streak revives healed_hp build_supplies garrison_spawns repaired_hp spot_assists vehicles_destroyed vehicle_damage_hp driving_distance_mm role_xp match_xp account_level".split_whitespace() {
        assert!(row.get(k).is_some(), "missing {k}");
    }
    assert_eq!(
        serde_json::from_value::<MatchSummary>(serialized).unwrap(),
        summary
    );
    let rows: Vec<_> = summary.players.iter().filter(|p| p.player == 0).collect();
    assert_eq!((rows[0].total_score, rows[1].total_score), (6, 0));
    assert_ne!(rows[0].session, rows[1].session);
}

#[test]
fn live_capture_awards_and_hash_replay() {
    use crate::{
        CollisionChunk, CollisionData, GameData, GameMode, MapId, ObjectiveSpec, SimConfig,
        SpawnReq, Vec3, World,
    };
    let flat = CollisionData::from_chunks(vec![CollisionChunk::new(
        0,
        0,
        vec![5.0; 65 * 65],
        vec![0; 64 * 64],
    )]);
    let run = || {
        let mut w = World::new(
            SimConfig {
                tick_hz: 30,
                match_seed: 1,
                map: MapId(0),
                mode: GameMode::Frontline,
                data: GameData::embedded(),
            },
            &flat,
        );
        w.set_objectives(&[
            ObjectiveSpec {
                id: 1,
                pos: Vec3::new(15.0, 5.0, 15.0),
                radius_m: 10.0,
                owner: None,
            },
            ObjectiveSpec {
                id: 2,
                pos: Vec3::new(45.0, 5.0, 45.0),
                radius_m: 10.0,
                owner: Some(Team::Ger),
            },
        ]);
        w.spawn_player(SpawnReq {
            player: 0,
            team: Team::Us,
            pos: Vec3::new(15.0, 5.02, 15.0),
            yaw: 0.0,
        })
        .unwrap();
        w.configure_match(0.0, 90.0);
        w.begin_warmup();
        #[cfg(feature = "test-util")]
        w.debug_set_objective(1, 95.0, None, None);
        let mut out = Vec::new();
        let mut hashes = Vec::new();
        for _ in 0..1000 {
            out.extend(w.step(&[]).events.into_iter().filter_map(|e| match e {
                SimEvent::ScoreDelta(d) => Some(d),
                _ => None,
            }));
            hashes.push(w.state_hash());
        }
        let s = w.score_summary();
        #[cfg(feature = "test-util")]
        {
            assert_eq!(s.timeline.len(), 1);
            assert_eq!(s.timeline[0].objective, 1);
            assert_eq!(points(&out, K::Capture), 50);
        }
        assert_eq!(
            s.players[0].total_score,
            out.iter().map(|d| i64::from(d.value)).sum::<i64>()
        );
        assert!(points(&out, K::CaptureAdvance) > 0);
        (hashes, s, out)
    };
    assert_eq!(run(), run());

    let mut engine = engine();
    let mut before = HashWriter::new();
    engine.hash(&mut before);
    action(&mut engine, 0, Action::Build(10));
    let mut after = HashWriter::new();
    engine.hash(&mut after);
    assert_ne!(before.finish(), after.finish());
}

#[test]
fn assist_damage_window_is_inclusive_and_aggregates() {
    use crate::{combat::credit::RecentDamage, Tick};
    let c = ScoreRules::embedded();
    let window = c.assist_s * 30;
    let mut damage = RecentDamage::default();
    damage.record(2, 25.0, Tick(10), window);
    damage.record(3, 11.0, Tick(11), window);
    damage.record(3, 9.0, Tick(12), window);
    assert_eq!(
        damage.attackers_within(Tick(window + 11), window),
        [(3, c.assist_min_hp)]
    );
    assert_eq!(
        damage.attackers_within(Tick(window + 12), window),
        [(3, 9.0)]
    );
}

proptest! {
    #[test]
    fn positive_events_neither_wrap_nor_decrease_score(supplies in any::<u32>(), increment in any::<u32>()) {
        let mut e = engine();
        let a = action(&mut e, 0, Action::Build(supplies));
        let b = action(&mut e, 0, Action::Build(supplies.saturating_add(increment)));
        let (a, b) = (points(&a, K::Build), points(&b, K::Build));
        prop_assert!((0..=i32::from(i16::MAX)).contains(&a));
        prop_assert!((a..=i32::from(i16::MAX)).contains(&b));
        prop_assert_eq!(e.summary(0, "frontline", None, None).players[0].total_score, i64::from(a + b));
    }
}
