//! M2-019 comms acceptance over real WebSocket sessions and the real sim thread.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::{Client, Got};
use frontline_protocol::msg::c0::{
    decode_c0_server, encode_c0_client, Chat, ChatChannel, ClientMsgC0, ServerMsgC0, SquadOp,
    SquadOpKind,
};
use frontline_protocol::msg::c4::{decode_c4, ServerEventC4};
use frontline_protocol::msg::{PingKind, PingPlace};
use frontline_protocol::snapshot::events::{Event, MarkerKind};
use frontline_protocol::snapshot::SnapshotDecoder;
use frontline_protocol::Channel;
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::{InboundQueue, Sessions};
use frontline_server::shutdown::StopFlag;
use frontline_server::sim_bridge::NetLink;
use frontline_server::sim_thread::{self, SimThreadConfig};
use frontline_server::world_loader::{load_map, LoadedMap};
use frontline_sim::{GameMode, MapId};
use futures_util::future::join_all;

const CLIENTS: usize = 10;
const TIMEOUT: Duration = Duration::from_secs(3);
const COOLDOWN_NOTICE: &str = "comms.marker.cooldown";

// The test must run even when the generated terrain is absent. A flat 64 m chunk keeps all
// players alive and gives the pings a known map extent and an empty patch away from either HQ.
fn flat_map() -> LoadedMap {
    let data = tempfile::tempdir().unwrap();
    let map = data.path().join("maps/comms");
    fs::create_dir_all(map.join("chunks")).unwrap();
    let mut chunk = vec![0u8; 16 + 65 * 65 * 2 + 64 * 64];
    chunk[..8].copy_from_slice(b"FCOL\x01\x00\x41\x00");
    fs::write(map.join("chunks/c_0_0.bin"), chunk).unwrap();
    fs::write(map.join("meta.json"), r#"{"crs":{"size_m":64}}"#).unwrap();
    fs::write(
        map.join("spawns.json"),
        r#"{"hq":{"us":{"pos":[12,48]},"ger":{"pos":[48,48]}}}"#,
    )
    .unwrap();
    load_map(data.path(), "comms").unwrap()
}

struct StopOnDrop(StopFlag);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.stop();
    }
}

struct Peer {
    socket: Client,
    id: u8,
    team: u8,
    squad: u8,
}

impl Peer {
    async fn send(&mut self, msg: ClientMsgC0) {
        let bytes = encode_c0_client(&msg).unwrap();
        assert!(
            self.socket.send(Channel::C0, &bytes).await,
            "player {}",
            self.id
        );
    }

    async fn squad_op(&mut self, op: SquadOpKind, squad: u8) -> u8 {
        self.send(ClientMsgC0::SquadOp(SquadOp {
            op,
            squad,
            target: 0,
            name_key: String::new(),
        }))
        .await;
        let end = Instant::now() + TIMEOUT;
        loop {
            match self
                .socket
                .recv_within(end.saturating_duration_since(Instant::now()))
                .await
            {
                Got::Frame(Channel::C4, bytes) => {
                    if let ServerEventC4::PlayerRoster(row) = decode_c4(&bytes).unwrap() {
                        if row.player == self.id && row.squad != 0 {
                            assert_eq!(row.team, self.team);
                            if squad != 0 {
                                assert_eq!(row.squad, squad);
                            }
                            self.squad = row.squad;
                            return row.squad;
                        }
                    }
                }
                Got::Frame(..) => (),
                Got::Closed | Got::Timeout => {
                    panic!("no squad confirmation for player {}", self.id)
                }
            }
        }
    }

    async fn chat(&mut self, channel: ChatChannel, text: &str) -> Chat {
        self.send(ClientMsgC0::Chat(Chat {
            channel,
            text: text.into(),
            sender: u8::MAX, // The server must stamp the connection's player ID.
        }))
        .await;
        Chat {
            channel,
            text: text.into(),
            sender: self.id,
        }
    }

    async fn ping(&mut self, metre_code: u16) {
        self.send(ClientMsgC0::PingPlace(PingPlace {
            // An explicit Thanks has squad scope regardless of the current aim ray.
            kind: PingKind::Thanks,
            pos: [metre_code, metre_code],
            target: u16::MAX,
        }))
        .await;
    }

    async fn observe(&mut self, end: Instant) -> Observed {
        let mut observed = Observed::default();
        let mut decoder = SnapshotDecoder::new();
        while Instant::now() < end {
            match self
                .socket
                .recv_within(end.saturating_duration_since(Instant::now()))
                .await
            {
                Got::Frame(Channel::C0, bytes) => match decode_c0_server(&bytes).unwrap() {
                    ServerMsgC0::Chat(chat) => observed.chat.push(chat),
                    ServerMsgC0::Notice(notice) => observed.notices.push(notice.key),
                    _ => (),
                },
                Got::Frame(Channel::C2, bytes) => {
                    for entry in decoder.decode(&bytes).unwrap().events {
                        if let Event::MarkerUpsert {
                            id,
                            kind,
                            pos,
                            ttl,
                            owner,
                            target,
                        } = entry.event
                        {
                            assert_eq!(kind, MarkerKind::Ping(PingKind::Thanks));
                            assert_eq!(pos, [4, 4], "unexpected ping from player {owner}");
                            assert!((1..=8).contains(&ttl));
                            assert_eq!(target, None);
                            // The same marker can be repeated in snapshots for loss recovery.
                            observed.marker_ids.entry(owner).or_default().insert(id);
                        }
                    }
                }
                Got::Frame(..) => (),
                Got::Closed => panic!("connection closed for player {}", self.id),
                Got::Timeout => break,
            }
        }
        observed
    }
}

