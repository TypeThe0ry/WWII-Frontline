//! `Predictor`: local-player prediction session (movement M0-011 + weapon state machine M0-012).

use frontline_protocol::bitio::BitReader;
use frontline_protocol::codec::Codec;
use frontline_protocol::msg::input::InputCmd;
use frontline_sim::hash::HashWriter;
use frontline_sim::predict::PredictSim;
use frontline_sim::weapon::{Fsm, WeaponParams, WeaponState, TICK_UNITS};
use frontline_sim::{
    CollisionChunk, CollisionData, GameData, GameMode, MapId, PlayerAuthState, SimConfig, Tick,
};
use js_sys::Float32Array;
use wasm_bindgen::prelude::*;

use crate::js_err;

/// Simulation rate (PI-006: fixed step 1000/30 ms).
const TICK_HZ: u8 = 30;
/// Bytes of one full-width `InputCmd` (`packInputCmd`): 144 bits.
pub const INPUT_CMD_BYTES: usize = 18;

/// Index table of the `Float32Array` returned by `Predictor.step()` / `state()`.
pub mod layout {
    pub const X: usize = 0;
    pub const Y: usize = 1;
    pub const Z: usize = 2;
    pub const VX: usize = 3;
    pub const VY: usize = 4;
    pub const VZ: usize = 5;
    pub const YAW: usize = 6;
    pub const PITCH: usize = 7;
    /// `Stance` code (0 stand, 1 crouch).
    pub const STANCE: usize = 8;
    pub const GROUNDED: usize = 9;
    pub const AMMO_IN_MAG: usize = 10;
    pub const AMMO_RESERVE: usize = 11;
    /// Ticks until the weapon can fire / finish its cycle (0 when ready).
    pub const FIRE_COOLDOWN_TICKS: usize = 12;
    /// Accumulated recoil offset of the view, degrees.
    pub const RECOIL_PITCH: usize = 13;
    pub const RECOIL_YAW: usize = 14;
    /// Crouch fraction 0..1 of the capsule transition.
    pub const STANCE_BLEND: usize = 15;
    /// 1 when the last step could not move (no loaded collision under the player).
    pub const BLOCKED: usize = 16;
    pub const BLOOM_DEG: usize = 17;
    /// `Fsm` code (0 Ready, 1 Cooldown, 2 BoltCycle, 3 Reloading tactical, 4 Reloading empty, 5 Empty); -1 without weapon.
    pub const WEAPON_FSM: usize = 18;
    /// Bit set describing what the last step did: see `EV_*`.
    pub const EVENTS: usize = 19;
    /// Predicted tick (wraps at 2^24 exactly representable in f32; use `Predictor.tick()` for the full value).
    pub const TICK: usize = 20;
    /// ADS progress 0..1 (0 without weapon).
    pub const ADS: usize = 21;
    pub const STATE_LEN: usize = 22;

    pub const EV_SHOT: u32 = 1;
    pub const EV_DRY_FIRE: u32 = 2;
    pub const EV_RELOAD_STARTED: u32 = 4;
    pub const EV_RELOAD_FINISHED: u32 = 8;
}

pub const LAYOUT_STATE_LEN: usize = layout::STATE_LEN;

