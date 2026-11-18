//! The per-tick vehicle system (runs right after `movement`).

use bevy_ecs::prelude::*;
use frontline_protocol::msg::input::buttons;
use glam::Vec3;

use super::collision::{self, CRUSH_HURT_HP, CRUSH_HURT_MPS, CRUSH_KIA_MPS};
use super::seats::*;
use super::{step_one, Vehicle, VehicleCtrl, Vehicles};
use crate::components::{
    CollisionRes, CurrentTick, EventBuf, LastInput, Motion, PlayerIndex, SimCfgRes, Transform,
};
use crate::health::{Health, HitLog, HitRec, LifeState};
use crate::step::SimEvent;
use crate::throwables::EXPLOSIVE_WEAPON_IDX;
use crate::weapon::HitZone;

/// Where a seat sits relative to the hull centre (local, metres).
fn seat_offset(seat: usize) -> Vec3 {
    match seat {
        0 => Vec3::new(0.0, 0.9, 1.4),
        1 => Vec3::new(0.0, 1.3, 0.0),
        _ => Vec3::new(0.0, 1.5, -0.3),
    }
}

fn driver_ctrl(last: &LastInput) -> VehicleCtrl {
    let c = last.cmd;
    VehicleCtrl {
        throttle: f32::from(c.move_z) / 127.0,
        steer: f32::from(c.move_x) / 127.0,
        boost: c.buttons & buttons::SPRINT != 0,
        // Handbrake on Jump (PI-066).
        brake: c.buttons & buttons::JUMP != 0,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn step_vehicles(
    mut vehicles: ResMut<Vehicles>,
    col: Res<CollisionRes>,
    cfg: Res<SimCfgRes>,
    tick: Res<CurrentTick>,
    index: Res<PlayerIndex>,
    mut log: ResMut<HitLog>,
    mut events: ResMut<EventBuf>,
    mut q: Query<(
        &mut Transform,
        &mut Motion,
        &LastInput,
        &mut Health,
        &LifeState,
        &mut InVehicle,
    )>,
) {
    if vehicles.list.is_empty() {
        return;
    }
    let hz = f32::from(cfg.hz);
    let dt = 1.0 / hz;
    let now = tick.0 .0;
    for v in vehicles.list.values_mut() {
        // 1. Driver input.
        let driver = v.seats.first().copied().flatten();
        let scripted = v.scripted;
        v.ctrl = match driver {
            None if scripted => v.ctrl,
            Some(o) => {
                let last = index.map.get(&o.player).and_then(|&(_, e)| q.get(e).ok());
                match last {
                    Some((_, _, last, _, _, _)) => driver_ctrl(last),
                    None => parked(),
                }
            }
            None => parked(),
        };
        // 2. Physics.
        step_one(v, &col.0, dt);
        // 3. Hull vs terrain.
        match collision::probe(v, &col.0) {
            Some(hit) => {
                let vn = v.vel.dot(hit.normal);
                if vn < 0.0 {
                    v.vel -= hit.normal * vn;
                }
                if !v.in_contact {
                    let damage = if hit.speed > collision::IMPACT_MIN_MPS {
                        0.5 * hit.speed * hit.speed
                    } else {
                        0.0
                    };
                    v.hull_hp -= damage;
                    events.0.push(SimEvent::VehicleCollision {
                        vehicle: v.id,
                        speed_mps: hit.speed,
                        damage,
                    });
                }
                v.in_contact = true;
            }
            None => v.in_contact = false,
        }
        // 4. Overturn.
        if collision::tilt_deg(v) > collision::OVERTURN_DEG {
            v.flip_ticks += 1;
            if !v.overturned && v.flip_ticks as f32 >= collision::OVERTURN_S * hz {
                v.overturned = true;
                events.0.push(SimEvent::VehicleOverturned { vehicle: v.id });
            }
        } else {
            v.flip_ticks = 0;
        }
        // 5. Seat timers.
        for s in 0..v.seats.len() {
            let Some(mut o) = v.seats[s] else { continue };
            if let Some((to, at)) = o.switch {
                if now >= at {
                    o.switch = None;
                    let to = usize::from(to);
                    if v.seats.get(to).is_some_and(|x| x.is_none()) {
                        v.seats[s] = None;
                        v.seats[to] = Some(o);
                        set_seat(&index, &mut q, o.player, v.id, Some(to as u8));
                        events.0.push(SimEvent::SeatChanged {
                            player: o.player,
                            vehicle: v.id,
                            seat: Some(to as u8),
                        });
                        continue;
                    }
                    v.seats[s] = Some(o);
                }
            }
            if let Some(at) = o.exit_at {
                if now >= at {
                    v.seats[s] = None;
                    finish_exit(v, o.player, &col.0, &index, &mut q, &mut log, &mut events);
                    continue;
                }
            }
            v.seats[s] = Some(o);
        }
        // 6. Occupants ride along.
        for (s, occ) in v.seats.iter().enumerate() {
            let Some(o) = occ else { continue };
            let Some(&(_, e)) = index.map.get(&o.player) else {
                continue;
            };
            if let Ok((mut t, mut m, _, _, _, _)) = q.get_mut(e) {
                t.pos = v.pos + v.rot * seat_offset(s);
                t.yaw = (v.forward().x)
                    .atan2(v.forward().z)
                    .rem_euclid(core::f32::consts::TAU);
                m.vel = v.vel;
            }
        }
        // 7. Crushing.
        let speed = Vec3::new(v.vel.x, 0.0, v.vel.z).length();
        if speed >= CRUSH_HURT_MPS {
            let repeat = (collision::CRUSH_REPEAT_S * hz) as u32;
            for (&pid, &(eid, e)) in &index.map {
                let Ok((t, _, _, mut hp, life, inv)) = q.get_mut(e) else {
                    continue;
                };
                if inv.0.is_some() || !matches!(life, LifeState::Alive) {
                    continue;
                }
                if !collision::under_hull(v, t.pos) {
                    continue;
                }
                if v.crushed
                    .get(&pid)
                    .is_some_and(|&t0| now.saturating_sub(t0) < repeat)
                {
                    continue;
                }
                v.crushed.insert(pid, now);
                let kia = speed >= CRUSH_KIA_MPS;
                let dmg = if kia { hp.hp.max(1.0) } else { CRUSH_HURT_HP };
                hp.hp -= dmg;
                log.0.push(HitRec {
                    target: eid,
                    shooter: v.id,
                    weapon: EXPLOSIVE_WEAPON_IDX,
                    zone: HitZone::Torso,
                    dist_m: 0.0,
                    damage: dmg,
                    kind: crate::combat::DamageKind::Vehicle,
                    direct_kia: kia,
                });
                events.0.push(SimEvent::Crush {
                    vehicle: v.id,
                    victim: pid,
                    speed_kph: speed * 3.6,
                });
            }
        }
    }
}

fn parked() -> VehicleCtrl {
    VehicleCtrl {
        brake: true,
        ..Default::default()
    }
}

fn set_seat(
    index: &PlayerIndex,
    q: &mut Query<(
        &mut Transform,
        &mut Motion,
        &LastInput,
        &mut Health,
        &LifeState,
        &mut InVehicle,
    )>,
    player: crate::ids::PlayerId,
    vehicle: u16,
    seat: Option<u8>,
) {
    if let Some(&(_, e)) = index.map.get(&player) {
        if let Ok((_, _, _, _, _, mut inv)) = q.get_mut(e) {
            inv.0 = seat.map(|s| (vehicle, s));
        }
    }
}

fn finish_exit(
    v: &Vehicle,
    player: crate::ids::PlayerId,
    col: &crate::collision::CollisionData,
    index: &PlayerIndex,
    q: &mut Query<(
        &mut Transform,
        &mut Motion,
        &LastInput,
        &mut Health,
        &LifeState,
        &mut InVehicle,
    )>,
    log: &mut HitLog,
    events: &mut EventBuf,
) {
    let Some(&(eid, e)) = index.map.get(&player) else {
        return;
    };
    if let Ok((mut t, mut m, _, mut hp, _, mut inv)) = q.get_mut(e) {
        inv.0 = None;
        let side = v.rot * Vec3::X;
        let mut p = v.pos + side * (v.params.half_extents.x + 1.0);
        p.y = col.height_at(p.x, p.z).unwrap_or(v.pos.y) + 0.02;
        t.pos = p;
        m.vel = Vec3::ZERO;
        let speed = Vec3::new(v.vel.x, 0.0, v.vel.z).length();
        if speed > EXIT_HURT_MPS {
            hp.hp -= EXIT_DAMAGE_HP;
            log.0.push(HitRec {
                target: eid,
                shooter: eid,
                weapon: EXPLOSIVE_WEAPON_IDX,
                zone: HitZone::Torso,
                dist_m: 0.0,
                damage: EXIT_DAMAGE_HP,
                kind: crate::combat::DamageKind::Fall,
                direct_kia: false,
            });
        }
    }
    events.0.push(SimEvent::SeatChanged {
        player,
        vehicle: v.id,
        seat: None,
    });
}
