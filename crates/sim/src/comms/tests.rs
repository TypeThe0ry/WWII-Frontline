use super::*;
use crate::squad::{SquadKind, SquadManager, SquadOp};
use crate::{GameData, Team, Tick, Vec3};
use proptest::prelude::*;
use MarkerKind::*;

const HZ: u32 = 30;
const KINDS: [MarkerKind; 25] = [
    Attention,
    Enemy,
    NeedMedic,
    NeedAmmo,
    NeedEngineer,
    NeedAt,
    NeedSupplies,
    Thanks,
    OrderMove,
    OrderAttack,
    OrderDefend,
    OrderHold,
    OrderBuild,
    TeamInfantry,
    TeamVehicle,
    TeamArtillery,
    TeamAtGun,
    TeamGarrison,
    CommandAttack,
    CommandDefend,
    FireRequest,
    GarrisonRequest,
    SupplyRequest,
    FireMission,
    RallyMarker,
];

fn registry() -> MarkerRegistry {
    MarkerRegistry::new(GameData::comms(), HZ)
}

fn member(player: u8, role: &'static str) -> Member {
    Member {
        player,
        team: Team::Us,
        squad: Some(1),
        role: Some(role),
        lead: false,
        joined: Tick(0),
    }
}

#[test]
fn gd_placement_matrix() {
    let roles = [
        "commander",
        "officer",
        "rifleman",
        "assault",
        "automatic_rifleman",
        "medic",
        "support",
        "machine_gunner",
        "anti_tank",
        "engineer",
        "tank_commander",
        "crewman",
        "spotter",
        "sniper",
    ];
    let registry = registry();
    for role in roles {
        for lead in [false, true] {
            for kind in KINDS {
                let player = Member {
                    lead,
                    ..member(1, role)
                };
                let allowed = match kind {
                    Attention | Enemy | NeedMedic | NeedAmmo | NeedEngineer | NeedAt
                    | NeedSupplies | Thanks => true,
                    OrderMove | OrderAttack | OrderDefend | OrderHold | OrderBuild => lead,
                    TeamInfantry | TeamVehicle | TeamArtillery | TeamAtGun | TeamGarrison => {
                        matches!(role, "commander" | "officer" | "spotter")
                    }
                    CommandAttack | CommandDefend => role == "commander",
                    FireRequest | GarrisonRequest => matches!(role, "commander" | "officer"),
                    SupplyRequest => matches!(role, "commander" | "officer" | "engineer"),
                    FireMission | RallyMarker => false,
                };
                assert_eq!(
                    registry.can_place(player, kind).is_ok(),
                    allowed,
                    "{role} {lead} {kind:?}"
                );
            }
        }
    }
    let solo = Member {
        squad: None,
        lead: true,
        ..member(1, "officer")
    };
    assert_eq!(
        registry.can_place(solo, OrderAttack),
        Err(MarkerErr::NeedSquad)
    );
    assert_eq!(
        registry.can_place(solo, Attention),
        Err(MarkerErr::NeedSquad)
    );
    assert!(registry.can_place(solo, Enemy).is_ok());
    assert!(registry.can_place(solo, NeedSupplies).is_ok());
}

