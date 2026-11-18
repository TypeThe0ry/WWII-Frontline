//! Join / switch balance checks (GAMEDESIGN 2.4). Pure functions over head counts.
//!
//! PI-064: "H_t - H_other <= 2" is read as the difference *after* the join, the conservative
//! reading: with `max_diff = 2` a team that is already 2 ahead cannot take another player.

use crate::ids::Tick;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamError {
    /// Not on the roster.
    Unknown,
    AlreadyOnRoster,
    /// The team already has `team_max_players`.
    Full,
    /// Joining would make the team more than `max_diff_players` larger than the other.
    Unbalanced,
    /// Less than `switch_min_after_join_s` since the player entered the match.
    TooSoon,
    /// Less than `switch_interval_s` since the previous switch.
    SwitchCooldown,
}

/// May one more player join a team holding `own` players while the other holds `other`?
pub fn can_join(own: u32, other: u32, cap: u32, max_diff: u32) -> Result<(), TeamError> {
    if own >= cap {
        return Err(TeamError::Full);
    }
    if own + 1 > other + max_diff {
        return Err(TeamError::Unbalanced);
    }
    Ok(())
}

/// Ticks elapsed from `since` to `now` (never negative).
pub fn elapsed(now: Tick, since: Tick) -> u32 {
    now.diff(since).max(0) as u32
}

/// Switch check: the mover leaves the `own` team for the `other` team.
#[allow(clippy::too_many_arguments)]
pub fn can_switch(
    own: u32,
    other: u32,
    cap: u32,
    max_diff: u32,
    now: Tick,
    joined: Tick,
    last_switch: Option<Tick>,
    min_after_join: u32,
    interval: u32,
) -> Result<(), TeamError> {
    if elapsed(now, joined) < min_after_join {
        return Err(TeamError::TooSoon);
    }
    if let Some(t) = last_switch {
        if elapsed(now, t) < interval {
            return Err(TeamError::SwitchCooldown);
        }
    }
    can_join(other, own.saturating_sub(1), cap, max_diff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_balance_and_cap() {
        assert_eq!(can_join(3, 3, 50, 2), Ok(())); // diff after = 1
        assert_eq!(can_join(4, 3, 50, 2), Ok(())); // diff after = 2
        assert_eq!(can_join(5, 3, 50, 2), Err(TeamError::Unbalanced));
        assert_eq!(can_join(6, 3, 50, 2), Err(TeamError::Unbalanced));
        assert_eq!(can_join(50, 60, 50, 2), Err(TeamError::Full));
    }
}
