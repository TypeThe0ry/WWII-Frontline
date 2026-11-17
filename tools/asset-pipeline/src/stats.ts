/**
 * `work/{asset_id}/stats.json` (M1-002): per-LOD triangle / vertex / byte counts and texture formats.
 * `lods[]` uses the field names of ARCHITECTURE 11.3 (`lod`, `tris`, `bytes`) so that M1-005 can copy them into the manifest.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import type { ChainSuffix } from './asset-class.ts';
import type { TexResult } from './textures.ts';

export interface LodStat {
  lod: number;
  tris: number;
  vertices: number;
  bytes: number;
  /** meshoptimizer error relative to the primitive extent / in metres. */
  simplify_error: number;
  simplify_error_m: number;
  seam_lock: 'none' | 'hard' | 'relaxed';
  file: string;
}

export interface ChainStat {
  /** '' | '_fp' | '_tp' (D-FP: first / third person weapon models). */
  suffix: ChainSuffix;
  lods: LodStat[];
  /** max |bbox corner error| / diagonal after quantization. */
  bbox_error: number;
}

export interface CollisionStat {
  mode: 'trimesh' | 'convex';
  tris: number;
  hulls: number;
  source_tris: number;
  bytes: number;
  file: string;
}

export interface Stats {
  schema: 'frontline.asset-stats/1';
  asset_id: string;
  logical_id: string;
  /** Total bytes of the normalized glTF (json + .bin + images): the "original" of the size-ratio check. */
  source_bytes: number;
  /** Chain of the unsuffixed asset or the first chain (`lods` alias for consumers that do not know about chains). */
  lods: LodStat[];
  chains: ChainStat[];
  collision?: CollisionStat;
  textures: TexResult[];
  warnings: string[];
}

export function writeStats(path: string, s: Stats): void {
  writeFileSync(path, JSON.stringify(s, null, 2) + '\n');
}

export function readStats(path: string): Stats {
  return JSON.parse(readFileSync(path, 'utf8')) as Stats;
}

export function statsPath(work: string, assetId: string): string {
  return join(work, assetId, 'stats.json');
}

/** `check-size-ratio`: LOD0 bytes of every chain (+ nothing else) against the source size. */
export function sizeRatio(s: Stats, suffix?: ChainSuffix): number {
  const chain = suffix === undefined ? s.chains[0] : s.chains.find((c) => c.suffix === suffix);
  const lod0 = chain?.lods[0];
  if (!lod0 || s.source_bytes <= 0) return NaN;
  return lod0.bytes / s.source_bytes;
}

/**
 * Regression comparison of two stats documents: numeric fields may drift by `tolerance` (default 1 %), everything else must
 * match. Returns the list of differences (empty = equal).
 */
export function diffStats(a: unknown, b: unknown, tolerance = 0.01, path = ''): string[] {
  if (typeof a === 'number' && typeof b === 'number') {
    const d = Math.abs(a - b);
    return d <= Math.abs(a) * tolerance + 1e-9 ? [] : [`${path}: ${a} vs ${b}`];
  }
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return [`${path}: length ${a.length} vs ${b.length}`];
    return a.flatMap((x, i) => diffStats(x, b[i], tolerance, `${path}[${i}]`));
  }
  if (a && b && typeof a === 'object' && typeof b === 'object') {
    const ka = Object.keys(a as object).sort();
    const kb = Object.keys(b as object).sort();
    if (ka.join() !== kb.join()) return [`${path}: keys ${ka.join()} vs ${kb.join()}`];
    return ka.flatMap((k) => diffStats((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k], tolerance, `${path}.${k}`));
  }
  return a === b ? [] : [`${path}: ${String(a)} vs ${String(b)}`];
}