/// `(name, value)` of every state layout constant, in JSON order. Names are exported to TS as
/// `LAYOUT_<NAME>`.
pub fn layout_table() -> &'static [(&'static str, u32)] {
    use layout::*;
    &[
        ("STATE_X", X as u32),
        ("STATE_Y", Y as u32),
        ("STATE_Z", Z as u32),
        ("STATE_VX", VX as u32),
        ("STATE_VY", VY as u32),
        ("STATE_VZ", VZ as u32),
        ("STATE_YAW", YAW as u32),
        ("STATE_PITCH", PITCH as u32),
        ("STATE_STANCE", STANCE as u32),
        ("STATE_GROUNDED", GROUNDED as u32),
        ("STATE_AMMO_IN_MAG", AMMO_IN_MAG as u32),
        ("STATE_AMMO_RESERVE", AMMO_RESERVE as u32),
        ("STATE_FIRE_COOLDOWN_TICKS", FIRE_COOLDOWN_TICKS as u32),
        ("STATE_RECOIL_PITCH", RECOIL_PITCH as u32),
        ("STATE_RECOIL_YAW", RECOIL_YAW as u32),
        ("STATE_STANCE_BLEND", STANCE_BLEND as u32),
        ("STATE_BLOCKED", BLOCKED as u32),
        ("STATE_BLOOM_DEG", BLOOM_DEG as u32),
        ("STATE_WEAPON_FSM", WEAPON_FSM as u32),
        ("STATE_EVENTS", EVENTS as u32),
        ("STATE_TICK", TICK as u32),
        ("STATE_ADS", ADS as u32),
        ("STATE_LEN", STATE_LEN as u32),
        ("EV_SHOT", EV_SHOT),
        ("EV_DRY_FIRE", EV_DRY_FIRE),
        ("EV_RELOAD_STARTED", EV_RELOAD_STARTED),
        ("EV_RELOAD_FINISHED", EV_RELOAD_FINISHED),
        ("INPUT_CMD_BYTES", INPUT_CMD_BYTES as u32),
        ("AUTH_STATE_BYTES", PlayerAuthState::BYTES as u32),
        ("WEAPON_STATE_BYTES", WeaponState::BYTES as u32),
    ]
}

/// Decodes the fixed-width `InputCmd` encoding.
pub(crate) fn parse_cmd(bytes: &[u8]) -> Result<InputCmd, String> {
    if bytes.len() != INPUT_CMD_BYTES {
        return Err(format!(
            "input command must be {INPUT_CMD_BYTES} bytes, got {}",
            bytes.len()
        ));
    }
    let mut r = BitReader::new(bytes);
    let c = InputCmd::decode(&mut r).map_err(|e| format!("bad input command: {e}"))?;
    r.finish().map_err(|e| format!("bad input command: {e}"))?;
    Ok(c)
}

/// Prediction session for the local player.
#[wasm_bindgen]
pub struct Predictor {
    sim: PredictSim,
    out: [f32; layout::STATE_LEN],
}

impl Predictor {
    pub fn new_core(match_seed: u64, map_id: u16, my_player_id: u8) -> Result<Self, String> {
        if my_player_id > frontline_sim::ids::MAX_PLAYER_ID {
            return Err(format!("player id {my_player_id} out of range 0..=127"));
        }
        let cfg = SimConfig {
            tick_hz: TICK_HZ,
            match_seed,
            map: MapId(map_id),
            mode: GameMode::Sandbox,
            data: GameData::embedded(),
        };
        let mut p = Self {
            sim: PredictSim::new(cfg, &CollisionData::empty(), my_player_id),
            out: [0.0; layout::STATE_LEN],
        };
        p.refresh();
        Ok(p)
    }

    pub fn step_core(&mut self, cmd: &[u8]) -> Result<(), String> {
        let c = parse_cmd(cmd)?;
        self.sim.step_player(&c);
        self.refresh();
        Ok(())
    }

    pub fn reset_core(
        &mut self,
        auth: &[u8],
        tick: u32,
        weapon: Option<&[u8]>,
    ) -> Result<(), String> {
        let a = PlayerAuthState::from_bytes(auth).ok_or_else(|| {
            format!(
                "invalid authoritative state ({} bytes, expected {})",
                auth.len(),
                PlayerAuthState::BYTES
            )
        })?;
        // Validate the weapon first so that a bad argument leaves the session untouched.
        let w = match weapon {
            Some(b) => {
                let w = WeaponState::from_bytes(b).ok_or("invalid weapon state")?;
                Some(w)
            }
            None => None,
        };
        if let Some(w) = w {
            let data = GameData::embedded();
            let def = data
                .weapons
                .get(usize::from(w.def_id))
                .ok_or("weapon state refers to an unknown weapon")?;
            WeaponParams::from_def(def, TICK_HZ).map_err(|e| format!("unusable weapon: {e:?}"))?;
        }
        self.sim.reset_to(&a, Tick(tick));
        if let Some(w) = w {
            self.sim.reset_weapon(Some(w));
        }
        self.refresh();
        Ok(())
    }

