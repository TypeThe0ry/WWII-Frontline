//! Anti-farm record (GAMEDESIGN 8.4): who suppressed a player in the last 5 s. A teammate's kill
//! of the suppressed player then credits each suppressor once (`SuppressionAssist`).

use crate::components::Team;
use crate::ids::EntityId;

/// A suppressor on record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Source {
    pub who: EntityId,
    pub team: Team,
    pub tick: u32,
}

/// Records (or refreshes) `who`, keeping at most 8 entries (the oldest drops).
pub fn record(list: &mut Vec<Source>, who: EntityId, team: Team, tick: u32) {
    if let Some(s) = list.iter_mut().find(|s| s.who == who) {
        s.tick = tick;
        return;
    }
    if list.len() >= 8 {
        list.remove(0);
    }
    list.push(Source { who, team, tick });
}

/// Suppressors still inside the window at `now` that are teammates of `killer_team` but not the
/// killer, ascending by id.
pub fn assisting(
    list: &[Source],
    now: u32,
    window_ticks: u32,
    killer: Option<EntityId>,
    killer_team: Option<Team>,
) -> Vec<EntityId> {
    let mut v: Vec<EntityId> = list
        .iter()
        .filter(|s| now.saturating_sub(s.tick) <= window_ticks)
        .filter(|s| Some(s.who) != killer && Some(s.team) == killer_team)
        .map(|s| s.who)
        .collect();
    v.sort();
    v
}
