//! M1-026: C0 gameplay messages. Rate limits (chat 5 / 8 s, deploy 1 / s, report 1 / min and 10 /
//! match), the deploy reason table against the sim's own codes, squad operations, chat routing
//! and the report queue. The net layer applies the gate and counts `c0_rate_limited_total`; the
//! handler is driven the way the sim thread drives it.

use std::time::{Duration, Instant};

use frontline_protocol::msg::c0::{
    Chat, ChatChannel, ClientMsgC0, DeployRequest, Report, ServerMsgC0, SquadOp, SquadOpKind,
};
use frontline_server::c0::{C0Gate, C0Handler, C0Kind, Outbox};
use frontline_server::metrics::Metrics;
use frontline_sim::deploy::{DeployError, SPAWN_HQ};
use frontline_sim::{CollisionData, GameData, GameMode, MapId, SimConfig, Team, Vec3, World};

fn world() -> World {
    let mut w = World::new(
        SimConfig {
            tick_hz: 30,
            match_seed: 3,
            map: MapId(0),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        },
        &CollisionData::empty(),
    );
    w.set_hq(Team::Us, Vec3::new(60.0, 5.0, 60.0));
    w.set_hq(Team::Ger, Vec3::new(1860.0, 5.0, 60.0));
    for p in 10..=15u8 {
        w.roster_add(p, Team::Us).unwrap();
        w.roster_add(p + 100, Team::Ger).unwrap();
    }
    w
}

fn ri(id: &str) -> u8 {
    GameData::embedded()
        .roles
        .iter()
        .position(|r| r.id == id)
        .unwrap() as u8
}
fn li(id: &str) -> u8 {
    GameData::embedded()
        .loadouts
        .iter()
        .position(|l| l.id == id)
        .unwrap() as u8
}

fn sq(op: SquadOpKind, squad: u8, target: u8) -> ClientMsgC0 {
    ClientMsgC0::SquadOp(SquadOp {
        op,
        squad,
        target,
        name_key: String::new(),
    })
}

fn chat(ch: ChatChannel, t: &str) -> ClientMsgC0 {
    ClientMsgC0::Chat(Chat {
        channel: ch,
        text: t.into(),
        sender: 0,
    })
}

fn rifle(point: u16, squad: u8) -> DeployRequest {
    DeployRequest {
        spawn_point: point,
        role: ri("rifleman"),
        loadout: li("us_rifleman_1"),
        squad,
    }
}

// ------------------------------------------------------------------ acceptance 1: rate limits

#[test]
fn chat_sixth_line_within_8_s_is_dropped() {
    let t0 = Instant::now();
    let mut g = C0Gate::new(t0);
    for i in 0..5 {
        assert!(
            g.allow(C0Kind::Chat, t0 + Duration::from_millis(i * 100)),
            "{i}"
        );
    }
    assert!(!g.allow(C0Kind::Chat, t0 + Duration::from_millis(600)));
    // Refills at 5 per 8 s: one more line after 1.6 s.
    assert!(g.allow(C0Kind::Chat, t0 + Duration::from_millis(2300)));
}

#[test]
fn deploy_is_one_per_second() {
    let t0 = Instant::now();
    let mut g = C0Gate::new(t0);
    assert!(g.allow(C0Kind::Deploy, t0));
    assert!(!g.allow(C0Kind::Deploy, t0 + Duration::from_millis(500)));
    assert!(g.allow(C0Kind::Deploy, t0 + Duration::from_millis(1600)));
}

#[test]
fn reports_are_one_per_minute_and_ten_per_match() {
    let t0 = Instant::now();
    let mut g = C0Gate::new(t0);
    assert!(g.allow(C0Kind::Report, t0));
    assert!(!g.allow(C0Kind::Report, t0 + Duration::from_secs(30)));
    let mut ok = 1;
    for m in 1..=20u64 {
        if g.allow(C0Kind::Report, t0 + Duration::from_secs(61 * m)) {
            ok += 1;
        }
    }
    assert_eq!(ok, 10);
}