    pub fn equip_core(&mut self, weapon_id: &str, reserve: u16) -> Result<(), String> {
        let data = GameData::embedded();
        let idx = data
            .weapons
            .iter()
            .position(|w| w.id == weapon_id)
            .ok_or_else(|| format!("unknown weapon {weapon_id:?}"))?;
        let params = WeaponParams::from_def(&data.weapons[idx], TICK_HZ)
            .map_err(|e| format!("unusable weapon: {e:?}"))?;
        self.sim.reset_weapon(Some(WeaponState::new(
            idx as u16,
            params.mag_capacity,
            reserve,
        )));
        self.refresh();
        Ok(())
    }

    pub fn load_core(&mut self, bx: i32, bz: i32, data: &[u8]) -> Result<(), String> {
        let chunk = CollisionChunk::from_bytes(bx, bz, data)
            .map_err(|e| format!("bad collision chunk ({bx},{bz}): {e}"))?;
        self.sim.collision_mut().insert_chunk(chunk);
        Ok(())
    }

    /// Sim terrain height (`CollisionData::height_at`, bilinear over the loaded chunks).
    pub fn height_at_core(&mut self, x: f32, z: f32) -> Option<f32> {
        self.sim.collision_mut().height_at(x, z)
    }

    pub fn state_slice(&self) -> &[f32] {
        &self.out
    }

    pub fn state_hash_core(&self) -> u64 {
        let mut h = HashWriter::new();
        h.write_bytes(&self.sim.state().to_bytes());
        match self.sim.weapon() {
            Some(w) => {
                h.write_u8(1);
                h.write_bytes(&w.to_bytes());
            }
            None => h.write_u8(0),
        }
        h.finish()
    }

    pub fn auth_bytes(&self) -> [u8; PlayerAuthState::BYTES] {
        self.sim.state().to_bytes()
    }

    pub fn weapon_bytes(&self) -> Vec<u8> {
        self.sim
            .weapon()
            .map(|w| w.to_bytes().to_vec())
            .unwrap_or_default()
    }

    fn refresh(&mut self) {
        use layout::*;
        let s = self.sim.state();
        let o = &mut self.out;
        o[X] = s.pos.x;
        o[Y] = s.pos.y;
        o[Z] = s.pos.z;
        o[VX] = s.vel.x;
        o[VY] = s.vel.y;
        o[VZ] = s.vel.z;
        o[YAW] = s.yaw;
        o[PITCH] = s.pitch;
        o[STANCE] = f32::from(s.stance as u8);
        o[GROUNDED] = f32::from(u8::from(s.grounded));
        o[STANCE_BLEND] = s.stance_blend;
        o[BLOCKED] = f32::from(u8::from(s.blocked));
        o[TICK] = self.sim.tick().0 as f32;
        match self.sim.weapon() {
            Some(w) => {
                o[AMMO_IN_MAG] = f32::from(w.mag);
                o[AMMO_RESERVE] = f32::from(w.reserve);
                o[FIRE_COOLDOWN_TICKS] = w.timer_units.max(0) as f32 / TICK_UNITS as f32;
                o[RECOIL_PITCH] = w.recoil_pitch_deg;
                o[RECOIL_YAW] = w.recoil_yaw_deg;
                o[BLOOM_DEG] = w.bloom_deg;
                o[WEAPON_FSM] = f32::from(fsm_code(w.fsm));
                o[ADS] = ads_fraction(&w);
            }
            None => {
                for i in [
                    AMMO_IN_MAG,
                    AMMO_RESERVE,
                    FIRE_COOLDOWN_TICKS,
                    RECOIL_PITCH,
                    RECOIL_YAW,
                    BLOOM_DEG,
                    ADS,
                ] {
                    o[i] = 0.0;
                }
                o[WEAPON_FSM] = -1.0;
            }
        }
        let out = self.sim.last_weapon_out();
        let mut ev = 0;
        if out.shot.is_some() {
            ev |= EV_SHOT;
        }
        if out.dry_fire {
            ev |= EV_DRY_FIRE;
        }
        if out.reload_started.is_some() {
            ev |= EV_RELOAD_STARTED;
        }
        if out.reload_finished {
            ev |= EV_RELOAD_FINISHED;
        }
        o[EVENTS] = ev as f32;
    }
}

