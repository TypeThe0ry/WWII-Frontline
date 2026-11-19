//! The placement slots use the real C0 dispatch, sim thread and map loader over loopback WS.

mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use common::{Client, Got};
use frontline_protocol::msg::c0::{
    decode_c0_server, encode_c0_client, ClientMsgC0, DeployRequest, PlacementAction,
    PlacementReason, PlacementRequest, PlacementResult, PlacementSlot, ServerMsgC0, SquadOp,
    SquadOpKind,
};
use frontline_protocol::Channel;
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::{InboundQueue, Sessions};
use frontline_server::shutdown::StopFlag;
use frontline_server::sim_bridge::NetLink;
use frontline_server::sim_thread::{self, SimThreadConfig};
use frontline_server::world_loader::{load_map, LoadedMap, MapError};
use frontline_sim::deploy::SPAWN_HQ;
use frontline_sim::{GameData, GameMode, MapId};

struct MapFixture(PathBuf);

impl MapFixture {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let this = Self(std::env::temp_dir().join(format!(
            "frontline-placement-{}-{stamp}",
            std::process::id()
        )));
        let map = this.0.join("maps/tactical");
        fs::create_dir_all(map.join("chunks")).unwrap();
        let mut chunk = vec![0u8; 16 + 65 * 65 * 2 + 64 * 64];
        chunk[..8].copy_from_slice(b"FCOL\x01\x00\x41\x00");
        chunk[8..12].copy_from_slice(&5f32.to_le_bytes());
        chunk[12..16].copy_from_slice(&5f32.to_le_bytes());
        fs::write(map.join("chunks/c_0_0.bin"), chunk).unwrap();
        fs::write(map.join("meta.json"), "{}").unwrap();
        fs::write(
            map.join("spawns.json"),
            r#"{"hq":{"us":{"pos":[12,24]},"ger":{"pos":[48,24]}}}"#,
        )
        .unwrap();
        fs::write(
            map.join("zones.json"),
            r#"{"lines":[{"points":[{"pos":[32,24],"radius_m":2}]}]}"#,
        )
        .unwrap();
        this
    }

    fn zones(&self, text: &str) {
        fs::write(self.0.join("maps/tactical/no_build_zones.json"), text).unwrap();
    }

    fn load(&self) -> Result<LoadedMap, MapError> {
        load_map(&self.0, "tactical")
    }
}

impl Drop for MapFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

async fn send(client: &mut Client, msg: ClientMsgC0) {
    assert!(
        client
            .send(Channel::C0, &encode_c0_client(&msg).unwrap())
            .await
    );
}

async fn reply(client: &mut Client) -> ServerMsgC0 {
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        match client
            .recv_within(end.saturating_duration_since(Instant::now()))
            .await
        {
            Got::Frame(Channel::C0, bytes) => return decode_c0_server(&bytes).unwrap(),
            Got::Frame(..) if Instant::now() < end => (),
            _ => panic!("no C0 reply"),
        }
    }
}

async fn placement(client: &mut Client, id: u16, action: PlacementAction) -> PlacementResult {
    send(
        client,
        ClientMsgC0::PlacementRequest(PlacementRequest { id, action }),
    )
    .await;
    let ServerMsgC0::PlacementResult(r) = reply(client).await else {
        panic!("wrong C0 reply")
    };
    assert_eq!(r.id, id);
    r
}

