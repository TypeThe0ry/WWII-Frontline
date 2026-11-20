//! M0-019 acceptance 3, 6, 7 and the R3 / hysteresis / removal behaviour of the AOI, driven
//! through the real `SnapshotScheduler` with in-process synthetic clients (no network).
//! Static scenes use an empty collision world (players stay where they spawn); the ones with
//! motion need the generated terrain.

mod common;

use common::sim_harness::terrain;
use common::synth::*;
use frontline_protocol::msg::input::InputCmd;
use frontline_server::snapshot::{LinkInfo, SnapshotScheduler};
use frontline_sim::{Team, Vec3};

fn cmd(t: u32, yaw: u16, btn: u16) -> InputCmd {
    InputCmd {
        seq: t as u16,
        tick: t,
        move_x: 0,
        move_z: 0,
        yaw,
        pitch: 0,
        buttons: btn,
        weapon_slot: 0,
        aux: 0,
        view_tick: 2,
    }
}

fn idle(_p: u8, t: u32) -> InputCmd {
    cmd(t, 0, 0)
}

fn building_cover_scene(top: f32, masonry: bool, enemy: Vec3) -> Synth {
    use frontline_sim::ballistics::materials::MaterialTable;
    use frontline_sim::collision::{CoverSlab, CHUNK_CELLS, CHUNK_SAMPLES};
    use frontline_sim::throwables::Occlusion;
    use frontline_sim::{
        CollisionChunk, CollisionData, GameData, GameMode, MapId, SimConfig, World,
    };

    let mut collision = CollisionData::from_chunks([CollisionChunk::new(
        0,
        0,
        vec![0.0; CHUNK_SAMPLES * CHUNK_SAMPLES],
        vec![0; CHUNK_CELLS * CHUNK_CELLS],
    )]);
    let (material, width, cover) = if masonry {
        ("brick", 0.30, Occlusion::Solid)
    } else {
        ("thin_wood", 0.03, Occlusion::Thin)
    };
    let material = MaterialTable::embedded().id_of(material).unwrap();
    collision.install_cover(vec![CoverSlab::new(
        [17.0, 34.0],
        [23.0, 34.0],
        -0.4,
        top,
        width,
        material,
        cover,
    )
    .unwrap()]);
    let mut s = Synth::new(World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 7,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        &collision,
    ));
    s.add(
        0,
        Team::Us,
        Vec3::new(20.0, 0.0, 8.0),
        0.0,
        Some(LinkInfo::WS),
    );
    s.add(1, Team::Ger, enemy, 0.0, None);
    s.add(2, Team::Us, Vec3::new(21.0, 0.0, 56.0), 0.0, None);
    s.add(3, Team::Ger, Vec3::new(20.0, 0.0, 2.0), 0.0, None);
    s
}

#[test]
fn building_cover_removes_enemy_and_rechecks_on_the_los_interval() {
    let mut s = building_cover_scene(3.0, true, Vec3::new(20.0, 0.0, 25.0));
    for _ in 0..2 {
        s.step(idle);
    }
    assert!(s.client(0).known.contains_key(&1));

    s.world.debug_teleport(1, Vec3::new(20.0, 0.0, 56.0));
    for _ in 0..2 {
        s.step(idle);
    }
    assert!(!s.client(0).known.contains_key(&1));
    assert!(s.client(0).last.as_ref().unwrap().removed_ids.contains(&1));

    // Moving around the edge leaves a clear ray, but does not bypass the cached result.
    s.world.debug_teleport(1, Vec3::new(28.0, 0.0, 56.0));
    for _ in 0..6 {
        s.step(idle);
        assert!(!s.client(0).known.contains_key(&1));
    }
    assert_eq!(s.sched.stats.remove_records, 3);
    for _ in 0..2 {
        s.step(idle);
    }
    let c = s.client(0);
    assert!(c.log.last().unwrap().kf_ids.contains(&1));
    assert!(c.known.contains_key(&2), "friends are not occlusion culled");
    assert!(
        c.known.contains_key(&3),
        "near enemies remain candidates outside the frustum"
    );
}