fn fsm_code(f: Fsm) -> u8 {
    // Same numbering as `WeaponState::to_bytes` (byte 6).
    let mut probe = WeaponState::new(0, 1, 0);
    probe.fsm = f;
    probe.to_bytes()[6]
}

fn ads_fraction(w: &WeaponState) -> f32 {
    let data = GameData::embedded();
    data.weapons
        .get(usize::from(w.def_id))
        .and_then(|d| WeaponParams::from_def(d, TICK_HZ).ok())
        .map_or(0.0, |p| w.ads_progress(p.ads_units))
}

#[wasm_bindgen]
impl Predictor {
    #[wasm_bindgen(constructor)]
    pub fn new(match_seed: u64, map_id: u32, my_player_id: u32) -> Result<Predictor, JsError> {
        let map = u16::try_from(map_id).map_err(|_| js_err("mapId out of range"))?;
        let me = u8::try_from(my_player_id).map_err(|_| js_err("player id out of range"))?;
        Self::new_core(match_seed, map, me).map_err(js_err)
    }

    /// Server `EntityId` of the player (part of the shot RNG stream key); default 0.
    #[wasm_bindgen(js_name = setEntity)]
    pub fn set_entity(&mut self, entity: u32) -> Result<(), JsError> {
        let e = u16::try_from(entity).map_err(|_| js_err("entity id out of range"))?;
        self.sim.set_entity(e);
        Ok(())
    }

    #[wasm_bindgen(js_name = loadCollision)]
    pub fn load_collision(&mut self, bx: i32, bz: i32, data: &[u8]) -> Result<(), JsError> {
        self.load_core(bx, bz, data).map_err(js_err)
    }

    /// Returns whether the chunk was loaded.
    #[wasm_bindgen(js_name = unloadCollision)]
    pub fn unload_collision(&mut self, bx: i32, bz: i32) -> bool {
        self.sim.collision_mut().remove_chunk(bx, bz)
    }

    /// Reconciliation reset to an authoritative `PlayerAuthState` (39 bytes, see
    /// `PlayerAuthState::to_bytes`) at `tick`; `weapon` (33 bytes, `WeaponState::to_bytes`)
    /// optionally resets the weapon state too.
    pub fn reset(
        &mut self,
        auth_state: &[u8],
        tick: u32,
        weapon: Option<Vec<u8>>,
    ) -> Result<(), JsError> {
        self.reset_core(auth_state, tick, weapon.as_deref())
            .map_err(js_err)
    }

    /// Resets only the weapon state (33 bytes); `undefined` disables weapon prediction.
    #[wasm_bindgen(js_name = resetWeapon)]
    pub fn reset_weapon(&mut self, weapon: Option<Vec<u8>>) -> Result<(), JsError> {
        match weapon {
            None => {
                self.sim.reset_weapon(None);
                self.refresh();
                Ok(())
            }
            Some(b) => {
                let a = self.auth_bytes();
                let t = self.sim.tick().0;
                self.reset_core(&a, t, Some(&b)).map_err(js_err)
            }
        }
    }

    /// Gives the player a fresh full weapon by data id with `reserve` spare rounds.
    #[wasm_bindgen(js_name = equipWeapon)]
    pub fn equip_weapon(&mut self, weapon_id: &str, reserve: u32) -> Result<(), JsError> {
        let r = u16::try_from(reserve).map_err(|_| js_err("reserve out of range"))?;
        self.equip_core(weapon_id, r).map_err(js_err)
    }

    /// Advances one tick with one 18-byte `InputCmd` (`packInputCmd`). Returns a view of the
    /// state array (layout `LAYOUT_STATE_*`), valid until the next call into this module.
    pub fn step(&mut self, cmd: &[u8]) -> Result<Float32Array, JsError> {
        self.step_core(cmd).map_err(js_err)?;
        Ok(self.state())
    }