#[test]
fn visibility_tracks_live_viewer_role_leader_and_team() {
    let mut registry = registry();
    let author = member(1, "rifleman");
    let ally = member(2, "rifleman");
    let lead = Member { lead: true, ..ally };
    let medic = Member {
        squad: Some(2),
        role: Some("medic"),
        ..ally
    };
    let enemy = Member {
        team: Team::Ger,
        ..medic
    };
    let attention = registry
        .place(author, Attention, Vec3::ZERO, None, Tick(0))
        .unwrap();
    let need = registry
        .place(author, NeedMedic, Vec3::ZERO, None, Tick(60))
        .unwrap();
    assert!(registry.visible_to(&attention, ally, Tick(60)));
    assert!(!registry.visible_to(&attention, medic, Tick(60)));
    assert!(!registry.visible_to(&need, ally, Tick(60)));
    assert!(registry.visible_to(&need, lead, Tick(60)));
    assert!(registry.visible_to(&need, medic, Tick(60)));
    assert!(!registry.visible_to(&need, enemy, Tick(60)));
    let other_author = member(3, "officer");
    let squad_enemy = registry
        .place(author, Enemy, Vec3::ZERO, Some(10), Tick(120))
        .unwrap();
    let team_enemy = registry
        .place(other_author, Enemy, Vec3::ZERO, Some(10), Tick(120))
        .unwrap();
    assert!(!registry.visible_to(&squad_enemy, medic, Tick(120)));
    assert!(registry.visible_to(&team_enemy, medic, Tick(120)));
    assert_eq!(registry.visible(enemy, Tick(120)).count(), 0);
}

#[test]
fn cooldown_shared_ping_cap_and_replacement() {
    let mut registry = registry();
    let player = member(1, "rifleman");
    let place = |registry: &mut MarkerRegistry, kind, tick| {
        registry.place(player, kind, Vec3::ZERO, None, Tick(tick))
    };
    assert!(place(&mut registry, Attention, 0).is_ok());
    assert_eq!(place(&mut registry, Enemy, 57), Err(MarkerErr::Cooldown));
    assert_eq!(place(&mut registry, Enemy, 59), Err(MarkerErr::Cooldown));
    assert!(place(&mut registry, Enemy, 60).is_ok());
    assert!(place(&mut registry, Thanks, 120).is_ok());
    assert_eq!(
        place(&mut registry, Attention, 180),
        Err(MarkerErr::PlayerCap)
    );
    let medic = place(&mut registry, NeedMedic, 180).unwrap();
    let medic_again = place(&mut registry, NeedMedic, 240).unwrap();
    let ammo = place(&mut registry, NeedAmmo, 300).unwrap();
    assert_eq!(medic.id, medic_again.id);
    assert_ne!(medic.id, ammo.id);
    assert_eq!(medic_again.expires_at, Some(Tick(840)));
    assert!(place(&mut registry, Attention, 360).is_ok());
    assert_eq!(registry.tick(Tick(360)).len(), 3);
}

#[test]
fn quota_groups_and_squad_order_replacement() {
    let mut registry = registry();
    let lead = Member {
        lead: true,
        ..member(1, "officer")
    };
    let other_lead = Member { player: 2, ..lead };
    let commander = member(3, "commander");
    let spotter = member(4, "spotter");
    let place = |registry: &mut MarkerRegistry, member, kind, target| {
        registry.place(member, kind, Vec3::ZERO, target, Tick(0))
    };
    let order = place(&mut registry, lead, OrderMove, None).unwrap();
    for kind in [OrderAttack, OrderDefend, OrderHold, OrderBuild] {
        place(&mut registry, lead, kind, None).unwrap();
    }
    let replacement = place(&mut registry, other_lead, OrderMove, None).unwrap();
    assert_eq!(order.id, replacement.id);
    assert_eq!(replacement.author, other_lead);
    assert_eq!(registry.remove(lead, order.id), Err(MarkerErr::NotOwner));
    for (author, cap) in [(lead, 5), (spotter, 10), (commander, 20)] {
        for target in 0..cap {
            place(&mut registry, author, TeamInfantry, Some(target)).unwrap();
        }
        assert_eq!(
            place(&mut registry, author, TeamVehicle, Some(cap)),
            Err(MarkerErr::PlayerCap)
        );
    }
    for _ in 0..12 {
        let command = place(&mut registry, commander, CommandAttack, None).unwrap();
        assert_eq!(command.expires_at, None);
    }
    assert_eq!(
        place(&mut registry, commander, CommandDefend, None),
        Err(MarkerErr::PlayerCap)
    );
    for (author, kind, cap) in [
        (lead, FireRequest, 1),
        (commander, FireRequest, 3),
        (lead, GarrisonRequest, 3),
        (member(5, "engineer"), SupplyRequest, 3),
    ] {
        for _ in 0..cap {
            place(&mut registry, author, kind, None).unwrap();
        }
        assert_eq!(
            place(&mut registry, author, kind, None),
            Err(MarkerErr::PlayerCap)
        );
    }
}

