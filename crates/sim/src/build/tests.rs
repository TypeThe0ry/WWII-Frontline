use super::repair_station::{RepairStationDef, RestockError};
use super::*;
use crate::data::GameData;
use crate::ids::MAX_SLOTS;
use crate::supply::{Carrying, SupplyCrate};
use crate::vehicle::modules::ModuleId;
use crate::vehicle::{Vehicle, Vehicles};
use glam::Quat;

const HZ: u8 = 30;
const STATION: StructureKind = StructureKind::RepairStation;
const AT: StructureKind = StructureKind::AtGun;

fn station_def() -> RepairStationDef {
    serde_json::from_str(include_str!(
        "../../../../data/structures/repair_station.json"
    ))
    .unwrap()
}

fn at_def() -> ConstructionDef {
    ConstructionDef {
        role: "anti_tank".into(),
        cost_supplies: 200,
        build_s: 20.0,
        sector_cap: Some(1),
        ..station_def().construction
    }
}

fn site(sector: u16, x: f32) -> BuildSite {
    BuildSite {
        sector,
        pos: Vec3::X * x,
        friendly_or_contested: true,
        ..BuildSite::default()
    }
}

fn actor(id: PlayerId, role: &str, pos: Vec3) -> BuildActor<'_> {
    BuildActor {
        id,
        team: Team::Us,
        role,
        pos,
    }
}

fn stock(
    sw: &mut SupplyWorld,
    id: u32,
    team: Team,
    x: f32,
    supplies: u32,
    carrier: Option<PlayerId>,
) {
    sw.crates.insert(
        id,
        SupplyCrate {
            id,
            team,
            pos: Vec3::X * x,
            supplies,
            hp: 150.0,
            carried_by: carrier,
            seq: id,
        },
    );
}

#[derive(Default)]
struct Fixture {
    builds: BuildSystem,
    supplies: SupplyWorld,
    alloc: EntityAllocator,
}

impl Fixture {
    fn place(
        &mut self,
        kind: StructureKind,
        b: BuildActor<'_>,
        site: BuildSite,
        def: &ConstructionDef,
    ) -> Result<EntityId, BuildError> {
        self.builds
            .start_build(b, kind, site, def, HZ, &mut self.supplies, &mut self.alloc)
    }

    fn station_under_construction(&mut self, def: &RepairStationDef) -> EntityId {
        stock(&mut self.supplies, 1, Team::Us, 0.0, 100, None);
        stock(&mut self.supplies, 2, Team::Us, 0.0, 100, None);
        self.place(
            STATION,
            actor(0, "engineer", Vec3::ZERO),
            site(1, 0.0),
            &def.construction,
        )
        .unwrap()
    }

    fn built_station(&mut self, def: &RepairStationDef) -> EntityId {
        let id = self.station_under_construction(def);
        for _ in 0..449 {
            assert!(self
                .builds
                .tick_build(|_, _, _| Some(Vec3::ZERO))
                .is_empty());
        }
        assert_eq!(self.builds.tick_build(|_, _, _| Some(Vec3::ZERO)), [id]);
        id
    }

    fn damage(&mut self, id: EntityId, kind: StructureDamage, amount: f32) -> DamageOutcome {
        self.builds
            .structure_damage(id, kind, amount, &mut self.alloc)
    }

    fn supply(&self, id: EntityId) -> u32 {
        self.builds
            .get(id)
            .unwrap()
            .repair_station()
            .unwrap()
            .supplies()
    }
}

#[test]
fn construction_pays_friendly_ground_crates_nearest_first_atomically() {
    let def = station_def();
    let mut f = Fixture::default();
    let at = site(1, 0.0);
    let engineer = actor(0, "engineer", at.pos);
    for (id, team, x, amount, carried) in [
        (1, Team::Us, 15.01, 100, None),
        (2, Team::Ger, 0.0, 100, None),
        (3, Team::Us, 0.0, 100, Some(5)),
        (9, Team::Us, 2.0, 70, None),
    ] {
        stock(&mut f.supplies, id, team, x, amount, carried);
    }
    let before = f.supplies.crates.clone();
    assert_eq!(
        f.place(STATION, engineer, at, &def.construction),
        Err(BuildError::NoSupplies)
    );
    assert_eq!(f.supplies.crates, before);
    assert_eq!(f.builds.structures().count(), 0);
    stock(&mut f.supplies, 8, Team::Us, -2.0, 50, None);
    stock(&mut f.supplies, 7, Team::Us, 15.0, 100, None);
    let id = f.place(STATION, engineer, at, &def.construction).unwrap();
    assert_eq!(id, 0);
    assert!(!f.supplies.crates.contains_key(&8));
    assert!(!f.supplies.crates.contains_key(&9));
    assert_eq!(f.supplies.crates[&7].supplies, 100);
    for unaffected in [1, 2, 3] {
        assert_eq!(f.supplies.crates[&unaffected], before[&unaffected]);
    }

    let mut tie = Fixture::default();
    stock(&mut tie.supplies, 8, Team::Us, -2.0, 100, None);
    stock(&mut tie.supplies, 9, Team::Us, 2.0, 100, None);
    tie.place(STATION, engineer, at, &def.construction).unwrap();
    assert!(!tie.supplies.crates.contains_key(&8));
    assert_eq!(tie.supplies.crates[&9].supplies, 80);

    let mut limit = Fixture::default();
    stock(&mut limit.supplies, 7, Team::Us, 15.0, 120, None);
    for _ in 0..MAX_SLOTS {
        limit.alloc.alloc().unwrap();
    }
    assert_eq!(
        limit.place(STATION, engineer, at, &def.construction),
        Err(BuildError::NoEntitySlot)
    );
    assert_eq!(limit.supplies.crates[&7].supplies, 120);
}

