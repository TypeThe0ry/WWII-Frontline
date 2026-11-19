//! C0 gameplay messages (M1-026): rate limits, routing and the translation between the wire
//! messages and the `sim` rules API. No game rules live here (ARCHITECTURE §4.3.4): deploy and
//! squad checks are `World::deploy` / `World::squad_op`; this module limits, routes and replies.

use std::time::{Duration, Instant};

use frontline_protocol::msg::c0::{
    Chat, ChatChannel, ClientMsgC0, DeployRequest, DeployResult, Report, ServerMsgC0, SquadOp,
    SquadOpKind,
};
use frontline_sim::deploy::DeployReq;
use frontline_sim::squad::{SquadKind, SquadOp as SimSquadOp};
use frontline_sim::{PlayerId, World};

use crate::metrics::Metrics;
use crate::net::limits::RateLimiter;
use crate::text::{sanitize_chat, DupFilter};

mod placement;

/// Rate-limit classes; `label()` is the `c0_rate_limited_total{type}` label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C0Kind {
    Chat,
    Deploy,
    Report,
    Other,
}

impl C0Kind {
    pub const LABELS: [&'static str; 4] = ["chat", "deploy", "report", "other"];

    pub fn label(self) -> &'static str {
        match self {
            C0Kind::Chat => "chat",
            C0Kind::Deploy => "deploy",
            C0Kind::Report => "report",
            C0Kind::Other => "other",
        }
    }

    /// The class of a gameplay message; `None` for handshake / keep-alive messages.
    pub fn of(m: &ClientMsgC0) -> Option<C0Kind> {
        Some(match m {
            ClientMsgC0::Chat(_) => C0Kind::Chat,
            ClientMsgC0::DeployRequest(_) => C0Kind::Deploy,
            ClientMsgC0::Report(_) => C0Kind::Report,
            ClientMsgC0::SquadOp(_)
            | ClientMsgC0::MarkerPlace(_)
            | ClientMsgC0::Settings(_)
            | ClientMsgC0::PlacementRequest(_) => C0Kind::Other,
            ClientMsgC0::Hello(_) | ClientMsgC0::Ping(_) | ClientMsgC0::Disconnect(_) => {
                return None
            }
        })
    }
}

/// Chat: 5 lines per 8 s (GD §16.2). Deploy: 1/s. Report: 1/min and 10 per match.
pub const CHAT_BURST: f64 = 5.0;
pub const CHAT_WINDOW_S: f64 = 8.0;
pub const DEPLOY_PER_S: f64 = 1.0;
pub const REPORT_PER_MIN: f64 = 1.0;
pub const REPORTS_PER_MATCH: u32 = 10;
pub const OTHER_PER_S: f64 = 10.0;

/// Per-connection limiter state.
pub struct C0Gate {
    chat: RateLimiter,
    deploy: RateLimiter,
    report: RateLimiter,
    other: RateLimiter,
    reports_sent: u32,
}

impl C0Gate {
    pub fn new(now: Instant) -> Self {
        C0Gate {
            chat: RateLimiter::new(CHAT_BURST / CHAT_WINDOW_S, CHAT_BURST, now),
            deploy: RateLimiter::new(DEPLOY_PER_S, 1.0, now),
            report: RateLimiter::new(REPORT_PER_MIN / 60.0, 1.0, now),
            other: RateLimiter::new(OTHER_PER_S, OTHER_PER_S, now),
            reports_sent: 0,
        }
    }

    /// `false` = drop the message (the caller counts it).
    pub fn allow(&mut self, kind: C0Kind, now: Instant) -> bool {
        match kind {
            C0Kind::Chat => self.chat.allow(now),
            C0Kind::Deploy => self.deploy.allow(now),
            C0Kind::Other => self.other.allow(now),
            C0Kind::Report => {
                if self.reports_sent >= REPORTS_PER_MATCH || !self.report.allow(now) {
                    return false;
                }
                self.reports_sent += 1;
                true
            }
        }
    }
}

/// A stored player report (M2 adds the review queue).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportRec {
    pub from: PlayerId,
    pub report: Report,
}

/// Sim-thread side: per-player chat duplicate filters, the report queue, outgoing messages.
#[derive(Default)]
pub struct C0Handler {
    dup: std::collections::BTreeMap<PlayerId, DupFilter>,
    placement: placement::PlacementSlots,
    pub reports: Vec<ReportRec>,
}

pub type Outbox = Vec<(PlayerId, ServerMsgC0)>;

impl C0Handler {
    pub fn forget(&mut self, p: PlayerId) {
        self.dup.remove(&p);
        self.placement.forget(p);
    }

