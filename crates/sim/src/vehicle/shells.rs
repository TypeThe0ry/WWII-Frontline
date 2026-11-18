//! Vehicle shells in flight and their consequences (M1-023c): gravity-only flight, hits on
//! enemy vehicles through [`super::shell::resolve_ap_hit`], terrain bursts, explosions,
//! knock-out, ammunition cook-off and the burning wreck (GD 14.4, 14.5).

use bevy_ecs::prelude::*;
use glam::Vec3;

use super::armor::{ShellKind, ShellParams};
use super::shell::{self, ArmorKind, HitReport, HitRolls, ShellDef, ShellType};
use super::{Vehicle, Vehicles, GRAVITY};
use crate::combat::DamageKind;
use crate::components::{
    CollisionRes, CurrentTick, EntityTable, EventBuf, PlayerIndex, SimCfgRes, SimRng, Team,
    Transform,
};
use crate::hash::HashWriter;
use crate::health::{Health, HitLog, HitRec, LifeState};
use crate::ids::EntityId;
use crate::rng::{unit_f32, Purpose};
use crate::step::SimEvent;
use crate::throwables::smoke::{SmokeCloud, SmokeParams};
use crate::throwables::system::{explode, Smokes};
use crate::weapon::HitZone;

/// Flight sub-steps per tick.
pub const SHELL_SUBSTEPS: u32 = 3;
/// A shell that has flown this long is removed.
pub const SHELL_MAX_LIFE_S: f32 = 8.0;
/// Burning time before the wreck explodes (GD 14.5), seconds.
pub const BURN_S: f32 = 8.0;
pub const BURN_DAMAGE_HP_S: f32 = 20.0;
pub const BURN_EXPLOSION_R: f32 = 8.0;
pub const BURN_EXPLOSION_DMAX: f32 = 200.0;
pub const WRECK_S: f32 = 60.0;
pub const COOKOFF_R: f32 = 12.0;
pub const COOKOFF_DMAX: f32 = 350.0;
/// `SimEvent::Explosion.kind` values of this module.
pub const EXPL_HE: u8 = 1;
pub const EXPL_COOKOFF: u8 = 2;
pub const EXPL_BURNOUT: u8 = 3;
pub const EXPL_HEAT: u8 = 4;
/// `SimEvent::ArmorHit.result` values.
pub const RES_PEN: u8 = 0;
pub const RES_NOPEN: u8 = 1;
pub const RES_RICOCHET: u8 = 2;
/// `SimEvent::VehicleDamaged.module` of hull damage.
pub const MODULE_HULL: u8 = 0xFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warhead {
    Ap,
    He,
    Smoke,
    Heat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shell {
    pub id: u32,
    /// Vehicle (or player, for a rocket) that fired it.
    pub owner: EntityId,
    pub team: Team,
    pub warhead: Warhead,
    pub params: ShellParams,
    /// Blast at the burst point (`Dmax`, radius); zero for plain AP.
    pub blast_dmax: f32,
    pub blast_radius: f32,
    /// Smoke cloud (radius, seconds) for WP.
    pub smoke_radius: f32,
    pub smoke_s: f32,
    /// The shell is removed past this distance from `origin` (0 = unlimited).
    pub max_range_m: f32,
    pub origin: Vec3,
    pub pos: Vec3,
    pub vel: Vec3,
    pub substeps: u32,
}

#[derive(Resource, Debug, Default)]
pub struct Shells {
    pub list: Vec<Shell>,
    pub next_id: u32,
}

impl Shells {
    pub fn push(&mut self, mut s: Shell) -> u32 {
        s.id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.list.push(s);
        s.id
    }

    pub fn hash_into(&self, h: &mut HashWriter) {
        h.write_u8(0xF2);
        h.write_u32(self.next_id);
        for s in &self.list {
            h.write_u32(s.id);
            for f in [s.pos.x, s.pos.y, s.pos.z, s.vel.x, s.vel.y, s.vel.z] {
                h.write_f32(f);
            }
        }
    }
}

/// Builds a shell from a vehicle shell definition.
pub fn shell_from_def(
    def: &ShellDef,
    owner: EntityId,
    team: Team,
    origin: Vec3,
    dir: Vec3,
) -> Shell {
    let (warhead, params) = match def.kind {
        ShellType::Ap => (Warhead::Ap, shell::ap_params(def)),
        // HE meets armour with a 20% of calibre penetration (GD 14.4.1).
        ShellType::He => (
            Warhead::He,
            ShellParams {
                kind: ShellKind::Ap,
                caliber_mm: def.caliber_mm,
                p100_mm: def.he_pen_mm,
                drop_pct_per_100m: 0.0,
                structure_damage_hp: def.explosion_damage_max_hp,
            },
        ),
        ShellType::Smoke => (
            Warhead::Smoke,
            ShellParams {
                kind: ShellKind::Ap,
                caliber_mm: def.caliber_mm,
                p100_mm: 0.0,
                drop_pct_per_100m: 0.0,
                structure_damage_hp: 0.0,
            },
        ),
    };
    Shell {
        id: 0,
        owner,
        team,
        warhead,
        params,
        blast_dmax: if warhead == Warhead::He {
            def.explosion_damage_max_hp
        } else {
            0.0
        },
        blast_radius: def.explosion_radius_m,
        smoke_radius: def.cloud_radius_m,
        smoke_s: def.duration_s,
        max_range_m: 0.0,
        origin,
        pos: origin,
        vel: dir * def.muzzle_velocity_mps,
        substeps: 0,
    }
}

fn crew_alive(
    v: &Vehicle,
    index: &PlayerIndex,
    q: &Query<(&Transform, &Team, &LifeState, &mut Health)>,
) -> [bool; 3] {
    let mut out = [false; 3];
    for (s, slot) in v.seats.iter().enumerate().take(3) {
        if let Some(o) = slot {
            if let Some(&(_, e)) = index.map.get(&o.player) {
                if let Ok((_, _, life, hp)) = q.get(e) {
                    out[s] = life.is_alive() && hp.hp > 0.0;
                }
            }
        }
    }
    out
}

/// Applies `dmg` to the crewman in `seat`; emits `CrewKilled` when it kills.
#[allow(clippy::too_many_arguments)]
fn hurt_crew(
    v: &Vehicle,
    seat: usize,
    dmg: f32,
    shooter: EntityId,
    index: &PlayerIndex,
    q: &mut Query<(&Transform, &Team, &LifeState, &mut Health)>,
    events: &mut Vec<SimEvent>,
    log: &mut Vec<HitRec>,
) {
    let Some(Some(o)) = v.seats.get(seat) else {
        return;
    };
    let Some(&(eid, e)) = index.map.get(&o.player) else {
        return;
    };
    let Ok((_, _, life, mut hp)) = q.get_mut(e) else {
        return;
    };
    if life.is_dead() || hp.hp <= 0.0 || dmg <= 0.0 {
        return;
    }
    let before = hp.hp;
    hp.hp -= dmg;
    log.push(HitRec {
        target: eid,
        shooter,
        weapon: crate::throwables::EXPLOSIVE_WEAPON_IDX,
        zone: HitZone::Torso,
        dist_m: 0.0,
        damage: dmg.min(before),
        kind: DamageKind::Tank,
        direct_kia: false,
    });
    if hp.hp <= 0.0 {
        events.push(SimEvent::CrewKilled {
            player: o.player,
            vehicle: v.id,
        });
    }
}

/// Puts the vehicle out of action: no driving, burning (or already exploded) wreck.
fn knock_out(v: &mut Vehicle, hz: u8, cookoff: bool, events: &mut Vec<SimEvent>) {
    if v.knocked_out {
        return;
    }
    v.knocked_out = true;
    events.push(SimEvent::VehicleKnockedOut { vehicle: v.id });
    if cookoff {
        v.wreck_left = (WRECK_S * f32::from(hz)) as u32;
    } else {
        v.burn_left = (BURN_S * f32::from(hz)) as u32;
    }
}

/// Applies a resolved hit: events, crew damage, knock-out and cook-off.
#[allow(clippy::too_many_arguments)]
fn apply_report(
    v: &mut Vehicle,
    rep: &HitReport,
    shooter: EntityId,
    hz: u8,
    index: &PlayerIndex,
    q: &mut Query<(&Transform, &Team, &LifeState, &mut Health)>,
    table: &EntityTable,
    col: &crate::collision::CollisionData,
    events: &mut Vec<SimEvent>,
    log: &mut Vec<HitRec>,
) {
    let result = match rep.armor {
        Some(ArmorKind::Pen) => RES_PEN,
        Some(ArmorKind::NoPen) => RES_NOPEN,
        _ => RES_RICOCHET,
    };
    events.push(SimEvent::ArmorHit {
        vehicle: v.id,
        zone: rep.zone as u8,
        result,
    });
    if rep.hull_damage > 0.0 {
        events.push(SimEvent::VehicleDamaged {
            vehicle: v.id,
            module: MODULE_HULL,
            hp: v.hull_hp.max(0.0),
        });
    }
    for id in super::modules::ModuleId::ALL {
        if rep.module_damage[id as usize] > 0.0 {
            events.push(SimEvent::VehicleDamaged {
                vehicle: v.id,
                module: id as u8,
                hp: v.modules.hp[id as usize],
            });
        }
    }
    for (s, d) in rep.crew_damage.iter().enumerate() {
        hurt_crew(v, s, *d, shooter, index, q, events, log);
    }
    if v.knocked_out {
        return;
    }
    if rep.cookoff {
        events.push(SimEvent::VehicleAmmoExplosion { vehicle: v.id });
        for s in 0..v.seats.len() {
            hurt_crew(v, s, 1.0e4, shooter, index, q, events, log);
        }
        knock_out(v, hz, true, events);
        explode(
            v.pos + Vec3::Y,
            COOKOFF_DMAX,
            COOKOFF_R,
            EXPL_COOKOFF,
            shooter,
            v.team,
            col,
            table,
            q,
            events,
            log,
        );
        v.hull_hp = 0.0;
    } else if v.hull_hp <= 0.0 || rep.engine_fire {
        knock_out(v, hz, false, events);
    }
}

type Victims<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static Team,
        &'static LifeState,
        &'static mut Health,
    ),