#[test]
fn at_gun_build_permission_and_team_sector_caps() {
    let mut f = Fixture::default();
    let def = at_def();
    let start = site(1, 0.0);
    let anti_tank = actor(0, "anti_tank", start.pos);
    stock(&mut f.supplies, 1, Team::Us, 0.0, 1000, None);
    assert_eq!(
        f.place(AT, actor(9, "engineer", start.pos), start, &def),
        Err(BuildError::WrongRole)
    );
    let first = f.place(AT, anti_tank, start, &def).unwrap();
    assert_eq!(
        f.place(AT, actor(1, "anti_tank", Vec3::X * 4.0), site(1, 4.0), &def),
        Err(BuildError::SectorCapReached)
    );
    for i in [2, 3] {
        let at = site(u16::from(i), f32::from(i) * 4.0);
        f.place(AT, actor(i, "anti_tank", at.pos), at, &def)
            .unwrap();
    }
    let next = site(4, 12.0);
    let builder = actor(4, "anti_tank", next.pos);
    assert_eq!(
        f.place(AT, builder, next, &def),
        Err(BuildError::TypeCapReached)
    );
    assert_eq!(f.supplies.crates[&1].supplies, 400);
    assert_eq!(
        f.damage(first, StructureDamage::Explosion, 120.0),
        DamageOutcome::Destroyed
    );
    assert!(!f.alloc.is_alive(first));
    let replacement = f.place(AT, builder, next, &def).unwrap();
    assert_ne!(first, replacement);
}

#[test]
fn station_type_cap_and_combined_team_cap_include_unfinished_sites() {
    let mut f = Fixture::default();
    let mut def = station_def().construction;
    stock(&mut f.supplies, 1, Team::Us, 0.0, 1000, None);
    for i in 0..3 {
        let at = site(1, f32::from(i) * 4.0);
        f.place(STATION, actor(i, "engineer", at.pos), at, &def)
            .unwrap();
    }
    let at = site(1, 12.0);
    assert_eq!(
        f.place(STATION, actor(3, "engineer", at.pos), at, &def),
        Err(BuildError::TypeCapReached)
    );

    let mut f = Fixture::default();
    let mut gun = at_def();
    gun.cost_supplies = 0;
    def.cost_supplies = 0;
    def.team_cap = TEAM_BUILD_CAP;
    f.place(AT, actor(0, "anti_tank", Vec3::ZERO), site(1, 0.0), &gun)
        .unwrap();
    for i in 1..TEAM_BUILD_CAP as u8 {
        let at = site(1, f32::from(i) * 4.0);
        f.place(STATION, actor(i, "engineer", at.pos), at, &def)
            .unwrap();
    }
    let at = site(1, 1000.0);
    let b = actor(120, "engineer", at.pos);
    assert_eq!(
        f.place(STATION, b, at, &def),
        Err(BuildError::TeamCapReached)
    );
    f.place(
        STATION,
        BuildActor {
            team: Team::Ger,
            ..b
        },
        at,
        &def,
    )
    .unwrap();
}

