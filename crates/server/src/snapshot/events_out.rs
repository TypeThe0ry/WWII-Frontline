//! C2 event segment: conversion of `SimEvent`s and the per-client redundancy window
//! (ARCHITECTURE §6.6.1, §6.8.4).
//!
//! * `Shot`: clients that may see the shooter (R3 rules, <= 400 m, not the shooter itself, whose
//!   own client predicts the shot). `Impact`: clients within 400 m of the impact.
//!   `HitConfirm`: the shooter only. `DamageDir`: the victim only.
//! * Every event gets a global 16-bit `event_id`. A packet carries the oldest unexpired events
//!   first; an event is repeated in at most 3 packets and dropped as soon as a packet that
//!   carried it is acknowledged (the client de-duplicates by `event_id`).
//! * Position codes are 14 bit with 1/64 m steps (the M0 convention of PLAN_ISSUES PI-023:
//!   16 384 codes cover the 256 m test map); angles: yaw 10 bit, pitch 9 bit; normals use a
//!   12-bit octahedral encoding (6 + 6 bit).

use frontline_protocol::quant::{quant_pitch, quant_yaw, PITCH_BITS_EVENT};
use frontline_protocol::snapshot::{Event, EventEntry};
use frontline_sim::Vec3;

/// Metres per position code of Shot / Impact events (M0 convention).
pub const EVENT_POS_STEP_M: f32 = 256.0 / 16384.0;
/// Maximum packets that carry the same event.
pub const MAX_SENDS: u8 = 3;
/// Cap on the unexpired events kept per client (oldest dropped first).
pub const MAX_PENDING: usize = 48;

pub fn pos_code(v: f32) -> u16 {
    if !v.is_finite() {
        return 0;
    }
    (v / EVENT_POS_STEP_M + 0.5).clamp(0.0, 16383.0) as u16
}

/// 12-bit octahedral normal (6 bit per axis).
pub fn oct_normal(n: Vec3) -> u16 {
    let l1 = n.x.abs() + n.y.abs() + n.z.abs();
    if l1.is_nan() || l1 <= 1e-6 {
        return 0;
    }
    let (mut u, mut v) = (n.x / l1, n.z / l1);
    if n.y < 0.0 {
        let (ou, ov) = (u, v);
        u = (1.0 - ov.abs()) * if ou >= 0.0 { 1.0 } else { -1.0 };
        v = (1.0 - ou.abs()) * if ov >= 0.0 { 1.0 } else { -1.0 };
    }
    let q = |t: f32| (((t * 0.5 + 0.5) * 63.0 + 0.5).clamp(0.0, 63.0)) as u16;
    (q(u) << 6) | q(v)
}

/// Direction -> (yaw code 10 bit, pitch code 9 bit). Yaw 0 faces +z, grows toward +x.
pub fn dir_codes(dir: Vec3) -> (u16, u16) {
    let yaw = dir.x.atan2(dir.z);
    let pitch = dir.y.clamp(-1.0, 1.0).asin();
    (
        quant_yaw(yaw, 10).code,
        quant_pitch(pitch, PITCH_BITS_EVENT).code,
    )
}

pub fn shot_event(shooter: u8, weapon: u16, muzzle: Vec3, dir: Vec3) -> Event {
    let (dir_yaw, dir_pitch) = dir_codes(dir);
    Event::Shot {
        shooter,
        weapon: weapon as u8,
        muzzle: [pos_code(muzzle.x), pos_code(muzzle.y), pos_code(muzzle.z)],
        dir_yaw,
        dir_pitch,
    }
}

pub fn impact_event(pos: Vec3, normal: Vec3, material: u8) -> Event {
    Event::Impact {
        pos: [pos_code(pos.x), pos_code(pos.y), pos_code(pos.z)],
        normal: oct_normal(normal),
        material,
    }
}

/// Radius code of Explosion events: 1/8 m steps, clamped to 255 (31.9 m).
pub const EXPLOSION_RADIUS_STEP_M: f32 = 0.125;

pub fn explosion_event(pos: Vec3, radius_m: f32, kind: u8) -> Event {
    let r = if radius_m.is_finite() {
        (radius_m / EXPLOSION_RADIUS_STEP_M + 0.5).clamp(0.0, 255.0) as u8
    } else {
        0
    };
    Event::Explosion {
        pos: [pos_code(pos.x), pos_code(pos.y), pos_code(pos.z)],
        kind,
        radius_q: r,
    }
}

/// Bearing of the attacker relative to the victim's facing, `[0, 2pi) -> 0..=255`
/// (0 = in front, growing clockwise seen from above).
pub fn damage_angle(victim_yaw: f32, from: Vec3, victim: Vec3) -> u8 {
    let d = from - victim;
    let bearing = d.x.atan2(d.z);
    let rel = (bearing - victim_yaw).rem_euclid(std::f32::consts::TAU);
    ((rel / std::f32::consts::TAU * 256.0) as u32 & 0xFF) as u8
}

#[derive(Debug, Clone, Copy)]
struct Pending {
    entry: EventEntry,
    sends: u8,
    /// Ticks of the packets that carried it.
    sent_in: [u32; MAX_SENDS as usize],
}

/// Events waiting to be (re)sent to one client.
#[derive(Debug, Default)]
pub struct EventWindow {
    pending: Vec<Pending>,
    /// Events dropped because the window overflowed.
    pub overflowed: u64,
    /// Events handed out at least once / confirmed by an ack / expired unconfirmed.
    pub queued_total: u64,
    pub confirmed_total: u64,
    pub expired_total: u64,
}

