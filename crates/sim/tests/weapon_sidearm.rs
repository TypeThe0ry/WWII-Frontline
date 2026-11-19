mod common;

use common::{cmd, data_with, duel_world, synth_data};
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::deploy::{DeployReq, SPAWN_HQ};
use frontline_sim::{GameData, SimEvent, Team, Vec3, WeaponState, World};

fn fixture(data: &'static GameData, enemy_dist: Option<f32>) -> World {
    let col = synth_data(2, 2, &|_, _| 5.0);
    let players = enemy_dist.map(|d| (1, Team::Ger, 30.0 + d, 30.0, 49152));
    let mut w = duel_world(data, &col, 3, players.as_slice());
    w.set_team_spawn(Team::Us, Vec3::new(30.0, 5.02, 30.0));
    w.roster_add(0, Team::Us).unwrap();
    w
}

fn deploy(w: &mut World, loadout: &str) {
    let d = w.config().data;
    let request = DeployReq {
        role: d.roles.iter().position(|r| r.id == "rifleman").unwrap() as u8,
        loadout: d.loadouts.iter().position(|l| l.id == loadout).unwrap() as u8,
        squad: 0,
        spawn_point: SPAWN_HQ,
    };
    w.deploy(0, &request).unwrap();
}

fn step(w: &mut World, buttons: u16, slot: u8) -> Vec<SimEvent> {
    let input = InputCmd {
        weapon_slot: slot,
        ..cmd(0, 0, 16384, buttons, 0)
    };
    w.step(&[(0, input)]).events
}

fn gun(w: &World, id: &str) -> WeaponState {
    let gun = w.weapon_state(0).unwrap();
    assert_eq!(w.config().data.weapons[usize::from(gun.def_id)].id, id);
    gun
}

fn shots(events: &[SimEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SimEvent::Shot { .. }))
        .count()
}

#[test]
fn independent_ammo_reload_cancel_and_incoming_draw_times() {
    let data = data_with(|w| {
        w.gameplay.swap_in_ms = Some(if w.class == "pistol" { 80 } else { 134 });
    });
    let mut w = fixture(data, None);
    deploy(&mut w, "us_rifleman_2");
    let carbine = gun(&w, "us_m1_carbine");
    assert_eq!((carbine.mag, carbine.reserve), (15, 75));
    assert_eq!(shots(&step(&mut w, buttons::FIRE, 0)), 1);
    step(&mut w, buttons::RELOAD, 0);
    assert!(gun(&w, "us_m1_carbine").fsm.is_reloading());

    // The 80 ms draw blocks exactly three ticks including the switch tick at 30 Hz.
    assert_eq!(shots(&step(&mut w, buttons::SWAP | buttons::FIRE, 0)), 0);
    let pistol = gun(&w, "us_m1911a1");
    assert_eq!((pistol.mag, pistol.reserve), (7, 7));
    assert_eq!(shots(&step(&mut w, buttons::SWAP | buttons::FIRE, 0)), 0);
    assert_eq!(shots(&step(&mut w, buttons::SWAP | buttons::AIM, 0)), 0);
    assert_eq!(gun(&w, "us_m1911a1").ads_units, 0);
    assert_eq!(shots(&step(&mut w, buttons::SWAP | buttons::FIRE, 0)), 1);
    assert_eq!(gun(&w, "us_m1911a1").mag, 6);

    step(&mut w, 0, 0);
    assert_eq!(shots(&step(&mut w, buttons::SWAP | buttons::FIRE, 0)), 0);
    let carbine = gun(&w, "us_m1_carbine");
    assert!(!carbine.fsm.is_reloading());
    assert_eq!((carbine.mag, carbine.reserve), (14, 75));
    for _ in 0..3 {
        assert_eq!(shots(&step(&mut w, 0, 0)), 0);
    }
    // 134 ms rounds up to five ticks; four ticks after the switch, ADS is still blocked.
    step(&mut w, buttons::AIM, 0);
    assert_eq!(gun(&w, "us_m1_carbine").ads_units, 0);
    assert_eq!(shots(&step(&mut w, buttons::FIRE, 0)), 1);
    step(&mut w, 0, 1);
    assert_eq!(gun(&w, "us_m1911a1").mag, 6);
    step(&mut w, 0, 9);
    gun(&w, "us_m1911a1");
    step(&mut w, 0, 0);
    assert_eq!(gun(&w, "us_m1_carbine").mag, 13);
}

#[test]
fn no_secondary_leaves_primary_usable() {
    let mut w = fixture(GameData::embedded(), None);
    deploy(&mut w, "us_rifleman_1");
    assert_eq!(shots(&step(&mut w, buttons::SWAP | buttons::FIRE, 1)), 1);
    assert_eq!(gun(&w, "us_m1_garand").mag, 7);
}