#[test]
fn lifetimes_are_exact_across_tick_wrap() {
    let player = Member {
        lead: true,
        ..member(1, "officer")
    };
    let start = u32::MAX - 100;
    for (kind, seconds) in [
        (Attention, 8),
        (Enemy, 6),
        (Thanks, 8),
        (NeedMedic, 20),
        (OrderHold, 180),
        (TeamInfantry, 8),
        (TeamVehicle, 15),
        (TeamArtillery, 60),
        (TeamAtGun, 60),
        (TeamGarrison, 60),
        (FireRequest, 90),
        (GarrisonRequest, 120),
        (SupplyRequest, 120),
    ] {
        let mut registry = registry();
        let marker = registry
            .place(player, kind, Vec3::ZERO, None, Tick(start))
            .unwrap();
        let end = start.wrapping_add(seconds * HZ);
        assert_eq!(marker.expires_at, Some(Tick(end)));
        assert!(registry.tick(Tick(end - 1)).is_empty());
        assert!(
            registry.visible_to(&marker, player, Tick(end - 1)),
            "{kind:?}"
        );
        assert!(!registry.visible_to(&marker, player, Tick(end)));
        assert!(registry.audience_allows(&marker, player, Tick(end)));
        assert_eq!(registry.tick(Tick(end)), vec![marker]);
        assert!(registry.tick(Tick(end + 1)).is_empty());
        assert!(registry.take_due(Tick(end + 1)).is_empty());
    }
}

#[test]
fn move_keep_ttl_and_enemy_reobservation_refresh() {
    let mut registry = registry();
    let actor = member(1, "spotter");
    let original = registry
        .place(
            actor,
            TeamVehicle,
            Vec3::new(2.49, 800.0, -7.5),
            Some(42),
            Tick(0),
        )
        .unwrap();
    assert_eq!(original.pos, Vec3::new(0.0, 0.0, -10.0));
    // Late first drain cannot yield a second transmission on the very next tick.
    assert_eq!(registry.take_due(Tick(29)), vec![original]);
    assert_eq!(
        registry.move_marker(actor, original.id, Vec3::splat(25.0), Tick(29)),
        Err(MarkerErr::RefreshTooSoon)
    );
    let changed = registry
        .move_marker(actor, original.id, Vec3::new(12.5, 42.0, -3.0), Tick(30))
        .unwrap();
    assert_eq!(changed.pos, Vec3::new(15.0, 0.0, -5.0));
    assert_eq!(changed.expires_at, original.expires_at);
    assert!(registry.take_due(Tick(58)).is_empty());
    assert_eq!(registry.take_due(Tick(59)), vec![changed]);
    assert_eq!(
        registry.place(actor, TeamVehicle, Vec3::ZERO, Some(42), Tick(59)),
        Err(MarkerErr::RefreshTooSoon)
    );
    let refreshed = registry
        .place(actor, TeamVehicle, Vec3::ONE, Some(42), Tick(60))
        .unwrap();
    assert_eq!(refreshed.id, original.id);
    assert_eq!(refreshed.expires_at, Some(Tick(510)));
    assert_eq!(
        registry.move_marker(
            Member {
                joined: Tick(1),
                ..actor
            },
            original.id,
            Vec3::ZERO,
            Tick(99)
        ),
        Err(MarkerErr::NotOwner)
    );
    assert_eq!(registry.remove(actor, original.id).unwrap(), refreshed);
    assert!(registry.get(original.id).is_none());
    assert!(registry.take_due(Tick(100)).is_empty());
    assert_eq!(
        registry.place(actor, TeamVehicle, Vec3::splat(f32::NAN), None, Tick(100)),
        Err(MarkerErr::InvalidPosition)
    );
}

