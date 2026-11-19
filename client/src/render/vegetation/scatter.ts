/**
 * Vegetation scatter (M0-024, ARCHITECTURE §9.6): pure, deterministic generation of one chunk's instance table.
 * M1-035 replaces the source with map-pipeline baked tables; the interface is the table bytes
 * (`INSTANCE_BYTES` = 6 per entry, layout at `packInstance`; x/z relative to the chunk origin in 1/64 m), so nothing
 * downstream depends on how the bytes were made.
 *
 * Determinism: only integer hashing, + - * / and Math.floor/sqrt are used (all exactly specified by IEEE-754), no
 * Math.sin/cos/random, so the same (seed, chunk) gives the same bytes on every engine. Candidates live on a
 * *global* cell grid, so patterns continue across chunk borders and the spacing rules hold across them.
 */
import { CHUNK_SIZE_M, INSTANCE_BYTES, SPECIES, SPECIES_GRASS, SPECIES_SHRUB, SPECIES_TREE, type SpeciesId } from './species';

export interface ScatterPoint {
  x: number;
  z: number;
}

export interface ScatterInput {
  chunkX: number;
  chunkZ: number;
  seed: number;
  /** Terrain surface height at world (x, z). */
  heightAt: (x: number, z: number) => number;
  /** Spawn points: nothing grows within `exclusionRadius` of them. */
  exclusions?: readonly ScatterPoint[];
  exclusionRadius?: number;
  /** Map-derived placement mask (M1-051b): nothing grows on blocked cells; trees concentrate in forest cells. */
  mask?: { blocked(x: number, z: number): boolean; forest(x: number, z: number): boolean };
  /** Multiplier on the grass keep probability (default 1; the M1 world scene thins grass to bound memory). */
  grassScale?: number;
  /** Emit only this species, preserving the corresponding entries in a full table. */
  species?: SpeciesId;
}

/** Probability multiplier for trees outside forest cells when a mask is given (single trees, hedgerow trees come later). */
export const TREE_OUTSIDE_FOREST = 0.12;
/** Keep probability of a tree candidate inside a forest cell when a mask is given. */
export const TREE_IN_FOREST_P = 0.9;

export const MAX_SLOPE_DEG = 30;
export const SPAWN_EXCLUSION_M = 6;
export const TREE_SPACING_M = 4;
export const SHRUB_SPACING_M = 1.5;
/** tan(30 deg), compared against the height gradient magnitude (no atan: keeps the function exactly portable). */
const TAN_MAX_SLOPE = 0.5773502691896257;
const GRASS_CELL_M = 0.75;

/**
 * Entry layout, 6 B (PI-017: §9.6 lists x:u16 z:u16 species:u8 scale:u8 rot:u8, which is 7 B, so x and z are 12 bit):
 *   b0 = x[7:0]   b1 = x[11:8] | z[3:0] << 4   b2 = z[11:4]   b3 = species   b4 = scale   b5 = rot
 * x, z in 1/64 m steps from the chunk origin (1.5625 cm; chunk edge 64 m = 4096 steps).
 */
export const POS_STEPS_PER_M = 64;

export function packInstance(out: Uint8Array, o: number, x12: number, z12: number, species: number, scale: number, rot: number): void {
  out[o] = x12 & 0xff;
  out[o + 1] = ((x12 >> 8) & 0xf) | ((z12 & 0xf) << 4);
  out[o + 2] = (z12 >> 4) & 0xff;
  out[o + 3] = species;
  out[o + 4] = scale;
  out[o + 5] = rot;
}

export interface DecodedInstance {
  /** Offset from the chunk origin, metres. */
  x: number;
  z: number;
  species: number;
  scaleByte: number;
  rotByte: number;
}

export function unpackInstance(t: Uint8Array, o: number): DecodedInstance {
  const x12 = t[o]! | ((t[o + 1]! & 0xf) << 8);
  const z12 = (t[o + 1]! >> 4) | (t[o + 2]! << 4);
  return { x: x12 / POS_STEPS_PER_M, z: z12 / POS_STEPS_PER_M, species: t[o + 3]!, scaleByte: t[o + 4]!, rotByte: t[o + 5]! };
}

