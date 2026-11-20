//! Match XP settlement and cumulative level curves (GAMEDESIGN 5.2, 18.2).

use serde::{Deserialize, Serialize};

use super::ScoreRules;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XpRules {
    pub win_bonus_pct: u8,
    pub loss_bonus_pct: u8,
    pub draw_bonus_pct: u8,
    pub outcome_min_s: u32,
    pub participation_min_pct: u8,
    pub participation_bonus_points: u32,
    pub account_xp_coefficient: u16,
    pub account_max_level: u8,
    pub role_xp_coefficient: u16,
    pub role_max_level: u8,
}

/// The match result from the player's perspective, if the match has ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchOutcome {
    Win,
    Loss,
    Draw,
    Unfinished,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchXp {
    pub base_score: i64,
    pub outcome_bonus_xp: u32,
    pub participation_bonus_xp: u32,
    pub total_xp: u32,
}

/// Calculate XP from the signed total score and authoritative match ticks.
///
/// Outcome bonuses use only positive score and round down to whole XP. A
/// negative score still offsets the flat participation award before the final
/// XP is clamped. Online ticks cannot exceed elapsed match ticks. A zero-length
/// match earns neither bonus; an unfinished match or a zero tick rate disables
/// only the outcome bonus.
pub fn match_xp(
    rules: &XpRules,
    total_score: i64,
    outcome: MatchOutcome,
    online_ticks: u32,
    match_ticks: u32,
    tick_hz: u8,
) -> MatchXp {
    rules.match_xp(total_score, online_ticks, match_ticks, tick_hz, outcome)
}

impl XpRules {
    /// Calculate [`match_xp`] using these rules.
    pub fn match_xp(
        &self,
        total_score: i64,
        online_ticks: u32,
        match_ticks: u32,
        tick_hz: u8,
        outcome: MatchOutcome,
    ) -> MatchXp {
        let online = u64::from(online_ticks.min(match_ticks));
        let mut xp = MatchXp {
            base_score: total_score,
            ..MatchXp::default()
        };

        if match_ticks != 0 {
            if tick_hz != 0 && online >= u64::from(self.outcome_min_s) * u64::from(tick_hz) {
                let percent = match outcome {
                    MatchOutcome::Win => self.win_bonus_pct,
                    MatchOutcome::Loss => self.loss_bonus_pct,
                    MatchOutcome::Draw => self.draw_bonus_pct,
                    MatchOutcome::Unfinished => 0,
                };
                let bonus = i128::from(total_score.max(0)) * i128::from(percent) / 100;
                xp.outcome_bonus_xp = bonus.min(i128::from(u32::MAX)) as u32;
            }
            if online * 100 >= u64::from(match_ticks) * u64::from(self.participation_min_pct) {
                xp.participation_bonus_xp = self.participation_bonus_points;
            }
        }

        let bonuses = i64::from(xp.outcome_bonus_xp) + i64::from(xp.participation_bonus_xp);
        xp.total_xp = total_score
            .saturating_add(bonuses)
            .clamp(0, i64::from(u32::MAX)) as u32;
        xp
    }

    /// Cumulative account XP, rounded down and clamped to the configured level range.
    pub fn account_xp_threshold(&self, level: u8) -> u32 {
        let n = u64::from(level.clamp(1, self.account_max_level.max(1)) - 1);
        let coefficient = u64::from(self.account_xp_coefficient);
        // floor(c * n^(3/2)) = isqrt(c^2 * n^3); the configured ranges fit in u64.
        (coefficient * coefficient * n * n * n).isqrt() as u32
    }

    /// Cumulative role XP, clamped to the configured level range.
    pub fn role_xp_threshold(&self, level: u8) -> u32 {
        let n = u32::from(level.clamp(1, self.role_max_level.max(1)));
        u32::from(self.role_xp_coefficient) * n * (n - 1)
    }

    /// Highest account level whose cumulative XP threshold has been reached.
    pub fn account_level(&self, xp: u32) -> u8 {
        level_at_xp(xp, self.account_max_level, |n| self.account_xp_threshold(n))
    }

    /// Highest role level whose cumulative XP threshold has been reached.
    pub fn role_level(&self, xp: u32) -> u8 {
        level_at_xp(xp, self.role_max_level, |n| self.role_xp_threshold(n))
    }
}

/// Cumulative account XP required for `level` under the embedded scoring rules.
pub fn account_xp_threshold(level: u8) -> u32 {
    ScoreRules::embedded().xp.account_xp_threshold(level)
}

/// Cumulative role XP required for `level` under the embedded scoring rules.
pub fn role_xp_threshold(level: u8) -> u32 {
    ScoreRules::embedded().xp.role_xp_threshold(level)
}

/// Account level under the embedded scoring rules.
pub fn account_level(xp: u32) -> u8 {
    ScoreRules::embedded().xp.account_level(xp)
}

/// Role level under the embedded scoring rules.
pub fn role_level(xp: u32) -> u8 {
    ScoreRules::embedded().xp.role_level(xp)
}