>;

/// Burning and wreck timers (per vehicle, per tick).
#[allow(clippy::too_many_arguments)]
fn wreck_step(
    v: &mut Vehicle,
    hz: u8,
    index: &PlayerIndex,
    q: &mut Victims,
    table: &EntityTable,
    col: &crate::collision::CollisionData,
    events: &mut Vec<SimEvent>,
    log: &mut Vec<HitRec>,
) {
    if v.burn_left > 0 {
        let d = BURN_DAMAGE_HP_S / f32::from(hz);
        for s in 0..v.seats.len() {
            hurt_crew(v, s, d, v.id, index, q, events, log);
        }
        v.burn_left -= 1;
        if v.burn_left == 0 {
            explode(
                v.pos + Vec3::Y,
                BURN_EXPLOSION_DMAX,
                BURN_EXPLOSION_R,
                EXPL_BURNOUT,
                v.id,
                v.team,
                col,
                table,
                q,
                events,
                log,
            );
            v.wreck_left = (WRECK_S * f32::from(hz)) as u32;
        }
    } else if v.wreck_left > 0 {
        v.wreck_left -= 1;
        if v.wreck_left == 0 {
            v.gone = true;
        }
    }
}

/// The shell stage: wreck timers, then every shell flies one tick.
#[allow(clippy::too_many_arguments)]
pub fn shell_flight(
    cfg: Res<SimCfgRes>,
    col: Res<CollisionRes>,
    tick: Res<CurrentTick>,
    index: Res<PlayerIndex>,
    table: Res<EntityTable>,
    mut rng: ResMut<SimRng>,
    mut shells: ResMut<Shells>,
    mut vehicles: ResMut<Vehicles>,
    mut smokes: ResMut<Smokes>,
    mut events: ResMut<EventBuf>,
    mut log: ResMut<HitLog>,
    mut q: Victims,
) {
    let hz = cfg.hz;
    for v in vehicles.list.values_mut() {
        wreck_step(
            v,
            hz,
            &index,
            &mut q,
            &table,
            &col.0,
            &mut events.0,
            &mut log.0,
        );
    }
    if shells.list.is_empty() {
        return;
    }
    let h = 1.0 / (f32::from(hz) * SHELL_SUBSTEPS as f32);
    let max_sub = (SHELL_MAX_LIFE_S * f32::from(hz)) as u32 * SHELL_SUBSTEPS;
    let list = std::mem::take(&mut shells.list);
    let mut keep = Vec::with_capacity(list.len());
    'shells: for mut s in list {
        for _ in 0..SHELL_SUBSTEPS {
            s.vel.y -= GRAVITY * h;
            let seg = s.vel * h;
            let len = seg.length();
            s.substeps += 1;
            if len <= 1e-6
                || s.substeps > max_sub
                || (s.max_range_m > 0.0 && (s.pos - s.origin).length() > s.max_range_m)
            {
                continue 'shells;
            }
            let dir = seg / len;
            let terrain = col.0.raycast(s.pos, dir, len);
            // Nearest enemy vehicle along the segment.
            let mut best: Option<(f32, EntityId)> = None;
            for v in vehicles.list.values() {
                if v.id == s.owner || v.team == s.team {
                    continue;
                }
                if let Some(l) = shell::locate(v, s.pos, dir, len) {
                    if best.is_none_or(|(t, _)| l.t < t) {
                        best = Some((l.t, v.id));
                    }
                }
            }
            let t_terr = terrain.map(|r| r.t);
            match (best, t_terr) {
                (Some((tv, vid)), tt) if tt.is_none_or(|tt| tv <= tt) => {
                    let at = s.pos + dir * tv;
                    let dist = (at - s.origin).length();
                    let mut r = rng.stream(tick.0, s.owner, Purpose::Armor);
                    let rolls = HitRolls {
                        armor: unit_f32(&mut r),
                        damage: unit_f32(&mut r),
                        cookoff: unit_f32(&mut r),
                        fire: unit_f32(&mut r),
                    };
                    if let Some(v) = vehicles.list.get_mut(&vid) {
                        let alive = crew_alive(v, &index, &q);
                        let params = s.params;
                        if let Some(rep) =
                            shell::resolve_ap_hit(v, &params, s.pos, dir, dist, alive, rolls, len)
                        {
                            apply_report(
                                v,
                                &rep,
                                s.owner,
                                hz,
                                &index,
                                &mut q,
                                &table,
                                &col.0,
                                &mut events.0,
                                &mut log.0,
                            );
                        }
                    }
                    burst(
                        &s,
                        at,
                        &col.0,
                        &table,
                        &mut q,
                        &mut events.0,
                        &mut log.0,
                        &mut smokes,
                        tick.0,
                        hz,
                    );
                    continue 'shells;
                }
                (_, Some(tt)) => {
                    let at = s.pos + dir * tt;
                    burst(
                        &s,
                        at,
                        &col.0,
                        &table,
                        &mut q,
                        &mut events.0,
                        &mut log.0,
                        &mut smokes,
                        tick.0,
                        hz,
                    );
                    continue 'shells;
                }
                _ => {}
            }
            s.pos += seg;
        }
        keep.push(s);
    }
    shells.list = keep;
}

