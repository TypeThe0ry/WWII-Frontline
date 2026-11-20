//! M2-018: real loopback WebSockets, production `SimLoop`, and 30 headless clients per case.
//! The generated tactical map avoids external terrain and leaves idle clients undeployed while
//! `StepClock` advances the switch lockout. The TK case sets up bodies with `SpawnPlayer` only;
//! damage, TK prompts, punishments and the kick go through production code and the wire.
//! Not a deployment, gateway cooldown, vote, AFK, bot-fill, or forced-rebalance acceptance test.

mod common;

use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::sim_harness::{start_listeners, StepClock, TestServer};
use common::{ping_msg, wait_until, Client, Got};
use crossbeam_channel::Sender;
use frontline_protocol::msg::c0::{
    decode_c0_server, encode_c0_client, ClientMsgC0, ServerMsgC0, Welcome,
};
use frontline_protocol::msg::c4::{decode_c4, ServerEventC4};
use frontline_protocol::msg::input::{buttons, encode_input_bundle, InputBundle, InputCmd};
use frontline_protocol::msg::vote::{ForgivePunish, TeamBalanceCode, TeamSwitchRequest, TkPrompt};
use frontline_protocol::Channel;
use frontline_server::metrics::Metrics;
use frontline_server::net::conn::{InboundQueue, Sessions};
use frontline_server::shutdown::StopFlag;
use frontline_server::sim_bridge::NetLink;
use frontline_server::sim_thread::{build_loop, Command, SimLoop, SimThreadConfig};
use frontline_server::world_loader::load_map;
use frontline_sim::serverrules::{RuleConfig, RuleError};
use frontline_sim::{GameMode, MapId, PlayerId, SpawnReq, Team, Vec3};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

const CLIENTS: usize = 30;
const HALF: usize = CLIENTS / 2;
const TIMEOUT: Duration = Duration::from_secs(5);

struct Rig {
    sim: SimLoop<StepClock>,
    server: TestServer,
    commands: Sender<Command>,
}