#[test]
fn building_cover_requires_a_solid_wall_hiding_the_whole_target() {
    for (top, masonry, x) in [(1.5, true, 20.0), (3.0, false, 20.0), (3.0, true, 25.3)] {
        let mut s = building_cover_scene(top, masonry, Vec3::new(x, 0.0, 56.0));
        for _ in 0..2 {
            s.step(idle);
        }
        assert!(
            s.client(0).known.contains_key(&1),
            "top={top}, masonry={masonry}, x={x}"
        );
    }
}

fn scene(players: &[(u8, Team, [f32; 3])], clients: &[u8]) -> Synth {
    let mut s = Synth::new(new_world(None));
    for &(id, team, p) in players {
        s.add(
            id,
            team,
            Vec3::new(p[0], p[1], p[2]),
            0.0,
            clients.contains(&id).then_some(LinkInfo::DATAGRAM),
        );
    }
    s
}

fn sends_of(s: &Synth, client: u8, id: u8, from_tick: u32) -> usize {
    s.client(client)
        .log
        .iter()
        .filter(|l| l.tick >= from_tick && l.ids.contains(&id))
        .count()
}

#[test]
fn rates_follow_distance_and_view_cone() {
    // client 0 at (500, 500) facing +z (yaw 0); everybody is a friend, so R3 does not interfere
    let mut s = scene(
        &[
            (0, Team::Us, [500.0, 0.0, 500.0]),
            (1, Team::Us, [508.0, 0.0, 510.0]), // 12.8 m, 38 deg off: 20 Hz
            (2, Team::Us, [540.0, 0.0, 700.0]), // 204 m, 11 deg: 2 Hz
            (3, Team::Us, [550.0, 0.0, 650.0]), // 158 m, 18 deg: 5 Hz
            (4, Team::Us, [650.0, 0.0, 500.0]), // 150 m, 90 deg (outside the cone): 5 x 0.5
            (5, Team::Us, [500.0, 0.0, 300.0]), // 200 m straight behind: 2 x 0.35 -> floor 2
        ],
        &[0],
    );
    for _ in 0..30 {
        s.step(idle);
    }
    let from = s.tick + 1;
    for _ in 0..300 {
        s.step(idle);
    }
    let per_s = |id| sends_of(&s, 0, id, from) as f64 / 10.0;
    let r: Vec<f64> = (1..=5).map(per_s).collect();
    eprintln!(
        "sends/s over 10 s: near {:.1}, 204 m {:.1}, 158 m {:.1}, side {:.1}, behind {:.1}",
        r[0], r[1], r[2], r[3], r[4]
    );
    assert!((18.0..=22.0).contains(&r[0]), "near {}", r[0]);
    assert!((1.8..=2.4).contains(&r[1]), "far {}", r[1]);
    assert!((4.5..=5.5).contains(&r[2]), "mid {}", r[2]);
    assert!(
        (2.25..=2.75).contains(&r[3]),
        "outside cone halves the rate: {}",
        r[3]
    );
    assert!((1.8..=2.4).contains(&r[4]), "behind {}", r[4]);
}

