/**
 * Procedural placeholder splat map (M1-050b): 8 layer weights per cell, packed into two RGBA8 textures (layers 0-3 / 4-7), generated
 * from the height field and the map layers. Real KTX2 layers and a baked splat wait on M1-064 / `terrain_layers.json` (PI-113); the
 * shader (terrain-material.*) is written against this interface, so swapping the source changes nothing downstream.
 * Pure and deterministic (integer hash noise only).
 */
import { BLOCK, BUILDING, FOREST, ROAD, WATER } from '../world/veg-mask';
import { LAYERS, normalizeSplat, slopeDeg } from './splat-math';

export const LAYER = { grass: 0, dryGrass: 1, dirt: 2, mud: 3, rock: 4, gravel: 5, sand: 6, forestFloor: 7 } as const;

export interface SplatInput {
  /** Map edge (m); cell centres at (i + 0.5) * resM. */
  sizeM: number;
  resM: number;
  heightAt(x: number, z: number): number;
  /** Optional map layers (flags from `veg-mask.ts`). */
  flags?(x: number, z: number): number;
  seed?: number;
}

export interface SplatMaps {
  /** Texels per side. */
  n: number;
  resM: number;
  sizeM: number;
  /** RGBA8, layers 0-3 and 4-7, row j = z, column i = x, weights quantised to 0..255 (the two texels of a cell sum to 255 +- 4). */
  a: Uint8Array;
  b: Uint8Array;
}

function hash2(x: number, z: number, seed: number): number {
  let h = Math.imul(x | 0, 0x9e3779b1) ^ Math.imul(z | 0, 0x85ebca6b) ^ Math.imul(seed | 0, 0xc2b2ae35);
  h ^= h >>> 15;
  h = Math.imul(h, 0x2c1b3c6d);
  h ^= h >>> 12;
  return (h >>> 0) / 4294967296;
}

const smooth = (t: number): number => t * t * (3 - 2 * t);
const step = (e0: number, e1: number, x: number): number => smooth(Math.min(1, Math.max(0, (x - e0) / (e1 - e0))));

function noise(x: number, z: number, scale: number, seed: number): number {
  const u = x / scale;
  const v = z / scale;
  const iu = Math.floor(u);
  const iv = Math.floor(v);
  const fu = smooth(u - iu);
  const fv = smooth(v - iv);
  const a = hash2(iu, iv, seed);
  const b = hash2(iu + 1, iv, seed);
  const c = hash2(iu, iv + 1, seed);
  const d = hash2(iu + 1, iv + 1, seed);
  return (a + (b - a) * fu) + ((c + (d - c) * fu) - (a + (b - a) * fu)) * fv;
}

/** Raw (un-normalised) layer weights of one cell. Exposed for the unit tests. */
export function cellWeights(x: number, z: number, h: number, slopeD: number, flags: number, seed: number, out: Float32Array): void {
  out.fill(0);
  const patch = noise(x, z, 22, seed + 1);
  const wet = noise(x, z, 40, seed + 2);
  out[LAYER.grass] = 0.55 + 0.45 * (1 - patch);
  out[LAYER.dryGrass] = 0.9 * step(0.5, 0.8, patch);
  out[LAYER.dirt] = 0.5 * step(6, 22, slopeD) + 0.1 * noise(x, z, 9, seed + 3);
  out[LAYER.mud] = 0.7 * step(0.55, 0.85, wet) * (1 - step(1.5, 3.5, h)); // low, wet ground
  out[LAYER.rock] = 1.4 * step(30, 44, slopeD);
  out[LAYER.sand] = 0.15 * step(0.7, 0.95, noise(x, z, 31, seed + 4));
  if (flags & FOREST) {
    out[LAYER.forestFloor] = 1.6;
    out[LAYER.grass] = out[LAYER.grass]! * 0.25;
    out[LAYER.dryGrass] = out[LAYER.dryGrass]! * 0.25;
  }
  if (flags & BUILDING) out[LAYER.dirt] = out[LAYER.dirt]! + 1.5; // yards and ruins
  if (flags & WATER) {
    out[LAYER.mud] = out[LAYER.mud]! + 2.5;
    out[LAYER.sand] = out[LAYER.sand]! + 0.8;
  }
  if (flags & ROAD) out[LAYER.gravel] = 4;
  void BLOCK;
}

export function buildSplat(inp: SplatInput): SplatMaps {
  const n = Math.ceil(inp.sizeM / inp.resM);
  const a = new Uint8Array(n * n * 4);
  const b = new Uint8Array(n * n * 4);
  const w = new Float32Array(LAYERS);
  const nw = new Float32Array(LAYERS);
  const seed = inp.seed ?? 1;
  const e = inp.resM; // central-difference half span = 1 cell
  for (let j = 0; j < n; j++) {
    for (let i = 0; i < n; i++) {
      const x = (i + 0.5) * inp.resM;
      const z = (j + 0.5) * inp.resM;
      const h = inp.heightAt(x, z);
      const gx = (inp.heightAt(x + e, z) - inp.heightAt(x - e, z)) / (2 * e);
      const gz = (inp.heightAt(x, z + e) - inp.heightAt(x, z - e)) / (2 * e);
      const ny = 1 / Math.sqrt(1 + gx * gx + gz * gz);
      cellWeights(x, z, h, slopeDeg(ny), inp.flags ? inp.flags(x, z) : 0, seed, w);
      normalizeSplat(w, nw);
      const o = (j * n + i) * 4;
      for (let k = 0; k < 4; k++) {
        a[o + k] = Math.round(nw[k]! * 255);
        b[o + k] = Math.round(nw[4 + k]! * 255);
      }
    }
  }
  return { n, resM: inp.resM, sizeM: inp.sizeM, a, b };
}

/** Dominant layer of the texel containing (x, z): the CPU twin of `getSurfaceAt` (footsteps, impacts). */
export function surfaceAt(m: SplatMaps, x: number, z: number): number {
  const i = Math.min(m.n - 1, Math.max(0, Math.floor(x / m.resM)));
  const j = Math.min(m.n - 1, Math.max(0, Math.floor(z / m.resM)));
  const o = (j * m.n + i) * 4;
  let best = 0;
  let bw = -1;
  for (let k = 0; k < LAYERS; k++) {
    const v = k < 4 ? m.a[o + k]! : m.b[o + k - 4]!;
    if (v > bw) {
      bw = v;
      best = k;
    }
  }
  return best;
}
