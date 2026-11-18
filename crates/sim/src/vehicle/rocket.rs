//! Panzerfaust 60 (M1-023, GD 6.9.4): a one-shot HEAT rocket launched by infantry. Flight,
//! hit resolution and the blast go through the vehicle shell machinery
//! ([`super::shells`]); this module adds the launch and the back-blast.

use bevy_ecs::prelude::*;
use frontline_protocol::msg::input::buttons;
use glam::Vec3;

use super::armor::{ShellKind, ShellParams};
use super::shells::{Shell, Shells, Warhead};
use crate::combat::DamageKind;
use crate::components::{EntityTable, EventBuf, LastInput, SimCfgRes, Team, Transform};
use crate::data::defs::{ExplosiveClass, ExplosiveGameplay, Faction};
use crate::health::{Health, HitLog, HitRec, LifeState};
use crate::step::SimEvent;
use crate::throwables::system::Gadgets;
use crate::vehicle::seats::InVehicle;
use crate::weapon::spread::view_dir;
use crate::weapon::HitZone;

/// Eye height above the feet, metres.
pub const EYE_M: f32 = 1.6;
/// Half angle of the back-blast cone.
pub const BACKBLAST_HALF_DEG: f32 = 30.0;
/// Warhead calibre for the overmatch rule, mm.
pub const WARHEAD_CALIBER_MM: f32 = 60.0;

fn faction(t: Team) -> Faction {
    match t {
        Team::Us => Faction::Us,
        Team::Ger => Faction::Ger,
    }
}

/// Launch: FIRE (rising edge) with a rocket in hand and not seated in a vehicle.
#[allow(clippy::type_complexity)]
pub fn rockets(
    cfg: Res<SimCfgRes>,
    table: Res<EntityTable>,
    mut shells: ResMut<Shells>,
    mut events: ResMut<EventBuf>,
    mut log: ResMut<HitLog>,
    mut shooters: Query<(
        &Transform,
        &LastInput,
        &Team,
        &LifeState,
        &InVehicle,
        &mut Gadgets,
    )>,
    mut victims: Query<(&Transform, &Team, &LifeState, &mut Health), Without<Gadgets>>,
) {
    let mut launches: Vec<(crate::ids::EntityId, Transform, Team)> = Vec::new();
    for (id, e) in table.sorted() {
        let Ok((t, last, team, life, inv, mut g)) = shooters.get_mut(e) else {
            continue;
        };
        let held = last.cmd.buttons & buttons::FIRE != 0;
        let edge = held && !g.fire_held;
        g.fire_held = held;
        if g.rocket == 0 || !edge || !life.is_alive() || inv.0.is_some() {
            continue;
        }
        g.rocket -= 1;
        launches.push((id, *t, *team));
    }
    for (id, t, team) in launches {
        let Some(rk) = cfg.data.explosives.iter().find_map(|x| match &x.gameplay {
            ExplosiveGameplay::Rocket(r)
                if x.class == ExplosiveClass::AtRocket && x.faction == faction(team) =>
            {
                Some(r)
            }
            _ => None,
        }) else {
            continue;
        };
        let dir = view_dir(t.yaw, t.pitch);
        let origin = t.pos + Vec3::Y * EYE_M + dir * 0.5;
        shells.push(Shell {
            id: 0,
            owner: id,
            team,
            warhead: Warhead::Heat,
            params: ShellParams {
                kind: ShellKind::Heat,
                caliber_mm: WARHEAD_CALIBER_MM,
                p100_mm: rk.heat_pen_mm as f32,
                drop_pct_per_100m: 0.0,
                structure_damage_hp: rk.structure_damage_hp as f32,
            },
            blast_dmax: rk.damage_max_hp as f32,
            blast_radius: rk.damage_radius_m as f32,
            smoke_radius: 0.0,
            smoke_s: 0.0,
            max_range_m: rk.max_range_m as f32,
            origin,
            pos: origin,
            vel: dir * rk.muzzle_velocity_mps as f32,
            substeps: 0,
        });
        events.0.push(SimEvent::RocketFired { shooter: id });
        // Back-blast: a cone behind the shooter.
        let back = -Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
        let cos_half = libm::cosf(BACKBLAST_HALF_DEG.to_radians());
        for (vid, ve) in table.sorted() {
            if vid == id {
                continue;
            }
            let Ok((vt, _, vlife, mut hp)) = victims.get_mut(ve) else {
                continue;
            };
            if vlife.is_dead() || hp.hp <= 0.0 {
                continue;
            }
            let d = vt.pos - t.pos;
            let flat = Vec3::new(d.x, 0.0, d.z);
            let len = flat.length();
            if len > rk.backblast_length_m as f32 || len < 1e-3 {
                continue;
            }
            if (flat / len).dot(back) < cos_half {
                continue;
            }
            let dmg = rk.backblast_damage_hp as f32;
            hp.hp -= dmg;
            log.0.push(HitRec {
                target: vid,
                shooter: id,
                weapon: crate::throwables::EXPLOSIVE_WEAPON_IDX,
                zone: HitZone::Torso,
                dist_m: len,
                damage: dmg,
                kind: DamageKind::Explosion,
                direct_kia: false,
            });
        }
    }
}