#[test]
fn every_candidate_is_sent_at_least_twice_a_second_even_when_crowded() {
    // 120 players inside 150 m around the client, mixed teams all friends (R3 off the table),
    // scattered over distance bands; default 1150 B budget.
    let mut players = vec![(0u8, Team::Us, [500.0f32, 0.0, 500.0])];
    let mut r = Rng(42);
    for i in 1..=120u8 {
        let a = r.f64() * std::f64::consts::TAU;
        let d = 5.0 + r.f64() * 380.0;
        players.push((
            i,
            Team::Us,
            [
                500.0 + (a.cos() * d) as f32,
                0.0,
                500.0 + (a.sin() * d) as f32,
            ],
        ));
    }
    let mut s = scene(&players, &[0]);
    for _ in 0..60 {
        s.step(idle);
    }
    let from = s.tick + 1;
    for _ in 0..300 {
        s.step(idle);
    }
    let c = s.client(0);
    let mut worst = usize::MAX;
    let mut worst_gap = 0u32;
    for id in 1..=120u8 {
        let ticks: Vec<u32> = c
            .log
            .iter()
            .filter(|l| l.tick >= from && l.ids.contains(&id))
            .map(|l| l.tick)
            .collect();
        worst = worst.min(ticks.len());
        for w in ticks.windows(2) {
            worst_gap = worst_gap.max(w[1] - w[0]);
        }
    }
    let starved = s.sched.stats.starved;
    eprintln!(
        "120 candidates: min sends in 10 s = {worst} (floor 20), longest gap {worst_gap} ticks, starved records {starved}, mean packet {} B",
        s.sched.stats.bytes / s.sched.stats.packets
    );
    assert!(worst >= 19, "2 Hz floor violated: {worst} sends in 10 s");
    assert!(worst_gap <= 20, "gap of {worst_gap} ticks (> 0.67 s)");
}

#[test]
fn starvation_protection_under_a_tiny_budget() {
    // 60 candidates but only ~330 B per packet: not everybody fits every packet, the
    // accumulator order must still serve the most overdue first (no entity is forgotten).
    let mut players = vec![(0u8, Team::Us, [500.0f32, 0.0, 500.0])];
    for i in 1..=60u8 {
        let a = i as f32 * 0.7;
        players.push((
            i,
            Team::Us,
            [
                500.0 + a.cos() * (30.0 + i as f32 * 5.0),
                0.0,
                500.0 + a.sin() * (30.0 + i as f32 * 5.0),
            ],
        ));
    }
    let mut s = Synth::new(new_world(None));
    for &(id, team, p) in &players {
        let link = LinkInfo {
            budget: 330,
            ..LinkInfo::DATAGRAM
        };
        s.add(
            id,
            team,
            Vec3::new(p[0], p[1], p[2]),
            0.0,
            (id == 0).then_some(link),
        );
    }
    for _ in 0..600 {
        s.step(idle);
    }
    let c = s.client(0);
    assert!(
        s.sched.stats.budget_skipped > 0,
        "scene should overflow the budget"
    );
    for id in 1..=60u8 {
        let n = c.log.iter().filter(|l| l.ids.contains(&id)).count();
        assert!(n >= 8, "entity {id} sent only {n} times in 20 s");
    }
    assert!(c.log.iter().all(|l| l.bytes <= 330));
}

#[test]
fn r3_enemies_are_trimmed_and_never_leak() {
    // client 0 (US) at (500, 500) facing +z.
    let mut s = scene(
        &[
            (0, Team::Us, [500.0, 0.0, 500.0]),
            (1, Team::Ger, [500.0, 0.0, 200.0]), // enemy 300 m behind: hidden
            (2, Team::Ger, [500.0, 0.0, 800.0]), // enemy 300 m ahead: visible
            (3, Team::Ger, [500.0, 0.0, 440.0]), // enemy 60 m behind: visible (<= 96 m)
            (
                4,
                Team::Ger,
                [500.0 + 300.0 * 0.9962, 0.0, 500.0 - 300.0 * 0.0872],
            ), // 95 deg: visible
            (
                5,
                Team::Ger,
                [500.0 + 300.0 * 0.9659, 0.0, 500.0 - 300.0 * 0.2588],
            ), // 105 deg: hidden
            (6, Team::Us, [500.0, 0.0, 200.0]),  // friend 300 m behind: visible
            (7, Team::Ger, [500.0, 0.0, 950.0]), // enemy 450 m ahead: hidden (> 400 m)
            (8, Team::Us, [500.0, 0.0, 960.0]),  // friend 460 m: hidden (> 440 m)
        ],
        &[0],
    );
    for _ in 0..90 {
        s.step(idle);
    }
    let c = s.client(0);
    let known: Vec<u8> = c.known.keys().copied().collect();
    assert_eq!(known, vec![2, 3, 4, 6], "client view after 3 s");
    // not a single packet ever mentioned a hidden enemy
    for hidden in [1u8, 5, 7, 8] {
        assert!(!c.ever_ids.contains(&hidden), "entity {hidden} leaked");
    }

    // turn around (yaw 180 deg): enemy 1 becomes visible, enemy 2 must be removed again
    let turn = |_p: u8, t: u32| cmd(t, 32768, 0);
    for _ in 0..60 {
        s.step(turn);
    }
    let c = s.client(0);
    let known: Vec<u8> = c.known.keys().copied().collect();
    // 4 and 5 were 95 / 105 deg to the right, now 85 / 75 deg to the left: both inside the cone
    assert_eq!(known, vec![1, 3, 4, 5, 6], "after turning around");
    for hidden in [7u8, 8] {
        assert!(
            !s.client(0).ever_ids.contains(&hidden),
            "entity {hidden} leaked"
        );
    }
}

