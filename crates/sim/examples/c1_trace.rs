//! Native and WASI fixture for full authority state, independent of the browser Predictor ABI.
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::ballistics::materials::MaterialTable;
use frontline_sim::collision::CoverSlab;
use frontline_sim::deploy::{DeployReq, SPAWN_HQ};
use frontline_sim::throwables::Occlusion;
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, SimConfig, SimEvent, SpawnReq, Team,
    Vec3, World,
};

fn main() {
    let data = GameData::embedded();
    let mut col =
        CollisionData::from_chunks([CollisionChunk::new(0, 0, vec![0.0; 4225], vec![0; 4096])]);
    let materials = MaterialTable::embedded();
    col.install_cover(vec![CoverSlab::new(
        [24.0, 13.0],
        [24.0, 20.0],
        0.0,
        3.0,
        0.03,
        materials.id_of("thin_wood").unwrap(),
        Occlusion::Thin,
    )
    .unwrap()]);
    let mut world = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 9,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data,
        },
        &col,
    );
    world.set_team_spawn(Team::Us, Vec3::new(17.0, 0.03, 16.0));
    world.roster_add(0, Team::Us).unwrap();
    world
        .deploy(
            0,
            &DeployReq {
                role: data.roles.iter().position(|r| r.id == "rifleman").unwrap() as u8,
                loadout: data
                    .loadouts
                    .iter()
                    .position(|l| l.id == "us_rifleman_2")
                    .unwrap() as u8,
                squad: 0,
                spawn_point: SPAWN_HQ,
            },
        )
        .unwrap();
    let mut ids = Vec::new();
    for (player, team, x, z) in [(1, Team::Ger, 27.0, 16.0), (2, Team::Us, 17.0, 20.0)] {
        ids.push(
            world
                .spawn_player(SpawnReq {
                    player,
                    team,
                    pos: Vec3::new(x, 0.03, z),
                    yaw: 0.0,
                })
                .unwrap(),
        );
    }
    let weapon = data
        .weapons
        .iter()
        .position(|w| w.id == "us_m1_garand")
        .unwrap() as u16;
    let sidearm = data
        .weapons
        .iter()
        .position(|w| w.id == "us_m1911a1")
        .unwrap() as u16;
    let mut coverage = [0_u32; 6];
    for tick in 1..=480 {
        if [16, 27].contains(&tick) {
            world
                .spawn_projectile(0, weapon, Vec3::new(17.0, 1.23, 16.0), Vec3::X * 850.0)
                .unwrap();
        }
        let a = match tick {
            35 => buttons::SWAP,
            54 => buttons::FIRE,
            60..=179 => buttons::GADGET,
            _ => 0,
        };
        let b = if (62..80).contains(&tick) {
            buttons::GADGET | buttons::AIM
        } else {
            0
        };
        let cmd = |buttons| InputCmd {
            seq: tick as u16,
            tick,
            yaw: 16384,
            buttons,
            move_x: 0,
            move_z: 0,
            pitch: 0,
            weapon_slot: 0,
            aux: 0,
            view_tick: 0,
        };
        let out = world.step(&[(0, cmd(a)), (1, cmd(0)), (2, cmd(b))]);
        println!("{tick:03} {:016x}", world.state_hash());
        for e in out.events {
            let i = match e {
                SimEvent::Impact { .. } => 0,
                SimEvent::Hit { target, .. } if target == ids[0] => 1,
                SimEvent::Explosion { .. } => 2,
                SimEvent::Suppress { .. } => 3,
                SimEvent::SmokeStarted { .. } => 4,
                SimEvent::Shot { weapon, .. } if weapon == sidearm => 5,
                _ => continue,
            };
            coverage[i] += 1;
        }
    }
    assert!(
        coverage.iter().all(|&x| x > 0),
        "missing coverage: {coverage:?}"
    );
    println!("coverage impact/body/blast/suppression/smoke/sidearm={coverage:?}");
}
