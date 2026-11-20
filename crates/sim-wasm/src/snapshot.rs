//! `SnapshotDecoder`: wraps the protocol decoder and exposes the decoded snapshot as
//! struct-of-arrays views into wasm memory (valid until the next `decode`/`reset`).

use frontline_protocol::snapshot::events::{MARKER_EXPIRED_TAG, MARKER_UPSERT_TAG};
use frontline_protocol::snapshot::{DecodedSnapshot, Event, EventEntry, SnapshotDecoder as Inner};
use js_sys::{Float32Array, Uint8Array};
use wasm_bindgen::prelude::*;

use crate::js_err;

/// Floats in the `self` array.
pub const SELF_LEN: usize = 15;
/// Bytes per packed event record (see `pack_event`).
pub const EVENT_RECORD_BYTES: usize = 16;

/// Floats per vehicle in `vehicleState`: `pos[3] vel[3] quat[4] (xyzw) turret_yaw gun_pitch`.
/// `turret_yaw` is radians in [0, 2pi) relative to the hull, `gun_pitch` radians in [-pi/2, pi/2].
pub const VEHICLE_STRIDE: usize = 12;
/// Bytes per vehicle in `vehicleMeta`: `seats, damage_flags, health_q, flags`.
pub const VEHICLE_META_BYTES: usize = 4;

pub mod self_idx {
    pub const PRESENT: usize = 0;
    /// 1 when the server sent a position correction (`CORR_*` valid).
    pub const HAS_CORR: usize = 1;
    pub const CORR_X: usize = 2;
    pub const CORR_Y: usize = 3;
    pub const CORR_Z: usize = 4;
    pub const AMMO_IN_MAG: usize = 5;
    pub const AMMO_RESERVE: usize = 6;
    pub const HEALTH: usize = 7;
    pub const STAMINA: usize = 8;
    pub const SUPPRESSION: usize = 9;
    pub const STANCE: usize = 10;
    pub const DEPLOY_STATE: usize = 11;
    pub const COOLDOWN_BITS: usize = 12;
    /// 0 on foot, otherwise the vehicle slot (128..=255).
    pub const VEHICLE_SLOT: usize = 13;
    /// Seat index (0..=2); 0 on foot.
    pub const SEAT: usize = 14;
}

pub fn layout_table() -> &'static [(&'static str, u32)] {
    use self_idx::*;
    &[
        ("SELF_PRESENT", PRESENT as u32),
        ("SELF_HAS_CORR", HAS_CORR as u32),
        ("SELF_CORR_X", CORR_X as u32),
        ("SELF_CORR_Y", CORR_Y as u32),
        ("SELF_CORR_Z", CORR_Z as u32),
        ("SELF_AMMO_IN_MAG", AMMO_IN_MAG as u32),
        ("SELF_AMMO_RESERVE", AMMO_RESERVE as u32),
        ("SELF_HEALTH", HEALTH as u32),
        ("SELF_STAMINA", STAMINA as u32),
        ("SELF_SUPPRESSION", SUPPRESSION as u32),
        ("SELF_STANCE", STANCE as u32),
        ("SELF_DEPLOY_STATE", DEPLOY_STATE as u32),
        ("SELF_COOLDOWN_BITS", COOLDOWN_BITS as u32),
        ("SELF_VEHICLE_SLOT", VEHICLE_SLOT as u32),
        ("SELF_SEAT", SEAT as u32),
        ("SELF_LEN", SELF_LEN as u32),
        ("EVENT_RECORD_BYTES", EVENT_RECORD_BYTES as u32),
    ]
}