impl Rig {
    async fn start() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let map = dir.path().join("maps/server-rules");
        fs::create_dir_all(map.join("chunks")).unwrap();
        // FCOL: zero-height samples and zero-valued material IDs.
        let mut chunk = vec![0u8; 16 + 65 * 65 * 2 + 64 * 64];
        chunk[..8].copy_from_slice(b"FCOL\x01\x00\x41\x00");
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
        let map = Arc::new(load_map(dir.path(), "server-rules").unwrap());
        let metrics = Metrics::new();
        let sessions = Sessions::with_reuse_delay(
            metrics.clone(),
            CLIENTS,
            frontline_server::net::limits::MAX_CONNECTIONS,
            Duration::ZERO,
        );
        let inbound = InboundQueue::new();
        let cfg = SimThreadConfig {
            map_data: Some(map),
            net: Some(NetLink {
                inbound: inbound.rx.clone(),
                sessions: sessions.clone(),
            }),
            ..SimThreadConfig::bare(0x5EED, MapId(0), GameMode::Sandbox)
        };
        let (sim, _, commands) =
            build_loop(StepClock::new(), &cfg, metrics.clone(), StopFlag::new());
        let server = start_listeners(metrics, sessions, &inbound, commands.clone(), false).await;
        Self {
            sim,
            server,
            commands,
        }
    }

    fn advance(&mut self, ticks: u32) {
        tokio::task::block_in_place(|| {
            for _ in 0..ticks {
                self.sim.tick();
            }
        });
    }

    fn assert_counts(&self, expected: [usize; 2]) {
        let mut world_counts = [0, 0];
        for (_, entry) in self.sim.world().squads().roster() {
            if entry.online {
                world_counts[entry.team as usize] += 1;
            }
        }
        assert_eq!(world_counts, expected, "World roster");
        assert_eq!(self.server.sessions.team_counts(), expected, "net sessions");
        assert!(expected[0].abs_diff(expected[1]) <= 2);
    }

    async fn join(&mut self, expected: [usize; 2]) -> (Client, Welcome) {
        let (client, welcome) = Client::handshake(&self.server.ws_target.url).await;
        self.advance(1);
        let entry = self.sim.world().squads().entry(welcome.player_id).unwrap();
        assert_eq!(entry.team as u8, welcome.team);
        self.assert_counts(expected);
        (client, welcome)
    }

    async fn fill(&mut self) -> Vec<(Client, Welcome)> {
        let mut clients = Vec::with_capacity(CLIENTS);
        for n in 1..=CLIENTS {
            clients.push(self.join([n.div_ceil(2), n / 2]).await);
        }
        assert_eq!(self.server.sessions.connections(), CLIENTS);
        clients
    }

    async fn wait_for_removal(&mut self, id: PlayerId) {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            self.advance(1);
            let in_sessions = self.server.sessions.handle_of(id).is_some();
            let in_roster = self.sim.world().squads().entry(id).is_some();
            if !in_sessions && !in_roster {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "player {id} was not removed (net session: {in_sessions}, roster: {in_roster})"
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }

    async fn leave(&mut self, (mut client, welcome): (Client, Welcome), expected: [usize; 2]) {
        client.ws.close(None).await.unwrap();
        self.wait_for_removal(welcome.player_id).await;
        self.assert_counts(expected);
    }

    fn spawn(&mut self, player: PlayerId, team: Team, z: f32) {
        assert!(self.sim.world().player_state(player).is_none());
        self.commands
            .try_send(Command::SpawnPlayer {
                req: SpawnReq {
                    player,
                    team,
                    pos: Vec3::new(24.0, 0.02, z),
                    yaw: 0.0,
                },
            })
            .unwrap();
        self.advance(1);
        assert!(self.sim.world().player_state(player).is_some());
    }

    async fn input(&mut self, client: &mut Client, buttons: u16) {
        let tick = self.sim.world().tick().0 + 1;
        let bundle = InputBundle {
            ack_snapshot_tick: 0,
            pred_hash: 0,
            cmds: std::iter::once(InputCmd {
                seq: tick as u16,
                tick,
                move_x: 0,
                move_z: 0,
                yaw: 0,
                pitch: 0,
                buttons,
                weapon_slot: 0,
                aux: 0,
                view_tick: 0,
            })
            .collect(),
            fire: Default::default(),
        };
        let before = self.server.metrics.net_c1_inbound_total.get();
        assert!(
            client
                .send(Channel::C1, &encode_input_bundle(&bundle).unwrap())
                .await
        );
        assert!(
            wait_until(TIMEOUT, || self.server.metrics.net_c1_inbound_total.get()
                > before)
            .await,
            "C1 input for tick {tick} did not arrive"
        );
        self.advance(1);
    }
}

async fn recv<T>(
    client: &mut Client,
    what: &str,
    mut decode: impl FnMut(Channel, &[u8]) -> Option<T>,
) -> T {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        assert!(!left.is_zero(), "timed out waiting for {what}");
        match client.recv_within(left).await {
            Got::Frame(ch, bytes) => {
                if let Some(value) = decode(ch, &bytes) {
                    return value;
                }
            }
            Got::Closed => panic!("WebSocket closed before {what}"),
            Got::Timeout => panic!("timed out waiting for {what}"),
        }
    }
}

async fn submit(client: &mut Client, message: ClientMsgC0) {
    assert!(
        client
            .send(Channel::C0, &encode_c0_client(&message).unwrap())
            .await
    );
    // With the sim stopped, Pong fences the preceding C0 on the same ordered WS stream.
    assert!(client.send(Channel::C0, &ping_msg(1337)).await);
    recv(client, "C0 Pong", |ch, bytes| {
        (ch == Channel::C0
            && matches!(decode_c0_server(bytes).unwrap(),
            ServerMsgC0::Pong(p) if p.client_time == 1337))
        .then_some(())
    })
    .await;
}