impl EventWindow {
    pub fn push(&mut self, entry: EventEntry) {
        self.queued_total += 1;
        if self.pending.len() >= MAX_PENDING {
            self.pending.remove(0);
            self.overflowed += 1;
        }
        self.pending.push(Pending {
            entry,
            sends: 0,
            sent_in: [0; MAX_SENDS as usize],
        });
    }

    pub(crate) fn push_comms(&mut self, entry: EventEntry) {
        let marker_id = |event| match event {
            Event::MarkerUpsert { id, .. } | Event::MarkerExpired { id } => Some(id),
            _ => None,
        };
        self.pending
            .retain(|p| marker_id(p.entry.event) != marker_id(entry.event));
        self.push(entry);
    }

    pub(crate) fn clear_comms(&mut self) {
        self.pending.retain(|p| {
            !matches!(
                p.entry.event,
                Event::MarkerUpsert { .. } | Event::MarkerExpired { .. }
            )
        });
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// The client acknowledged the packet of tick `ack`: events it carried are delivered.
    pub fn on_ack(&mut self, ack: u32) {
        let before = self.pending.len();
        self.pending
            .retain(|p| !p.sent_in[..p.sends as usize].contains(&ack));
        self.confirmed_total += (before - self.pending.len()) as u64;
    }

    /// Events for the packet of tick `now`, oldest first.
    pub fn entries(&self) -> Vec<EventEntry> {
        self.pending.iter().map(|p| p.entry).collect()
    }

    /// The packet of `now` carried the first `n` entries returned by [`Self::entries`].
    pub fn mark_sent(&mut self, n: usize, now: u32) {
        for p in self.pending.iter_mut().take(n) {
            if (p.sends as usize) < p.sent_in.len() {
                p.sent_in[p.sends as usize] = now;
                p.sends += 1;
            }
        }
        let before = self.pending.len();
        // an event stays until acknowledged; once it has used its 3 packets it expires
        // (acknowledgement of the last one can still arrive and is simply ignored)
        self.pending.retain(|p| {
            let enemy =
                matches!(p.entry.event, Event::MarkerUpsert { kind, .. } if kind.is_enemy());
            p.sends < if enemy { 1 } else { MAX_SENDS }
        });
        self.expired_total += (before - self.pending.len()) as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: u16) -> EventEntry {
        EventEntry {
            id,
            event: Event::DamageDir { angle: 1, dmg_q: 2 },
        }
    }

    #[test]
    fn explosion_radius_is_eighth_metre_and_clamped() {
        let q = |r| match explosion_event(Vec3::new(1.0, 0.0, 0.0), r, 0) {
            Event::Explosion { radius_q, pos, .. } => {
                assert_eq!(pos[0], 64);
                radius_q
            }
            _ => unreachable!(),
        };
        assert_eq!(q(8.0), 64);
        assert_eq!(q(500.0), 255);
        assert_eq!(q(f32::NAN), 0);
    }

    #[test]
    fn pos_code_is_one_64th_metre_and_clamped() {
        assert_eq!(pos_code(0.0), 0);
        assert_eq!(pos_code(1.0), 64);
        assert_eq!(pos_code(1e9), 16383);
        assert_eq!(pos_code(-4.0), 0);
        assert_eq!(pos_code(f32::NAN), 0);
    }

    #[test]
    fn oct_normal_up_and_down_differ() {
        let up = oct_normal(Vec3::Y);
        let dn = oct_normal(-Vec3::Y);
        assert_ne!(up, dn);
        assert!(up < 4096 && dn < 4096);
    }

    #[test]
    fn dir_codes_follow_the_sim_convention() {
        let (yaw, _) = dir_codes(Vec3::Z); // yaw 0
        assert_eq!(yaw, 0);
        let (yaw, _) = dir_codes(Vec3::X); // +x = quarter turn
        assert_eq!(yaw, 256);
        let (_, p) = dir_codes(Vec3::Y);
        assert_eq!(p, 511);
    }

    #[test]
    fn an_event_is_sent_in_three_packets_then_expires() {
        let mut w = EventWindow::default();
        w.push(ev(1));
        for t in [10, 11, 12] {
            let e = w.entries();
            assert_eq!(e.len(), 1, "tick {t}");
            w.mark_sent(e.len(), t);
        }
        assert!(w.entries().is_empty());
        assert_eq!(w.expired_total, 1);
    }

    #[test]
    fn ack_of_a_carrying_packet_stops_the_repeats() {
        let mut w = EventWindow::default();
        w.push(ev(1));
        w.mark_sent(1, 10);
        w.mark_sent(1, 11);
        w.on_ack(11);
        assert!(w.entries().is_empty());
        assert_eq!(w.confirmed_total, 1);
        // an ack for a packet that did not carry it does nothing
        w.push(ev(2));
        w.mark_sent(1, 12);
        w.on_ack(10);
        assert_eq!(w.entries().len(), 1);
    }

    #[test]
    fn window_is_bounded() {
        let mut w = EventWindow::default();
        for i in 0..(MAX_PENDING as u16 + 5) {
            w.push(ev(i));
        }
        assert_eq!(w.len(), MAX_PENDING);
        assert_eq!(w.overflowed, 5);
        assert_eq!(w.entries()[0].id, 5);
    }
}