#[derive(Default)]
struct Observed {
    chat: Vec<Chat>,
    marker_ids: BTreeMap<u8, BTreeSet<u16>>,
    notices: Vec<String>,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ten_sessions_route_ping_and_chat_by_scope() {
    let metrics = Metrics::new();
    let sessions = Sessions::with_limits(metrics.clone(), CLIENTS, CLIENTS);
    let inbound = InboundQueue::new();
    let stop = StopOnDrop(StopFlag::new());
    let sim = sim_thread::spawn(
        SimThreadConfig {
            map_data: Some(Arc::new(flat_map())),
            net: Some(NetLink {
                inbound: inbound.rx.clone(),
                sessions: sessions.clone(),
            }),
            ..SimThreadConfig::bare(1, MapId(0), GameMode::Sandbox)
        },
        metrics.clone(),
        stop.0.clone(),
    )
    .unwrap();
    let server = common::sim_harness::start_listeners(
        metrics.clone(),
        sessions.clone(),
        &inbound,
        sim.commands_tx.clone(),
        false,
    )
    .await;

    // Sequential admission alternates teams. Each team gets a squad of three and a squad of two.
    let mut peers = Vec::new();
    let mut squads = [0; 4];
    for index in 0..CLIENTS {
        let (socket, welcome) = Client::handshake(&server.ws_target.url).await;
        assert_eq!(welcome.team as usize, index % 2);
        let mut peer = Peer {
            socket,
            id: welcome.player_id,
            team: welcome.team,
            squad: 0,
        };
        let slot = index % squads.len();
        if index < squads.len() {
            squads[slot] = peer.squad_op(SquadOpKind::Create, 0).await;
        } else {
            peer.squad_op(SquadOpKind::Join, squads[slot]).await;
        }
        peers.push(peer);
    }
    assert_eq!(sessions.connections(), CLIENTS);
    assert_eq!(squads.into_iter().collect::<BTreeSet<_>>().len(), 4);

    let mut sent = Vec::new();
    for (index, peer) in peers.iter_mut().enumerate() {
        let channel = match index {
            0 | 1 => ChatChannel::All,
            2 | 3 => ChatChannel::Team,
            _ => ChatChannel::Squad,
        };
        sent.push(peer.chat(channel, &format!("message-{index}")).await);
        peer.ping(20_480).await; // 20 m on a 64 m map.
        if index == 0 {
            peer.ping(30_720).await; // A second ping inside the cooldown would be visible at 30 m.
            for line in 1..=4 {
                sent.push(
                    peer.chat(ChatChannel::Squad, &format!("burst-{line}"))
                        .await,
                );
            }
            peer.chat(ChatChannel::All, "rate-limited").await;
        } else if index == 1 {
            peer.chat(ChatChannel::Command, "unauthorized-command")
                .await;
        } else if index == 2 {
            peer.chat(ChatChannel::All, "rejected\u{202e}text").await;
        } else if index == 3 {
            for _ in 0..2 {
                sent.push(peer.chat(ChatChannel::Team, "repeat").await);
            }
            peer.chat(ChatChannel::Team, "repeat").await;
        }
    }

    // Read every connection through the same deadline so out-of-scope traffic counts as a failure
    // even if it arrives after all expected traffic. Both C0 and C2 use the real socket here.
    let end = Instant::now() + Duration::from_secs(2);
    let observations = join_all(peers.iter_mut().map(|peer| peer.observe(end))).await;
    let sort_chat =
        |lines: &mut [Chat]| lines.sort_by(|a, b| (a.sender, &a.text).cmp(&(b.sender, &b.text)));
    for (peer, mut observed) in peers.iter().zip(observations) {
        let mut expected_chat: Vec<_> = sent
            .iter()
            .filter(|line| {
                let author = peers.iter().find(|p| p.id == line.sender).unwrap();
                match line.channel {
                    ChatChannel::All => true,
                    ChatChannel::Team => author.team == peer.team,
                    ChatChannel::Squad => author.team == peer.team && author.squad == peer.squad,
                    ChatChannel::Command => unreachable!(),
                }
            })
            .cloned()
            .collect();
        sort_chat(&mut expected_chat);
        sort_chat(&mut observed.chat);
        assert_eq!(
            observed.chat, expected_chat,
            "C0 chat for player {}",
            peer.id
        );

        let expected_owners: BTreeSet<_> = peers
            .iter()
            .filter(|p| p.team == peer.team && p.squad == peer.squad)
            .map(|p| p.id)
            .collect();
        let actual_owners: BTreeSet<_> = observed.marker_ids.keys().copied().collect();
        assert_eq!(
            actual_owners, expected_owners,
            "C2 pings for player {}",
            peer.id
        );
        assert!(observed.marker_ids.values().all(|ids| ids.len() == 1));
        for (sender, key) in [
            (peers[0].id, COOLDOWN_NOTICE),
            (peers[0].id, "comms.chat.rate"),
            (peers[1].id, "comms.chat.channel"),
            (peers[2].id, "comms.chat.bidi"),
            (peers[3].id, "comms.chat.duplicate"),
        ] {
            let count = observed
                .notices
                .iter()
                .filter(|notice| notice.as_str() == key)
                .count();
            assert_eq!(
                count,
                usize::from(peer.id == sender),
                "{key} for player {}",
                peer.id
            );
        }
    }
    assert_eq!(
        metrics
            .c0_rate_limited_total
            .with_label_values(&["chat"])
            .get(),
        1
    );
    for reason in ["channel", "bidi", "duplicate"] {
        assert_eq!(
            metrics
                .chat_rejected_total
                .with_label_values(&[reason])
                .get(),
            1,
            "{reason}"
        );
    }

    drop(peers);
    server.stop().await;
    stop.0.stop();
    sim.join().unwrap();
}