/// The warhead goes off at `at` (HE/HEAT blast, WP cloud; plain AP does nothing).
#[allow(clippy::too_many_arguments)]
fn burst(
    s: &Shell,
    at: Vec3,
    col: &crate::collision::CollisionData,
    table: &EntityTable,
    q: &mut Victims,
    events: &mut Vec<SimEvent>,
    log: &mut Vec<HitRec>,
    smokes: &mut Smokes,
    now: crate::ids::Tick,
    hz: u8,
) {
    match s.warhead {
        Warhead::He | Warhead::Heat => {
            let kind = if s.warhead == Warhead::He {
                EXPL_HE
            } else {
                EXPL_HEAT
            };
            explode(
                at,
                s.blast_dmax,
                s.blast_radius,
                kind,
                s.owner,
                s.team,
                col,
                table,
                q,
                events,
                log,
            );
        }
        Warhead::Smoke => {
            let t = |sec: f32| (sec * f32::from(hz)) as u32;
            smokes.clouds.push(SmokeCloud {
                pos: at,
                start: now,
                params: SmokeParams {
                    radius_m: s.smoke_radius,
                    expand_ticks: t(3.0),
                    duration_ticks: t(s.smoke_s),
                    decay_ticks: t(8.0),
                },
            });
            events.push(SimEvent::SmokeStarted {
                pos: at,
                radius_m: s.smoke_radius,
            });
        }
        Warhead::Ap => {}
    }
}