    /// View of the current state array (same layout as `step`).
    pub fn state(&self) -> Float32Array {
        // SAFETY: the view aliases `self.out`, which lives in wasm linear memory; the
        // documented contract is that callers consume it before the next call into wasm.
        unsafe { Float32Array::view(&self.out) }
    }

    /// Copy of the predicted `WeaponState` bytes (33), or empty without a weapon.
    #[wasm_bindgen(js_name = weaponState)]
    pub fn weapon_state(&self) -> Vec<u8> {
        self.weapon_bytes()
    }

    /// Copy of the predicted `PlayerAuthState` bytes (39).
    #[wasm_bindgen(js_name = authState)]
    pub fn auth_state(&self) -> Vec<u8> {
        self.auth_bytes().to_vec()
    }

    /// Sim terrain height at world (x, z) from the loaded collision chunks (the value the
    /// server's `height_at` returns), or NaN where no chunk is loaded. Used by the client's
    /// terrain-vs-sim consistency test (M0-029).
    #[wasm_bindgen(js_name = heightAt)]
    pub fn height_at(&mut self, x: f64, z: f64) -> f64 {
        self.height_at_core(x as f32, z as f32)
            .map_or(f64::NAN, f64::from)
    }

    /// Full predicted tick counter.
    pub fn tick(&self) -> u32 {
        self.sim.tick().0
    }

    /// FNV-1a hash over the predicted movement + weapon state bytes.
    #[wasm_bindgen(js_name = stateHash)]
    pub fn state_hash(&self) -> u64 {
        self.state_hash_core()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd_bytes(mz: i8, buttons: u16) -> [u8; INPUT_CMD_BYTES] {
        crate::messages::pack_input_cmd_core(1, 1, 0, mz, 0, 0, buttons, 0, 0, 0)
            .unwrap()
            .try_into()
            .unwrap()
    }

    #[test]
    fn step_without_collision_is_blocked_but_counts_ticks() {
        let mut p = Predictor::new_core(1, 0, 3).unwrap();
        p.step_core(&cmd_bytes(127, 0)).unwrap();
        assert_eq!(p.sim.tick().0, 1);
        assert_eq!(p.state_slice()[layout::TICK], 1.0);
    }

    #[test]
    fn equip_and_fire_changes_ammo() {
        let mut p = Predictor::new_core(1, 0, 3).unwrap();
        p.equip_core("us_m1_garand", 40).unwrap();
        let before = p.state_slice()[layout::AMMO_IN_MAG];
        assert!(before > 0.0);
        p.step_core(&cmd_bytes(0, frontline_protocol::msg::input::buttons::FIRE))
            .unwrap();
        let s = p.state_slice();
        assert_eq!(s[layout::AMMO_IN_MAG], before - 1.0);
        assert!(s[layout::EVENTS] as u32 & layout::EV_SHOT != 0);
    }

    #[test]
    fn bad_inputs_are_errors() {
        let mut p = Predictor::new_core(1, 0, 3).unwrap();
        assert!(p.step_core(&[0; 5]).is_err());
        assert!(p.step_core(&[0; 19]).is_err());
        // move_x = -128 is invalid on the wire.
        let mut b = cmd_bytes(0, 0);
        b[6] = 0x80;
        assert!(p.step_core(&b).is_err());
        assert!(p.reset_core(&[0; 10], 0, None).is_err());
        assert!(p.load_core(0, 0, &[1, 2, 3]).is_err());
        assert!(p.equip_core("no_such_weapon", 1).is_err());
        assert!(Predictor::new_core(1, 0, 200).is_err());
    }

    #[test]
    fn reset_roundtrip() {
        let mut p = Predictor::new_core(1, 0, 3).unwrap();
        p.equip_core("us_m1_garand", 40).unwrap();
        let (a, w) = (p.auth_bytes(), p.weapon_bytes());
        let h = p.state_hash_core();
        p.step_core(&cmd_bytes(0, frontline_protocol::msg::input::buttons::FIRE))
            .unwrap();
        assert_ne!(p.state_hash_core(), h);
        p.reset_core(&a, 0, Some(&w)).unwrap();
        assert_eq!(p.state_hash_core(), h);
    }
}