#[test]
fn melee_uses_loadout_then_pistol_butt() {
    let data = data_with(|w| w.gameplay.swap_in_ms = Some(0));
    let run = |slot, melee_ticks, extra| {
        let mut w = fixture(data, Some(0.9));
        deploy(&mut w, "us_rifleman_2");
        if slot == 1 {
            step(&mut w, 0, slot);
        }
        let mut hits = Vec::new();
        for tick in 0..=melee_ticks {
            let events = step(
                &mut w,
                buttons::MELEE | if tick == 1 { extra } else { 0 },
                slot,
            );
            hits.extend(events.into_iter().filter_map(|e| match e {
                SimEvent::MeleeHit { damage_hp, .. } => Some((tick, damage_hp)),
                _ => None,
            }));
        }
        (hits, w.state_hash())
    };
    // SWAP during knife windup does not transform the strike into a pistol hit.
    let knife = run(0, 8, buttons::SWAP);
    assert_eq!(knife.0, vec![(8, 70.0)]);
    assert_eq!(knife, run(0, 8, buttons::SWAP));
    let pistol = run(1, 9, 0);
    assert_eq!(pistol.0, vec![(9, 20.0)]);
    assert_eq!(pistol, run(1, 9, 0));
}

#[test]
fn rifle_butt_when_loadout_omits_melee() {
    let e = GameData::embedded();
    let mut loadouts = e.loadouts.to_vec();
    loadouts
        .iter_mut()
        .find(|l| l.id == "us_rifleman_2")
        .unwrap()
        .melee = None;
    let data = Box::leak(Box::new(GameData {
        loadouts: Box::leak(loadouts.into_boxed_slice()),
        ..*e
    }));
    let mut w = fixture(data, Some(1.1));
    deploy(&mut w, "us_rifleman_2");
    let hits: Vec<_> = (0..15)
        .flat_map(|tick| {
            step(&mut w, buttons::MELEE, 0)
                .into_iter()
                .filter_map(move |e| match e {
                    SimEvent::MeleeHit { damage_hp, .. } => Some((tick, damage_hp)),
                    _ => None,
                })
        })
        .collect();
    assert_eq!(hits, vec![(14, 40.0)]);
}

#[cfg(feature = "test-util")]
#[test]
fn new_life_resets_slots_and_applies_pending_loadout() {
    let mut w = fixture(GameData::embedded(), None);
    deploy(&mut w, "us_rifleman_2");
    step(&mut w, buttons::FIRE, 0);
    step(&mut w, buttons::SWAP, 0);
    gun(&w, "us_m1911a1");
    for loadout in [None, Some("us_rifleman_1")] {
        w.debug_kill(0);
        step(&mut w, 0, 0);
        if let Some(id) = loadout {
            deploy(&mut w, id);
            gun(&w, "us_m1911a1"); // pending deploy has not yet replaced equipment
        }
        let now = w.tick().0;
        while !matches!(w.player_life(0), Some(frontline_sim::LifeStateView::Alive)) {
            w.step(&[]);
            assert!(w.tick().0 < now + 1_000);
        }
        if loadout.is_none() {
            let gun_state = gun(&w, "us_m1_carbine");
            assert_eq!((gun_state.mag, gun_state.reserve), (15, 75));
            step(&mut w, buttons::SWAP, 0);
            assert_eq!(gun(&w, "us_m1911a1").mag, 7);
        } else {
            let gun_state = gun(&w, "us_m1_garand");
            assert_eq!((gun_state.mag, gun_state.reserve), (8, 72));
            step(&mut w, buttons::SWAP, 0);
            gun(&w, "us_m1_garand");
        }
    }
}

#[cfg(feature = "test-util")]
#[test]
fn resupply_respects_both_slot_caps_while_pistol_is_equipped() {
    let mut w = fixture(GameData::embedded(), None);
    deploy(&mut w, "us_rifleman_2");
    w.set_support(0, true);
    w.place_ammo_crate(0).unwrap();
    w.debug_set_ammo(0, 60, 2, 2);
    step(&mut w, 0, 1);
    w.debug_set_ammo(0, 0, 2, 2);
    let points: Vec<_> = (0..60)
        .flat_map(|_| {
            step(&mut w, buttons::USE, 1)
                .into_iter()
                .filter_map(|e| match e {
                    SimEvent::Resupplied { points, .. } => Some(points),
                    _ => None,
                })
        })
        .collect();
    assert_eq!(points, vec![5]); // one 4-point carbine magazine, one 1-point pistol magazine
    assert_eq!(gun(&w, "us_m1911a1").reserve, 7);
    step(&mut w, 0, 0);
    assert_eq!(gun(&w, "us_m1_carbine").reserve, 75);
}

#[cfg(feature = "test-util")]
#[test]
fn carrying_selects_sidearm_and_rejects_primary() {
    let mut w = fixture(GameData::embedded(), None);
    deploy(&mut w, "us_rifleman_2");
    w.set_support(0, true);
    w.add_depot(Team::Us, Vec3::new(30.0, 5.02, 30.0));
    w.debug_add_depot_stock(0, 1);
    for _ in 0..60 {
        step(&mut w, buttons::USE, 0);
    }
    assert!(w.carrying(0).is_some());
    gun(&w, "us_m1911a1");
    step(&mut w, buttons::SWAP, 0);
    gun(&w, "us_m1911a1");
}