#[test]
fn leaving_entities_get_a_repeated_remove_and_state_is_reclaimed() {
    let mut s = scene(
        &[
            (0, Team::Us, [500.0, 0.0, 500.0]),
            (1, Team::Us, [520.0, 0.0, 520.0]),
            (2, Team::Us, [480.0, 0.0, 530.0]),
        ],
        &[0],
    );
    for _ in 0..30 {
        s.step(idle);
    }
    assert!(s.client(0).known.contains_key(&1));
    s.world.remove_player(1);
    for _ in 0..30 {
        s.step(idle);
    }
    let c = s.client(0);
    assert!(!c.known.contains_key(&1));
    assert!(c.known.contains_key(&2));
    // the remove record is repeated in 3 consecutive packets
    let removes = c.log.iter().filter(|l| l.removed > 0).count();
    assert_eq!(removes, 3, "entity_remove repeated in exactly 3 packets");
    assert_eq!(s.sched.stats.remove_records, 3);
}

#[test]
fn packets_are_staggered_evenly_across_ticks() {
    // pure schedule: 20 clients with consecutive ids, every window of 3 ticks
    for hz15 in [false, true] {
        let mut counts = Vec::new();
        for t in 0..600u32 {
            counts.push(
                (0..20u8)
                    .filter(|&p| SnapshotScheduler::is_due(hz15, p, t))
                    .count(),
            );
        }
        for w in counts.windows(3) {
            let (mn, mx) = (*w.iter().min().unwrap(), *w.iter().max().unwrap());
            assert!(mx - mn <= 1, "hz15={hz15}: counts {w:?}");
        }
        // each client's packet rate: 20 Hz = 2 of 3 ticks, 15 Hz = 1 of 2
        for p in 0..20u8 {
            let n = (0..600u32)
                .filter(|&t| SnapshotScheduler::is_due(hz15, p, t))
                .count();
            assert_eq!(n, if hz15 { 300 } else { 400 }, "client {p}");
        }
        eprintln!(
            "hz15={hz15}: per-tick packet counts for 20 clients: min {} max {}",
            counts.iter().min().unwrap(),
            counts.iter().max().unwrap()
        );
    }
}

#[test]
fn staggering_also_holds_in_a_live_run() {
    let mut players = Vec::new();
    for i in 0..20u8 {
        players.push((i, Team::Us, [300.0 + i as f32 * 3.0, 0.0, 300.0]));
    }
    let ids: Vec<u8> = (0..20).collect();
    let mut s = scene(&players, &ids);
    let mut per_tick = Vec::new();
    for _ in 0..120 {
        let before: u64 = s.clients.iter().map(|c| c.packets_ok).sum();
        s.step(idle);
        per_tick.push(s.clients.iter().map(|c| c.packets_ok).sum::<u64>() - before);
    }
    for w in per_tick.windows(3) {
        assert!(
            w.iter().max().unwrap() - w.iter().min().unwrap() <= 1,
            "{w:?}"
        );
    }
}