    /// Handles one gameplay message of `from`; replies and routed chat are pushed to `out`.
    pub fn handle(
        &mut self,
        world: &mut World,
        metrics: &Metrics,
        from: PlayerId,
        msg: ClientMsgC0,
        out: &mut Outbox,
    ) {
        match msg {
            ClientMsgC0::DeployRequest(r) => out.push((
                from,
                ServerMsgC0::DeployResult(deploy(world, metrics, from, r)),
            )),
            ClientMsgC0::SquadOp(op) => {
                squad_op(world, metrics, from, &op);
            }
            ClientMsgC0::PlacementRequest(req) => out.push((
                from,
                ServerMsgC0::PlacementResult(self.placement.handle(world, from, req)),
            )),
            ClientMsgC0::Chat(c) => self.chat(world, metrics, from, c, out),
            ClientMsgC0::Report(r) => {
                metrics.reports_total.inc();
                tracing::info!(
                    event = "player_report",
                    from = from as u64,
                    target = r.target as u64,
                    reason = r.reason as u64,
                );
                self.reports.push(ReportRec { from, report: r });
            }
            // The sim loop routes MarkerPlace through the C3 team channel. Settings are client-only.
            ClientMsgC0::MarkerPlace(_) | ClientMsgC0::Settings(_) => {}
            ClientMsgC0::Hello(_) | ClientMsgC0::Ping(_) | ClientMsgC0::Disconnect(_) => {}
        }
    }

    fn chat(
        &mut self,
        world: &World,
        metrics: &Metrics,
        from: PlayerId,
        c: Chat,
        out: &mut Outbox,
    ) {
        let reject = |label: &str| {
            metrics
                .chat_rejected_total
                .with_label_values(&[label])
                .inc()
        };
        let line = match sanitize_chat(&c.text) {
            Ok(l) => l,
            Err(e) => return reject(e.label()),
        };
        if let Err(e) = self.dup.entry(from).or_default().check(&line) {
            return reject(e.label());
        }
        let Some(me) = world.squads().entry(from) else {
            return reject("not_on_roster");
        };
        let recipients: Vec<PlayerId> = match c.channel {
            ChatChannel::All => world.squads().roster().map(|(p, _)| p).collect(),
            ChatChannel::Team => world
                .squads()
                .roster()
                .filter(|(_, e)| e.team == me.team)
                .map(|(p, _)| p)
                .collect(),
            ChatChannel::Squad => match me.squad {
                Some(s) => world
                    .squads()
                    .roster()
                    .filter(|(_, e)| e.squad == Some(s))
                    .map(|(p, _)| p)
                    .collect(),
                None => return reject("no_squad"),
            },
            ChatChannel::Command => return reject("channel"),
        };
        metrics.chat_routed_total.inc();
        for p in recipients {
            out.push((
                p,
                ServerMsgC0::Chat(Chat {
                    channel: c.channel,
                    text: line.clone(),
                    sender: from,
                }),
            ));
        }
    }
}

pub(crate) fn deploy_req(r: DeployRequest) -> DeployReq {
    DeployReq {
        spawn_point: r.spawn_point,
        role: r.role,
        loadout: r.loadout,
        squad: r.squad,
    }
}

pub(crate) fn deploy(
    world: &mut World,
    metrics: &Metrics,
    from: PlayerId,
    r: DeployRequest,
) -> DeployResult {
    let (label, res) = match world.deploy(from, &deploy_req(r)) {
        Ok(done) => (
            "ok",
            DeployResult {
                ok: true,
                spawn_tick: world.tick().0.saturating_add(done.spawn_in_ticks),
                reason: 0,
            },
        ),
        Err(e) => (
            "rejected",
            DeployResult {
                ok: false,
                spawn_tick: 0,
                reason: e.code(),
            },
        ),
    };
    metrics
        .deploy_requests_total
        .with_label_values(&[label])
        .inc();
    res
}

/// Maps the wire op onto the sim API; `true` when the sim accepted it.
pub(crate) fn squad_op(world: &mut World, metrics: &Metrics, from: PlayerId, op: &SquadOp) -> bool {
    let sim = match op.op {
        SquadOpKind::Create => SimSquadOp::Create {
            kind: match op.squad {
                1 => SquadKind::ArmorCrew,
                2 => SquadKind::Recon,
                _ => SquadKind::Infantry,
            },
            suffix: op.name_key.clone(),
        },
        SquadOpKind::Join => SimSquadOp::Join {
            squad: u16::from(op.squad),
        },
        SquadOpKind::Leave => SimSquadOp::Leave,
        SquadOpKind::Kick => SimSquadOp::Kick { target: op.target },
        // A pending handover is accepted by the target promoting itself (`target == self`).
        SquadOpKind::Promote if op.target == from => SimSquadOp::AcceptLead,
        SquadOpKind::Promote => SimSquadOp::Promote { target: op.target },
        SquadOpKind::Rename => SimSquadOp::Rename {
            suffix: op.name_key.clone(),
        },
        SquadOpKind::Lock => SimSquadOp::Lock,
        SquadOpKind::Unlock => SimSquadOp::Unlock,
        SquadOpKind::Invite => SimSquadOp::Invite { target: op.target },
    };
    let ok = world.squad_op(from, sim).is_ok();
    metrics
        .squad_ops_total
        .with_label_values(&[if ok { "ok" } else { "rejected" }])
        .inc();
    ok
}

/// How long the net layer waits before it forgets rate-limit history (kept for tests).
pub const GATE_IDLE: Duration = Duration::from_secs(60);