/** 32-bit integer mixer (murmur3 finalizer) over four ints. */
export function hash4(a: number, b: number, c: number, d: number): number {
  let h = Math.imul(a | 0, 0x9e3779b1) ^ Math.imul(b | 0, 0x85ebca6b) ^ Math.imul(c | 0, 0xc2b2ae35) ^ Math.imul(d | 0, 0x27d4eb2f);
  h ^= h >>> 16;
  h = Math.imul(h, 0x85ebca6b);
  h ^= h >>> 13;
  h = Math.imul(h, 0xc2b2ae35);
  h ^= h >>> 16;
  return h >>> 0;
}

/** Uniform [0,1) from a 32-bit hash. */
export function unit(h: number): number {
  return h / 4294967296;
}

const smooth = (t: number): number => {
  const c = Math.min(1, Math.max(0, t));
  return c * c * (3 - 2 * c);
};

/** Smooth lattice value noise in [0,1) with lattice spacing `scale` metres. */
export function valueNoise(x: number, z: number, scale: number, seed: number, salt: number): number {
  const u = x / scale;
  const v = z / scale;
  const iu = Math.floor(u);
  const iv = Math.floor(v);
  const fu = smooth(u - iu);
  const fv = smooth(v - iv);
  const n00 = unit(hash4(iu, iv, seed, salt));
  const n10 = unit(hash4(iu + 1, iv, seed, salt));
  const n01 = unit(hash4(iu, iv + 1, seed, salt));
  const n11 = unit(hash4(iu + 1, iv + 1, seed, salt));
  const a = n00 + (n10 - n00) * fu;
  const b = n01 + (n11 - n01) * fu;
  return a + (b - a) * fv;
}

function fbm(x: number, z: number, scale: number, seed: number, salt: number): number {
  return (valueNoise(x, z, scale, seed, salt) * 2 + valueNoise(x, z, scale * 0.37, seed, salt + 101)) / 3;
}

interface Candidate {
  x: number;
  z: number;
  /** Selection priority in [0,1): higher wins spacing conflicts and survives the per-chunk cap. */
  prio: number;
  scale: number;
  rot: number;
  cellX: number;
  cellZ: number;
}

export interface SlopeTester {
  /** True when the terrain at (x,z) is flatter than MAX_SLOPE_DEG and not near a spawn point. */
  ok(x: number, z: number): boolean;
  forest?(x: number, z: number): boolean;
  grassScale?: number;
}

function makeTester(input: ScatterInput): SlopeTester {
  const ex = input.exclusions ?? [];
  const r = input.exclusionRadius ?? SPAWN_EXCLUSION_M;
  const r2 = r * r;
  const h = input.heightAt;
  const mask = input.mask;
  return {
    forest: mask ? (x, z) => mask.forest(x, z) : undefined,
    grassScale: input.grassScale,
    ok(x, z) {
      if (mask && mask.blocked(x, z)) return false;
      for (const p of ex) {
        const dx = x - p.x;
        const dz = z - p.z;
        if (dx * dx + dz * dz < r2) return false;
      }
      const gx = (h(x + 0.5, z) - h(x - 0.5, z)) / 1.0;
      const gz = (h(x, z + 0.5) - h(x, z - 0.5)) / 1.0;
      return gx * gx + gz * gz <= TAN_MAX_SLOPE * TAN_MAX_SLOPE;
    },
  };
}

