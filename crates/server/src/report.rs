//! Bounded match-local evidence. Only signed-ticket players can create gateway reports.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use frontline_protocol::msg::c0::{Report, ServerMsgC0, MAX_REPORT_NOTE_LEN};
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::{PlayerId, SimEvent};
use tokio::sync::{mpsc, watch};

use crate::c0::Outbox;
use crate::gateway_link::reports::{ChatLine, ReportJob};
use crate::gateway_link::Admitted;

const HISTORY_MS: u32 = 300_000;
const EXCERPT_MS: u32 = 60_000;
const INPUT_HISTORY: usize = 30 * 300;
const EVENT_HISTORY: usize = 256;
const CHAT_EXCERPT: usize = 20;
const LIVE_CHAT_HISTORY: usize = 100;
const LIVE_CHAT_MS: u32 = 24 * 60 * 60 * 1000;
const MAX_EVIDENCE_BYTES: usize = 300_000;
const MAX_REPORTS: u8 = 10;
const SAMPLE_LEN: usize = 12;

/// Each sample is a little-endian monotonic match-relative timestamp in ms and eight data bytes.
struct Sample([u8; SAMPLE_LEN]);

impl Sample {
    fn new(at: u32, data: [u8; 8]) -> Self {
        let mut slot = [0; SAMPLE_LEN];
        slot[..4].copy_from_slice(&at.to_le_bytes());
        slot[4..].copy_from_slice(&data);
        Self(slot)
    }

    fn recent(&self, at: u32, window: u32) -> bool {
        let then = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        at.wrapping_sub(then) <= window
    }
}

struct TimedChat {
    at: u32,
    line: ChatLine,
}

struct PlayerEvidence {
    identity: Admitted,
    inputs: VecDeque<Sample>,
    events: VecDeque<Sample>,
    chat: VecDeque<Arc<TimedChat>>,
    last_report: Option<u32>,
    reports_sent: u8,
}

pub(crate) struct Recorder {
    reports: mpsc::Sender<ReportJob>,
    chat_updates: watch::Sender<Arc<Vec<ChatLine>>>,
    start: Instant,
    players: BTreeMap<PlayerId, PlayerEvidence>,
    live_chat: VecDeque<Arc<TimedChat>>,
    last_chat_update: Option<u32>,
    chat_dirty: bool,
}

impl Recorder {
    pub(crate) fn new(
        reports: mpsc::Sender<ReportJob>,
        chat_updates: watch::Sender<Arc<Vec<ChatLine>>>,
        start: Instant,
    ) -> Self {
        Self {
            reports,
            chat_updates,
            start,
            players: BTreeMap::new(),
            live_chat: VecDeque::new(),
            last_chat_update: None,
            chat_dirty: false,
        }
    }

    pub(crate) fn join(&mut self, slot: PlayerId, identity: Admitted) {
        self.players.insert(
            slot,
            PlayerEvidence {
                identity,
                inputs: VecDeque::new(),
                events: VecDeque::new(),
                chat: VecDeque::new(),
                last_report: None,
                reports_sent: 0,
            },
        );
    }

    pub(crate) fn leave(&mut self, slot: PlayerId) {
        self.players.remove(&slot);
    }

    fn ms(&self, now: Instant) -> u32 {
        now.saturating_duration_since(self.start).as_millis() as u32
    }