fn wrap_pi(a: f32) -> f32 {
    let tau = core::f32::consts::TAU;
    let mut x = a % tau;
    if x > core::f32::consts::PI {
        x -= tau;
    } else if x <= -core::f32::consts::PI {
        x += tau;
    }
    x
}

/// Muzzle distance from the hull centre along the barrel, metres.
pub const MUZZLE_M: f32 = 3.2;
/// Muzzle height above the hull centre, metres.
pub const MUZZLE_Y: f32 = 0.85;

/// What the gunner asks of the gun this tick.
#[derive(Debug, Clone, Copy)]
pub struct GunnerInput {
    pub yaw: f32,
    pub pitch: f32,
    pub fire: bool,
    /// Cycle AP -> HE -> WP (acts on the rising edge).
    pub swap: bool,
}

/// Turret slew, reload clock, shell selection and firing for one vehicle and tick.
pub fn gunnery(
    v: &mut Vehicle,
    input: Option<GunnerInput>,
    commander_present: bool,
    hz: u8,
    shells: &mut Shells,
    events: &mut Vec<SimEvent>,
) {
    use super::gun::{ShellSlot, NO_COMMANDER_RELOAD_MULT};
    let fx = v.modules.effects(&v.params.module_specs);
    let dt = 1.0 / f32::from(hz);
    let hull_yaw = libm::atan2f(v.forward().x, v.forward().z);
    if let Some(i) = input {
        v.turret.step(
            &v.params.turret,
            wrap_pi(i.yaw - hull_yaw),
            i.pitch,
            fx.traverse_mult,
            fx.turret_jammed,
            dt,
        );
    }
    let mult = fx.reload_mult
        * if commander_present {
            1.0
        } else {
            NO_COMMANDER_RELOAD_MULT
        };
    v.gun.tick();
    let Some(i) = input else {
        v.swap_held = false;
        return;
    };
    if i.swap && !v.swap_held {
        let next = match v.gun.loaded {
            ShellSlot::Ap => ShellSlot::He,
            ShellSlot::He => ShellSlot::Wp,
            ShellSlot::Wp => ShellSlot::Ap,
        };
        v.gun.select(&v.params.gun, next, mult, f32::from(hz));
    }
    v.swap_held = i.swap;
    if !i.fire || !fx.can_fire {
        return;
    }
    let Ok(slot) = v.gun.fire(&v.params.gun, mult, f32::from(hz)) else {
        return;
    };
    let want = match slot {
        ShellSlot::Ap => ShellType::Ap,
        ShellSlot::He => ShellType::He,
        ShellSlot::Wp => ShellType::Smoke,
    };
    let Some(def) = v.params.shells.iter().find(|d| d.kind == want) else {
        return;
    };
    let (sy, cy) = (libm::sinf(v.turret.yaw), libm::cosf(v.turret.yaw));
    let (sp, cp) = (libm::sinf(v.turret.pitch), libm::cosf(v.turret.pitch));
    let dir = (v.rot * Vec3::new(sy * cp, sp, cy * cp)).normalize();
    let origin = v.pos + v.rot * Vec3::new(0.0, MUZZLE_Y, 0.0) + dir * MUZZLE_M;
    let shell = shell_from_def(def, v.id, v.team, origin, dir);
    shells.push(shell);
    events.push(SimEvent::VehicleFired {
        vehicle: v.id,
        shell: slot as u8,
    });
}