// ------------------------------------------------------------------ acceptance 4: deploy

#[test]
fn deploy_reason_codes_match_the_sim_errors() {
    use DeployError::*;
    type Case = (&'static str, u8, DeployRequest, Option<DeployError>);
    let mut h = C0Handler::default();
    let m = Metrics::new();
    let mut w = world();
    // 10 leads infantry squad 1; 11 joined it; 12 is unassigned.
    let mut out = Outbox::new();
    h.handle(&mut w, &m, 10, sq(SquadOpKind::Create, 0, 0), &mut out);
    h.handle(&mut w, &m, 11, sq(SquadOpKind::Join, 1, 0), &mut out);
    let cases: Vec<Case> = vec![
        ("not on roster", 99, rifle(SPAWN_HQ, 0), Some(NotOnRoster)),
        (
            "unknown role",
            11,
            DeployRequest {
                role: 250,
                ..rifle(SPAWN_HQ, 1)
            },
            Some(UnknownRole),
        ),
        (
            "unknown loadout",
            11,
            DeployRequest {
                loadout: 250,
                ..rifle(SPAWN_HQ, 1)
            },
            Some(UnknownLoadout),
        ),
        ("wrong squad", 11, rifle(SPAWN_HQ, 9), Some(WrongSquad)),
        (
            "unassigned officer",
            12,
            DeployRequest {
                role: ri("officer"),
                loadout: li("us_officer_1"),
                ..rifle(SPAWN_HQ, 0)
            },
            Some(UnassignedRole),
        ),
        (
            "member officer",
            11,
            DeployRequest {
                role: ri("officer"),
                loadout: li("us_officer_1"),
                ..rifle(SPAWN_HQ, 1)
            },
            Some(LeaderOnly),
        ),
        (
            "unassigned off-HQ",
            12,
            rifle(5, 0),
            Some(UnassignedNeedsHq),
        ),
        ("no such point", 11, rifle(77, 1), Some(NoSuchPoint)),
        (
            "loadout of another role",
            11,
            DeployRequest {
                loadout: li("us_officer_1"),
                ..rifle(SPAWN_HQ, 1)
            },
            Some(LoadoutWrongRole),
        ),
        (
            "other faction loadout",
            11,
            DeployRequest {
                loadout: li("ger_rifleman_1"),
                ..rifle(SPAWN_HQ, 1)
            },
            Some(LoadoutWrongFaction),
        ),
        ("valid member", 11, rifle(SPAWN_HQ, 1), None),
        ("valid unassigned", 12, rifle(SPAWN_HQ, 0), None),
    ];
    assert!(cases.iter().filter(|c| c.3.is_some()).count() >= 10);
    for (name, p, req, want) in cases {
        let mut out = Outbox::new();
        h.handle(&mut w, &m, p, ClientMsgC0::DeployRequest(req), &mut out);
        let (to, ServerMsgC0::DeployResult(r)) = out.pop().expect(name) else {
            panic!("{name}: no DeployResult");
        };
        assert_eq!(to, p, "{name}");
        match want {
            Some(e) => assert!(!r.ok && r.reason == e.code(), "{name}: {r:?} vs {e:?}"),
            None => assert!(
                r.ok && r.reason == 0 && r.spawn_tick >= w.tick().0,
                "{name}: {r:?}"
            ),
        }
    }
    assert_eq!(m.deploy_requests_total.with_label_values(&["ok"]).get(), 2);
    assert_eq!(
        m.deploy_requests_total
            .with_label_values(&["rejected"])
            .get(),
        10
    );
}

// ------------------------------------------------------------------ squads, chat, reports

#[test]
fn squad_ops_reach_the_sim_roster() {
    let mut h = C0Handler::default();
    let m = Metrics::new();
    let mut w = world();
    let mut out = Outbox::new();
    h.handle(&mut w, &m, 10, sq(SquadOpKind::Create, 0, 0), &mut out);
    h.handle(&mut w, &m, 11, sq(SquadOpKind::Join, 1, 0), &mut out);
    h.handle(&mut w, &m, 12, sq(SquadOpKind::Join, 1, 0), &mut out);
    assert_eq!(w.squads().squad(1).unwrap().members.len(), 3);
    h.handle(&mut w, &m, 10, sq(SquadOpKind::Kick, 1, 12), &mut out);
    assert_eq!(w.squads().squad(1).unwrap().members.len(), 2);
    // A non-leader cannot kick.
    h.handle(&mut w, &m, 11, sq(SquadOpKind::Kick, 1, 10), &mut out);
    assert_eq!(w.squads().squad(1).unwrap().members.len(), 2);
    assert_eq!(m.squad_ops_total.with_label_values(&["rejected"]).get(), 1);
    assert!(out.is_empty());
}

#[test]
fn chat_routes_by_channel_and_filters() {
    let mut h = C0Handler::default();
    let m = Metrics::new();
    let mut w = world();
    let mut out = Outbox::new();
    h.handle(&mut w, &m, 10, sq(SquadOpKind::Create, 0, 0), &mut out);
    h.handle(&mut w, &m, 11, sq(SquadOpKind::Join, 1, 0), &mut out);

    let to = |out: &Outbox| -> Vec<u8> { out.iter().map(|(p, _)| *p).collect() };
    h.handle(
        &mut w,
        &m,
        10,
        chat(ChatChannel::Squad, "hold the line"),
        &mut out,
    );
    assert_eq!(to(&out), vec![10, 11]);
    out.clear();
    h.handle(
        &mut w,
        &m,
        10,
        chat(ChatChannel::Team, "ｇｏ ｇｏ"),
        &mut out,
    );
    assert_eq!(to(&out), vec![10, 11, 12, 13, 14, 15]);
    assert!(matches!(&out[0].1, ServerMsgC0::Chat(c) if c.text == "go go" && c.sender == 10));
    out.clear();
    h.handle(&mut w, &m, 10, chat(ChatChannel::All, "gg"), &mut out);
    assert_eq!(out.len(), 12);
    out.clear();
    h.handle(&mut w, &m, 10, chat(ChatChannel::All, "gg"), &mut out);
    assert_eq!(out.len(), 12);
    out.clear();
    // Reject the third matching line and send notices to the originators only.
    h.handle(&mut w, &m, 12, chat(ChatChannel::Squad, "hi"), &mut out);
    h.handle(&mut w, &m, 10, chat(ChatChannel::All, "gg"), &mut out);
    h.handle(
        &mut w,
        &m,
        10,
        chat(ChatChannel::All, "a\u{202E}b"),
        &mut out,
    );
    h.handle(&mut w, &m, 10, chat(ChatChannel::All, "a\nb"), &mut out);
    let notices: Vec<_> = out
        .iter()
        .map(|(id, message)| {
            let ServerMsgC0::Notice(notice) = message else {
                panic!("unexpected broadcast {message:?}")
            };
            (*id, notice.key.as_str())
        })
        .collect();
    assert_eq!(
        notices,
        [
            (12, "comms.chat.no_squad"),
            (10, "comms.chat.duplicate"),
            (10, "comms.chat.bidi"),
            (10, "comms.chat.control"),
        ]
    );
    let rej = |l: &str| m.chat_rejected_total.with_label_values(&[l]).get();
    assert_eq!(
        (
            rej("no_squad"),
            rej("duplicate"),
            rej("bidi"),
            rej("control")
        ),
        (1, 1, 1, 1)
    );
}

#[test]
fn reports_are_queued_and_logged() {
    let mut h = C0Handler::default();
    let m = Metrics::new();
    let mut w = world();
    let mut out = Outbox::new();
    h.handle(
        &mut w,
        &m,
        10,
        ClientMsgC0::Report(Report {
            target: 111,
            reason: 2,
            note: "spam".into(),
        }),
        &mut out,
    );
    assert_eq!(h.reports.len(), 1);
    assert_eq!(h.reports[0].from, 10);
    assert_eq!(m.reports_total.get(), 1);
}
