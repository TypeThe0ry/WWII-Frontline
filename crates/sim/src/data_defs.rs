// Typed mirror of the /data JSON schemas (data/schemas/*.schema.json).
//
// Shared between `build.rs` (build-time parse; any mismatch fails the build) and the
// runtime `GameData` loader (`include!`d from `data.rs`). Every struct is
// `deny_unknown_fields`: adding a field means updating the JSON Schema *and* this file.
// The full JSON Schema / unit / range / reference checks live in tools/data-validate.

use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Faction {
    Us,
    Ger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataStatus {
    Initial,
    Reviewed,
    Measured,
}

/// A historical value that may be explicitly marked `"未核实"` (unverified).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Hist<T> {
    Known(T),
    Unverified(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub status: DataStatus,
    pub tunable: Vec<String>,
    #[serde(default)]
    pub calibration_ref: Option<String>,
    /// Historical fields explicitly registered as unverified (M0-031, `hist-source-required`).
    #[serde(default)]
    pub unverified: Vec<String>,
    /// Gameplay value path -> why it deviates from the historical value (REFERENCE_MEASUREMENTS 6.1).
    #[serde(default)]
    pub deviation_reason: std::collections::BTreeMap<String, String>,
}

/// One structured provenance record of a historical value (M0-031).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub field: String,
    pub value: serde_json::Value,
    pub unit: String,
    pub source_id: String,
    pub title: String,
    pub locator: String,
    #[serde(default)]
    pub url: Option<String>,
    pub accessed: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// `historical._source`: free text (initial data) or structured records (reviewed data).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum HistSource {
    Text(String),
    Records(Vec<SourceRecord>),
}