/// Event record: `id:u16le, type:u8, payload (13 bytes, u16 fields little endian, zero padded)`.
/// Types and payloads follow `protocol::snapshot::events` (1 Shot: shooter u8, weapon u8,
/// muzzle 3xu16, dir_yaw u16, dir_pitch u16; 2 Impact: pos 3xu16, normal u16, material u8;
/// 3 HitConfirm: target u8, zone u8, damage_q u8; 4 DamageDir: angle u8, dmg_q u8;
/// 5 Explosion: pos 3xu16, kind u8, radius_q u8; 6 Suppress: source_dir u8, intensity u8;
/// 7 Marker: kind u8, pos 2xu16, ttl u8;
/// 10 MarkerUpsert: marker_id u16, kind u8, pos 2xu16, ttl u8, owner u8,
/// has_target u8, target u16 (zero when absent); 11 MarkerExpired: marker_id u16).
pub fn pack_event(e: &EventEntry, out: &mut Vec<u8>) {
    let mut rec = [0u8; EVENT_RECORD_BYTES];
    rec[0..2].copy_from_slice(&e.id.to_le_bytes());
    let mut n = 3;
    let mut put8 = |rec: &mut [u8; EVENT_RECORD_BYTES], v: u8| {
        rec[n] = v;
        n += 1;
    };
    match e.event {
        Event::Shot {
            shooter,
            weapon,
            muzzle,
            dir_yaw,
            dir_pitch,
        } => {
            rec[2] = 1;
            put8(&mut rec, shooter);
            put8(&mut rec, weapon);
            for v in muzzle.into_iter().chain([dir_yaw, dir_pitch]) {
                for b in v.to_le_bytes() {
                    put8(&mut rec, b);
                }
            }
        }
        Event::Impact {
            pos,
            normal,
            material,
        } => {
            rec[2] = 2;
            for v in pos.into_iter().chain([normal]) {
                for b in v.to_le_bytes() {
                    put8(&mut rec, b);
                }
            }
            put8(&mut rec, material);
        }
        Event::HitConfirm {
            target,
            zone,
            damage_q,
        } => {
            rec[2] = 3;
            put8(&mut rec, target);
            put8(&mut rec, zone);
            put8(&mut rec, damage_q);
        }
        Event::DamageDir { angle, dmg_q } => {
            rec[2] = 4;
            put8(&mut rec, angle);
            put8(&mut rec, dmg_q);
        }
        Event::Explosion {
            pos,
            kind,
            radius_q,
        } => {
            rec[2] = 5;
            for v in pos {
                for b in v.to_le_bytes() {
                    put8(&mut rec, b);
                }
            }
            put8(&mut rec, kind);
            put8(&mut rec, radius_q);
        }
        Event::Suppress {
            source_dir,
            intensity,
        } => {
            rec[2] = 6;
            put8(&mut rec, source_dir);
            put8(&mut rec, intensity);
        }
        Event::Marker { kind, pos, ttl } => {
            rec[2] = 7;
            put8(&mut rec, kind);
            for v in pos {
                for b in v.to_le_bytes() {
                    put8(&mut rec, b);
                }
            }
            put8(&mut rec, ttl);
        }
        Event::MarkerUpsert {
            id,
            kind,
            pos,
            ttl,
            owner,
            target,
        } => {
            rec[2] = MARKER_UPSERT_TAG;
            rec[3..5].copy_from_slice(&id.to_le_bytes());
            rec[5] = kind.wire_tag();
            rec[6..8].copy_from_slice(&pos[0].to_le_bytes());
            rec[8..10].copy_from_slice(&pos[1].to_le_bytes());
            rec[10] = ttl;
            rec[11] = owner;
            if let Some(target) = target {
                rec[12] = 1;
                rec[13..15].copy_from_slice(&target.to_le_bytes());
            }
        }
        Event::MarkerExpired { id } => {
            rec[2] = MARKER_EXPIRED_TAG;
            rec[3..5].copy_from_slice(&id.to_le_bytes());
        }
    }
    out.extend_from_slice(&rec);
}

#[wasm_bindgen]
pub struct SnapshotDecoder {
    inner: Inner,
    last: Option<DecodedSnapshot>,
    self_out: [f32; SELF_LEN],
    events: Vec<u8>,
    veh_ids: Vec<u8>,
    veh_state: Vec<f32>,
    veh_meta: Vec<u8>,
}

impl SnapshotDecoder {
    pub fn decode_core(&mut self, bytes: &[u8]) -> Result<(), String> {
        let snap = self
            .inner
            .decode(bytes)
            .map_err(|e| format!("snapshot decode failed: {e}"))?;
        use self_idx::*;
        let s = &mut self.self_out;
        *s = [0.0; SELF_LEN];
        if let Some(st) = &snap.self_state {
            s[PRESENT] = 1.0;
            if let Some(c) = st.pos_corr {
                let m = c.to_meters();
                s[HAS_CORR] = 1.0;
                s[CORR_X] = m[0];
                s[CORR_Y] = m[1];
                s[CORR_Z] = m[2];
            }
            s[AMMO_IN_MAG] = f32::from(st.ammo_in_mag);
            s[AMMO_RESERVE] = f32::from(st.ammo_reserve);
            s[HEALTH] = f32::from(st.health);
            s[STAMINA] = f32::from(st.stamina);
            s[SUPPRESSION] = f32::from(st.suppression);
            s[STANCE] = f32::from(st.stance);
            s[DEPLOY_STATE] = f32::from(st.deploy_state);
            s[COOLDOWN_BITS] = f32::from(st.cooldown_bits);
            s[VEHICLE_SLOT] = f32::from(st.vehicle_slot);
            s[SEAT] = f32::from(st.seat);
        }
        self.events.clear();
        for e in &snap.events {
            pack_event(e, &mut self.events);
        }
        self.veh_ids.clear();
        self.veh_state.clear();
        self.veh_meta.clear();
        let e = &snap.entities;
        for i in 0..e.len() {
            if e.kind[i] != frontline_protocol::snapshot::EntityKind::Vehicle {
                continue;
            }
            self.veh_ids.push(e.ids[i]);
            self.veh_state.extend_from_slice(&e.pos[3 * i..3 * i + 3]);
            for k in 0..3 {
                self.veh_state
                    .push(f32::from(e.vel_q[3 * i + k]) * frontline_protocol::snapshot::VEL_STEP);
            }
            let x = &e.veh[i];
            self.veh_state
                .extend_from_slice(&frontline_protocol::quant::quat_unpack(x.quat));
            self.veh_state
                .push(f32::from(x.turret_yaw) / 4096.0 * std::f32::consts::TAU);
            self.veh_state.push(
                f32::from(x.gun_pitch) / 4095.0 * std::f32::consts::PI
                    - std::f32::consts::FRAC_PI_2,
            );
            self.veh_meta
                .extend_from_slice(&[x.seats, x.damage_flags, e.health_q[i], e.flags[i]]);
        }
        debug_assert_eq!(self.veh_state.len(), self.veh_ids.len() * VEHICLE_STRIDE);
        debug_assert_eq!(self.veh_meta.len(), self.veh_ids.len() * VEHICLE_META_BYTES);
        self.last = Some(snap);
        Ok(())
    }

