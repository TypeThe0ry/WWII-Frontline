//! Deterministic text policy, routing, and match-side sliding windows (GD §16.2).

use std::collections::VecDeque;
use unicode_normalization::UnicodeNormalization;

use crate::squad::SquadManager;
use crate::{PlayerId, Tick};

pub use frontline_protocol::msg::c0::ChatChannel;
use frontline_protocol::msg::c0::MAX_CHAT_LEN;

const LINES: usize = 5;
const WINDOW_S: u32 = 8;
const DUP_WINDOW_S: u32 = 10;
const DUP_ALLOWED: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatReject {
    Empty,
    TooLong,
    Control,
    Bidi,
    Duplicate,
    Rate,
    Channel,
    NoSquad,
    NoRoster,
}

impl ChatReject {
    pub const LABELS: [&'static str; 9] = [
        "empty",
        "too_long",
        "control",
        "bidi",
        "duplicate",
        "rate",
        "channel",
        "no_squad",
        "not_on_roster",
    ];

    pub fn label(self) -> &'static str {
        Self::LABELS[self as usize]
    }
}

fn check_chars(text: &str) -> Result<(), ChatReject> {
    for (i, c) in text.chars().enumerate() {
        if i >= MAX_CHAT_LEN {
            return Err(ChatReject::TooLong);
        }
        if c.is_control() {
            return Err(ChatReject::Control);
        }
        if matches!(c, '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
        {
            return Err(ChatReject::Bidi);
        }
    }
    Ok(())
}

pub fn sanitize_chat(text: &str) -> Result<String, ChatReject> {
    check_chars(text)?;
    let normalized: String = text.nfkc().collect();
    check_chars(&normalized)?;
    let line = normalized.trim();
    if line.is_empty() {
        return Err(ChatReject::Empty);
    }
    Ok(line.to_owned())
}

fn command_role(role: Option<&str>) -> bool {
    matches!(
        role,
        Some("commander" | "officer" | "spotter" | "tank_commander")
    )
}

/// Text is available through all life states. Voice Dead/Downed rules are separate.
pub fn recipients(
    roster: &SquadManager,
    sender: PlayerId,
    channel: ChatChannel,
) -> Result<impl Iterator<Item = PlayerId> + '_, ChatReject> {
    let from = roster
        .entry(sender)
        .filter(|entry| entry.online)
        .ok_or(ChatReject::NoRoster)?;
    if channel == ChatChannel::Command && !command_role(from.role) {
        return Err(ChatReject::Channel);
    }
    if channel == ChatChannel::Squad && from.squad.is_none() {
        return Err(ChatReject::NoSquad);
    }
    Ok(roster.roster().filter_map(move |(id, to)| {
        let permitted = match channel {
            ChatChannel::All => true,
            ChatChannel::Team => to.team == from.team,
            ChatChannel::Squad => to.team == from.team && to.squad == from.squad,
            ChatChannel::Command => to.team == from.team && command_role(to.role),
        };
        (to.online && permitted).then_some(id)
    }))
}

/// Applied after the wall-clock C0 token bucket. An exact sliding match-tick window
/// guards against the burst-plus-refill admitted by a token bucket on its own.
#[derive(Default)]
pub struct ChatWindow {
    accepted: VecDeque<(Tick, String)>,
}

impl ChatWindow {
    pub fn admit(&mut self, now: Tick, hz: u32, text: &str) -> Result<(), ChatReject> {
        let age = |tick: Tick| now.0.wrapping_sub(tick.0);
        while self
            .accepted
            .front()
            .is_some_and(|(tick, _)| age(*tick) >= hz * DUP_WINDOW_S)
        {
            self.accepted.pop_front();
        }
        if self
            .accepted
            .iter()
            .rev()
            .take_while(|(tick, _)| age(*tick) < hz * WINDOW_S)
            .count()
            >= LINES
        {
            return Err(ChatReject::Rate);
        }
        if self
            .accepted
            .iter()
            .filter(|(_, line)| line == text)
            .count()
            >= DUP_ALLOWED
        {
            return Err(ChatReject::Duplicate);
        }
        self.accepted.push_back((now, text.to_owned()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_nfkc_and_input_rejection() {
        assert_eq!(sanitize_chat("  ＡⅧﬁe\u{301}  ").unwrap(), "AVIIIfié");
        assert!(sanitize_chat(&"界".repeat(200)).is_ok());
        assert_eq!(sanitize_chat(&"x".repeat(201)), Err(ChatReject::TooLong));
        assert_eq!(sanitize_chat(&"Ⅷ".repeat(51)), Err(ChatReject::TooLong));
        assert_eq!(sanitize_chat("x\u{202e}y"), Err(ChatReject::Bidi));
        assert_eq!(sanitize_chat("x\nx"), Err(ChatReject::Control));
        assert_eq!(sanitize_chat("\u{3000}"), Err(ChatReject::Empty));
    }

    #[test]
    fn exact_window_duplicate_and_wrap_boundaries() {
        let start = u32::MAX - 100;
        let time = |offset| Tick(start.wrapping_add(offset));
        let mut gate = ChatWindow::default();
        for i in 0..5 {
            assert_eq!(gate.admit(time(i), 30, &i.to_string()), Ok(()));
        }
        assert_eq!(gate.admit(time(239), 30, "extra"), Err(ChatReject::Rate));
        assert_eq!(gate.admit(time(240), 30, "extra"), Ok(()));
        let mut gate = ChatWindow::default();
        assert_eq!(gate.admit(time(0), 30, "same"), Ok(()));
        assert_eq!(gate.admit(time(1), 30, "same"), Ok(()));
        assert_eq!(
            gate.admit(time(299), 30, "same"),
            Err(ChatReject::Duplicate)
        );
        assert_eq!(gate.admit(time(300), 30, "same"), Ok(()));
    }

    #[test]
    fn command_roles() {
        for role in ["commander", "officer", "spotter", "tank_commander"] {
            assert!(command_role(Some(role)));
        }
        for role in [
            None,
            Some("crewman"),
            Some("sniper"),
            Some("engineer"),
            Some("medic"),
        ] {
            assert!(!command_role(role));
        }
    }
}