#[test]
fn four_unique_mutes_suppress_only_that_squads_pings_for_ten_minutes() {
    let mut registry = registry();
    let officer = member(1, "officer");
    let viewer = member(2, "medic");
    let outside = Member {
        squad: Some(2),
        ..viewer
    };
    let enemy = Member {
        team: Team::Ger,
        ..viewer
    };
    let start = Tick(u32::MAX - 300);
    let at = |delta| Tick(start.0.wrapping_add(delta));
    let ping = registry
        .place(officer, Enemy, Vec3::ZERO, None, start)
        .unwrap();
    let mark = registry
        .place(officer, TeamInfantry, Vec3::ZERO, None, start)
        .unwrap();
    assert!(!registry.report_squad_mute(officer, officer, start));
    assert!(!registry.report_squad_mute(outside, officer, start));
    assert!(!registry.report_squad_mute(enemy, officer, start));
    for _ in 0..8 {
        assert!(!registry.report_squad_mute(viewer, officer, start));
    }
    for player in 3..=5 {
        assert_eq!(
            registry.report_squad_mute(Member { player, ..viewer }, officer, start),
            player == 5
        );
    }
    assert!(registry.take_audience_changed());
    assert!(!registry.take_audience_changed());
    assert!(!registry.visible_to(&ping, viewer, start));
    assert!(registry.visible_to(&ping, outside, start));
    assert!(registry.visible_to(&mark, viewer, start));
    assert!(!registry.visible_to(&ping, enemy, start));
    registry.tick(at(17_999));
    assert!(!registry.take_audience_changed());
    let fresh = registry
        .place(officer, Enemy, Vec3::ONE, None, at(17_999))
        .unwrap();
    assert!(!registry.visible_to(&fresh, viewer, at(17_999)));
    assert!(registry.visible_to(&fresh, viewer, at(18_000)));
    registry.tick(at(18_000));
    assert!(registry.take_audience_changed());
    assert!(!registry.take_audience_changed());
    assert!(!registry.report_squad_mute(viewer, officer, at(18_000)));
}

#[test]
fn roster_uses_active_invalidated_role_and_reconciles_departures() {
    let data = GameData::embedded();
    let mut roster = SquadManager::new(data, HZ as u8);
    roster.roster_add(1, Team::Us, Tick(5)).unwrap();
    roster
        .apply(
            1,
            SquadOp::Create {
                kind: SquadKind::Infantry,
                suffix: String::new(),
            },
            Tick(5),
        )
        .unwrap();
    let entry = roster.entry_mut(1).unwrap();
    entry.role = Some("officer");
    entry.role_invalid = true;
    let author = Member::from_roster(&roster, 1).unwrap();
    assert_eq!(author.role, Some("officer"));
    assert_eq!(author.joined, Tick(5));
    assert!(author.lead);
    let mut registry = registry();
    let placed = registry
        .place(author, TeamInfantry, Vec3::ZERO, None, Tick(6))
        .unwrap();
    assert!(registry.reconcile(&roster).is_empty());
    roster.roster_remove(1, Tick(7));
    assert!(Member::from_roster(&roster, 1).is_none());
    assert_eq!(registry.reconcile(&roster), vec![placed]);
    assert!(registry.reconcile(&roster).is_empty());
}

