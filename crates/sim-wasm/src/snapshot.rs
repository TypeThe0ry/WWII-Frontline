//! `SnapshotDecoder`: wraps the protocol decoder and exposes the decoded snapshot as
//! struct-of-arrays views into wasm memory (valid until the next `decode`/`reset`).

use frontline_protocol::snapshot::{DecodedSnapshot, Event, EventEntry, SnapshotDecoder as Inner};
use js_sys::{Float32Array, Uint8Array};
use wasm_bindgen::prelude::*;

use crate::js_err;

/// Floats in the `self` array.
pub const SELF_LEN: usize = 13;
/// Bytes per packed event record (see `pack_event`).
pub const EVENT_RECORD_BYTES: usize = 16;

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
        ("SELF_LEN", SELF_LEN as u32),
        ("EVENT_RECORD_BYTES", EVENT_RECORD_BYTES as u32),
    ]
}

/// Event record: `id:u16le, type:u8, payload (13 bytes, u16 fields little endian, zero padded)`.
/// Types and payloads follow `protocol::snapshot::events` (1 Shot: shooter u8, weapon u8,
/// muzzle 3xu16, dir_yaw u16, dir_pitch u16; 2 Impact: pos 3xu16, normal u16, material u8;
/// 3 HitConfirm: target u8, zone u8, damage_q u8; 4 DamageDir: angle u8, dmg_q u8;
/// 5 Explosion: pos 3xu16, kind u8, radius_q u8; 6 Suppress: source_dir u8, intensity u8;
/// 7 Marker: kind u8, pos 2xu16, ttl u8).
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
    }
    out.extend_from_slice(&rec);
}

#[wasm_bindgen]
pub struct SnapshotDecoder {
    inner: Inner,
    last: Option<DecodedSnapshot>,
    self_out: [f32; SELF_LEN],
    events: Vec<u8>,
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
        }
        self.events.clear();
        for e in &snap.events {
            pack_event(e, &mut self.events);
        }
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
    /// `EVENT_RECORD_BYTES` bytes per event.
    pub fn events(&self) -> Uint8Array {
        u8_view(&self.events)
    }
}
