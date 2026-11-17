//! M0-007 acceptance: determinism of the World/step foundation.

use frontline_protocol::msg::input::InputCmd;
use frontline_sim::ids::{entity_gen, entity_slot};
use frontline_sim::rng::{Rng, RngCore};
use frontline_sim::{
    CollisionData, GameData, GameMode, MapId, PlayerId, SimConfig, SpawnError, SpawnReq, Team,
    Tick, Vec3, World,
};

const PLAYERS: u8 = 10;

fn cfg(seed: u64) -> SimConfig {
    SimConfig {
        tick_hz: 30,
        match_seed: seed,
        map: MapId(0),
        mode: GameMode::Sandbox,
        data: GameData::embedded(),
    }
}

fn new_world(seed: u64) -> World {
    let mut w = World::new(cfg(seed), &CollisionData::empty());
    w.set_test_jitter(true);
    for p in 0..PLAYERS {
        w.spawn_player(SpawnReq {
            player: p,
            team: if p % 2 == 0 { Team::Us } else { Team::Ger },
            pos: Vec3::new(p as f32 * 2.0, 0.0, 10.0),
            yaw: 0.0,
        })
        .unwrap();
    }
    w
}

/// Deterministic input script (independent of the sim RNG).
fn inputs_for(tick: u32, flip: Option<(u32, u8)>) -> Vec<(PlayerId, InputCmd)> {
    (0..PLAYERS)
        .map(|p| {
            let mut r = Rng::for_(
                0xfeed,
                Tick(tick),
                p as u16,
                frontline_sim::rng::Purpose::Bot,
            );
            let mut cmd = InputCmd {
                seq: tick as u16,
                tick,
                move_x: ((r.next_u32() % 255) as i32 - 127) as i8,
                move_z: ((r.next_u32() % 255) as i32 - 127) as i8,
                yaw: r.next_u32() as u16,
                pitch: r.next_u32() as u16 as i16,
                buttons: r.next_u32() as u16,
                weapon_slot: (r.next_u32() % 4) as u8,
                aux: 0,
                view_tick: 0,
            };
            if let Some((t, bit)) = flip {
                if t == tick && bit == p {
                    cmd.buttons ^= 1;
                }
            }
            (p, cmd)
        })
        .collect()
}

fn run_hashes(seed: u64, ticks: u32) -> Vec<u64> {
    let mut w = new_world(seed);
    (1..=ticks)
        .map(|t| {
            w.step(&inputs_for(t, None));
            w.state_hash()
        })
        .collect()
}

/// Regression pin of the 10 000-tick final hash. Update only on a deliberate change of the
/// hash/RNG algorithm or of the hashed state layout, and say why in the commit.
/// M0-011: re-pinned because `state_hash` now also folds the movement state (`Motion`:
/// velocity, stance, blend/timer, grounded/blocked flags). With the empty collision used here
/// every player is `blocked` and does not move, so only the new hashed fields changed.
const GOLDEN_FINAL_HASH: u64 = 0x63ac_47f1_5c23_31d8;

#[test]
fn ten_thousand_ticks_identical_across_threads() {
    let a = std::thread::spawn(|| run_hashes(42, 10_000))
        .join()
        .unwrap();
    // Perturb the allocator between runs.
    let _junk: Vec<Vec<u8>> = (0..64).map(|i| vec![i as u8; 1000 + i * 37]).collect();
    let b = std::thread::spawn(|| run_hashes(42, 10_000))
        .join()
        .unwrap();
    assert_eq!(a.len(), 10_000);
    let equal = a.iter().zip(&b).filter(|(x, y)| x == y).count();
    assert_eq!(equal, 10_000, "{equal}/10000 ticks equal");
    println!("final hash = {:#018x}", a[9_999]);
    assert_eq!(a[9_999], GOLDEN_FINAL_HASH, "golden hash changed");
}

#[test]
fn perturbation_diverges_within_two_ticks() {
    let base = run_hashes(7, 5);
    let mut r = Rng::for_(1, Tick(0), 0, frontline_sim::rng::Purpose::Misc);
    let mut diverged = 0;
    for i in 0..100u32 {
        let h = if i % 2 == 0 {
            run_hashes(7 + 1 + (r.next_u32() % 1000) as u64, 2)
        } else {
            let t = 1 + r.next_u32() % 2;
            let p = (r.next_u32() % PLAYERS as u32) as u8;
            let mut w = new_world(7);
            (1..=2)
                .map(|tk| {
                    w.step(&inputs_for(tk, Some((t, p))));
                    w.state_hash()
                })
                .collect()
        };
        // Flipping an input at tick t must show by tick t; seed change by tick 1.
        if h[..2] != base[..2] {
            diverged += 1;
        }
    }
    assert_eq!(
        diverged, 100,
        "{diverged}/100 perturbations diverged within 2 ticks"
    );
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "strictly ascending by PlayerId")]
fn unsorted_inputs_panic_in_debug() {
    let mut w = new_world(1);
    let mut inp = inputs_for(1, None);
    inp.swap(0, 3);
    w.step(&inp);
}

#[cfg(not(debug_assertions))]
#[test]
fn unsorted_inputs_are_sorted_in_release() {
    let mut a = new_world(1);
    let mut b = new_world(1);
    let sorted = inputs_for(1, None);
    let mut shuffled = sorted.clone();
    shuffled.swap(0, 3);
    shuffled.swap(5, 9);
    a.step(&sorted);
    b.step(&shuffled);
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn entity_slot_reuse_bumps_generation_and_stale_id_is_none() {
    let mut w = World::new(cfg(1), &CollisionData::empty());
    let req = |p| SpawnReq {
        player: p,
        team: Team::Us,
        pos: Vec3::ZERO,
        yaw: 0.0,
    };
    let a = w.spawn_player(req(0)).unwrap();
    assert!(w.view().entity_state(a).is_some());
    w.remove_player(0);
    assert!(w.view().entity_state(a).is_none());
    let a2 = w.spawn_player(req(0)).unwrap();
    assert_eq!(entity_slot(a2), entity_slot(a));
    assert_eq!(entity_gen(a2), (entity_gen(a) + 1) & 0xf);
    assert!(w.view().entity_state(a).is_none(), "old id stays stale");
    assert!(w.view().entity_state(a2).is_some());
    assert_eq!(w.spawn_player(req(0)), Err(SpawnError::PlayerExists));
    assert_eq!(w.spawn_player(req(200)), Err(SpawnError::InvalidPlayerId));
}

#[test]
fn players_view_is_sorted_and_data_hash_exposed() {
    let w = new_world(1);
    let ids: Vec<_> = w.view().players().iter().map(|p| p.entity).collect();
    let mut s = ids.clone();
    s.sort_unstable();
    assert_eq!(ids, s);
    assert_eq!(ids.len(), PLAYERS as usize);
    assert_eq!(w.data_hash(), frontline_sim::data::DATA_HASH);
}
