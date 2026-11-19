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
    #[serde(default)]
    pub swap_in_ms: Option<u32>,
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

// ---- explosives / melee / ammo (M1-008b; PLAN_ISSUES PI-042) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplosiveClass {
    FragGrenade,
    SmokeGrenade,
    AtRocket,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExplosiveHistorical {
    pub mass_kg: f64,
    #[serde(rename = "_source")]
    pub source: HistSource,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FragGameplay {
    pub fuse_s: f64,
    pub throw_speed_mps: f64,
    pub can_cook: bool,
    pub bounce_mult: f64,
    pub friction_mult: f64,
    pub damage_max_hp: f64,
    pub damage_radius_m: f64,
    pub lethal_radius_m: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmokeGameplay {
    pub fuse_s: f64,
    pub throw_speed_mps: f64,
    pub cloud_radius_m: f64,
    pub expand_s: f64,
    pub duration_s: f64,
    pub decay_s: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RocketGameplay {
    pub muzzle_velocity_mps: f64,
    pub gravity_mps2: f64,
    pub effective_range_m: f64,
    pub max_range_m: f64,
    pub heat_pen_mm: f64,
    pub structure_damage_hp: f64,
    pub damage_max_hp: f64,
    pub damage_radius_m: f64,
    pub backblast_length_m: f64,
    pub backblast_damage_hp: f64,
    pub single_use: bool,
}

/// Class-specific gameplay block; the variant must agree with [`ExplosiveDef::class`]
/// (checked by the JSON Schema and by `GameData` tests).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ExplosiveGameplay {
    Frag(FragGameplay),
    Smoke(SmokeGameplay),
    Rocket(RocketGameplay),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExplosiveDef {
    pub id: String,
    pub name_key: String,
    pub faction: Faction,
    pub class: ExplosiveClass,
    pub historical: ExplosiveHistorical,
    pub gameplay: ExplosiveGameplay,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeleeClass {
    Bayonet,
    Knife,
    Butt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MeleeFaction {
    Us,
    Ger,
    Any,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeleeGameplay {
    pub damage_hp: f64,
    pub windup_s: f64,
    pub cooldown_s: f64,
    pub range_m: f64,
    pub backstab_angle_deg: f64,
    pub backstab_kill: bool,
    pub ff_mult: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeleeDef {
    pub id: String,
    pub name_key: String,
    pub faction: MeleeFaction,
    pub class: MeleeClass,
    pub gameplay: MeleeGameplay,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmmoHistorical {
    pub caliber: String,
    pub mass_kg: f64,
    #[serde(rename = "_source")]
    pub source: HistSource,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmmoGameplay {
    pub drag_k_per_m: f64,
    pub pen_power: f64,
    pub suppression_mult: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmmoDef {
    pub id: String,
    pub name_key: String,
    pub historical: AmmoHistorical,
    pub gameplay: AmmoGameplay,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

/// Surface material of the penetration table (`data/materials/`, M1-012).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialGameplay {
    /// Penetration points spent per millimetre.
    pub cost_per_mm: f64,
    /// A layer thicker than this always stops the bullet (0 = no cap).
    pub stop_mm: f64,
    /// Layers thinner than this are ignored (water < 0.5 m).
    pub min_mm: f64,
    /// Direction change on exit, +- degrees.
    pub deflect_deg: f64,
    /// May ricochet at shallow angles.
    pub ricochet: bool,
    pub sound_class: String,
    pub impact_fx_class: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialDef {
    pub id: String,
    pub name_key: String,
    pub gameplay: MaterialGameplay,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GadgetClass {
    Consumable,
    Deployable,
    Tool,
    Kit,
}

/// Non-weapon gadget; `gameplay` is a flat map of unit-suffixed numbers/booleans whose keys
/// are validated by `data/schemas/gadget.schema.json` (consumers read the keys they need).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GadgetDef {
    pub id: String,
    pub name_key: String,
    pub class: GadgetClass,
    pub gameplay: BTreeMap<String, serde_json::Value>,
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
    /// Linear ground acceleration time from rest to the target speed, seconds (M0-012,
    /// PLAN_ISSUES PI-015). Absent = instantaneous.
    #[serde(default)]
    pub accel_time_s: Option<f64>,
    /// Linear ground deceleration time from the reference speed to a stop, seconds. Absent = instant.
    #[serde(default)]
    pub decel_time_s: Option<f64>,
    // M1-011 additions (stance times, stamina, speed and stance modifiers; GAMEDESIGN 9).
    #[serde(default)]
    pub prone_fast_mps: Option<f64>,
    #[serde(default)]
    pub prone_fast_cost_per_s: Option<f64>,
    #[serde(default)]
    pub crouch_to_prone_ms: Option<f64>,
    #[serde(default)]
    pub prone_to_crouch_ms: Option<f64>,
    #[serde(default)]
    pub stand_to_prone_ms: Option<f64>,
    #[serde(default)]
    pub prone_to_stand_ms: Option<f64>,
    #[serde(default)]
    pub ads_stand_mult: Option<f64>,
    #[serde(default)]
    pub ads_crouch_mult: Option<f64>,
    #[serde(default)]
    pub ads_prone_mult: Option<f64>,
    #[serde(default)]
    pub reload_move_mult: Option<f64>,
    #[serde(default)]
    pub crawl_mps: Option<f64>,
    #[serde(default)]
    pub load_ref_kg: Option<f64>,
    #[serde(default)]
    pub load_penalty_per_kg: Option<f64>,
    #[serde(default)]
    pub load_min_mult: Option<f64>,
    #[serde(default)]
    pub sprint_cost_per_s: Option<f64>,
    #[serde(default)]
    pub stamina_regen_per_s: Option<f64>,
    #[serde(default)]
    pub stamina_regen_delay_s: Option<f64>,
    #[serde(default)]
    pub stamina_regen_crouch_mult: Option<f64>,
    #[serde(default)]
    pub stamina_regen_prone_mult: Option<f64>,
    #[serde(default)]
    pub stamina_low_points: Option<f64>,
    #[serde(default)]
    pub stamina_unlock_points: Option<f64>,
    #[serde(default)]
    pub jump_cost_points: Option<f64>,
    #[serde(default)]
    pub stance_recoil_crouch_mult: Option<f64>,
    #[serde(default)]
    pub stance_recoil_prone_mult: Option<f64>,
    #[serde(default)]
    pub stance_spread_crouch_mult: Option<f64>,
    #[serde(default)]
    pub stance_spread_prone_mult: Option<f64>,
    #[serde(default)]
    pub visibility_crouch_mult: Option<f64>,
    #[serde(default)]
    pub visibility_prone_mult: Option<f64>,
    #[serde(default)]
    pub hitbox_height_stand_m: Option<f64>,
    #[serde(default)]
    pub hitbox_height_crouch_m: Option<f64>,
    #[serde(default)]
    pub hitbox_height_prone_m: Option<f64>,
    #[serde(default)]
    pub move_spread_walk_mult: Option<f64>,
    #[serde(default)]
    pub move_spread_run_mult: Option<f64>,
    #[serde(default)]
    pub vault_max_m: Option<f64>,
    #[serde(default)]
    pub climb_max_m: Option<f64>,
    #[serde(default)]
    pub vault_ms: Option<f64>,
    #[serde(default)]
    pub climb_ms: Option<f64>,
    #[serde(default)]
    pub vault_cost_points: Option<f64>,
    #[serde(default)]
    pub climb_cost_points: Option<f64>,
    #[serde(default)]
    pub climb_headroom_m: Option<f64>,
    #[serde(default)]
    pub vault_reach_m: Option<f64>,
    #[serde(default)]
    pub vault_top_width_m: Option<f64>,
    #[serde(default)]
    pub vault_time_mult: Option<f64>,
    #[serde(default)]
    pub lean_deg: Option<f64>,
    #[serde(default)]
    pub lean_head_offset_m: Option<f64>,
    #[serde(default)]
    pub lean_ms: Option<f64>,
    #[serde(default)]
    pub lean_spread_mult: Option<f64>,
    #[serde(default)]
    pub prone_peek_m: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capsule {
    pub radius_m: f64,
    pub height_stand_m: f64,
    pub height_crouch_m: f64,
    /// Eye (muzzle origin) height above the feet, standing / crouched (M0-012).
    #[serde(default)]
    pub eye_height_stand_m: Option<f64>,
    #[serde(default)]
    pub eye_height_crouch_m: Option<f64>,
    /// Prone capsule / eye height (M1-011).
    #[serde(default)]
    pub height_prone_m: Option<f64>,
    #[serde(default)]
    pub eye_height_prone_m: Option<f64>,
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
    /// Role level that unlocks this loadout (GAMEDESIGN 5.2).
    pub unlock_level: u32,
    /// Magazines of the secondary weapon (counting the one in the gun).
    #[serde(default)]
    pub secondary_mags: Option<u32>,
    /// Non-weapon gadgets (`data/gadgets/`): bandages, kits, tools.
    #[serde(default)]
    pub tools: Vec<Gadget>,
    /// `m1_only`: present only in the M1 build (reserved for `ger_rifleman_pf60`).
    #[serde(default)]
    pub tags: Vec<String>,
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
    #[serde(default)]
    pub dimensions_m: Option<Dimensions>,
    #[serde(rename = "_source")]
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dimensions {
    pub length: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArmorKind {
    Plate,
    Mantlet,
}

/// One armour plate zone (GAMEDESIGN 14.3.3): thickness and angle from the vertical.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmorZone {
    pub id: String,
    pub thickness_mm: f64,
    pub slope_deg: f64,
    pub kind: ArmorKind,
}

/// Damageable module (GAMEDESIGN 14.5.1); `effects.at_50pct` / `at_0` are loose maps of
/// multipliers and flags that M1-023 interprets.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleModule {
    pub id: String,
    pub health_hp: f64,
    pub effects: BTreeMap<String, BTreeMap<String, serde_json::Value>>,
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
    /// Game top speed, m/s (= 0.75 x historical road speed, GAMEDESIGN 14.3).
    #[serde(default)]
    pub max_speed_mps: Option<f64>,
    #[serde(default)]
    pub hull_turn_deg_s: Option<f64>,
    /// Time to reach top speed: `[min, max]` seconds.
    #[serde(default)]
    pub accel_time_s: Option<[f64; 2]>,
    #[serde(default)]
    pub max_slope_deg: Option<f64>,
    #[serde(default)]
    pub armor_zones: Vec<ArmorZone>,
    #[serde(default)]
    pub modules: Vec<VehicleModule>,
    /// Cannon / machine-gun mounts with inline shell data (PI-045); consumed by M1-023.
    #[serde(default)]
    pub weapons: Vec<serde_json::Value>,
    /// Spawn gating incl. the M1 test override (no resource system before M2).
    #[serde(default)]
    pub spawn: Option<serde_json::Value>,
    /// Placeholder model attachment points.
    #[serde(default)]
    pub sockets: Vec<String>,
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

// ---- rules (M0-013) ----

/// `data/rules/respawn.json` (GAMEDESIGN 10.6).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespawnRulesDef {
    /// Death camera: the spawn point is picked automatically this long after death.
    pub death_cam_ms: f64,
    /// HQ-type spawn points: `ready_time = t_kia + base_wait`.
    pub base_wait_ms: f64,
    pub wave_interval_ms: f64,
    /// `false` skips the wave rounding (tests / sandbox).
    pub wave_aligned: bool,
    /// Maximum deterministic horizontal offset around the spawn point, metres.
    #[serde(default)]
    pub spawn_jitter_m: Option<f64>,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

/// One bot difficulty tier (GAMEDESIGN 20.6).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotDifficultyDef {
    /// Reaction delay range (min, max), recognition to first shot.
    pub reaction_ms: [f64; 2],
    pub aim_error_mult: f64,
    /// How long a lost target is chased ("hit memory").
    pub memory_s: f64,
    pub view_distance_m: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotDifficultiesDef {
    pub recruit: BotDifficultyDef,
    pub regular: BotDifficultyDef,
    pub veteran: BotDifficultyDef,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotPerceptionDef {
    pub fov_deg: f64,
    pub view_distance_cap_m: f64,
    pub min_recognition_mult: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotNavDef {
    pub cell_size_m: f64,
    pub max_slope_deg: f64,
    pub goal_radius_m: f64,
    pub repath_min_interval_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotCombatDef {
    pub blind_fire_max_m: f64,
    pub turn_rate_deg_s: f64,
    pub aim_error_base_deg: f64,
    pub aim_error_per_100m_deg: f64,
    pub fire_tolerance_deg: f64,
    pub crouch_min_m: f64,
    pub sprint_min_m: f64,
    /// Visible enemies farther than this are approached instead of shot at.
    pub approach_beyond_m: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotStuckDef {
    pub window_s: f64,
    pub min_move_m: f64,
}

/// Bot tuning (`data/rules/bots.json`, M0-026). Server-side behaviour only; `sim` itself never
/// reads it (the `bots` crate does, through [`GameData`](crate::GameData)).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotsRulesDef {
    pub default_difficulty: String,
    pub difficulties: BotDifficultiesDef,
    pub think_interval_ms: f64,
    pub perception: BotPerceptionDef,
    pub nav: BotNavDef,
    pub combat: BotCombatDef,
    pub stuck: BotStuckDef,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

/// `data/rules/capture.json` (GAMEDESIGN 3.2).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureEffDef {
    pub full_weight_players: u32,
    pub max_net_players: u32,
    pub partial_weight_mult: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureRulesDef {
    pub k_cap_pct_per_s: f64,
    pub regress_pct_per_s: f64,
    pub height_tolerance_m: f64,
    pub default_radius_m: f64,
    pub eff: CaptureEffDef,
    pub vehicle_occupants_counted: bool,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

/// `data/rules/match_m1.json` (GAMEDESIGN 2.2, M1 simplified match).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchRulesDef {
    pub warmup_s: f64,
    pub early_start_players: u32,
    pub time_limit_s: f64,
    pub ended_freeze_s: f64,
    pub ended_total_s: f64,
    pub repeat_capture_s: f64,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

/// `data/rules/squads.json` (GAMEDESIGN 2.4, 4.1, 4.4; M1-021).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SquadsRulesDef {
    pub team_max_players: u32,
    pub infantry: SquadKindDef,
    pub armor_crew: SquadKindDef,
    pub recon: SquadKindDef,
    pub rate_window_s: f64,
    pub create_limit_squads: u32,
    pub kick_limit_players: u32,
    pub kick_ban_s: f64,
    pub handover_confirm_s: f64,
    pub leader_absent_s: f64,
    pub role_switch_cooldown_s: f64,
    pub balance: BalanceDef,
    pub m1_override: SquadsM1Override,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SquadKindDef {
    pub max_players: u32,
    #[serde(default)]
    pub max_squads: Option<u32>,
    #[serde(default)]
    pub crewman_max_players: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BalanceDef {
    pub max_diff_players: u32,
    pub switch_min_after_join_s: f64,
    pub switch_interval_s: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SquadsM1Override {
    pub armor_crew_leader_role: String,
}

/// `data/rules/supplies.json` (GAMEDESIGN 12; M1-020).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuppliesRulesDef {
    #[serde(rename = "crate")]
    pub supply_crate: SupplyCrateDef,
    pub depot: DepotDef,
    pub point_depot: DepotDef,
    pub ammo_crate: AmmoCrateDef,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyCrateDef {
    pub content_supplies: u32,
    pub health_hp: f64,
    pub ground_max_crates: u32,
    pub place_s: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DepotDef {
    pub interval_s: f64,
    pub stock_max_crates: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmmoCrateDef {
    pub content_points: u32,
    pub max_uses: u32,
    pub place_cooldown_s: f64,
    pub lifetime_s: f64,
    pub health_hp: f64,
    pub price_points: std::collections::BTreeMap<String, u32>,
}

/// All rule families under `data/rules/`.
#[derive(Debug, Clone, PartialEq)]
pub struct RulesDef {
    pub respawn: RespawnRulesDef,
    pub bots: BotsRulesDef,
    pub capture: CaptureRulesDef,
    pub match_m1: MatchRulesDef,
    pub supplies: SuppliesRulesDef,
    pub squads: SquadsRulesDef,
    pub aoi: AoiRulesDef,
}

/// `data/rules/aoi.json` (ARCHITECTURE 6.5.4, R3; M1-024).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AoiRulesDef {
    pub enemy_max_m: f64,
    pub enemy_close_m: f64,
    pub friendly_max_m: f64,
    pub leave_max_m: f64,
    pub leave_close_m: f64,
    pub frustum_half_deg: f64,
    pub frustum_margin_deg: f64,
    pub frustum_leave_extra_deg: f64,
    pub turn_union_ms: f64,
    pub los_check_s: f64,
    pub los_min_dist_m: f64,
    pub vehicle_importance_mult: f64,
    pub team_channel_period_s: f64,
    pub icon_quantum_m: f64,
    pub icon_ttl_s: f64,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}
