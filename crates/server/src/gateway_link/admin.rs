//! Match-scoped admin delivery. Actions execute on the authoritative sim thread.

use std::collections::VecDeque;
use std::time::Duration;

use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot, watch};
use tokio::time::{self, MissedTickBehavior};

use crate::sim_thread::{AdminReply, Command};

use super::client::{bounded_json, GatewayClient, LinkError};
use super::parse_lower_hex;

pub const POLL_INTERVAL: Duration = Duration::from_millis(200);
const MAX_RESPONSE_BYTES: usize = 128 * 1024;
const MAX_ACTIONS_PER_POLL: usize = 32;
const RECENT_ACKS: usize = 256;

#[derive(Clone, Debug, Deserialize)]
pub struct Action {
    pub id: i64,
    pub action: String,
    pub player_id: Option<u8>,
    pub jti: Option<String>,
    pub text: Option<String>,
    // `force_team` and `map_change` are recognized, but no safe live transition exists yet.
    #[serde(rename = "team")]
    pub _team: Option<u8>,
    #[serde(rename = "map")]
    pub _map: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionResult {
    Applied,
    TargetMissing,
    Unsupported,
}

pub async fn run(
    client: GatewayClient,
    match_id: String,
    commands: Sender<Command>,
    mut stop: watch::Receiver<bool>,
) {
    if parse_lower_hex(&match_id).is_none() {
        return;
    }
    tokio::select! {
        _ = stop.wait_for(|v| *v) => {},
        _ = poll(client, match_id, commands) => {},
    }
}

async fn poll(client: GatewayClient, match_id: String, commands: Sender<Command>) {
    let path = format!("/internal/matches/{match_id}/actions");
    let mut interval = time::interval(POLL_INTERVAL);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut recent = VecDeque::with_capacity(RECENT_ACKS);
    let mut pending = None;
    let mut poll_failed = false;
    'poll: loop {
        interval.tick().await;
        if let Some((id, result)) = pending {
            if ack(&client, &path, id, result).await.is_err() {
                continue;
            }
            pending = None;
        }
        let actions = match fetch(&client, &path).await {
            Ok(actions) => {
                poll_failed = false;
                actions
            }
            Err(error) => {
                if !poll_failed {
                    tracing::warn!(event = "admin_poll_failed", %error);
                    poll_failed = true;
                }
                continue;
            }
        };
        for action in actions.into_iter().take(MAX_ACTIONS_PER_POLL) {
            let id = action.id;
            let result = if let Some((_, result)) = recent.iter().find(|(seen, _)| *seen == id) {
                *result
            } else {
                let (reply, rx) = oneshot::channel();
                if commands
                    .try_send(Command::Admin {
                        action,
                        reply: AdminReply::new(reply),
                    })
                    .is_err()
                {
                    // No ACK until the authoritative thread has handled the action.
                    continue 'poll;
                }
                let Ok(result) = rx.await else { return };
                if recent.len() == RECENT_ACKS {
                    recent.pop_front();
                }
                recent.push_back((id, result));
                result
            };
            if let Err(error) = ack(&client, &path, id, result).await {
                tracing::warn!(event = "admin_ack_failed", action_id = id, %error);
                pending = Some((id, result));
                continue 'poll;
            }
        }
    }
}

async fn fetch(client: &GatewayClient, path: &str) -> Result<Vec<Action>, LinkError> {
    bounded_json(client.get(path).await?, MAX_RESPONSE_BYTES).await
}

async fn ack(
    client: &GatewayClient,
    path: &str,
    id: i64,
    result: ActionResult,
) -> Result<(), LinkError> {
    #[derive(Serialize)]
    struct Ack {
        result: ActionResult,
    }
    match client
        .post(&format!("{path}/{id}/ack"), &Ack { result })
        .await
    {
        Ok(_) | Err(LinkError::Status(404 | 410)) => Ok(()),
        Err(error) => Err(error),
    }
}
