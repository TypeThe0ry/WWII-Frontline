//! Runs `--drive-vehicle` against the real server with generated flat terrain. The relay
//! observes the driver's own C2 stream without substituting any server messages or inputs.
//!
//! Build `frontline-server` first; `FRONTLINE_REQUIRE_SERVER=1` forbids skipping.

use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clap::Parser;
use frontline_headless::cli::Args;
use frontline_headless::run::{build_ctx, run};
use frontline_protocol::framing::decode_ws_frame;
use frontline_protocol::snapshot::{DecodedSnapshot, EntityKind, SnapshotDecoder};
use frontline_protocol::Channel;
use frontline_sim::collision::{CHUNK_CELLS, CHUNK_SAMPLES};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message};

struct Fixture {
    dir: PathBuf,
    server: Option<Child>,
}

impl Fixture {
    fn new() -> Self {
        let tag = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let dir = std::env::temp_dir().join(format!(
            "frontline-vehicle-flow-{}-{}",
            std::process::id(),
            tag.as_nanos()
        ));
        fs::create_dir(&dir).expect("create fixture directory");
        let fixture = Self { dir, server: None };
        let map = fixture.dir.join("maps/m0");
        fs::create_dir_all(map.join("chunks")).unwrap();
        fs::write(map.join("meta.json"), r#"{"crs":{"size_m":128}}"#).unwrap();
        fs::write(
            map.join("spawns.json"),
            r#"{"hq":{"us":{"pos":[32,32]},"ger":{"pos":[96,96]}}}"#,
        )
        .unwrap();
        // FCOL v1: zero height at every sample, with a uniform surface material.
        let mut chunk = vec![0; 16 + 2 * CHUNK_SAMPLES.pow(2) + CHUNK_CELLS.pow(2)];
        chunk[..4].copy_from_slice(b"FCOL");
        chunk[4] = 1;
        chunk[6..8].copy_from_slice(&(CHUNK_SAMPLES as u16).to_le_bytes());
        for x in 0..2 {
            for z in 0..2 {
                fs::write(map.join(format!("chunks/c_{x}_{z}.bin")), &chunk).unwrap();
            }
        }
        fixture
    }

    async fn start(&mut self, bin: PathBuf) -> u16 {
        let log = self.dir.join("server.log");
        let out = File::create(&log).unwrap();
        self.server = Some(
            Command::new(bin)
                .current_dir(&self.dir)
                .env_remove("GATEWAY_URL")
                .env_remove("FRONTLINE_DEV_NO_GATEWAY")
                .args([
                    "--map",
                    "m0",
                    "--mode",
                    "sandbox",
                    "--bots",
                    "0",
                    "--bot-fill",
                    "0",
                    "--ws-port",
                    "0",
                    "--metrics-port",
                    "0",
                    "--log-level",
                    "info",
                    "--dev-spawn-vehicle",
                    "--data-dir",
                ])
                .arg(&self.dir)
                .stdout(Stdio::from(out.try_clone().unwrap()))
                .stderr(Stdio::from(out))
                .spawn()
                .expect("start frontline-server"),
        );

        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let text = fs::read_to_string(&log).unwrap();
            if let Some(port) = text.lines().find_map(|line| {
                let event: Value = serde_json::from_str(line).ok()?;
                (event["event"] == "ws_listening").then(|| event["port"].as_u64().unwrap() as u16)
            }) {
                return port;
            }
            let status = self.server.as_mut().unwrap().try_wait().unwrap();
            assert!(status.is_none(), "server exited: {status:?}\n{text}");
            assert!(Instant::now() < deadline, "server did not listen:\n{text}");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(server) = &mut self.server {
            let _ = server.kill();
            let _ = server.wait();
        }
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn server_bin() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("FRONTLINE_SERVER_BIN") {
        // Resolve relative to the caller before switching to the fixture's working directory.
        return Some(fs::canonicalize(path).expect("invalid FRONTLINE_SERVER_BIN"));
    }
    let exe = std::env::current_exe().unwrap();
    let bin = exe.parent()?.parent()?.join("frontline-server");
    bin.is_file().then_some(bin)
}

async fn relay(listener: TcpListener, port: u16, snapshots: &mut Vec<DecodedSnapshot>) {
    let (socket, _) = listener.accept().await.unwrap();
    socket.set_nodelay(true).unwrap();
    let mut client = accept_async(socket).await.unwrap();
    let (mut server, _) = connect_async(format!("ws://127.0.0.1:{port}"))
        .await
        .unwrap();
    let mut decoder = SnapshotDecoder::new();
    // Once either peer hangs up, the driver's report distinguishes a regular end from a failure.
    loop {
        tokio::select! {
            msg = client.next() => {
                let Some(Ok(Message::Binary(frame))) = msg else { break };
                if server.send(Message::Binary(frame)).await.is_err() { break }
            }
            msg = server.next() => {
                let Some(Ok(Message::Binary(frame))) = msg else { break };
                let (channel, payload) = decode_ws_frame(&frame).expect("server WS frame");
                if channel == Channel::C2 {
                    snapshots.push(decoder.decode(payload).expect("server C2 snapshot"));
                }
                if client.send(Message::Binary(frame)).await.is_err() { break }
            }
        }
    }
}

#[tokio::test]
async fn headless_vehicle_boards_drives_and_exits_over_websocket() {
    let Some(bin) = server_bin() else {
        let msg = "frontline-server binary not found (cargo build -p frontline-server)";
        assert_ne!(
            std::env::var("FRONTLINE_REQUIRE_SERVER").as_deref(),
            Ok("1"),
            "{msg}"
        );
        eprintln!("SKIP: {msg}");
        return;
    };

    let mut fixture = Fixture::new();
    let port = fixture.start(bin).await;
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let args = Args::try_parse_from([
        "headless-client",
        "--url",
        &listener.local_addr().unwrap().to_string(),
        "--count",
        "1",
        "--duration",
        "8",
        "--warmup",
        "0",
        "--data-dir",
        fixture.dir.to_str().unwrap(),
        "--drive-vehicle",
    ])
    .expect("vehicle-mode CLI");
    args.validate().unwrap();
    let mut snapshots = Vec::new();
    let driver = run(&args, build_ctx(&args).unwrap());
    tokio::pin!(driver);
    let report = tokio::time::timeout(Duration::from_secs(15), async {
        tokio::select! {
            report = &mut driver => report,
            () = relay(listener, port, &mut snapshots) => driver.await,
        }
    })
    .await
    .expect("driver run timed out");
    let client = &report.per_client[0];
    assert!(client.join_error.is_none(), "{:?}", client.join_error);
    assert_eq!(client.disconnects, 0, "{:?}", client.disconnect_reason);
    assert_eq!(client.decode_errors, 0);
    let vehicle = client.vehicle.as_ref().expect("vehicle-mode report");

    let self_states: Vec<_> = snapshots.iter().filter_map(|s| s.self_state).collect();
    let first = self_states.first().expect("initial self state");
    assert_eq!(
        (first.vehicle_slot, first.seat),
        (0, 0),
        "initially on foot"
    );
    let fixture_slot = snapshots
        .iter()
        .find_map(|s| {
            s.entities
                .kind
                .iter()
                .position(|&kind| kind == EntityKind::Vehicle)
                .map(|row| s.entities.ids[row])
        })
        .expect("server never sent the fixture vehicle");
    let entry = self_states
        .iter()
        .position(|s| s.vehicle_slot != 0)
        .unwrap_or_else(|| {
            panic!("fixture vehicle {fixture_slot} visible but never boarded: {vehicle:?}")
        });
    let slot = self_states[entry].vehicle_slot;
    assert!((128..=255).contains(&slot), "invalid vehicle slot {slot}");
    assert_eq!(
        slot, fixture_slot,
        "SelfState must refer to the fixture vehicle"
    );
    let (driving, on_foot) = self_states[entry..].split_at(
        self_states[entry..]
            .iter()
            .position(|s| s.vehicle_slot == 0)
            .expect("no exit SelfState"),
    );
    assert!(driving
        .iter()
        .all(|s| (s.vehicle_slot, s.seat) == (slot, 0)));
    assert!(on_foot.iter().all(|s| (s.vehicle_slot, s.seat) == (0, 0)));
    assert!(
        self_states.iter().all(|s| s.health > 0),
        "exit must not be a death"
    );

    let mut positions = snapshots
        .iter()
        .filter(|s| {
            s.self_state
                .is_some_and(|state| (state.vehicle_slot, state.seat) == (slot, 0))
        })
        .filter_map(|s| {
            let row = s.entities.ids.iter().position(|&id| id == slot)?;
            assert_eq!(s.entities.kind[row], EntityKind::Vehicle);
            Some((s.entities.pos[3 * row], s.entities.pos[3 * row + 2]))
        });
    let (x0, z0) = positions
        .next()
        .expect("occupied slot has no vehicle entity");
    let displacement = positions
        .map(|(x, z)| (x - x0).hypot(z - z0))
        .fold(0.0f32, f32::max);
    assert!(
        displacement >= 1.0,
        "vehicle {slot} only moved {displacement:.2} m"
    );
    assert_eq!((vehicle.entries, vehicle.exits), (1, 1));
    assert_eq!(vehicle.driver_vehicle_slot, Some(slot));
    assert_eq!((vehicle.last_vehicle_slot, vehicle.last_seat), (0, 0));
    assert!(vehicle.driver_input_ticks > 0, "{vehicle:?}");
    assert!(vehicle.max_displacement_m >= 1.0, "{vehicle:?}");
    eprintln!("driver seat 0 on vehicle {slot}: {} snapshots, {displacement:.2} m authoritative displacement, then exited", driving.len());
}