#[test]
fn pos_corr_only_when_the_client_hash_disagrees() {
    let mut s = scene(
        &[
            (0, Team::Us, [500.0, 0.0, 500.0]),
            (1, Team::Us, [520.0, 0.0, 520.0]),
        ],
        &[0],
    );
    s.ack_delay = 1;
    // the very first packet carries pos_corr (the client's start position is a guess)
    for _ in 0..3 {
        s.step(idle);
    }
    assert!(s.client(0).log[0].has_corr, "first packet must correct");
    // in sync: after the client acked a packet that carried it, no more corrections
    for _ in 0..30 {
        s.step(idle);
    }
    let n0 = s.client(0).log.len();
    let corr_before = s.client(0).log.iter().filter(|l| l.has_corr).count();
    assert!((1..=4).contains(&corr_before), "corr packets {corr_before}");
    for _ in 0..60 {
        s.step(idle);
    }
    let later: Vec<_> = s.client(0).log[n0..].to_vec();
    assert!(!later.is_empty());
    assert!(
        later.iter().all(|l| !l.has_corr),
        "no correction while in sync"
    );

    // tamper with the reported hash: the next packets carry the full position again
    let good = s.world.player_state(0).map(|st| {
        use frontline_protocol::snapshot::{pred_hash::pred_hash_q, QPos};
        pred_hash_q(QPos::from_meters(st.pos.x, st.pos.y, st.pos.z))
    });
    s.client_mut(0).tamper = Some(good.unwrap() ^ 0x5A);
    let n1 = s.client(0).log.len();
    for _ in 0..10 {
        s.step(idle);
    }
    let seen: Vec<bool> = s.client(0).log[n1..].iter().map(|l| l.has_corr).collect();
    assert!(
        seen.iter().any(|&x| x),
        "tampered hash must trigger pos_corr: {seen:?}"
    );
    let st = s.client(0).last.as_ref().unwrap().self_state.unwrap();
    let corr = st.pos_corr.expect("still pending (hash stays wrong)");
    let p = s.world.player_state(0).unwrap().pos;
    let want = frontline_protocol::snapshot::QPos::from_meters(p.x, p.y, p.z);
    assert_eq!(corr, want, "pos_corr carries the authoritative position");
    // client repairs its hash: corrections stop
    s.client_mut(0).tamper = None;
    for _ in 0..20 {
        s.step(idle);
    }
    let n2 = s.client(0).log.len();
    for _ in 0..30 {
        s.step(idle);
    }
    assert!(s.client(0).log[n2..].iter().all(|l| !l.has_corr));
    // 4-bit "ok" tag: a snapshot without correction has a 7 B self_state, with one 13 B
    // (see protocol::snapshot::self_state docs)
}

#[test]
fn moving_players_keep_delta_coding_for_near_entities() {
    let Some(map) = terrain() else { return };
    let mut s = Synth::new(new_world(Some(&map)));
    s.sched.near_every_packet = true; // option under test: everything within 96 m is a delta
    spawn_squads(&mut s, &map, 20, 1, LinkInfo::DATAGRAM);
    let (mut near_rec, mut near_kf, mut all_rec, mut all_kf) = (0usize, 0usize, 0usize, 0usize);
    for t in 0..600 {
        s.step(common::sim_harness::gen_cmd);
        if t < 200 {
            continue;
        }
        let me = s.world.player_state(0).unwrap().pos;
        let Some(l) = s.client(0).log.last().filter(|l| l.tick == s.tick) else {
            continue;
        };
        for id in &l.ids {
            let Some(o) = s.world.player_state(*id) else {
                continue;
            };
            let kf = l.kf_ids.contains(id);
            all_rec += 1;
            all_kf += kf as usize;
            if me.distance(o.pos) <= 90.0 {
                near_rec += 1;
                near_kf += kf as usize;
            }
        }
    }
    eprintln!(
        "20 players, 400 ticks: records within 90 m {near_rec} (keyframes {near_kf}); all records {all_rec} (keyframes {all_kf} = {:.1} %)",
        100.0 * all_kf as f64 / all_rec as f64
    );
    assert!(
        near_rec > 200,
        "scene has too few near records ({near_rec})"
    );
    assert!(
        (near_kf as f64) / (near_rec as f64) < 0.03,
        "near entities must be delta coded"
    );
}