    pub(crate) fn record(
        &mut self,
        now: Instant,
        inputs: &[(PlayerId, InputCmd)],
        events: &[SimEvent],
    ) {
        let at = self.ms(now);
        for (slot, cmd) in inputs {
            if let Some(p) = self.players.get_mut(slot) {
                let [yaw0, yaw1] = cmd.yaw.to_le_bytes();
                let [pitch0, pitch1] = cmd.pitch.to_le_bytes();
                let [buttons0, buttons1] = cmd.buttons.to_le_bytes();
                let sample = Sample::new(
                    at,
                    [
                        cmd.move_x as u8,
                        cmd.move_z as u8,
                        yaw0,
                        yaw1,
                        pitch0,
                        pitch1,
                        buttons0,
                        buttons1,
                    ],
                );
                push_sample(&mut p.inputs, sample, at, INPUT_HISTORY);
            }
        }
        for event in events {
            // These events identify players directly in the authoritative world. Store only on
            // the involved players' rings so report evidence does not sweep up the whole match.
            let (kind, subject, other, param) = match *event {
                SimEvent::Kill {
                    victim,
                    killer,
                    weapon,
                    ..
                } => (1, victim, killer, weapon),
                SimEvent::Downed {
                    victim,
                    killer,
                    weapon,
                    ..
                } => (2, victim, killer, weapon),
                SimEvent::KillCredit {
                    victim,
                    killer,
                    team_kill,
                    ..
                } => (3, victim, killer, u8::from(team_kill)),
                SimEvent::PlayerSpawned { player, .. } | SimEvent::Spawn { player, .. } => {
                    (4, player, None, 0)
                }
                SimEvent::Revive { target_player, .. } => (5, target_player, None, 0),
                _ => continue,
            };
            for slot in [Some(subject), other.filter(|&other| other != subject)]
                .into_iter()
                .flatten()
            {
                if let Some(p) = self.players.get_mut(&slot) {
                    let sample = Sample::new(
                        at,
                        [kind, subject, other.unwrap_or(u8::MAX), param, 0, 0, 0, 0],
                    );
                    push_sample(&mut p.events, sample, at, EVENT_HISTORY);
                }
            }
        }
        while self
            .live_chat
            .front()
            .is_some_and(|line| at.wrapping_sub(line.at) >= LIVE_CHAT_MS)
        {
            self.live_chat.pop_front();
            self.chat_dirty = true;
        }
        if self.chat_dirty
            && self
                .last_chat_update
                .is_none_or(|last| at.wrapping_sub(last) >= 1_000)
        {
            self.chat_dirty = false;
            self.last_chat_update = Some(at);
            self.chat_updates.send_replace(Arc::new(
                self.live_chat
                    .iter()
                    .map(|line| line.line.clone())
                    .collect(),
            ));
        }
    }

    /// Accept only chat that passed C0 routing. Keep a per-recipient view so a report cannot
    /// expose private messages the reporter did not receive.
    pub(crate) fn chat(&mut self, now: Instant, routed: &Outbox) {
        let Some((_, ServerMsgC0::Chat(chat))) = routed.first() else {
            return;
        };
        let Some(author) = self.players.get(&chat.sender) else {
            return;
        };
        let line = Arc::new(TimedChat {
            at: self.ms(now),
            line: ChatLine {
                player_id: author.identity.gateway_player,
                text: chat.text.clone(),
            },
        });
        for (recipient, _) in routed {
            if let Some(p) = self.players.get_mut(recipient) {
                push_bounded(&mut p.chat, line.clone(), CHAT_EXCERPT);
            }
        }
        push_bounded(&mut self.live_chat, line, LIVE_CHAT_HISTORY);
        self.chat_dirty = true;
    }

    /// The caller verifies that both slots are still connected, then we match those identities
    /// against the evidence owner's ticket before reading history.
    pub(crate) fn submit(
        &mut self,
        now: Instant,
        from: PlayerId,
        reporter_jti: [u8; 16],
        target_jti: [u8; 16],
        report: Report,
    ) -> bool {
        if from == report.target
            || reporter_jti == target_jti
            || report.reason > 3
            || report.note.len() > MAX_REPORT_NOTE_LEN
            || report.note.chars().any(char::is_control)
        {
            return false;
        }
        let at = self.ms(now);
        let Some(reporter) = self
            .players
            .get(&from)
            .filter(|p| p.identity.jti == reporter_jti)
        else {
            return false;
        };
        let Some(target) = self
            .players
            .get(&report.target)
            .filter(|p| p.identity.jti == target_jti)
        else {
            return false;
        };
        if reporter.reports_sent >= MAX_REPORTS
            || reporter
                .last_report
                .is_some_and(|last| at.wrapping_sub(last) < EXCERPT_MS)
        {
            return false;
        }
        let Ok(permit) = self.reports.try_reserve() else {
            tracing::warn!(event = "report_queue_unavailable");
            return false;
        };
        let replay = excerpt(at, from, report.target, reporter, target);
        let chat_excerpt = reporter
            .chat
            .iter()
            .filter(|line| at.wrapping_sub(line.at) <= HISTORY_MS)
            .map(|line| line.line.clone())
            .collect();
        let reporter = self.players.get_mut(&from).expect("validated above");
        reporter.last_report = Some(at);
        reporter.reports_sent += 1;
        permit.send(ReportJob {
            reporter_jti,
            target_jti,
            reason: report.reason,
            note: report.note,
            replay,
            chat_excerpt,
        });
        true
    }
}

fn push_bounded<T>(queue: &mut VecDeque<T>, entry: T, capacity: usize) {
    if queue.len() == capacity {
        queue.pop_front();
    }
    queue.push_back(entry);
}

fn push_sample(queue: &mut VecDeque<Sample>, entry: Sample, at: u32, capacity: usize) {
    while queue.front().is_some_and(|s| !s.recent(at, HISTORY_MS)) {
        queue.pop_front();
    }
    // Avoid VecDeque doubling past the explicit sample bound.
    if queue.len() == queue.capacity() && queue.len() < capacity {
        queue.reserve_exact((queue.capacity().max(1)).min(capacity - queue.len()));
    }
    push_bounded(queue, entry, capacity);
}