/** Candidate of one global grid cell for a species, or null (density gate / slope / spawn exclusion). */
function candidateAt(species: number, cs: number, cx: number, cz: number, seed: number, tester: SlopeTester): Candidate | null {
  const h0 = hash4(cx, cz, seed, species * 16 + 1);
  // Snapped to the table's 1/64 m grid up front, so the spacing rules hold for the stored (quantised) positions.
  const x = Math.floor((cx + unit(h0)) * cs * POS_STEPS_PER_M) / POS_STEPS_PER_M;
  const z = Math.floor((cz + unit(hash4(cx, cz, seed, species * 16 + 2))) * cs * POS_STEPS_PER_M) / POS_STEPS_PER_M;
  const keep = unit(hash4(cx, cz, seed, species * 16 + 3));
  let p: number;
  if (species === SPECIES_GRASS) {
    // clustered meadows: patches ~14 m wide with bare gaps
    const d = smooth((fbm(x, z, 14, seed, 11) - 0.32) / 0.38);
    p = (0.06 + 0.5 * d) * (tester.grassScale ?? 1);
  } else if (species === SPECIES_TREE) {
    // copses: ~35 m wide, a few scattered singles elsewhere
    const d = smooth((fbm(x, z, 35, seed, 21) - 0.5) / 0.2);
    p = 0.05 + 0.85 * d;
    if (tester.forest) p = tester.forest(x, z) ? TREE_IN_FOREST_P : p * TREE_OUTSIDE_FOREST;
  } else {
    // shrubs: loose scatter, denser at copse edges
    const d = smooth((fbm(x, z, 18, seed, 31) - 0.4) / 0.3);
    p = 0.12 + 0.6 * d;
  }
  if (keep >= p) return null;
  if (!tester.ok(x, z)) return null;
  return {
    x,
    z,
    prio: unit(hash4(cx, cz, seed, species * 16 + 4)),
    scale: hash4(cx, cz, seed, species * 16 + 5) & 255,
    rot: hash4(cx, cz, seed, species * 16 + 6) & 255,
    cellX: cx,
    cellZ: cz,
  };
}

/** Memoised candidate lookup + Poisson-disk acceptance (a candidate survives iff no higher-priority candidate lies within `r`). */
class PoissonField {
  private readonly cands = new Map<string, Candidate | null>();
  private readonly acc = new Map<string, boolean>();
  readonly cs: number;
  constructor(
    private readonly species: number,
    private readonly r: number,
    private readonly seed: number,
    private readonly tester: SlopeTester,
  ) {
    this.cs = r / Math.SQRT2;
  }
  private key(cx: number, cz: number): string {
    return `${cx},${cz}`;
  }
  cand(cx: number, cz: number): Candidate | null {
    const k = this.key(cx, cz);
    let c = this.cands.get(k);
    if (c === undefined) {
      c = candidateAt(this.species, this.cs, cx, cz, this.seed, this.tester);
      this.cands.set(k, c);
    }
    return c;
  }
  accepted(cx: number, cz: number): Candidate | null {
    const c = this.cand(cx, cz);
    if (!c) return null;
    const k = this.key(cx, cz);
    let ok = this.acc.get(k);
    if (ok === undefined) {
      ok = true;
      const r2 = this.r * this.r;
      // Two points closer than r are at most 2 cells apart in each axis (cell = r/sqrt2).
      for (let dz = -2; dz <= 2 && ok; dz++) {
        for (let dx = -2; dx <= 2; dx++) {
          if (dx === 0 && dz === 0) continue;
          const o = this.cand(cx + dx, cz + dz);
          if (!o) continue;
          const ddx = o.x - c.x;
          const ddz = o.z - c.z;
          if (ddx * ddx + ddz * ddz >= r2) continue;
          if (o.prio > c.prio || (o.prio === c.prio && (o.cellZ > c.cellZ || (o.cellZ === c.cellZ && o.cellX > c.cellX)))) {
            ok = false;
            break;
          }
        }
      }
      this.acc.set(k, ok);
    }
    return ok ? c : null;
  }
}

function inChunk(c: Candidate, ox: number, oz: number): boolean {
  return c.x >= ox && c.x < ox + CHUNK_SIZE_M && c.z >= oz && c.z < oz + CHUNK_SIZE_M;
}

function cap(list: Candidate[], max: number): Candidate[] {
  if (list.length <= max) return list;
  // Keep the highest priorities; ties broken by cell so the result is deterministic.
  const sorted = [...list].sort((a, b) => b.prio - a.prio || a.cellZ - b.cellZ || a.cellX - b.cellX);
  const keep = new Set(sorted.slice(0, max));
  return list.filter((c) => keep.has(c));
}

/**
 * Generate the instance table of chunk (chunkX, chunkZ): grass first, then shrubs, then trees, each in a fixed
 * (z-major, x-minor) cell order. Returns `INSTANCE_BYTES * n` bytes.
 */