#[test]
fn r3_enemy_gunfire_outside_the_view_is_not_reported() {
    use frontline_protocol::msg::input::buttons;
    use frontline_protocol::snapshot::Event;
    let mut s = scene(
        &[
            (0, Team::Us, [500.0, 0.0, 500.0]),
            (1, Team::Ger, [500.0, 0.0, 200.0]), // enemy 300 m behind
            (2, Team::Ger, [500.0, 0.0, 800.0]), // enemy 300 m ahead
            (3, Team::Us, [500.0, 0.0, 200.0]),  // friend 300 m behind
        ],
        &[0],
    );
    for _ in 0..240 {
        s.step(|p, t| {
            let b = if p != 0 && t % 8 == 0 {
                buttons::FIRE
            } else {
                0
            };
            cmd(t, 0, b)
        });
    }
    let shooters: std::collections::BTreeSet<u8> = s
        .client(0)
        .events
        .iter()
        .filter_map(|(_, e)| match e.event {
            Event::Shot { shooter, .. } => Some(shooter),
            _ => None,
        })
        .collect();
    eprintln!("shooters seen by the client: {shooters:?}");
    assert!(shooters.contains(&2), "enemy ahead must be reported");
    assert!(shooters.contains(&3), "friend must be reported");
    assert!(
        !shooters.contains(&1),
        "enemy behind must not leak through events"
    );
}

#[test]
fn degradation_guard_halves_to_15hz_for_3_seconds_then_recovers() {
    let mut players = Vec::new();
    for i in 0..6u8 {
        players.push((i, Team::Us, [300.0 + i as f32 * 5.0, 0.0, 300.0]));
    }
    let ids: Vec<u8> = (0..6).collect();
    let mut s = scene(&players, &ids);
    for _ in 0..60 {
        s.step(idle);
    }
    let count = |s: &mut Synth, ticks: u32| {
        let before: u64 = s.clients.iter().map(|c| c.packets_ok).sum();
        for _ in 0..ticks {
            s.step(idle);
        }
        (s.clients.iter().map(|c| c.packets_ok).sum::<u64>() - before) as f64
            / 6.0
            / (ticks as f64 / 30.0)
    };
    let normal = count(&mut s, 90);
    s.sched
        .degrade(frontline_server::snapshot::DEGRADE_HOLD_TICKS);
    assert!(s.sched.is_degraded());
    let reduced = count(&mut s, 90);
    let after = count(&mut s, 90);
    eprintln!("packets/s per client: normal {normal:.1}, degraded {reduced:.1}, after {after:.1}");
    assert!((19.5..=20.5).contains(&normal));
    assert!((14.5..=15.5).contains(&reduced));
    assert!((19.5..=20.5).contains(&after));
    assert_eq!(s.sched.stats.degraded_ticks, 90);
}