    pub fn last(&self) -> Option<&DecodedSnapshot> {
        self.last.as_ref()
    }
}

fn f32_view(v: &[f32]) -> Float32Array {
    // SAFETY: the slice lives in wasm linear memory and is owned by the decoder; the contract
    // (module docs) is that the view is consumed before the next decode/reset.
    unsafe { Float32Array::view(v) }
}

fn u8_view(v: &[u8]) -> Uint8Array {
    // SAFETY: as `f32_view`.
    unsafe { Uint8Array::view(v) }
}

#[wasm_bindgen]
impl SnapshotDecoder {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> SnapshotDecoder {
        SnapshotDecoder {
            inner: Inner::new(),
            last: None,
            self_out: [0.0; SELF_LEN],
            events: Vec::new(),
            veh_ids: Vec::new(),
            veh_state: Vec::new(),
            veh_meta: Vec::new(),
        }
    }

    /// Decodes one snapshot datagram; throws on malformed input (the decoder state is left
    /// unchanged). Read the result through the accessors below.
    pub fn decode(&mut self, bytes: &[u8]) -> Result<u32, JsError> {
        self.decode_core(bytes).map_err(js_err)?;
        Ok(self.last.as_ref().map_or(0, |s| s.tick))
    }

    /// Forgets all baselines (reconnect).
    pub fn reset(&mut self) {
        self.inner.reset();
        self.last = None;
        self.self_out = [0.0; SELF_LEN];
        self.events.clear();
        self.veh_ids.clear();
        self.veh_state.clear();
        self.veh_meta.clear();
    }

    pub fn tick(&self) -> u32 {
        self.last.as_ref().map_or(0, |s| s.tick)
    }
    #[wasm_bindgen(js_name = ackInputSeq)]
    pub fn ack_input_seq(&self) -> u32 {
        self.last.as_ref().map_or(0, |s| u32::from(s.ack_input_seq))
    }
    #[wasm_bindgen(js_name = inputSlack)]
    pub fn input_slack(&self) -> u32 {
        self.last.as_ref().map_or(0, |s| u32::from(s.input_slack))
    }
    /// Entity records in the last snapshot.
    pub fn count(&self) -> u32 {
        self.last.as_ref().map_or(0, |s| s.entities.len() as u32)
    }

