/**
 * Data the placeholder actors are built from. Nothing is hard-coded in the render code:
 *  - capsule size, eye heights and the stance transition time come from `roleDef('rifleman')` (data/roles/rifleman.json),
 *  - faction, muzzle flash time, tracer flag and muzzle velocity come from `weaponDef(id)` (data/weapons/*.json),
 *  - grey-box model sizes and the first-person view placement come from data/placeholders/weapon_models.json
 *    (PLAN_ISSUES PI-024: the weapon schema has no `fx.dimensions` field yet; sim is out of this task's scope).
 */
import type { Faction } from './names';

export interface CapsuleDims {
  radius: number;
  heightStand: number;
  heightCrouch: number;
  eyeStand: number;
  eyeCrouch: number;
}

export interface WeaponModelDims {
  length: number;
  barrelLength: number;
  barrelDiameter: number;
  receiverHeight: number;
  receiverWidth: number;
  magLength: number;
  magHeight: number;
  magWidth: number;
}

export interface WeaponViewDef {
  hipOffset: [number, number, number];
  adsOffset: [number, number, number];
  adsFovMult: number;
}

export interface WeaponActorDef {
  /** `WeaponState.def_id` / the 6-bit weapon field of snapshot records. */
  index: number;
  id: string;
  faction: Faction;
  requirementId: string;
  muzzleFlashMs: number;
  tracer: boolean;
  muzzleVelocityMps: number;
  adsMs: number;
  reloadMs?: { tactical: number; empty: number };
  dims: WeaponModelDims;
  view: WeaponViewDef;
}

export interface ActorData {
  capsule: CapsuleDims;
  stanceTransitionMs: number;
  hipFovDeg: number;
  weapons: WeaponActorDef[];
}

/** The part of the sim-wasm module this needs. */
export interface DataSource {
  roleDef(id: string): Uint8Array;
  weaponDef(id: string): Uint8Array;
  weaponIndex(id: string): number;
}

/** Shape of data/placeholders/weapon_models.json. */
export interface WeaponModelsFile {
  weapons: Record<string, { requirement_id: string; length_m: number; barrel_length_m: number; barrel_diameter_m: number; receiver_height_m: number; receiver_width_m: number; magazine_length_m: number; magazine_height_m: number; magazine_width_m: number; view: { hip_offset_m: number[]; ads_offset_m: number[]; ads_fov_mult: number } }>;
  view: { hip_fov_deg: number };
}

const dec = new TextDecoder();
const json = <T>(b: Uint8Array): T => JSON.parse(dec.decode(b)) as T;

function num(v: unknown, what: string): number {
  if (typeof v !== 'number' || !Number.isFinite(v) || v <= 0) throw new Error(`actor data: ${what} must be a positive number (got ${String(v)})`);
  return v;
}
function vec3(v: unknown, what: string): [number, number, number] {
  if (!Array.isArray(v) || v.length !== 3 || v.some((x) => typeof x !== 'number' || !Number.isFinite(x))) throw new Error(`actor data: ${what} must be [x, y, z]`);
  return [v[0] as number, v[1] as number, v[2] as number];
}

export function buildActorData(src: DataSource, models: WeaponModelsFile): ActorData {
  const role = json<{ capsule: Record<string, number>; movement: Record<string, number> }>(src.roleDef('rifleman'));
  const c = role.capsule;
  const capsule: CapsuleDims = {
    radius: num(c.radius_m, 'capsule.radius_m'),
    heightStand: num(c.height_stand_m, 'capsule.height_stand_m'),
    heightCrouch: num(c.height_crouch_m, 'capsule.height_crouch_m'),
    eyeStand: num(c.eye_height_stand_m, 'capsule.eye_height_stand_m'),
    eyeCrouch: num(c.eye_height_crouch_m, 'capsule.eye_height_crouch_m'),
  };
  const weapons: WeaponActorDef[] = [];
  for (const [id, m] of Object.entries(models.weapons)) {
    const w = json<{ faction: string; gameplay: { ads_ms: number; reload?: { tactical_ms: number; empty_ms: number }; ballistics: { muzzle_velocity_mps: number } }; fx?: { muzzle_flash_ms?: number; tracer?: boolean } }>(src.weaponDef(id));
    if (w.faction !== 'us' && w.faction !== 'ger') throw new Error(`actor data: weapon ${id} has unknown faction ${w.faction}`);
    weapons.push({
      index: src.weaponIndex(id),
      id,
      faction: w.faction,
      requirementId: m.requirement_id,
      muzzleFlashMs: w.fx?.muzzle_flash_ms ?? 50,
      tracer: w.fx?.tracer ?? false,
      muzzleVelocityMps: num(w.gameplay.ballistics.muzzle_velocity_mps, `${id} muzzle velocity`),
      adsMs: num(w.gameplay.ads_ms, `${id} ads_ms`),
      reloadMs: w.gameplay.reload && {
        tactical: num(w.gameplay.reload.tactical_ms, `${id} reload.tactical_ms`),
        empty: num(w.gameplay.reload.empty_ms, `${id} reload.empty_ms`),
      },
      dims: {
        length: num(m.length_m, `${id}.length_m`),
        barrelLength: num(m.barrel_length_m, `${id}.barrel_length_m`),
        barrelDiameter: num(m.barrel_diameter_m, `${id}.barrel_diameter_m`),
        receiverHeight: num(m.receiver_height_m, `${id}.receiver_height_m`),
        receiverWidth: num(m.receiver_width_m, `${id}.receiver_width_m`),
        magLength: num(m.magazine_length_m, `${id}.magazine_length_m`),
        magHeight: num(m.magazine_height_m, `${id}.magazine_height_m`),
        magWidth: num(m.magazine_width_m, `${id}.magazine_width_m`),
      },
      view: { hipOffset: vec3(m.view.hip_offset_m, `${id}.view.hip_offset_m`), adsOffset: vec3(m.view.ads_offset_m, `${id}.view.ads_offset_m`), adsFovMult: num(m.view.ads_fov_mult, `${id}.view.ads_fov_mult`) },
    });
  }
  return {
    capsule,
    stanceTransitionMs: num(role.movement.stance_transition_ms, 'movement.stance_transition_ms'),
    hipFovDeg: num(models.view.hip_fov_deg, 'view.hip_fov_deg'),
    weapons,
  };
}

export async function fetchWeaponModels(baseUrl: string): Promise<WeaponModelsFile> {
  const r = await fetch(`${baseUrl}data/placeholders/weapon_models.json`);
  if (!r.ok) throw new Error(`weapon_models.json: HTTP ${r.status} (run \`pnpm -C client sync-map\`)`);
  return (await r.json()) as WeaponModelsFile;
}

export async function loadActorData(src: DataSource, baseUrl: string): Promise<ActorData> {
  return buildActorData(src, await fetchWeaponModels(baseUrl));
}