#[test]
fn construction_ticks_pause_resume_and_preserve_damage() {
    let def = station_def();
    let mut f = Fixture::default();
    let id = f.station_under_construction(&def);
    assert_eq!(f.builds.get(id).unwrap().hp, 120.0);
    for _ in 0..20 {
        assert!(f.builds.tick_build(|_, _, _| None).is_empty());
        assert!(f
            .builds
            .tick_build(|_, _, _| Some(Vec3::X * 3.01))
            .is_empty());
    }
    assert_eq!(f.builds.get(id).unwrap().progress(), 0.0);
    assert_eq!(
        f.damage(id, StructureDamage::Bullet, 100.0),
        DamageOutcome::Damaged
    );
    assert_eq!(f.builds.get(id).unwrap().hp, 118.0);
    for i in 1..=2 {
        f.builds
            .join_build(id, actor(i, "engineer", Vec3::X * 3.0))
            .unwrap();
    }
    assert_eq!(
        f.builds.join_build(id, actor(3, "engineer", Vec3::ZERO)),
        Err(BuildError::BuilderCapReached)
    );
    assert!(f.builds.cancel_build(id, 2));
    assert!(!f.builds.cancel_build(id, 2));
    f.builds
        .join_build(id, actor(2, "engineer", Vec3::ZERO))
        .unwrap();
    for _ in 0..204 {
        assert!(f.builds.tick_build(|_, _, _| Some(Vec3::ZERO)).is_empty());
    }
    assert_eq!(f.builds.tick_build(|_, _, _| Some(Vec3::ZERO)), [id]);
    let built = f.builds.get(id).unwrap();
    assert_eq!(built.hp, 598.0);
    assert_eq!(built.state, BuildState::Done);
    assert_eq!(built.builders().count(), 0);
    assert!(f.builds.tick_build(|_, _, _| Some(Vec3::ZERO)).is_empty());
    assert_eq!(
        f.damage(id, StructureDamage::He, 100.0),
        DamageOutcome::Damaged
    );
    assert_eq!(f.builds.get(id).unwrap().hp, 528.0);
    assert_eq!(
        f.damage(id, StructureDamage::Heat, 100.0),
        DamageOutcome::Damaged
    );
    assert_eq!(f.builds.get(id).unwrap().hp, 428.0);
}

fn tank(id: EntityId, team: Team, x: f32, hp: f32) -> Vehicle {
    let mut v = Vehicle::from_def(
        GameData::embedded(),
        "us_m4a1_75",
        id,
        team,
        Vec3::X * x,
        Quat::IDENTITY,
    )
    .unwrap();
    assert_eq!(v.params.hull_hp, 1600.0);
    v.hull_hp = hp;
    v
}

#[test]
fn repair_station_heal_rate_inventory_range_and_destroyed_modules() {
    let def = station_def();
    let mut f = Fixture::default();
    let mut vehicles = Vehicles::default();
    let mut v = tank(11, Team::Us, 8.0, 1000.0);
    let engine = ModuleId::Engine as usize;
    v.modules.hp[engine] = 0.0;
    vehicles.list.insert(11, v);
    vehicles.list.insert(12, tank(12, Team::Ger, 2.0, 1000.0));
    vehicles.list.insert(13, tank(13, Team::Us, 8.01, 1000.0));
    let mut wreck = tank(14, Team::Us, 0.0, 1000.0);
    wreck.knocked_out = true;
    vehicles.list.insert(14, wreck);
    let id = f.station_under_construction(&def);
    f.builds
        .tick_repair_stations(def.service, HZ, &mut vehicles);
    assert_eq!(vehicles.list[&11].hull_hp, 1000.0);
    for _ in 0..450 {
        f.builds.tick_build(|_, _, _| Some(Vec3::ZERO));
    }
    for _ in 0..30 {
        f.builds
            .tick_repair_stations(def.service, HZ, &mut vehicles);
    }
    let healed_first_second = vehicles.list[&11].hull_hp - 1000.0;
    assert!(
        (healed_first_second - 20.0).abs() < 0.4,
        "{healed_first_second}"
    );
    assert_eq!(f.supply(id), 196);
    for _ in 30..910 {
        f.builds
            .tick_repair_stations(def.service, HZ, &mut vehicles);
    }
    assert_eq!(vehicles.list[&11].hull_hp, 1600.0);
    assert_eq!(vehicles.list[&11].modules.hp[engine], 0.0);
    assert_eq!(f.supply(id), 80);
    for out in [12, 13, 14] {
        assert_eq!(vehicles.list[&out].hull_hp, 1000.0);
    }

    let mut v = tank(11, Team::Us, 8.0, 100.0);
    v.modules.hp[engine] = 0.0;
    vehicles.list.insert(11, v);
    let mut full_stock = Fixture::default();
    let station = full_stock.built_station(&def);
    for _ in 0..1600 {
        full_stock
            .builds
            .tick_repair_stations(def.service, HZ, &mut vehicles);
    }
    let healed_until_exhaustion = vehicles.list[&11].hull_hp - 100.0;
    assert_eq!(healed_until_exhaustion, 1000.0);
    assert_eq!(full_stock.supply(station), 0);
    assert_eq!(vehicles.list[&11].modules.hp[engine], 0.0);
    eprintln!(
        "first-second hull repair {healed_first_second} HP, exhausted stock after {healed_until_exhaustion} HP"
    );
}

