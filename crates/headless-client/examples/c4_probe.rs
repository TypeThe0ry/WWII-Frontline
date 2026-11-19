//! Subprocess probe for the server's C4 transport tests.

use std::net::{Ipv4Addr, SocketAddrV4};
use std::process::ExitCode;
use std::time::Duration;

use frontline_headless::transport::{self, Incoming, Kind, Link, Target};
use frontline_protocol::msg::c0::{
    decode_c0_server, encode_c0_client, ClientMsgC0, DeployRequest, Hello, ServerMsgC0,
};
use frontline_protocol::msg::c4::{decode_c4, Notice, ServerEventC4};
use frontline_protocol::{Channel, PROTOCOL_VERSION};
use frontline_sim::{data::GameData, deploy::SPAWN_HQ};
use serde::Serialize;
use tokio::sync::mpsc::error::TryRecvError;

const TIMEOUT: Duration = Duration::from_secs(10);
const SEQUENCE: u16 = 65534;
const BARRIER: u16 = 65535;

#[derive(Serialize)]
struct Trace {
    player: u8,
    team: u8,
    gameplay: Vec<Vec<u8>>,
    sequence: u32,
    spawn_tick: Option<u32>,
}

struct Peer {
    link: Link,
    trace: Trace,
    data_hash: u64,
    count: u32,
    welcomed: bool,
    barrier: bool,
    killed: bool,
    deploy_request: Option<Vec<u8>>,
    deploy_pending: bool,
}

impl Peer {
    async fn connect(
        target: &Target,
        player: u8,
        data_hash: u64,
        count: u32,
        deploy_request: Option<Vec<u8>>,
    ) -> Result<Self, String> {
        let link = transport::connect(target)
            .await
            .map_err(|e| format!("player {player}: connect: {e}"))?;
        let hello = encode_c0_client(&ClientMsgC0::Hello(Hello {
            protocol_version: PROTOCOL_VERSION,
            data_hash,
            ticket: Vec::new(),
            client_build: 1,
            caps: if target.kind == Kind::Wt { 0b10 } else { 0 },
        }))
        .map_err(|e| format!("player {player}: encode Hello: {e}"))?;
        if !link.send(Channel::C0, hello) {
            return Err(format!("player {player}: connection closed before Hello"));
        }
        Ok(Self {
            link,
            trace: Trace {
                player,
                team: player,
                gameplay: Vec::new(),
                sequence: 0,
                spawn_tick: None,
            },
            data_hash,
            count,
            welcomed: false,
            barrier: false,
            killed: false,
            deploy_request,
            deploy_pending: false,
        })
    }

    async fn receive(&mut self) -> Result<(), String> {
        // Snapshots and other unrelated traffic must not hide a C0/C4 stall.
        let incoming = tokio::time::timeout(TIMEOUT, async {
            loop {
                match self.link.rx.recv().await {
                    Some(Incoming::Frame(ch, _)) if !matches!(ch, Channel::C0 | Channel::C4) => {}
                    incoming => return incoming,
                }
            }
        })
        .await
        .map_err(|_| {
            format!(
                "player {}: 10s C0/C4 timeout (Welcome={}, sequence={}/{}, barrier={}, deploy_pending={})",
                self.trace.player, self.welcomed, self.trace.sequence, self.count,
                self.barrier, self.deploy_pending,
            )
        })?
        .ok_or_else(|| format!("player {}: connection dropped", self.trace.player))?;
        self.handle(incoming)
    }

    fn handle(&mut self, incoming: Incoming) -> Result<(), String> {
        let result = match incoming {
            Incoming::Frame(Channel::C0, bytes) => self.control(&bytes),
            Incoming::Frame(Channel::C4, bytes) => self.event(bytes),
            Incoming::Closed(reason) => Err(format!("connection closed: {reason}")),
            _ => Ok(()),
        };
        let player = self.trace.player;
        result.map_err(|e| format!("player {player}: {e}"))?;

        // WT can deliver the kill before Welcome; defer sending until the identity is checked.
        if self.welcomed && self.killed {
            if let Some(payload) = self.deploy_request.take() {
                if !self.link.send(Channel::C0, payload) {
                    return Err(format!("player {player}: connection closed before deploy"));
                }
                self.deploy_pending = true;
            }
        }
        Ok(())
    }

    fn control(&mut self, bytes: &[u8]) -> Result<(), String> {
        match decode_c0_server(bytes).map_err(|e| format!("decode C0: {e}"))? {
            ServerMsgC0::Welcome(w) => {
                if self.welcomed
                    || (w.player_id, w.team, w.data_hash)
                        != (self.trace.player, self.trace.team, self.data_hash)
                {
                    return Err(format!("unexpected Welcome: {w:?}"));
                }
                self.welcomed = true;
            }
            ServerMsgC0::Reject(r) => return Err(format!("rejected: {r:?}")),
            ServerMsgC0::Disconnect(d) => return Err(format!("disconnected: {d:?}")),
            ServerMsgC0::DeployResult(r) => {
                if !self.deploy_pending || !r.ok {
                    return Err(format!("unexpected or failed DeployResult: {r:?}"));
                }
                self.deploy_pending = false;
                self.trace.spawn_tick = Some(r.spawn_tick);
            }
            _ => {}
        }
        Ok(())
    }

