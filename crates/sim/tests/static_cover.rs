use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::ballistics::materials::MaterialTable;
use frontline_sim::ballistics::penetration::LayerWorld;
use frontline_sim::collision::CoverSlab;
use frontline_sim::throwables::Occlusion;
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, SimConfig, SimEvent, SpawnReq, Team,
    Vec3, World,
};

fn arena(slabs: &[(f32, f32, &str, Occlusion)]) -> CollisionData {
    let mut col =
        CollisionData::from_chunks([CollisionChunk::new(0, 0, vec![0.0; 4225], vec![0; 4096])]);
    let mats = MaterialTable::embedded();
    col.install_cover(
        slabs
            .iter()
            .map(|&(x, width, mat, occ)| {
                CoverSlab::new(
                    [x, 10.0],
                    [x, 16.0],
                    0.0,
                    4.0,
                    width,
                    mats.id_of(mat).unwrap(),
                    occ,
                )
                .unwrap()
            })
            .collect(),
    );
    col
}

#[test]
fn nearest_layer_has_unclipped_exit_and_los_checks_all_cover() {
    let col = arena(&[
        (24.0, 0.3, "brick", Occlusion::Solid),
        (20.0, 0.03, "thin_wood", Occlusion::Thin),
    ]);
    let start = Vec3::new(10.0, 1.2, 12.0);
    let end = Vec3::new(30.0, 1.2, 12.0);
    let layer = col.first_layer(start, Vec3::X, 10.0).unwrap();
    assert_eq!(layer.normal, Vec3::NEG_X);
    assert!(layer.entry.x < 20.0 && layer.exit.x > 20.0);
    assert!((layer.thickness_mm() - 30.0).abs() < 0.01);
    assert!(col.terrain_los(start, end) && !col.los(start, end));
    assert_eq!(col.blast_cover(start, end), Occlusion::Solid);
    assert_eq!(col.state_hash(), col.clone().state_hash());
    assert_ne!(col.state_hash(), arena(&[]).state_hash());
}

fn shoot(
    slabs: &[(f32, f32, &str, Occlusion)],
    origin: Vec3,
    vel: Vec3,
) -> (Vec<SimEvent>, Vec<Vec3>) {
    let data = GameData::embedded();
    let mut w = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 9,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data,
        },
        &arena(slabs),
    );
    for (player, team, x) in [(0, Team::Us, 10.0), (1, Team::Ger, 37.0)] {
        w.spawn_player(SpawnReq {
            player,
            team,
            pos: Vec3::new(x, 0.03, 12.0),
            yaw: 0.0,
        })
        .unwrap();
    }
    let weapon = data
        .weapons
        .iter()
        .position(|w| w.id == "us_m1_garand")
        .unwrap() as u16;
    w.spawn_projectile(0, weapon, origin, vel).unwrap();
    let (mut events, mut velocity) = (Vec::new(), Vec::new());
    for _ in 0..8 {
        events.extend(w.step(&[]).events);
        velocity.extend(w.projectiles().iter().map(|p| p.vel));
    }
    (events, velocity)
}

#[test]
fn projectile_integrates_wood_damage_and_stops_at_masonry_and_third_layer() {
    let origin = Vec3::new(10.0, 1.24, 12.0);
    let wood = |x| (x, 0.03, "thin_wood", Occlusion::Thin);
    let hit = |events: &[SimEvent]| {
        events.iter().find_map(|e| match e {
            SimEvent::Hit { damage_hp, .. } => Some(*damage_hp),
            _ => None,
        })
    };
    let clean = shoot(&[], origin, Vec3::X * 850.0).0;
    let through = shoot(&[wood(20.0)], origin, Vec3::X * 850.0).0;
    let ratio = hit(&through).unwrap() / hit(&clean).unwrap();
    println!("30 mm wood: health damage ratio {ratio:.6}");
    assert!((ratio - (190.0 / 220.0)).abs() < 0.002);
    for (slabs, impacts) in [
        (vec![(20.0, 0.3, "brick", Occlusion::Solid)], 1),
        (vec![wood(20.0), wood(21.0), wood(22.0)], 3),
    ] {
        let events = shoot(&slabs, origin, Vec3::X * 850.0).0;
        assert!(hit(&events).is_none());
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, SimEvent::Impact { .. }))
                .count(),
            impacts
        );
    }
}

#[test]
fn shallow_masonry_hit_reflects_and_continues() {
    let (events, velocity) = shoot(
        &[(20.0, 0.3, "brick", Occlusion::Solid)],
        Vec3::new(19.70, 1.24, 11.0),
        Vec3::new(85.0, 0.0, 850.0),
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, SimEvent::Impact { .. }))
            .count(),
        1
    );
    let v = velocity[0];
    assert!(v.x < 0.0 && (500.0..550.0).contains(&v.length()), "{v:?}");
}

#[test]
fn melee_cannot_cross_static_cover() {
    for wall in [false, true] {
        let slabs = [(20.0, 0.3, "brick", Occlusion::Solid)];
        let mut w = World::new(
            SimConfig {
                tick_hz: 30,
                match_seed: 9,
                map: MapId(0),
                mode: GameMode::Sandbox,
                data: GameData::embedded(),
            },
            &arena(if wall { &slabs } else { &[] }),
        );
        for (player, team, x) in [(0, Team::Us, 19.4), (1, Team::Ger, 20.6)] {
            w.spawn_player(SpawnReq {
                player,
                team,
                pos: Vec3::new(x, 0.03, 12.0),
                yaw: 0.0,
            })
            .unwrap();
        }
        let mut hits = 0;
        for tick in 1..=15 {
            let out = w.step(&[(
                0,
                InputCmd {
                    seq: tick,
                    tick: u32::from(tick),
                    yaw: 16384,
                    buttons: buttons::MELEE,
                    move_x: 0,
                    move_z: 0,
                    pitch: 0,
                    weapon_slot: 0,
                    aux: 0,
                    view_tick: 0,
                },
            )]);
            hits += out
                .events
                .iter()
                .filter(|e| matches!(e, SimEvent::MeleeHit { .. }))
                .count();
        }
        assert_eq!(hits, usize::from(!wall));
    }
}
