/**
 * Per-asset processing parameters (`asset-class.json`, schema `asset-class.schema.json`), M1-002.
 * Validated with ajv (the version already used by tools/data-validate).
 */
import { readFileSync } from 'node:fs';
import { Ajv2020 } from 'ajv/dist/2020.js';
import { ExitCode, GateError } from './errors.ts';

export type TextureProfile = 'bg' | 'hero' | 'ui' | 'normal';
export type AssetKind = 'prop' | 'building' | 'vehicle' | 'weapon' | 'character';
export type CollisionMode = 'trimesh' | 'convex';
export type ChainSuffix = '' | '_fp' | '_tp';

export interface CollisionSpec {
  mode: CollisionMode;
  max_tris?: number;
  max_hulls?: number;
  max_hull_vertices?: number;
}
export interface VariantSpec {
  suffix: '_fp' | '_tp';
  input?: string;
  lod_ratios?: number[];
}
export interface AssetClass {
  class?: AssetKind;
  texture_profile?: TextureProfile;
  textures?: Record<string, TextureProfile>;
  lod_ratios?: number[];
  collision?: CollisionSpec;
  first_person?: boolean;
  variants?: VariantSpec[];
}

/** ARCHITECTURE 11.1 step 3. The card forbids loosening these ("LOD ratios must not be relaxed"). */
export const DEFAULT_LOD_RATIOS: readonly number[] = [1, 0.5, 0.25, 0.1];
/** Trimesh budgets by class (card: vehicle 400, building 800; the rest 400 [to be tuned]). */
export const DEFAULT_COLLISION_TRIS: Record<AssetKind, number> = { vehicle: 400, building: 800, prop: 400, weapon: 400, character: 400 };
export const MAX_HULLS = 32;

const schema = JSON.parse(readFileSync(new URL('../asset-class.schema.json', import.meta.url), 'utf8')) as object;
const ajv = new Ajv2020({ allErrors: true, strict: true, useDefaults: false });
const validate = ajv.compile<AssetClass>(schema);

export function checkRatios(r: readonly number[], where: string): void {
  if (r[0] !== 1) throw new GateError(ExitCode.Usage, `${where}: lod_ratios must start with 1`);
  for (let i = 1; i < r.length; i++) {
    if (!((r[i] as number) < (r[i - 1] as number))) throw new GateError(ExitCode.Usage, `${where}: lod_ratios must be strictly decreasing`);
  }
}

/** Validate an already parsed asset-class document; throws GateError(Usage) listing every schema violation. */
export function parseAssetClass(json: unknown, where = 'asset-class'): AssetClass {
  if (!validate(json)) {
    const msg = (validate.errors ?? []).map((e) => `${e.instancePath || '/'} ${e.message ?? 'invalid'}`).join('; ');
    throw new GateError(ExitCode.Usage, `${where}: ${msg}`);
  }
  const cls = json as AssetClass;
  if (cls.lod_ratios) checkRatios(cls.lod_ratios, where);
  for (const v of cls.variants ?? []) if (v.lod_ratios) checkRatios(v.lod_ratios, `${where} variant ${v.suffix}`);
  if (cls.variants && new Set(cls.variants.map((v) => v.suffix)).size !== cls.variants.length) {
    throw new GateError(ExitCode.Usage, `${where}: duplicate variant suffix`);
  }
  return cls;
}

export function loadAssetClass(path: string): AssetClass {
  let text: string;
  try {
    text = readFileSync(path, 'utf8');
  } catch {
    throw new GateError(ExitCode.Usage, `${path}: cannot read asset-class file`);
  }
  let json: unknown;
  try {
    json = JSON.parse(text);
  } catch (e) {
    throw new GateError(ExitCode.Usage, `${path}: invalid JSON (${(e as Error).message})`);
  }
  return parseAssetClass(json, path);
}

/** Chains to produce: explicit variants, else a single chain (`_fp` when `first_person`). */
export function chainsOf(cls: AssetClass): { suffix: ChainSuffix; input?: string; lodRatios: number[] }[] {
  const base = cls.lod_ratios ?? [...DEFAULT_LOD_RATIOS];
  if (cls.variants?.length) {
    return cls.variants.map((v) => ({ suffix: v.suffix, ...(v.input ? { input: v.input } : {}), lodRatios: v.lod_ratios ?? base }));
  }
  return [{ suffix: cls.first_person ? '_fp' : '', lodRatios: base }];
}