/// FLE1: magic, reporter and target local slots, report time (u32 LE), then four groups
/// (reporter inputs, target inputs, reporter events, target events). Each group starts with
/// a u16 LE count followed by the newest matching 12-byte samples, in chronological order.
fn excerpt(
    at: u32,
    from: PlayerId,
    target_slot: PlayerId,
    reporter: &PlayerEvidence,
    target: &PlayerEvidence,
) -> Option<Vec<u8>> {
    const HEADER: usize = 10;
    const GROUPS: usize = 4;
    let mut budget = (MAX_EVIDENCE_BYTES - HEADER - GROUPS * 2) / SAMPLE_LEN;
    let mut out = Vec::new();
    out.extend_from_slice(b"FLE1");
    out.extend_from_slice(&[from, target_slot]);
    out.extend_from_slice(&at.to_le_bytes());
    let mut records = 0;
    for ring in [
        &reporter.inputs,
        &target.inputs,
        &reporter.events,
        &target.events,
    ] {
        let count = ring
            .iter()
            .rev()
            .take(budget)
            .take_while(|s| s.recent(at, EXCERPT_MS))
            .count();
        out.extend_from_slice(&(count as u16).to_le_bytes());
        for s in ring.iter().skip(ring.len() - count) {
            out.extend_from_slice(&s.0);
        }
        records += count;
        budget -= count;
    }
    (records != 0).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const REPORTER: [u8; 16] = [1; 16];
    const TARGET: [u8; 16] = [2; 16];

    fn recorder(capacity: usize) -> (Recorder, mpsc::Receiver<ReportJob>, Instant) {
        let start = Instant::now();
        let (tx, rx) = mpsc::channel(capacity);
        let (chat, _) = watch::channel(Arc::default());
        let mut recorder = Recorder::new(tx, chat, start);
        for (slot, jti) in [(0, REPORTER), (1, TARGET)] {
            recorder.join(
                slot,
                Admitted {
                    jti,
                    gateway_player: 20 + slot,
                    nick: String::new(),
                    is_admin: false,
                },
            );
        }
        (recorder, rx, start)
    }

    fn report(reason: u8) -> Report {
        Report {
            target: 1,
            reason,
            note: String::new(),
        }
    }

    #[test]
    fn ring_retains_five_minutes_and_export_keeps_only_sixty_seconds() {
        let (mut recorder, mut rx, start) = recorder(1);
        let events = [SimEvent::PlayerSpawned {
            player: 1,
            entity: 100,
        }];
        for seconds in [0, 1, 240, 241, 250, 301] {
            recorder.record(start + Duration::from_secs(seconds), &[], &events);
        }
        let target = &recorder.players[&1];
        assert_eq!(target.events.len(), 5);
        assert!(recorder.submit(
            start + Duration::from_secs(301),
            0,
            REPORTER,
            TARGET,
            report(0)
        ));
        let replay = rx.try_recv().unwrap().replay.unwrap();
        // Three empty groups, then target events with the 60-second boundary inclusive.
        assert_eq!(&replay[10..18], &[0, 0, 0, 0, 0, 0, 3, 0]);
        let times: Vec<_> = replay[18..]
            .chunks_exact(SAMPLE_LEN)
            .map(|record| u32::from_le_bytes(record[..4].try_into().unwrap()))
            .collect();
        assert_eq!(times, [241_000, 250_000, 301_000]);
        assert!(replay.len() <= MAX_EVIDENCE_BYTES);
    }

    #[test]
    fn report_validation_and_rate_limits_apply_before_enqueue() {
        let (mut recorder, mut rx, start) = recorder(1);
        assert!(!recorder.submit(start, 0, REPORTER, TARGET, report(4)));
        assert!(!recorder.submit(start, 0, REPORTER, [3; 16], report(0)));
        assert!(recorder.submit(start, 0, REPORTER, TARGET, report(0)));
        assert!(!recorder.submit(
            start + Duration::from_secs(59),
            0,
            REPORTER,
            TARGET,
            report(0)
        ));
        assert!(!recorder.submit(
            start + Duration::from_secs(60),
            0,
            REPORTER,
            TARGET,
            report(0)
        )); // full
        rx.try_recv().unwrap();
        for minute in 1..10 {
            assert!(recorder.submit(
                start + Duration::from_secs(minute * 60),
                0,
                REPORTER,
                TARGET,
                report(0)
            ));
            rx.try_recv().unwrap();
        }
        assert!(!recorder.submit(
            start + Duration::from_secs(600),
            0,
            REPORTER,
            TARGET,
            report(0)
        ));
    }
}