fn preview(slot: PlacementSlot) -> PlacementAction {
    PlacementAction::Preview {
        slot,
        yaw: 0,
        pitch: i16::MIN,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tactical_role_slots_and_single_use_preview_over_websocket() {
    let fixture = MapFixture::new();
    fixture.zones("[[[0,0],[1,1],[2,2]]]");
    assert!(matches!(fixture.load(), Err(MapError::BadJson { .. })));
    // The first client (US) spawns outside this polygon; the second (GER) inside.
    fixture.zones("[[[45,20],[51,20],[51,28],[45,28]]]");
    let map = Arc::new(fixture.load().unwrap());
    let metrics = Metrics::new();
    let sessions = Sessions::with_limits(metrics.clone(), 256, 128);
    let inbound = InboundQueue::new();
    let stop = StopFlag::new();
    let sim = sim_thread::spawn(
        SimThreadConfig {
            map_data: Some(map),
            net: Some(NetLink {
                inbound: inbound.rx.clone(),
                sessions: sessions.clone(),
            }),
            ..SimThreadConfig::bare(1, MapId(0), GameMode::Sandbox)
        },
        metrics.clone(),
        stop.clone(),
    )
    .unwrap();
    let server = common::sim_harness::start_listeners(
        metrics,
        sessions,
        &inbound,
        sim.commands_tx.clone(),
        false,
    )
    .await;

    let (mut us, _) = Client::handshake(&server.ws_target.url).await;
    let (mut ger, _) = Client::handshake(&server.ws_target.url).await;
    let (mut us2, _) = Client::handshake(&server.ws_target.url).await;
    let (mut ger2, _) = Client::handshake(&server.ws_target.url).await;
    let (mut us3, _) = Client::handshake(&server.ws_target.url).await;
    let (mut ger3, _) = Client::handshake(&server.ws_target.url).await;
    assert_eq!(
        placement(&mut us, 0, preview(PlacementSlot::Ammo))
            .await
            .reason,
        PlacementReason::NotAlive
    );
    for (client, squad, loadout, mates) in [
        (&mut us, 1, "us_support_1", [&mut us2, &mut us3]),
        (&mut ger, 2, "ger_support_1", [&mut ger2, &mut ger3]),
    ] {
        send(
            client,
            ClientMsgC0::SquadOp(SquadOp {
                op: SquadOpKind::Create,
                squad: 0,
                target: 0,
                name_key: String::new(),
            }),
        )
        .await;
        // A reply on the same stream confirms the preceding create/join reached the sim.
        assert_eq!(
            placement(client, 99, preview(PlacementSlot::Ammo))
                .await
                .reason,
            PlacementReason::NotAlive
        );
        for mate in mates {
            send(
                mate,
                ClientMsgC0::SquadOp(SquadOp {
                    op: SquadOpKind::Join,
                    squad,
                    target: 0,
                    name_key: String::new(),
                }),
            )
            .await;
            assert_eq!(
                placement(mate, 99, preview(PlacementSlot::Ammo))
                    .await
                    .reason,
                PlacementReason::NotAlive
            );
        }
        if squad == 1 {
            deploy(client, squad, "officer", "us_officer_1").await;
        } else {
            deploy(client, squad, "support", loadout).await;
        }
    }
    deploy(&mut us2, 1, "support", "us_support_1").await;
    // Let the new bodies settle on the synthetic ground.
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        placement(&mut us2, 1, preview(PlacementSlot::Garrison))
            .await
            .reason,
        PlacementReason::WrongRole
    );
    assert_eq!(
        placement(&mut us2, 2, preview(PlacementSlot::Supply))
            .await
            .reason,
        PlacementReason::NotCarrying
    );
    assert_eq!(
        placement(&mut ger, 1, preview(PlacementSlot::Ammo))
            .await
            .reason,
        PlacementReason::NoBuildZone
    );
    let aimed = placement(&mut us, 7, preview(PlacementSlot::Garrison)).await;
    assert_eq!(aimed.reason, PlacementReason::TooCloseObjective);
    assert_eq!(aimed.target_cm.unwrap()[1], 500);
    let sky = placement(
        &mut us,
        8,
        PlacementAction::Preview {
            slot: PlacementSlot::Garrison,
            yaw: 0,
            pitch: i16::MAX,
        },
    )
    .await;
    assert_eq!(sky.reason, PlacementReason::NoTarget);
    assert_eq!(sky.target_cm, None);
    let allowed = placement(&mut us2, 3, preview(PlacementSlot::Ammo)).await;
    assert_eq!(allowed.reason, PlacementReason::Ok);
    assert_eq!(allowed.target_cm.unwrap()[1], 500);
    assert_eq!(
        placement(&mut ger, 2, PlacementAction::Commit { preview_id: 3 })
            .await
            .reason,
        PlacementReason::NoTarget
    );
    assert_eq!(
        placement(&mut us2, 4, PlacementAction::Commit { preview_id: 3 })
            .await
            .reason,
        PlacementReason::Ok
    );
    assert_eq!(
        placement(&mut us2, 5, PlacementAction::Commit { preview_id: 3 })
            .await
            .reason,
        PlacementReason::NoTarget
    );
    assert_eq!(
        placement(&mut us2, 6, preview(PlacementSlot::Ammo))
            .await
            .reason,
        PlacementReason::Cooldown
    );

    drop((us, ger, us2, ger2, us3, ger3));
    stop.stop();
    sim.join().unwrap();
    server.stop().await;
}

async fn deploy(client: &mut Client, squad: u8, role: &str, loadout: &str) {
    let data = GameData::embedded();
    let role = data.roles.iter().position(|r| r.id == role).unwrap() as u8;
    let loadout = data.loadouts.iter().position(|l| l.id == loadout).unwrap() as u8;
    send(
        client,
        ClientMsgC0::DeployRequest(DeployRequest {
            spawn_point: SPAWN_HQ,
            squad,
            role,
            loadout,
        }),
    )
    .await;
    let r = reply(client).await;
    assert!(
        matches!(r, ServerMsgC0::DeployResult(ref d) if d.ok),
        "{r:?}"
    );
}