async fn balance_notice(client: &mut Client, code: TeamBalanceCode) -> [i32; 2] {
    recv(client, "C4 team balance notice", |ch, bytes| {
        if ch == Channel::C4 {
            if let ServerEventC4::TeamBalanceNotice(n) = decode_c4(bytes).unwrap() {
                assert_eq!(n.code, code);
                return Some(n.args);
            }
        }
        None
    })
    .await
}

fn switch_to(team: Team) -> ClientMsgC0 {
    ClientMsgC0::TeamSwitchRequest(TeamSwitchRequest {
        team: team as u8,
        accept_offer: false,
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thirty_ws_clients_join_leave_and_switch_within_balance_limit() {
    let mut rig = Rig::start().await;
    let mut clients = rig.fill().await;

    let (early, us) = &mut clients[0];
    assert_eq!(us.team, Team::Us as u8);
    submit(early, switch_to(Team::Ger)).await;
    rig.advance(1);
    assert_eq!(
        balance_notice(early, TeamBalanceCode::Denied).await,
        [i32::from(RuleError::SwitchTooSoon.code()), 0]
    );
    rig.assert_counts([HALF, HALF]);

    let leaver = clients.pop().unwrap();
    assert_eq!(leaver.1.team, Team::Ger as u8);
    rig.leave(leaver, [HALF, HALF - 1]).await;

    let elapsed = u32::from(clients[0].1.tick_hz)
        * u32::from(RuleConfig::default().balance.switch_min_after_join_s);
    rig.advance(elapsed);
    // GER -> US would take a one-player difference to three; the reverse move is legal.
    let (minority, ger) = &mut clients[1];
    assert_eq!(ger.team, Team::Ger as u8);
    submit(minority, switch_to(Team::Us)).await;
    rig.advance(1);
    assert_eq!(
        balance_notice(minority, TeamBalanceCode::Denied).await,
        [i32::from(RuleError::TeamUnbalanced.code()), 0]
    );
    rig.assert_counts([HALF, HALF - 1]);

    let (mover, us) = &mut clients[2];
    assert_eq!(us.team, Team::Us as u8);
    submit(mover, switch_to(Team::Ger)).await;
    let effective_tick = rig.sim.world().tick().0;
    rig.advance(1);
    assert_eq!(
        balance_notice(mover, TeamBalanceCode::Moved).await,
        [Team::Ger as i32, effective_tick as i32]
    );
    assert_eq!(
        rig.sim.world().squads().entry(us.player_id).unwrap().team,
        Team::Ger
    );
    rig.assert_counts([HALF - 1, HALF]);

    rig.leave(clients.swap_remove(2), [HALF - 1, HALF - 1])
        .await;
    clients.push(rig.join([HALF, HALF - 1]).await);
    clients.push(rig.join([HALF, HALF]).await);
    assert_eq!(rig.server.sessions.connections(), CLIENTS);
    drop(clients);
    rig.server.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thirty_ws_clients_fifth_punished_team_kill_closes_only_the_offender() {
    let mut rig = Rig::start().await;
    let mut clients = rig.fill().await;
    let (mut offender, shooter) = clients.remove(0);
    assert_eq!(shooter.team, Team::Us as u8);
    rig.spawn(shooter.player_id, Team::Us, 20.0);
    let tk = RuleConfig::default().tk;
    let mut event_ids = std::collections::BTreeSet::new();

    // Forgive the first incident, then punish the configured number (five by default).
    // Shoot the farthest position first so old bodies cannot block the next victim.
    for (i, (victim, welcome)) in clients
        .iter_mut()
        .filter(|(_, w)| w.team == shooter.team)
        .take(usize::from(tk.kick_match_strikes) + 1)
        .enumerate()
    {
        rig.spawn(welcome.player_id, Team::Us, 29.0 - i as f32);
        rig.input(&mut offender, buttons::AIM).await;
        rig.advance(30);
        for _ in 0..12 {
            if rig.sim.world().weapon_state(shooter.player_id).unwrap().mag == 0 {
                rig.input(&mut offender, buttons::RELOAD).await;
                rig.advance(150);
                assert!(rig.sim.world().weapon_state(shooter.player_id).unwrap().mag > 0);
                rig.input(&mut offender, buttons::AIM).await;
                rig.advance(30);
            }
            rig.input(&mut offender, buttons::AIM | buttons::FIRE).await;
            rig.advance(15);
            if !rig
                .sim
                .world()
                .player_life(welcome.player_id)
                .unwrap()
                .is_alive()
            {
                break;
            }
        }
        if let Some(frontline_sim::LifeState::Downed { .. }) =
            rig.sim.world().player_life(welcome.player_id)
        {
            // A down alone is not punishable. Let this attributed down bleed out.
            rig.advance(frontline_sim::medical::bleed_out_ticks(0, shooter.tick_hz));
        }
        assert!(
            rig.sim
                .world()
                .player_life(welcome.player_id)
                .unwrap()
                .is_dead(),
            "fixture fire did not kill teammate {} (HP {:?}, events {:?})",
            welcome.player_id,
            rig.sim.world().player_hp(welcome.player_id),
            rig.sim.tally()
        );
        let prompt = recv(victim, "C4 TkPrompt", |ch, bytes| {
            if ch == Channel::C4 {
                if let ServerEventC4::TkPrompt(p) = decode_c4(bytes).unwrap() {
                    return Some(p);
                }
            }
            None
        })
        .await;
        let TkPrompt {
            killer,
            expires_s,
            victim_event,
        } = prompt;
        assert_eq!(killer, shooter.player_id);
        assert_eq!(u16::from(expires_s), tk.prompt_s);
        assert!(event_ids.insert(victim_event), "reused TK event ID");

        submit(
            victim,
            ClientMsgC0::ForgivePunish(ForgivePunish {
                victim_event,
                punish: i != 0,
            }),
        )
        .await;
        rig.advance(1);
        if i < usize::from(tk.kick_match_strikes) {
            assert_eq!(rig.server.metrics.session_kicked_total.get(), 0);
            assert!(
                rig.server.sessions.handle_of(shooter.player_id).is_some(),
                "offender was removed after only {i} punishments"
            );
            rig.assert_counts([HALF, HALF]);
        }
    }

    assert_eq!(event_ids.len(), usize::from(tk.kick_match_strikes) + 1);
    assert_eq!(rig.server.metrics.session_kicked_total.get(), 1);
    recv(&mut offender, "C0 Disconnect", |ch, bytes| {
        (ch == Channel::C0
            && matches!(decode_c0_server(bytes).unwrap(), ServerMsgC0::Disconnect(_)))
        .then_some(())
    })
    .await;
    tokio::time::timeout(TIMEOUT, async {
        loop {
            match offender.ws.next().await {
                Some(Ok(Message::Close(_))) => break,
                Some(Ok(_)) => (),
                other => panic!("TK kick did not send a WebSocket Close: {other:?}"),
            }
        }
        // Tungstenite queues the close reply when read; flush it so the server can clean up.
        offender.ws.flush().await.unwrap();
    })
    .await
    .expect("TK WebSocket close handshake timed out");
    rig.wait_for_removal(shooter.player_id).await;
    rig.assert_counts([HALF - 1, HALF]);
    assert!(wait_until(TIMEOUT, || rig.server.sessions.connections() == CLIENTS - 1).await);
    for (_, welcome) in &clients {
        assert!(rig.server.sessions.handle_of(welcome.player_id).is_some());
    }
    drop(clients);
    rig.server.stop().await;
}