fn level_at_xp(xp: u32, max_level: u8, threshold: impl Fn(u8) -> u32) -> u8 {
    let (mut lo, mut hi) = (1, max_level.max(1));
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if threshold(mid) <= xp {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES: XpRules = XpRules {
        win_bonus_pct: 10,
        loss_bonus_pct: 5,
        draw_bonus_pct: 7,
        outcome_min_s: 300,
        participation_min_pct: 70,
        participation_bonus_points: 100,
        account_xp_coefficient: 400,
        account_max_level: 100,
        role_xp_coefficient: 125,
        role_max_level: 10,
    };

    #[test]
    fn rules_use_explicit_integer_json_keys() {
        let json = serde_json::json!({
            "win_bonus_pct": 10,
            "loss_bonus_pct": 5,
            "draw_bonus_pct": 7,
            "outcome_min_s": 300,
            "participation_min_pct": 70,
            "participation_bonus_points": 100,
            "account_xp_coefficient": 400,
            "account_max_level": 100,
            "role_xp_coefficient": 125,
            "role_max_level": 10,
        });
        assert_eq!(
            serde_json::from_value::<XpRules>(json.clone()).unwrap(),
            RULES
        );
        assert_eq!(serde_json::to_value(RULES).unwrap(), json);
    }

    #[test]
    fn account_thresholds_and_inverse() {
        assert_eq!(account_xp_threshold(10), 10_800);
        assert_eq!(account_xp_threshold(50), 137_200);
        assert_eq!(account_xp_threshold(100), 394_015);
        check_level_boundaries(RULES.account_max_level, account_xp_threshold, account_level);
    }

    #[test]
    fn role_thresholds_and_inverse() {
        assert_eq!(role_xp_threshold(2), 250);
        assert_eq!(role_xp_threshold(5), 2_500);
        assert_eq!(role_xp_threshold(10), 11_250);
        check_level_boundaries(RULES.role_max_level, role_xp_threshold, role_level);
    }

    #[test]
    fn configured_coefficients_and_level_caps_change_both_curves() {
        let rules = XpRules {
            account_xp_coefficient: 11,
            account_max_level: 5,
            role_xp_coefficient: 30,
            role_max_level: 3,
            ..RULES
        };
        assert_eq!(rules.account_xp_threshold(5), 88);
        assert_eq!(rules.role_xp_threshold(3), 180);
        check_level_boundaries(
            rules.account_max_level,
            |n| rules.account_xp_threshold(n),
            |xp| rules.account_level(xp),
        );
        check_level_boundaries(
            rules.role_max_level,
            |n| rules.role_xp_threshold(n),
            |xp| rules.role_level(xp),
        );
    }

    fn check_level_boundaries(
        max_level: u8,
        threshold: impl Fn(u8) -> u32,
        level: impl Fn(u32) -> u8,
    ) {
        assert_eq!(threshold(0), 0);
        assert_eq!(threshold(1), 0);
        assert_eq!(level(0), 1);
        for n in 2..=max_level {
            let xp = threshold(n);
            assert!(xp > threshold(n - 1));
            assert_eq!(level(xp - 1), n - 1);
            assert_eq!(level(xp), n);
            assert_eq!(level(xp + 1), n);
        }
        assert_eq!(threshold(u8::MAX), threshold(max_level));
        assert_eq!(level(u32::MAX), max_level);
    }

    #[test]
    fn match_xp_all_outcomes_and_rounding() {
        for (outcome, bonus) in [
            (MatchOutcome::Win, 100),
            (MatchOutcome::Loss, 50),
            (MatchOutcome::Draw, 70),
            (MatchOutcome::Unfinished, 0),
        ] {
            assert_eq!(
                match_xp(&RULES, 1_000, outcome, 9_000, 9_000, 30),
                MatchXp {
                    base_score: 1_000,
                    outcome_bonus_xp: bonus,
                    participation_bonus_xp: 100,
                    total_xp: 1_100 + bonus,
                }
            );
        }
        assert_eq!(
            match_xp(&RULES, 19, MatchOutcome::Win, 9_000, 9_000, 30).total_xp,
            120
        );
    }

    #[test]
    fn match_xp_uses_independent_inclusive_tick_boundaries() {
        let win = |online, elapsed| {
            match_xp(&RULES, 1_000, MatchOutcome::Win, online, elapsed, 30).total_xp
        };
        assert_eq!(win(8_999, 18_000), 1_000); // One tick short of five minutes.
        assert_eq!(win(9_000, 18_000), 1_100); // Outcome only.
        assert_eq!(win(12_599, 18_000), 1_100); // One tick short of 70%.
        assert_eq!(win(12_600, 18_000), 1_200); // Both bonuses.
        assert_eq!(win(6_300, 9_000), 1_100); // Participation only.
        assert_eq!(win(9_000, 8_999), 1_100); // Overreported online time is capped.
        assert_eq!(win(9_000, 0), 1_000); // No bonuses for an empty match.
        assert_eq!(
            match_xp(&RULES, 1_000, MatchOutcome::Win, 9_000, 9_000, 0).total_xp,
            1_100
        );
    }

    #[test]
    fn match_xp_clamps_signed_scores_and_saturates() {
        let win = |score| match_xp(&RULES, score, MatchOutcome::Win, 9_000, 9_000, 30);
        assert_eq!(win(i64::MIN).total_xp, 0);
        assert_eq!(win(-101).total_xp, 0);
        assert_eq!(
            win(-30),
            MatchXp {
                base_score: -30,
                outcome_bonus_xp: 0,
                participation_bonus_xp: 100,
                total_xp: 70,
            }
        );
        assert_eq!(win(0).total_xp, 100);
        assert_eq!(win(i64::MAX).outcome_bonus_xp, u32::MAX);
        assert_eq!(win(i64::MAX).total_xp, u32::MAX);
        assert_eq!(
            match_xp(&RULES, -30, MatchOutcome::Loss, 9_000, 18_000, 30).total_xp,
            0
        );

        let rules = XpRules {
            participation_bonus_points: u32::MAX,
            ..RULES
        };
        assert_eq!(
            match_xp(&rules, 1, MatchOutcome::Win, u32::MAX, u32::MAX, 30).total_xp,
            u32::MAX
        );
    }
}