#[test]
fn confirmed_terrain_cover_removes_enemy_until_cache_refresh() {
    use frontline_sim::collision::{CHUNK_CELLS, CHUNK_SAMPLES};
    use frontline_sim::{
        CollisionChunk, CollisionData, GameData, GameMode, MapId, SimConfig, World,
    };

    let heights = (0..CHUNK_SAMPLES)
        .flat_map(|z| {
            let height = if (33..=38).contains(&z) { 7.0 } else { 0.0 };
            [height; CHUNK_SAMPLES]
        })
        .collect();
    let terrain = CollisionData::from_chunks([CollisionChunk::new(
        0,
        0,
        heights,
        vec![0; CHUNK_CELLS * CHUNK_CELLS],
    )]);
    let mut s = Synth::new(World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 7,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        &terrain,
    ));
    s.add(
        0,
        Team::Us,
        Vec3::new(20.0, 0.0, 8.0),
        0.0,
        Some(LinkInfo::WS),
    );
    s.add(1, Team::Ger, Vec3::new(20.0, 0.0, 25.0), 0.0, None);
    s.add(2, Team::Us, Vec3::new(25.0, 0.0, 56.0), 0.0, None);
    for _ in 0..2 {
        s.step(idle);
    }
    assert!(s.client(0).known.contains_key(&1));
    assert!(s.client(0).known.contains_key(&2));

    s.world.debug_teleport(1, Vec3::new(20.0, 0.0, 56.0));
    for _ in 0..2 {
        s.step(idle);
    }
    assert!(!s.client(0).known.contains_key(&1));
    assert!(s.client(0).last.as_ref().unwrap().removed_ids.contains(&1));

    // A now-clear path beyond 20 m retains the cached answer through tick 11.
    s.world.debug_teleport(1, Vec3::new(20.0, 0.0, 30.0));
    for _ in 0..6 {
        s.step(idle);
        assert!(!s.client(0).known.contains_key(&1));
    }
    assert_eq!(s.sched.stats.remove_records, 3);
    for _ in 0..2 {
        s.step(idle);
    }
    assert_eq!(s.tick, 12);
    assert!(s.client(0).known.contains_key(&1));
    assert!(s.client(0).log.last().unwrap().kf_ids.contains(&1));
    assert!(s.client(0).known.contains_key(&2));
}

#[test]
fn vehicle_multiplier_reaches_the_wire_and_enemy_hulls_follow_r3() {
    use frontline_sim::collision::{CHUNK_CELLS, CHUNK_SAMPLES};
    use frontline_sim::{
        CollisionChunk, CollisionData, GameData, GameMode, MapId, SimConfig, World,
    };

    let terrain = CollisionData::from_chunks((0..4).flat_map(|z| {
        (0..3).map(move |x| {
            CollisionChunk::new(
                x,
                z,
                vec![0.0; CHUNK_SAMPLES * CHUNK_SAMPLES],
                vec![0; CHUNK_CELLS * CHUNK_CELLS],
            )
        })
    }));
    let mut s = Synth::new(World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 7,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        &terrain,
    ));
    s.add(
        0,
        Team::Us,
        Vec3::new(90.0, 0.0, 120.0),
        0.0,
        Some(LinkInfo::DATAGRAM),
    );
    s.add(1, Team::Us, Vec3::new(110.0, 0.0, 217.0), 0.0, None);
    for (team, pos) in [
        (Team::Us, Vec3::new(70.0, 0.8, 217.0)),
        (Team::Ger, Vec3::new(90.0, 0.8, 10.0)),
    ] {
        s.world.spawn_vehicle("us_m4a1_75", team, pos, 0.0).unwrap();
    }
    for _ in 0..330 {
        s.step(idle);
    }
    let player = sends_of(&s, 0, 1, 31);
    let vehicle = sends_of(&s, 0, 128, 31);
    eprintln!("99 m over 10 s: infantry {player}, vehicle {vehicle} records");
    assert!((49..=51).contains(&player));
    assert!((74..=76).contains(&vehicle));
    assert!(vehicle as f64 >= player as f64 * 1.4);
    assert!(!s.client(0).ever_ids.contains(&129));
    assert_eq!(s.sched.stats.enemy_leaks, 0);
}