    fn event(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let event = decode_c4(&bytes).map_err(|e| format!("decode C4: {e}"))?;
        if self.barrier {
            return Err(format!("C4 after final barrier: {event:?}"));
        }
        match event {
            ServerEventC4::Notice(Notice {
                code: SEQUENCE,
                args,
            }) => {
                let [seq, a, b] = args;
                let expected = self.trace.sequence;
                if u32::try_from(seq) != Ok(expected) || a != 0 || b != 0 || expected >= self.count
                {
                    return Err(format!(
                        "sequence notice {args:?}, expected [{expected}, 0, 0] below {}",
                        self.count
                    ));
                }
                self.trace.sequence += 1;
                return Ok(());
            }
            ServerEventC4::Notice(Notice {
                code: BARRIER,
                args,
            }) => {
                if args != [0, 0, 0] || self.trace.sequence != self.count {
                    return Err(format!(
                        "invalid barrier {args:?} at sequence {}/{}",
                        self.trace.sequence, self.count
                    ));
                }
                self.barrier = true;
                return Ok(());
            }
            ServerEventC4::Kill(k) if k.victim == self.trace.player => self.killed = true,
            _ => {}
        }
        self.trace.gameplay.push(bytes);
        Ok(())
    }

    async fn finish(&mut self) -> Result<(), String> {
        while !(self.welcomed && self.barrier && !self.deploy_pending) {
            self.receive().await?;
        }
        Ok(())
    }

    fn check_queued(&mut self) -> Result<(), String> {
        // Catch trailing duplicates already delivered by Link without waiting past the barrier.
        loop {
            match self.link.rx.try_recv() {
                Ok(incoming) => self.handle(incoming)?,
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => {
                    return Err(format!("player {}: connection dropped", self.trace.player));
                }
            }
        }
    }
}

fn deploy_payload(data: &GameData) -> Result<Vec<u8>, String> {
    let role = data.roles.iter().position(|r| r.id == "rifleman");
    let loadout = data.loadouts.iter().position(|l| l.id == "ger_rifleman_1");
    let index = |pos: Option<usize>, id: &str| {
        pos.and_then(|i| u8::try_from(i).ok())
            .ok_or_else(|| format!("no wire index for {id}"))
    };
    encode_c0_client(&ClientMsgC0::DeployRequest(DeployRequest {
        spawn_point: SPAWN_HQ,
        role: index(role, "rifleman")?,
        loadout: index(loadout, "ger_rifleman_1")?,
        squad: 0,
    }))
    .map_err(|e| format!("encode DeployRequest: {e}"))
}

async fn run() -> Result<[Trace; 2], String> {
    let [transport, addr, cert, count]: [String; 4] = std::env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "usage: c4_probe <ws|wt> <127.0.0.1:port> <cert hex or -> <sequence count>")?;
    let kind = match transport.as_str() {
        "ws" => Kind::Ws,
        "wt" => Kind::Wt,
        _ => return Err("transport must be ws or wt".into()),
    };
    let addr: SocketAddrV4 = addr.parse().map_err(|_| "expected 127.0.0.1:port")?;
    if *addr.ip() != Ipv4Addr::LOCALHOST || addr.port() == 0 {
        return Err("expected 127.0.0.1:<nonzero port>".into());
    }
    let cert_hash = match cert.as_str() {
        "-" if kind == Kind::Ws => None,
        hex if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) => {
            Some(transport::parse_cert_hash(hex)?)
        }
        _ => return Err("expected a 64-digit hex certificate hash (or - for ws)".into()),
    };
    let count = count.parse().map_err(|_| "invalid sequence count")?;
    if count > i32::MAX as u32 + 1 {
        return Err("sequence count exceeds the nonnegative i32 notice range".into());
    }
    let target = Target {
        kind,
        addr: addr.to_string(),
        cert_hash,
        bind_ip: None,
        connect_timeout: TIMEOUT,
    };

    let data = GameData::embedded();
    let request = deploy_payload(data)?;
    let mut a = Peer::connect(&target, 0, data.hash, count, None).await?;
    while !a.welcomed {
        a.receive().await?;
    }
    let ((), mut b) = tokio::try_join!(a.finish(), async {
        let mut b = Peer::connect(&target, 1, data.hash, count, Some(request)).await?;
        b.finish().await?;
        Ok(b)
    })?;
    a.check_queued()?;
    b.check_queued()?;
    Ok([a.trace, b.trace])
}

#[tokio::main]
async fn main() -> ExitCode {
    match run()
        .await
        .and_then(|traces| serde_json::to_string(&traces).map_err(|e| e.to_string()))
    {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("c4_probe: {e}");
            ExitCode::FAILURE
        }
    }
}