#[test]
fn repair_station_refills_one_round_per_interval_and_repairs_intact_modules() {
    let def = station_def();
    let mut f = Fixture::default();
    let station = f.built_station(&def);
    let mut vehicles = Vehicles::default();
    let mut first = tank(11, Team::Us, 2.0, 1600.0);
    let track = ModuleId::TrackR as usize;
    let track_hp = first.modules.hp[track];
    first.modules.hp[track] -= 5.0;
    let full = first.gun.ammo[0];
    first.gun.ammo[0] -= 1;
    let mut second = tank(12, Team::Us, 2.0, 1600.0);
    second.gun.ammo[0] -= 1;
    vehicles.list.insert(12, second);
    vehicles.list.insert(11, first);
    for _ in 0..59 {
        f.builds
            .tick_repair_stations(def.service, HZ, &mut vehicles);
    }
    assert_eq!(f.supply(station), 199);
    assert_eq!(vehicles.list[&11].modules.hp[track], track_hp);
    assert_eq!(vehicles.list[&11].gun.ammo[0], full - 1);
    f.builds
        .tick_repair_stations(def.service, HZ, &mut vehicles);
    assert_eq!(vehicles.list[&11].gun.ammo[0], full);
    assert_eq!(vehicles.list[&12].gun.ammo[0], full - 1);
    assert_eq!(f.supply(station), 195);
    for _ in 0..60 {
        f.builds
            .tick_repair_stations(def.service, HZ, &mut vehicles);
    }
    assert_eq!(vehicles.list[&12].gun.ammo[0], full);
    assert_eq!(f.supply(station), 191);
}

#[test]
fn repair_station_restock_transfers_the_actual_support_crate_once() {
    let def = station_def();
    let mut f = Fixture::default();
    let station = f.built_station(&def);
    let support = actor(2, "support", Vec3::X * 2.0);
    let mut carrying = Carrying(Some(5));
    stock(&mut f.supplies, 5, Team::Us, 0.0, 100, Some(support.id));
    assert_eq!(
        f.builds.restock_repair_station(
            station,
            BuildActor {
                role: "engineer",
                ..support
            },
            def.service,
            &mut f.supplies,
            &mut carrying
        ),
        Err(RestockError::WrongRole)
    );
    assert_eq!(carrying.0, Some(5));
    assert_eq!(f.supplies.crates[&5].supplies, 100);
    assert_eq!(
        f.builds.restock_repair_station(
            station,
            support,
            def.service,
            &mut f.supplies,
            &mut carrying
        ),
        Ok(100)
    );
    assert_eq!(f.supply(station), 300);
    assert_eq!(carrying.0, None);
    assert!(!f.supplies.crates.contains_key(&5));
    assert_eq!(
        f.builds.restock_repair_station(
            station,
            support,
            def.service,
            &mut f.supplies,
            &mut carrying
        ),
        Err(RestockError::NoCrate)
    );
    stock(&mut f.supplies, 6, Team::Us, 0.0, 33, Some(support.id));
    carrying.0 = Some(6);
    assert_eq!(
        f.builds.restock_repair_station(
            station,
            support,
            def.service,
            &mut f.supplies,
            &mut carrying
        ),
        Ok(33)
    );
    assert_eq!(f.supply(station), 333);
}

#[test]
fn build_module_state_hash_replays_with_stable_crate_and_vehicle_order() {
    fn run(reverse: bool) -> (u64, u64) {
        let mut f = Fixture::default();
        let def = station_def();
        let mut vehicles = Vehicles::default();
        f.built_station(&def);
        for id in if reverse { [12, 11] } else { [11, 12] } {
            let mut v = tank(id, Team::Us, 8.0, 1000.0);
            v.gun.ammo[0] -= 1;
            vehicles.list.insert(id, v);
        }
        let mut before = HashWriter::new();
        f.builds.hash_into(&mut before);
        for _ in 0..61 {
            f.builds
                .tick_repair_stations(def.service, HZ, &mut vehicles);
        }
        let mut after = HashWriter::new();
        f.builds.hash_into(&mut after);
        vehicles.hash_into(&mut after);
        (before.finish(), after.finish())
    }
    assert_eq!(run(false), run(true));
    let (before, after) = run(false);
    assert_ne!(before, after);
    let mut empty = HashWriter::new();
    BuildSystem::default().hash_into(&mut empty);
    assert_eq!(empty.finish(), HashWriter::new().finish());
}
