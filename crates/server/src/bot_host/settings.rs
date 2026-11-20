use std::collections::BTreeMap;

use frontline_bots::Difficulty;
use frontline_sim::{PlayerId, Team};

#[derive(Clone)]
pub struct BotOptions {
    pub(crate) teams: [Difficulty; 2],
    pub(crate) players: BTreeMap<PlayerId, Difficulty>,
    pub(crate) path_ms: f32,
}

impl Default for BotOptions {
    fn default() -> Self {
        Self {
            teams: [Difficulty::configured_default(); 2],
            players: BTreeMap::new(),
            path_ms: 1.0,
        }
    }
}

impl BotOptions {
    pub fn parse(
        us: Option<&str>,
        ger: Option<&str>,
        players: &[String],
        path_ms: f32,
    ) -> Result<Self, String> {
        let mut options = Self::default();
        for (slot, name) in options.teams.iter_mut().zip([us, ger]) {
            if let Some(name) = name {
                *slot = tier(name)?;
            }
        }
        for entry in players {
            let (id, name) = entry
                .split_once('=')
                .ok_or_else(|| format!("expected bot difficulty ID=TIER, got {entry}"))?;
            let id = id
                .parse::<PlayerId>()
                .ok()
                .filter(|&id| id <= 127)
                .ok_or_else(|| format!("bot difficulty ID must be 0..127, got {id}"))?;
            if options.players.insert(id, tier(name)?).is_some() {
                return Err(format!("duplicate difficulty for bot {id}"));
            }
        }
        if !(0.05..=1.0).contains(&path_ms) {
            return Err("bot path budget must be 0.05..=1.0 ms".into());
        }
        options.path_ms = path_ms;
        Ok(options)
    }

    pub(super) fn difficulty(&self, id: PlayerId, team: Team) -> Difficulty {
        self.players
            .get(&id)
            .copied()
            .unwrap_or(self.teams[team as usize])
    }
}

fn tier(name: &str) -> Result<Difficulty, String> {
    Difficulty::configured(name).ok_or_else(|| {
        format!("unknown bot difficulty {name}; expected recruit, regular or veteran")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_override_wins_over_team_and_default() {
        let opts = BotOptions::parse(Some("recruit"), None, &["127=veteran".into()], 1.0).unwrap();
        assert_eq!(opts.difficulty(127, Team::Us), tier("veteran").unwrap());
        assert_eq!(opts.difficulty(125, Team::Us), tier("recruit").unwrap());
        assert_eq!(opts.difficulty(126, Team::Ger), tier("regular").unwrap());
        assert!(BotOptions::parse(None, None, &["128=recruit".into()], 1.0).is_err());
        assert!(BotOptions::parse(None, None, &["127=easy".into()], 1.0).is_err());
        assert!(BotOptions::parse(None, None, &[], f32::NAN).is_err());
    }
}