export function scatterChunk(input: ScatterInput): Uint8Array {
  const ox = input.chunkX * CHUNK_SIZE_M;
  const oz = input.chunkZ * CHUNK_SIZE_M;
  const seed = input.seed | 0;
  const tester = makeTester(input);

  const out: { sp: number; c: Candidate }[] = [];

  // Grass: independent jittered 0.75 m grid with clustered density.
  if (input.species === undefined || input.species === SPECIES_GRASS) {
    const list: Candidate[] = [];
    const x0 = Math.floor(ox / GRASS_CELL_M);
    const x1 = Math.floor((ox + CHUNK_SIZE_M) / GRASS_CELL_M);
    const z0 = Math.floor(oz / GRASS_CELL_M);
    const z1 = Math.floor((oz + CHUNK_SIZE_M) / GRASS_CELL_M);
    for (let cz = z0; cz <= z1; cz++) {
      for (let cx = x0; cx <= x1; cx++) {
        const c = candidateAt(SPECIES_GRASS, GRASS_CELL_M, cx, cz, seed, tester);
        if (c && inChunk(c, ox, oz)) list.push(c);
      }
    }
    for (const c of cap(list, SPECIES[SPECIES_GRASS]!.cap)) out.push({ sp: SPECIES_GRASS, c });
  }

  const trees = new PoissonField(SPECIES_TREE, TREE_SPACING_M, seed, tester);
  const shrubs = new PoissonField(SPECIES_SHRUB, SHRUB_SPACING_M, seed, tester);

  // Shrubs: Poisson disk 1.5 m, also kept 1.5 m away from tree trunks.
  if (input.species === undefined || input.species === SPECIES_SHRUB) {
    const list: Candidate[] = [];
    const cs = shrubs.cs;
    const x0 = Math.floor(ox / cs);
    const x1 = Math.floor((ox + CHUNK_SIZE_M) / cs);
    const z0 = Math.floor(oz / cs);
    const z1 = Math.floor((oz + CHUNK_SIZE_M) / cs);
    const tcs = trees.cs;
    const r2 = SHRUB_SPACING_M * SHRUB_SPACING_M;
    for (let cz = z0; cz <= z1; cz++) {
      for (let cx = x0; cx <= x1; cx++) {
        const c = shrubs.accepted(cx, cz);
        if (!c || !inChunk(c, ox, oz)) continue;
        let blocked = false;
        const tx = Math.floor(c.x / tcs);
        const tz = Math.floor(c.z / tcs);
        for (let dz = -1; dz <= 1 && !blocked; dz++) {
          for (let dx = -1; dx <= 1; dx++) {
            const t = trees.accepted(tx + dx, tz + dz);
            if (!t) continue;
            const ddx = t.x - c.x;
            const ddz = t.z - c.z;
            if (ddx * ddx + ddz * ddz < r2) {
              blocked = true;
              break;
            }
          }
        }
        if (!blocked) list.push(c);
      }
    }
    for (const c of cap(list, SPECIES[SPECIES_SHRUB]!.cap)) out.push({ sp: SPECIES_SHRUB, c });
  }

  // Trees: Poisson disk 4 m.
  if (input.species === undefined || input.species === SPECIES_TREE) {
    const list: Candidate[] = [];
    const cs = trees.cs;
    const x0 = Math.floor(ox / cs);
    const x1 = Math.floor((ox + CHUNK_SIZE_M) / cs);
    const z0 = Math.floor(oz / cs);
    const z1 = Math.floor((oz + CHUNK_SIZE_M) / cs);
    for (let cz = z0; cz <= z1; cz++) {
      for (let cx = x0; cx <= x1; cx++) {
        const c = trees.accepted(cx, cz);
        if (c && inChunk(c, ox, oz)) list.push(c);
      }
    }
    for (const c of cap(list, SPECIES[SPECIES_TREE]!.cap)) out.push({ sp: SPECIES_TREE, c });
  }

  const bytes = new Uint8Array(out.length * INSTANCE_BYTES);
  for (let i = 0; i < out.length; i++) {
    const { sp, c } = out[i]!;
    packInstance(bytes, i * INSTANCE_BYTES, Math.floor((c.x - ox) * POS_STEPS_PER_M), Math.floor((c.z - oz) * POS_STEPS_PER_M), sp, c.scale, c.rot);
  }
  return bytes;
}
