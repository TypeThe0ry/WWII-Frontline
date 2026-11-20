//! M1-021b: `validate_deploy` negative matrix, role-cooldown, and the headless deploy flow.
mod common;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use common::*;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::deploy::{DeployError, DeployReq, SPAWN_HQ};
use frontline_sim::spawn::garrison::GarrisonState;
use frontline_sim::squad::{SquadKind, SquadOp};
use frontline_sim::supply_iface::SupplyProvider;
use frontline_sim::{GameData, LifeStateView, PlayerId, Team, World};
use glam::Vec3;

const HZ: u32 = 30;

#[derive(Clone)]
struct Stub(Arc<AtomicU32>);

impl SupplyProvider for Stub {
    fn available(&self, _: Vec3, _: f32) -> u32 {
        self.0.load(Ordering::Relaxed)
    }
    fn consume(&mut self, _: Vec3, _: f32, amount: u32) -> bool {
        let cur = self.0.load(Ordering::Relaxed);
        if cur >= amount {
            self.0.store(cur - amount, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
}

fn idle() -> InputCmd {
    cmd(0, 0, 49152, 0, 0)
}

fn use_btn() -> InputCmd {
    cmd(0, 0, 49152, buttons::USE, 0)
}

fn step_all(w: &mut World, special: &[(u8, InputCmd)]) {
    let ins: Vec<_> = w
        .view()
        .players()
        .iter()
        .map(|p| {
            let c = special
                .iter()
                .find(|(i, _)| *i == p.player)
                .map_or_else(idle, |(_, c)| *c);
            (p.player, c)
        })
        .collect();
    w.step(&ins);
}

fn ri(data: &GameData, id: &str) -> u8 {
    data.roles.iter().position(|r| r.id == id).expect("role") as u8
}

fn li(data: &GameData, id: &str) -> u8 {
    data.loadouts
        .iter()
        .position(|l| l.id == id)
        .expect("loadout") as u8
}

/// Default request for a US Rifleman at the HQ in squad `squad`.
fn rifle(squad: u8) -> DeployReq {
    let d = GameData::embedded();
    DeployReq {
        spawn_point: SPAWN_HQ,
        role: ri(d, "rifleman"),
        loadout: li(d, "us_rifleman_1"),
        squad,
    }
}

#[test]
fn compact_map_can_deploy_without_a_protected_hq() {
    let mut f = fx_with(GameData::embedded(), false, false);
    let pos = Vec3::new(60.0, 5.0, 60.0);
    f.w.set_team_spawn(Team::Us, pos);
    assert!(!f.w.has_hq(Team::Us));
    assert_eq!(f.w.deploy(11, &rifle(1)).unwrap().spawn_in_ticks, 0);
    assert_eq!(f.w.player_state(11).unwrap().pos, pos);
    assert_eq!(f.w.spawn_protection_left(11), 0);
}

#[test]
fn first_deploy_grants_role_abilities() {
    let mut f = fx_with(GameData::embedded(), false, false);
    let pos = Vec3::new(60.0, 5.0, 60.0);
    f.w.set_team_spawn(Team::Us, pos);
    f.w.deploy(10, &req("officer", "us_officer_1", SPAWN_HQ, 1))
        .unwrap();
    f.w.deploy(11, &req("support", "us_support_1", SPAWN_HQ, 1))
        .unwrap();
    assert!(f.w.place_garrison(10, pos).is_ok());
    assert!(f.w.place_ammo_crate(11).is_ok());
}

fn req(role: &str, loadout: &str, point: u16, squad: u8) -> DeployReq {
    let d = GameData::embedded();
    DeployReq {
        spawn_point: point,
        role: ri(d, role),
        loadout: li(d, loadout),
        squad,
    }
}

struct Fx {
    w: World,
    /// Active US Garrison.
    g_us: u16,
}

/// Bodies: 0 US (Officer), 1 Ger (Officer). Roster-only: US 10..=15 in squad 1 (leader 10),
/// US 20..=21 unassigned, matching Germans 110.. for balance. One Active US Garrison at x=500.
fn fx_with(data: &'static GameData, garrison: bool, hq: bool) -> Fx {
    let col = synth_data(30, 6, &|_, _| 5.0);
    let players = [
        (0u8, Team::Us, 300.0, 60.0, 16384u16),
        (1, Team::Ger, 1800.0, 60.0, 16384),
    ];
    let mut w = duel_world(data, &col, 5, &players);
    if hq {
        w.set_hq(Team::Us, Vec3::new(60.0, 5.0, 60.0));
        w.set_hq(Team::Ger, Vec3::new(1860.0, 5.0, 60.0));
    }
    w.set_officer(0, true);
    w.set_officer(1, true);
    w.set_supply_provider(Box::new(Stub(Arc::new(AtomicU32::new(5000)))));
    w.roster_add(0, Team::Us).unwrap();
    w.roster_add(1, Team::Ger).unwrap();
    for p in (10..=15).chain(20..=21) {
        w.roster_add(p, Team::Us).unwrap();
        w.roster_add(p + 100, Team::Ger).unwrap();
    }
    w.squad_op(
        10,
        SquadOp::Create {
            kind: SquadKind::Infantry,
            suffix: String::new(),
        },
    )
    .unwrap();
    for p in 11..=15 {
        w.squad_op(p, SquadOp::Join { squad: 1 }).unwrap();
    }
    let mut g_us = 0;
    if garrison {
        g_us = w.place_garrison(0, Vec3::new(500.0, 5.0, 60.0)).unwrap();
        w.debug_teleport(0, Vec3::new(502.0, 5.02, 60.0));
        for _ in 0..(40 * HZ) {
            step_all(&mut w, &[(0, use_btn())]);
            if w.garrison_list()[0].state == GarrisonState::Active {
                break;
            }
        }
        assert_eq!(w.garrison_list()[0].state, GarrisonState::Active);
        w.debug_teleport(0, Vec3::new(300.0, 5.02, 60.0));
        step_all(&mut w, &[]);
    }
    Fx { w, g_us }
}

fn fx() -> Fx {
    fx_with(GameData::embedded(), true, true)
}

#[test]
fn valid_requests_pass() {
    let f = fx();
    let d = GameData::embedded();
    assert!(f.w.validate_deploy(11, &rifle(1)).is_ok());
    assert!(f.w.validate_deploy(20, &rifle(0)).is_ok()); // unassigned Rifleman at the HQ
    let to_g = DeployReq {
        spawn_point: f.g_us,
        ..rifle(1)
    };
    assert!(f.w.validate_deploy(11, &to_g).is_ok());
    // Leader Officer.
    let off = req("officer", "us_officer_1", SPAWN_HQ, 1);
    assert_eq!(f.w.validate_deploy(10, &off).unwrap().role, "officer");
    assert_eq!(ri(d, "rifleman"), rifle(1).role);
}

/// Each case: a name, setup returning (player, request), the expected error.
#[test]
fn deploy_negative_matrix() {
    let d = GameData::embedded();
    type Setup = Box<dyn Fn(&mut Fx) -> (PlayerId, DeployReq)>;
    let mut cases: Vec<(&str, Setup, DeployError)> = Vec::new();
    macro_rules! case {
        ($name:expr, $err:expr, $f:expr) => {
            cases.push(($name, Box::new($f), $err));
        };
    }
    // --- player state / roster ---
    case!("not on roster", DeployError::NotOnRoster, |_| (
        99,
        rifle(0)
    ));
    case!("alive body", DeployError::NotDeployable, |_| (0, rifle(0)));
    case!("downed body", DeployError::NotDeployable, |f| {
        f.w.debug_down(0);
        (0, rifle(0))
    });
    // --- squad ---
    case!("member claims no squad", DeployError::WrongSquad, |_| (
        11,
        rifle(0)
    ));
    case!("unassigned claims a squad", DeployError::WrongSquad, |_| (
        20,
        rifle(1)
    ));
    case!(
        "claims a squad that does not exist",
        DeployError::WrongSquad,
        |_| (11, rifle(77))
    );
    // --- role ---
    case!("role index out of range", DeployError::UnknownRole, |_| (
        11,
        DeployReq {
            role: 99,
            ..rifle(1)
        }
    ));
    case!("medic 3rd in squad", DeployError::RoleFull, |f| {
        f.w.set_role(11, Some("medic")).unwrap();
        f.w.set_role(12, Some("medic")).unwrap();
        (13, req("medic", "us_medic_1", SPAWN_HQ, 1))
    });
    case!("support 2nd in squad", DeployError::RoleFull, |f| {
        f.w.set_role(11, Some("support")).unwrap();
        (13, req("support", "us_support_1", SPAWN_HQ, 1))
    });
    case!(
        "officer 2nd (leader only anyway)",
        DeployError::LeaderOnly,
        |f| {
            f.w.set_role(10, Some("officer")).unwrap();
            (12, req("officer", "us_officer_1", SPAWN_HQ, 1))
        }
    );
    case!("non-leader officer", DeployError::LeaderOnly, |_| (
        12,
        req("officer", "us_officer_1", SPAWN_HQ, 1)
    ));
    case!(
        "support in a 2-man squad",
        DeployError::SquadTooSmall,
        |f| {
            for p in 12..=15 {
                f.w.squad_op(p, SquadOp::Leave).unwrap();
            }
            (11, req("support", "us_support_1", SPAWN_HQ, 1))
        }
    );
    case!("medic alone", DeployError::SquadTooSmall, |f| {
        for p in 11..=15 {
            f.w.squad_op(p, SquadOp::Leave).unwrap();
        }
        (10, req("medic", "us_medic_1", SPAWN_HQ, 1))
    });
    case!(
        "crewman in an infantry squad",
        DeployError::NotInArmor,
        |_| (11, req("crewman", "us_crew_1", SPAWN_HQ, 1))
    );
    case!(
        "rifleman in an armor crew",
        DeployError::WrongSquadKind,
        |f| {
            f.w.squad_op(
                20,
                SquadOp::Create {
                    kind: SquadKind::ArmorCrew,
                    suffix: String::new(),
                },
            )
            .unwrap();
            (20, rifle(f.w.squads().squad_of(20).unwrap().id as u8))
        }
    );
    case!("3rd crewman in a crew", DeployError::RoleFull, |f| {
        f.w.squad_op(
            20,
            SquadOp::Create {
                kind: SquadKind::ArmorCrew,
                suffix: String::new(),
            },
        )
        .unwrap();
        let s = f.w.squads().squad_of(20).unwrap().id;
        f.w.squad_op(21, SquadOp::Join { squad: s }).unwrap();
        f.w.squad_op(11, SquadOp::Leave).unwrap();
        f.w.squad_op(11, SquadOp::Join { squad: s }).unwrap();
        f.w.set_role(20, Some("crewman")).unwrap();
        f.w.set_role(21, Some("crewman")).unwrap();
        (11, req("crewman", "us_crew_1", SPAWN_HQ, s as u8))
    });
    case!("unassigned medic", DeployError::UnassignedRole, |_| (
        20,
        req("medic", "us_medic_1", SPAWN_HQ, 0)
    ));
    case!("unassigned officer", DeployError::UnassignedRole, |_| (
        20,
        req("officer", "us_officer_1", SPAWN_HQ, 0)
    ));
    case!("unassigned support", DeployError::UnassignedRole, |_| (
        20,
        req("support", "us_support_1", SPAWN_HQ, 0)
    ));
    // --- loadout ---
    case!(
        "loadout index out of range",
        DeployError::UnknownLoadout,
        |_| (
            11,
            DeployReq {
                loadout: 250,
                ..rifle(1)
            }
        )
    );
    case!(
        "medic loadout on a rifleman",
        DeployError::LoadoutWrongRole,
        |_| (11, req("rifleman", "us_medic_1", SPAWN_HQ, 1))
    );
    case!(
        "support loadout on a rifleman",
        DeployError::LoadoutWrongRole,
        |_| (11, req("rifleman", "us_support_1", SPAWN_HQ, 1))
    );
    case!(
        "german loadout for an american",
        DeployError::LoadoutWrongFaction,
        |_| (11, req("rifleman", "ger_rifleman_1", SPAWN_HQ, 1))
    );
    case!(
        "american loadout for a german",
        DeployError::LoadoutWrongFaction,
        |_| (111, req("rifleman", "us_rifleman_1", SPAWN_HQ, 0))
    );
    // --- spawn point ---
    case!(
        "unassigned at a garrison",
        DeployError::UnassignedNeedsHq,
        |f| (
            20,
            DeployReq {
                spawn_point: f.g_us,
                ..rifle(0)
            }
        )
    );
    case!("no such garrison", DeployError::NoSuchPoint, |_| (
        11,
        DeployReq {
            spawn_point: 400,
            ..rifle(1)
        }
    ));
    case!("enemy garrison", DeployError::PointWrongTeam, |f| {
        f.w.debug_teleport(1, Vec3::new(1500.0, 5.02, 60.0));
        step_all(&mut f.w, &[]);
        let gid = f.w.place_garrison(1, Vec3::new(1500.0, 5.0, 60.0)).unwrap();
        (
            11,
            DeployReq {
                spawn_point: gid,
                ..rifle(1)
            },
        )
    });
    case!(
        "garrison still under construction",
        DeployError::PointNotActive,
        |f| {
            f.w.debug_teleport(0, Vec3::new(800.0, 5.02, 60.0));
            step_all(&mut f.w, &[]);
            let gid = f.w.place_garrison(0, Vec3::new(800.0, 5.0, 60.0)).unwrap();
            (
                11,
                DeployReq {
                    spawn_point: gid,
                    ..rifle(1)
                },
            )
        }
    );
    case!(
        "enemy next to the garrison",
        DeployError::PointBlocked,
        |f| {
            f.w.debug_teleport(1, Vec3::new(510.0, 5.02, 60.0));
            step_all(&mut f.w, &[]);
            (
                11,
                DeployReq {
                    spawn_point: f.g_us,
                    ..rifle(1)
                },
            )
        }
    );
    case!("garrison out of slots", DeployError::PointNoSlots, |f| {
        f.w.debug_set_garrison_slots(f.g_us, 0.5);
        (
            11,
            DeployReq {
                spawn_point: f.g_us,
                ..rifle(1)
            },
        )
    });
    // --- cooldown ---
    case!("role change inside 30 s", DeployError::RoleCooldown, |f| {
        f.w.set_role(11, Some("rifleman")).unwrap();
        f.w.set_role(11, Some("medic")).unwrap();
        (11, req("rifleman", "us_rifleman_1", SPAWN_HQ, 1))
    });
    let n = cases.len();
    assert!(n >= 30, "{n} cases");
    let mut seen = std::collections::BTreeSet::new();
    for (name, setup, want) in &cases {
        let mut f = fx();
        let (p, r) = setup(&mut f);
        // The setup may have taken a body down: make a dead player's state irrelevant by
        // validating as-is.
        assert_eq!(f.w.validate_deploy(p, &r).map(|_| ()), Err(*want), "{name}");
        seen.insert(*want);
    }
    // No-HQ world.
    let mut f = fx_with(d, false, false);
    f.w.set_role(11, None).unwrap();
    assert_eq!(f.w.validate_deploy(11, &rifle(1)), Err(DeployError::NoHq));
    seen.insert(DeployError::NoHq);
    // Locked loadout: one loadout above the M1 unlock level.
    let locked = {
        let e = GameData::embedded();
        let mut lo: Vec<_> = e.loadouts.to_vec();
        let i = lo.iter().position(|l| l.id == "us_rifleman_1").unwrap();
        lo[i].unlock_level = 4;
        Box::leak(Box::new(GameData {
            weapons: e.weapons,
            explosives: e.explosives,
            melee: e.melee,
            ammo: e.ammo,
            materials: e.materials,
            gadgets: e.gadgets,
            roles: e.roles,
            loadouts: Box::leak(lo.into_boxed_slice()),
            vehicles: e.vehicles,
            rules: e.rules,
            comms: e.comms,
            hash: e.hash,
        })) as &'static GameData
    };
    let f = fx_with(locked, false, true);
    assert_eq!(
        f.w.validate_deploy(11, &rifle(1)),
        Err(DeployError::LoadoutLocked)
    );
    seen.insert(DeployError::LoadoutLocked);
    // Team-wide Medic cap: 6 squads x 2 medics = 12; the 13th is refused.
    let mut f = fx();
    let mut next = 30u8;
    for _ in 0..7 {
        let (a, b) = (next, next + 1);
        next += 2;
        for p in [a, b] {
            f.w.roster_add(p, Team::Us).unwrap();
            f.w.roster_add(p + 100, Team::Ger).unwrap();
        }
        f.w.squad_op(
            a,
            SquadOp::Create {
                kind: SquadKind::Infantry,
                suffix: String::new(),
            },
        )
        .unwrap();
        let s = f.w.squads().squad_of(a).unwrap().id;
        f.w.squad_op(b, SquadOp::Join { squad: s }).unwrap();
        if next <= 42 {
            f.w.set_role(a, Some("medic")).unwrap();
            f.w.set_role(b, Some("medic")).unwrap();
        } else {
            assert_eq!(
                f.w.validate_deploy(a, &req("medic", "us_medic_1", SPAWN_HQ, s as u8))
                    .map(|_| ()),
                Err(DeployError::RoleTeamFull)
            );
        }
    }
    seen.insert(DeployError::RoleTeamFull);
    // Every error variant has a case, and the codes are unique and 1-based.
    for e in DeployError::ALL {
        assert!(seen.contains(&e), "no case for {e:?}");
    }
    let mut codes: Vec<u8> = DeployError::ALL.iter().map(|e| e.code()).collect();
    codes.sort_unstable();
    assert_eq!(
        codes,
        (1..=DeployError::ALL.len() as u8).collect::<Vec<_>>()
    );
}

#[test]
fn role_switch_cooldown_is_tick_exact() {
    let mut f = fx();
    f.w.set_role(11, Some("rifleman")).unwrap();
    f.w.set_role(11, Some("medic")).unwrap();
    let want = req("rifleman", "us_rifleman_1", SPAWN_HQ, 1);
    for _ in 0..(30 * HZ - 1) {
        f.w.step(&[]);
    }
    assert_eq!(
        f.w.validate_deploy(11, &want).map(|_| ()),
        Err(DeployError::RoleCooldown),
        "29.97 s"
    );
    f.w.step(&[]);
    assert!(f.w.validate_deploy(11, &want).is_ok(), "30 s");
    // Re-deploying the same role is never a switch.
    let same = req("medic", "us_medic_1", SPAWN_HQ, 1);
    let mut g = fx();
    g.w.set_role(11, Some("rifleman")).unwrap();
    g.w.set_role(11, Some("medic")).unwrap();
    assert!(g.w.validate_deploy(11, &same).is_ok());
}

#[test]
fn shrunk_squad_forces_a_new_role_on_the_next_deploy() {
    let mut f = fx();
    for p in 11..=13 {
        f.w.set_role(p, Some("rifleman")).unwrap();
    }
    f.w.set_role(14, Some("support")).unwrap();
    for p in [11, 12, 13, 15] {
        f.w.squad_op(p, SquadOp::Leave).unwrap();
    }
    let sup = req("support", "us_support_1", SPAWN_HQ, 1);
    assert_eq!(
        f.w.validate_deploy(14, &sup).map(|_| ()),
        Err(DeployError::SquadTooSmall)
    );
    assert_eq!(f.w.squads().entry(14).unwrap().role, Some("support"));
}

#[test]
fn headless_flow_squad_role_hq_spawn() {
    let col = synth_data(30, 6, &|_, _| 5.0);
    let mut w = duel_world(GameData::embedded(), &col, 7, &[]);
    w.set_hq(Team::Us, Vec3::new(60.0, 5.0, 60.0));
    w.set_hq(Team::Ger, Vec3::new(1860.0, 5.0, 60.0));
    for p in 0..3u8 {
        w.roster_add(p, Team::Us).unwrap();
        w.roster_add(p + 100, Team::Ger).unwrap();
    }
    w.squad_op(
        0,
        SquadOp::Create {
            kind: SquadKind::Infantry,
            suffix: String::new(),
        },
    )
    .unwrap();
    w.squad_op(1, SquadOp::Join { squad: 1 }).unwrap();
    w.squad_op(2, SquadOp::Join { squad: 1 }).unwrap();
    // Leader Officer, then a Medic and a Support (3-man squad).
    let done = w
        .deploy(0, &req("officer", "us_officer_1", SPAWN_HQ, 1))
        .unwrap();
    assert_eq!((done.role, done.spawn_in_ticks), ("officer", 0));
    w.deploy(1, &req("medic", "us_medic_1", SPAWN_HQ, 1))
        .unwrap();
    w.deploy(2, &req("support", "us_support_1", SPAWN_HQ, 1))
        .unwrap();
    for _ in 0..5 {
        w.step(&[]);
    }
    for p in 0..3 {
        assert!(
            matches!(w.player_life(p), Some(LifeStateView::Alive)),
            "{p}"
        );
    }
    // Bodies stand at the HQ; a second deploy of a living player is refused.
    let v = w.view();
    let pos = v.players().iter().find(|q| q.player == 1).unwrap().pos;
    assert!((pos.x - 60.0).hypot(pos.z - 60.0) < 1.0);
    assert_eq!(
        w.deploy(1, &req("medic", "us_medic_1", SPAWN_HQ, 1)),
        Err(DeployError::NotDeployable)
    );
    // The Officer role gives the Garrison ability; a Rifleman has none.
    assert!(w
        .squads()
        .can(0, frontline_sim::squad::Action::PlaceGarrison));
    assert!(!w
        .squads()
        .can(1, frontline_sim::squad::Action::PlaceGarrison));
    // Death then redeploy as Rifleman is refused inside the 30 s role-switch window.
    w.debug_kill(1);
    w.step(&[]);
    assert_eq!(
        w.deploy(1, &req("rifleman", "us_rifleman_1", SPAWN_HQ, 1)),
        Err(DeployError::RoleCooldown)
    );
    // Same role is fine.
    assert!(w
        .deploy(1, &req("medic", "us_medic_1", SPAWN_HQ, 1))
        .is_ok());
}

#[test]
fn deploy_to_garrison_returns_the_wait() {
    let mut f = fx();
    f.w.debug_kill(0);
    f.w.step(&[]);
    f.w.squad_op(
        0,
        SquadOp::Create {
            kind: SquadKind::Infantry,
            suffix: String::new(),
        },
    )
    .unwrap();
    let sq = f.w.squads().squad_of(0).unwrap().id as u8;
    let done =
        f.w.deploy(
            0,
            &DeployReq {
                spawn_point: f.g_us,
                ..req("rifleman", "us_rifleman_1", SPAWN_HQ, sq)
            },
        )
        .unwrap();
    assert_eq!(done.spawn_in_ticks, 8 * HZ);
}