#[test]
fn forget_clears_same_tick_reused_slot_markers_cooldown_and_mutes() {
    let mut registry = registry();
    let author = member(1, "officer");
    let viewer = member(2, "medic");
    let now = Tick(50);
    let first = registry
        .place(author, Enemy, Vec3::ZERO, None, now)
        .unwrap();
    for player in 2..=5 {
        registry.report_squad_mute(Member { player, ..viewer }, author, now);
    }
    assert!(!registry.visible_to(&first, viewer, now));
    assert_eq!(registry.forget(author.player), vec![first]);
    assert!(registry.forget(author.player).is_empty());
    let fresh = registry
        .place(author, Enemy, Vec3::ZERO, None, now)
        .unwrap();
    assert_ne!(fresh.id, first.id);
    assert!(registry.visible_to(&fresh, viewer, now));
    assert_eq!(registry.take_due(now), vec![fresh]);
}

#[test]
fn registry_caps_each_team_and_releases_capacity_after_tick() {
    let limit = markers::MAX_ACTIVE_MARKERS_PER_TEAM;
    let mut cfg = GameData::comms().clone();
    cfg.team_mark.spotter_max_markers = u8::MAX;
    let mut registry = MarkerRegistry::new(&cfg, HZ);
    for i in 0..limit {
        let author = member((i % 3) as u8, "spotter");
        registry
            .place(author, TeamInfantry, Vec3::ZERO, Some(i as u16), Tick(0))
            .unwrap();
    }
    let us = member(5, "spotter");
    assert_eq!(
        registry.place(us, TeamInfantry, Vec3::ZERO, None, Tick(0)),
        Err(MarkerErr::TeamCap)
    );
    let ger = Member {
        team: Team::Ger,
        ..us
    };
    registry
        .place(ger, TeamInfantry, Vec3::ZERO, None, Tick(1))
        .unwrap();
    assert_eq!(registry.tick(Tick(240)).len(), limit);
    assert!(registry
        .place(us, TeamInfantry, Vec3::ZERO, None, Tick(240))
        .is_ok());
}

proptest! {
    #[test]
    fn random_placement_sequences_respect_quotas(ops in prop::collection::vec((0..KINDS.len(), 0..90u32, any::<u16>()), 0..256)) {
        let mut registry = registry();
        let commander = Member { lead: true, ..member(1, "commander") };
        let mut now = Tick(u32::MAX - 500);
        for (kind, delay, target) in ops {
            now = Tick(now.0.wrapping_add(delay));
            registry.tick(now);
            let _ = registry.place(commander, KINDS[kind], Vec3::ZERO, Some(target), now);
            let count = |kinds: &[MarkerKind]| registry.visible(commander, now)
                .filter(|marker| kinds.contains(&marker.kind)).count();
            prop_assert!(count(&[Attention, Enemy, Thanks]) <= 3);
            prop_assert!(count(&[OrderMove, OrderAttack, OrderDefend, OrderHold, OrderBuild]) <= 5);
            prop_assert!(count(&[TeamInfantry, TeamVehicle, TeamArtillery, TeamAtGun, TeamGarrison]) <= 20);
            prop_assert!(count(&[CommandAttack, CommandDefend]) <= 12);
            for kind in [NeedMedic, NeedAmmo, NeedEngineer, NeedAt, NeedSupplies, OrderMove,
                OrderAttack, OrderDefend, OrderHold, OrderBuild] {
                prop_assert!(count(&[kind]) <= 1);
            }
            for kind in [FireRequest, GarrisonRequest, SupplyRequest] {
                prop_assert!(count(&[kind]) <= 3);
            }
        }
    }

    #[test]
    fn quantization_stays_within_half_a_cell_per_axis(x in -20000.0f32..20000.0, z in -20000.0f32..20000.0) {
        let marker = registry().place(member(1, "officer"), Enemy, Vec3::new(x, 20.0, z), None, Tick(0)).unwrap();
        prop_assert_eq!(marker.pos.x % 5.0, 0.0);
        prop_assert_eq!(marker.pos.z % 5.0, 0.0);
        prop_assert!((marker.pos.x - x).abs() <= 2.5);
        prop_assert!((marker.pos.z - z).abs() <= 2.5);
    }
}