    /// `LAYOUT_SELF_*` indexed floats.
    #[wasm_bindgen(js_name = selfState)]
    pub fn self_state(&self) -> Float32Array {
        f32_view(&self.self_out)
    }
    pub fn ids(&self) -> Uint8Array {
        u8_view(self.last().map_or(&[], |s| &s.entities.ids))
    }
    /// 3 floats (x, y, z metres) per entity.
    pub fn pos(&self) -> Float32Array {
        f32_view(self.last().map_or(&[], |s| &s.entities.pos))
    }
    pub fn yaw(&self) -> Float32Array {
        f32_view(self.last().map_or(&[], |s| &s.entities.yaw))
    }
    pub fn pitch(&self) -> Float32Array {
        f32_view(self.last().map_or(&[], |s| &s.entities.pitch))
    }
    pub fn anim(&self) -> Uint8Array {
        u8_view(self.last().map_or(&[], |s| &s.entities.anim))
    }
    pub fn stance(&self) -> Uint8Array {
        u8_view(self.last().map_or(&[], |s| &s.entities.stance))
    }
    pub fn weapon(&self) -> Uint8Array {
        u8_view(self.last().map_or(&[], |s| &s.entities.weapon))
    }
    pub fn flags(&self) -> Uint8Array {
        u8_view(self.last().map_or(&[], |s| &s.entities.flags))
    }
    pub fn health(&self) -> Uint8Array {
        u8_view(self.last().map_or(&[], |s| &s.entities.health_q))
    }
    pub fn removed(&self) -> Uint8Array {
        u8_view(self.last().map_or(&[], |s| &s.removed_ids))
    }
    /// Wire ids of the vehicle entities in the last snapshot (these rows also appear in `ids()`;
    /// consumers skip ids >= 128 there).
    #[wasm_bindgen(js_name = vehicleIds)]
    pub fn vehicle_ids(&self) -> Uint8Array {
        u8_view(&self.veh_ids)
    }
    /// `VEHICLE_STRIDE` floats per vehicle (see the constant).
    #[wasm_bindgen(js_name = vehicleState)]
    pub fn vehicle_state(&self) -> Float32Array {
        f32_view(&self.veh_state)
    }
    /// `VEHICLE_META_BYTES` bytes per vehicle.
    #[wasm_bindgen(js_name = vehicleMeta)]
    pub fn vehicle_meta(&self) -> Uint8Array {
        u8_view(&self.veh_meta)
    }
    /// `EVENT_RECORD_BYTES` bytes per event.
    pub fn events(&self) -> Uint8Array {
        u8_view(&self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontline_protocol::snapshot::{
        EntityKind, EntityState, QPos, QVel, SnapshotEncoder, VehicleExtra,
    };

    #[test]
    fn marker_lifecycle_packets_expose_fixed_client_records() {
        use frontline_protocol::msg::PingKind;
        use frontline_protocol::snapshot::MarkerKind;

        let mut encoder = SnapshotEncoder::new();
        let mut snapshot = encoder.begin(1, 7, 1200);
        let upsert = |seq, kind, target| EventEntry {
            id: seq,
            event: Event::MarkerUpsert {
                id: 0x1234,
                kind: MarkerKind::Ping(kind),
                pos: [4095, 2],
                ttl: 6,
                owner: 9,
                target,
            },
        };
        let events = [
            upsert(1, PingKind::Enemy, None),
            upsert(2, PingKind::Thanks, Some(u16::MAX)),
            EventEntry {
                id: 3,
                event: Event::MarkerExpired { id: 0x1234 },
            },
        ];
        assert_eq!(snapshot.add_events(&events).unwrap(), events.len());
        let mut decoder = SnapshotDecoder::new();
        decoder.decode_core(&snapshot.finish()).unwrap();
        assert_eq!(
            decoder.events,
            [
                1, 0, 10, 0x34, 0x12, 1, 0xff, 0x0f, 2, 0, 6, 9, 0, 0, 0, 0, 2, 0, 10, 0x34, 0x12,
                7, 0xff, 0x0f, 2, 0, 6, 9, 1, 0xff, 0xff, 0, 3, 0, 11, 0x34, 0x12, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0,
            ]
        );
    }

    #[test]
    fn vehicle_rows_are_exposed_with_decoded_units() {
        let mut enc = SnapshotEncoder::new();
        let quat = frontline_protocol::quant::quat_pack([0.0, 0.0, 0.0, 1.0]);
        let tank = EntityState {
            id: 130,
            kind: EntityKind::Vehicle,
            pos: QPos::from_meters(100.0, 5.0, 60.0),
            vel: Some(QVel::from_mps(0.0, 0.0, 4.0)),
            health_q: 200,
            flags: 0x80,
            vehicle: Some(VehicleExtra {
                quat,
                turret_yaw: 1024,
                gun_pitch: 4095,
                track_flags: 0,
                seats: 0b101,
                damage_flags: 0b10,
            }),
            ..Default::default()
        };
        let bytes = {
            let mut b = enc.begin(1, 7, 1200);
            assert!(b.add_entity(&tank, true).unwrap());
            b.finish()
        };
        let mut d = SnapshotDecoder::new();
        d.decode_core(&bytes).unwrap();
        assert_eq!(d.veh_ids, vec![130]);
        let v = &d.veh_state;
        assert_eq!(v.len(), VEHICLE_STRIDE);
        assert!((v[0] - 100.0).abs() < 0.1 && (v[2] - 60.0).abs() < 0.1);
        assert!((v[5] - 4.0).abs() < 0.02, "vz {}", v[5]);
        assert!((v[9] - 1.0).abs() < 1e-2, "quat w {}", v[9]);
        assert!(
            (v[10] - std::f32::consts::FRAC_PI_2).abs() < 1e-3,
            "turret {}",
            v[10]
        );
        assert!(
            (v[11] - std::f32::consts::FRAC_PI_2).abs() < 1e-3,
            "pitch {}",
            v[11]
        );
        assert_eq!(d.veh_meta, vec![0b101, 0b10, 200, 0x80]);
        d.reset();
        assert!(d.veh_ids.is_empty());
    }
}