#[test]
#[ignore = "manual optimized scheduler timing"]
fn aoi_select_sixty_candidates() {
    let Some(map) = terrain() else { return };
    let mut s = Synth::new(new_world(Some(&map)));
    for id in 0..61 {
        // 20 recipients, 60 neighbors per recipient in the grid query. Mix eligible friends
        // and enemies and give the latter positive/negative static LoS on the fixture terrain.
        let team = if id % 2 == 0 { Team::Us } else { Team::Ger };
        let x = 30.0 + f32::from(id % 8) * 22.0;
        let z = 35.0 + f32::from(id / 8) * 22.0;
        let y = map.collision.height_at(x, z).expect("fixture terrain");
        s.add(
            id,
            team,
            Vec3::new(x, y, z),
            0.0,
            (id < 20).then_some(LinkInfo::DATAGRAM),
        );
    }
    for _ in 0..120 {
        s.step(idle);
    }
    s.sched.stats.pass_ns.clear();
    for _ in 0..900 {
        s.step(idle);
    }
    let samples = &mut s.sched.stats.pass_ns;
    samples.sort_unstable();
    assert_eq!(samples.len(), 900);
    let ms = |n| f64::from(samples[n]) / 1_000_000.0;
    eprintln!(
        "scheduler (selection + encode), 20 recipients / 60 grid neighbors, {} ticks: p50 {:.3} ms, p99 {:.3} ms, max {:.3} ms",
        samples.len(),
        ms(samples.len() / 2),
        ms(samples.len() * 99 / 100),
        ms(samples.len() - 1),
    );
}

#[test]
fn occupied_hull_replaces_remote_player_position_until_exit() {
    use frontline_sim::squad::{SquadKind, SquadOp};

    let mut s = scene(
        &[
            (0, Team::Us, [32.0, 0.0, 32.0]),
            (1, Team::Us, [36.0, 0.0, 32.0]),
        ],
        &[0],
    );
    s.world.roster_add(1, Team::Us).unwrap();
    s.world.roster_add(100, Team::Ger).unwrap();
    s.world
        .squad_op(
            1,
            SquadOp::Create {
                kind: SquadKind::ArmorCrew,
                suffix: String::new(),
            },
        )
        .unwrap();
    s.world.set_role(1, Some("crewman")).unwrap();
    let vehicle = s
        .world
        .spawn_vehicle("us_m4a1_75", Team::Us, Vec3::new(39.0, 0.0, 32.0), 0.0)
        .unwrap();
    for _ in 0..6 {
        s.step(idle);
    }
    assert!(s.client(0).known.contains_key(&1));
    s.world.seat_enter(1, vehicle, 2).unwrap();
    for _ in 0..9 {
        s.step(idle);
    }
    assert!(!s.client(0).known.contains_key(&1));
    assert_eq!(s.client(0).known[&128].vehicle.unwrap().seats, 1 << 2);
    assert_eq!(s.sched.stats.remove_records, 3);

    s.world.seat_exit_begin(1).unwrap();
    for _ in 0..30 {
        s.step(idle);
    }
    assert!(s.client(0).known.contains_key(&1));
    assert_eq!(s.client(0).known[&128].vehicle.unwrap().seats, 0);
}

#[test]
fn removals_wait_for_packet_space_before_counting_repeats() {
    let players: Vec<_> = (0..=80)
        .map(|id| {
            (
                id,
                Team::Us,
                [500.0 + f32::from(id % 10), 0.0, 500.0 + f32::from(id / 10)],
            )
        })
        .collect();
    let mut s = scene(&players, &[0]);
    for _ in 0..15 {
        s.step(idle);
    }
    assert_eq!(s.client(0).known.len(), 80);
    s.client_mut(0).link.budget = 64;
    s.world.debug_teleport(0, Vec3::new(1000.0, 0.0, 1000.0));
    for _ in 0..30 {
        s.step(idle);
    }
    assert!(s.client(0).known.is_empty());
    assert_eq!(s.sched.stats.remove_records, 3 * 80);
}