// ---- weapon ----

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponHistorical {
    pub caliber: String,
    pub cartridge: String,
    pub magazine_capacity: Hist<u32>,
    pub magazine_type: String,
    pub muzzle_velocity_mps: Hist<f64>,
    pub rate_of_fire_rpm_practical: Hist<f64>,
    pub mass_kg: Hist<f64>,
    #[serde(default)]
    pub effective_range_m: Option<Hist<f64>>,
    #[serde(rename = "_source")]
    pub source: HistSource,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartMult {
    pub head: f64,
    pub neck: f64,
    pub torso: f64,
    /// Optional pelvis / lower-abdomen multiplier (GAMEDESIGN 6.6: 0.9).
    #[serde(default)]
    pub pelvis: Option<f64>,
    pub limb: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FalloffPoint {
    pub dist_m: f64,
    pub mult: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FireMode {
    Semi,
    Auto,
    Burst,
    Bolt,
    Pump,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fire {
    pub mode: FireMode,
    #[serde(default)]
    pub cap_rpm: Option<f64>,
    #[serde(default)]
    pub bolt_cycle_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reload {
    pub tactical_ms: f64,
    pub empty_ms: f64,
    #[serde(default)]
    pub per_round_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spread {
    pub hip_deg: f64,
    pub ads_deg: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recoil {
    pub vkick_deg: f64,
    pub hsigma_deg: f64,
    pub recover_deg_s: f64,
    pub bloom_per_shot_deg: f64,
    pub bloom_decay_deg_s: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ballistics {
    pub muzzle_velocity_mps: f64,
    pub drag_k_per_m: f64,
    pub gravity_mps2: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponGameplay {
    pub damage_torso_hp: f64,
    pub part_mult: PartMult,
    pub falloff: Vec<FalloffPoint>,
    pub fire: Fire,
    pub reload: Reload,
    pub ads_ms: f64,
    pub spread: Spread,
    pub recoil: Recoil,
    pub ballistics: Ballistics,
    pub pen_power: f64,
    pub mag_reserve: u32,
}

/// Presentation hints (display only; never read by sim logic).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponFx {
    #[serde(default)]
    pub muzzle_flash_ms: Option<f64>,
    #[serde(default)]
    pub tracer: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponDef {
    pub id: String,
    pub name_key: String,
    pub faction: Faction,
    pub class: String,
    pub historical: WeaponHistorical,
    pub gameplay: WeaponGameplay,
    #[serde(default)]
    pub fx: Option<WeaponFx>,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

// ---- role ----

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactionOverride {
    pub loadout_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleUnlock {
    pub squad_size_min: u32,
    pub requires_squad_leader: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Movement {
    pub walk_mps: f64,
    pub run_mps: f64,
    pub sprint_mps: f64,
    pub crouch_mps: f64,
    pub prone_mps: f64,
    pub stamina_max: f64,
    // Optional fields added by M0-011 (PLAN_ISSUES PI-014); `MoveParams::from_role` requires
    // the ones the movement code uses.
    #[serde(default)]
    pub crouch_walk_mps: Option<f64>,
    #[serde(default)]
    pub jump_height_m: Option<f64>,
    #[serde(default)]
    pub stance_transition_ms: Option<f64>,
    #[serde(default)]
    pub air_control_mult: Option<f64>,
    #[serde(default)]
    pub gravity_mps2: Option<f64>,
    #[serde(default)]
    pub max_slope_deg: Option<f64>,
    #[serde(default)]
    pub autostep_max_m: Option<f64>,
    #[serde(default)]
    pub autostep_min_width_m: Option<f64>,
    #[serde(default)]
    pub snap_to_ground_m: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capsule {
    pub radius_m: f64,
    pub height_stand_m: f64,
    pub height_crouch_m: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleDef {
    pub id: String,
    pub name_key: String,
    pub faction_overrides: BTreeMap<String, FactionOverride>,
    #[serde(default)]
    pub max_per_squad: Option<u32>,
    #[serde(default)]
    pub max_per_team: Option<u32>,
    #[serde(default)]
    pub unlock: Option<RoleUnlock>,
    pub movement: Movement,
    #[serde(default)]
    pub capsule: Option<Capsule>,
    pub health_hp: f64,
    #[serde(default)]
    pub abilities: Vec<String>,
    #[serde(default)]
    pub icon_id: Option<String>,
    #[serde(default)]
    pub manpower_cost: Option<u32>,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

// ---- loadout ----

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gadget {
    pub id: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadoutDef {
    pub id: String,
    pub role: String,
    pub faction: Faction,
    pub primary: String,
    #[serde(default)]
    pub secondary: Option<String>,
    #[serde(default)]
    pub gadget_1: Option<Gadget>,
    #[serde(default)]
    pub gadget_2: Option<Gadget>,
    #[serde(default)]
    pub melee: Option<String>,
    pub ammo_mags: u32,
    #[serde(default)]
    pub unlock_condition: Option<String>,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

// ---- vehicle (no instances before M3; nested tuning blocks stay loosely typed) ----

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleHistorical {
    pub mass_kg: Hist<f64>,
    pub crew: Hist<u32>,
    pub main_gun: Option<String>,
    pub max_speed_mps: Hist<f64>,
    #[serde(rename = "_source")]
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Armor {
    pub front: f64,
    pub side: f64,
    pub rear: f64,
    pub top: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seat {
    pub id: String,
    pub role: String,
    #[serde(default)]
    pub turret: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnCost {
    pub manpower: u32,
    pub munitions: u32,
    pub fuel: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleGameplay {
    pub hull_health_hp: f64,
    pub armor_mm: Armor,
    pub seats: Vec<Seat>,
    #[serde(default)]
    pub suspension: Option<serde_json::Value>,
    #[serde(default)]
    pub tracks: Option<serde_json::Value>,
    #[serde(default)]
    pub turret_traverse_deg_s: Option<f64>,
    #[serde(default)]
    pub gun_elevation_deg: Option<[f64; 2]>,
    pub spawn_cost: SpawnCost,
    pub respawn_cooldown_s: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleDef {
    pub id: String,
    pub name_key: String,
    pub faction: Faction,
    #[serde(rename = "type")]
    pub vehicle_type: String,
    pub historical: VehicleHistorical,
    pub gameplay: VehicleGameplay,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}
