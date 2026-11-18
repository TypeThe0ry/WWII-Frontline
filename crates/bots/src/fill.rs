//! Bot fill manager (M1-039, GAMEDESIGN 2.4 / 20.2): keeps a team at its target size, makes room
//! for joining humans in the prescribed order, and waits 10 s before back-filling a human who
//! dropped, so a reconnect flap causes no add/remove churn. Pure: the host executes the actions
//! through the normal deploy/squad path.

use std::collections::BTreeMap;

use frontline_sim::PlayerId;

use crate::difficulty::TICK_HZ;

pub const BOT_NAME_PREFIX: &str = "[BOT]";
/// A disconnected human keeps their slot this long.
pub const REJOIN_GRACE_S: f32 = 10.0;

/// `[BOT] <name>` — every bot nickname carries the prefix.
pub fn bot_nickname(name: &str) -> String {
    format!("{BOT_NAME_PREFIX} {name}")
}

/// Removal priority (GD 2.4): on the deploy screen first, then alive on foot, vehicles last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BotState {
    Deploying,
    Alive,
    InVehicle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BotInfo {
    pub id: PlayerId,
    pub state: BotState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveReason {
    HumanJoined,
    OverTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillAction {
    SpawnBot,
    RemoveBot { id: PlayerId, reason: RemoveReason },
}

#[derive(Debug, Default)]
pub struct FillManager {
    /// Humans currently disconnected, with the tick they left.
    left_at: BTreeMap<PlayerId, u32>,
}

impl FillManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// One team's reconcile. `humans` is `(id, connected)` for every human on the team.
    /// Disconnected humans still count as present until the grace period ends.
    pub fn reconcile(
        &mut self,
        tick: u32,
        humans: &[(PlayerId, bool)],
        bots: &[BotInfo],
        target: u8,
    ) -> Vec<FillAction> {
        let grace = (REJOIN_GRACE_S * TICK_HZ) as u32;
        self.left_at
            .retain(|id, _| humans.iter().any(|h| h.0 == *id));
        let mut present = 0usize;
        for &(id, connected) in humans {
            if connected {
                self.left_at.remove(&id);
                present += 1;
            } else {
                let since = *self.left_at.entry(id).or_insert(tick);
                if tick.saturating_sub(since) < grace {
                    present += 1;
                }
            }
        }
        let have = present + bots.len();
        let target = usize::from(target);
        let mut out = Vec::new();
        if have < target {
            out.extend(std::iter::repeat_n(FillAction::SpawnBot, target - have));
        } else if have > target {
            let mut order: Vec<BotInfo> = bots.to_vec();
            order.sort_by_key(|b| (b.state, b.id));
            for b in order.into_iter().take(have - target) {
                out.push(FillAction::RemoveBot {
                    id: b.id,
                    reason: if present > 0 {
                        RemoveReason::HumanJoined
                    } else {
                        RemoveReason::OverTarget
                    },
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bots(n: u8, st: BotState) -> Vec<BotInfo> {
        (0..n)
            .map(|i| BotInfo {
                id: 100 + i,
                state: st,
            })
            .collect()
    }

    /// Acceptance 1 (logic level): 2 humans of 20 -> each team tops up to 10.
    #[test]
    fn fills_to_target() {
        let mut f = FillManager::new();
        let a = f.reconcile(0, &[(1, true)], &[], 10);
        assert_eq!(a.len(), 9);
        assert!(a.iter().all(|x| *x == FillAction::SpawnBot));
        assert!(f
            .reconcile(1, &[(1, true)], &bots(9, BotState::Alive), 10)
            .is_empty());
    }

    /// Acceptance 2: a human joins a full team -> exactly one bot goes, deploy-screen first.
    #[test]
    fn human_replaces_bot_in_priority_order() {
        let mut f = FillManager::new();
        let mut b = vec![
            BotInfo {
                id: 5,
                state: BotState::InVehicle,
            },
            BotInfo {
                id: 3,
                state: BotState::Alive,
            },
            BotInfo {
                id: 9,
                state: BotState::Deploying,
            },
        ];
        b.extend((20..27).map(|i| BotInfo {
            id: i,
            state: BotState::Alive,
        }));
        // 10 bots, target 10, then a human joins.
        let a = f.reconcile(0, &[(1, true)], &b, 10);
        assert_eq!(
            a,
            [FillAction::RemoveBot {
                id: 9,
                reason: RemoveReason::HumanJoined
            }]
        );
        // Without a deploying bot, an alive one goes before the vehicle bot.
        b.retain(|x| x.id != 9);
        b.push(BotInfo {
            id: 30,
            state: BotState::Alive,
        });
        let a = f.reconcile(1, &[(1, true)], &b, 10);
        assert!(matches!(a[0], FillAction::RemoveBot { id, .. } if id != 5));
    }

    /// Acceptance 6: a flap inside 10 s changes nothing; after 10 s the slot is back-filled.
    #[test]
    fn flap_guard() {
        let mut f = FillManager::new();
        let b = bots(9, BotState::Alive);
        assert!(f.reconcile(0, &[(1, true)], &b, 10).is_empty());
        assert!(f.reconcile(10, &[(1, false)], &b, 10).is_empty());
        assert!(f.reconcile(10 + 299, &[(1, false)], &b, 10).is_empty());
        // Reconnects at 5 s: still nothing, and the timer resets.
        assert!(f.reconcile(10 + 150, &[(1, true)], &b, 10).is_empty());
        assert!(f.reconcile(400, &[(1, false)], &b, 10).is_empty());
        assert_eq!(
            f.reconcile(400 + 300, &[(1, false)], &b, 10),
            [FillAction::SpawnBot]
        );
    }

    #[test]
    fn nicknames_are_prefixed() {
        assert!(bot_nickname("Hans").starts_with("[BOT]"));
    }
}
