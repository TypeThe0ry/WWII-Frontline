import { describe, expect, it } from 'vitest';
import { MAX_SLOPE_DEG, POS_STEPS_PER_M, SHRUB_SPACING_M, SPAWN_EXCLUSION_M, TREE_SPACING_M, packInstance, scatterChunk, unpackInstance, type ScatterInput } from './scatter';
import { CHUNK_SIZE_M, INSTANCE_BYTES, SPECIES, SPECIES_GRASS, SPECIES_SHRUB, SPECIES_TREE } from './species';

const flat = (): number => 2;
const input = (cx: number, cz: number, extra: Partial<ScatterInput> = {}): ScatterInput => ({ chunkX: cx, chunkZ: cz, seed: 1, heightAt: flat, ...extra });

interface World {
  x: number;
  z: number;
  sp: number;
}
function decodeWorld(cx: number, cz: number, bytes: Uint8Array): World[] {
  const out: World[] = [];
  for (let o = 0; o < bytes.length; o += INSTANCE_BYTES) {
    const d = unpackInstance(bytes, o);
    out.push({ x: cx * CHUNK_SIZE_M + d.x, z: cz * CHUNK_SIZE_M + d.z, sp: d.species });
  }
  return out;
}
const count = (b: Uint8Array, sp: number): number => decodeWorld(0, 0, b).filter((w) => w.sp === sp).length;

describe('scatterChunk', () => {
  it('is byte-for-byte deterministic for the same seed and chunk, and differs between seeds/chunks', () => {
    const a = scatterChunk(input(1, 2));
    const b = scatterChunk(input(1, 2));
    expect(a.length).toBeGreaterThan(0);
    expect(Array.from(a)).toEqual(Array.from(b));
    expect(Array.from(scatterChunk(input(1, 2, { seed: 2 })))).not.toEqual(Array.from(a));
    expect(Array.from(scatterChunk(input(2, 1)))).not.toEqual(Array.from(a));
  });

  it('is deterministic across every chunk of the M0 block (hash over all 16 tables is stable)', () => {
    const run = () => {
      const parts: number[] = [];
      for (let cz = 0; cz < 4; cz++) for (let cx = 0; cx < 4; cx++) parts.push(...scatterChunk(input(cx, cz)));
      return parts;
    };
    expect(run()).toEqual(run());
  });

  it('emits whole 6-byte entries and respects the per-chunk caps 4096 / 256 / 192', () => {
    let maxG = 0;
    let maxS = 0;
    let maxT = 0;
    for (let cz = 0; cz < 4; cz++) {
      for (let cx = 0; cx < 4; cx++) {
        const t = scatterChunk(input(cx, cz));
        expect(t.length % INSTANCE_BYTES).toBe(0);
        maxG = Math.max(maxG, count(t, SPECIES_GRASS));
        maxS = Math.max(maxS, count(t, SPECIES_SHRUB));
        maxT = Math.max(maxT, count(t, SPECIES_TREE));
      }
    }
    expect(INSTANCE_BYTES).toBe(6);
    expect(maxG).toBeLessThanOrEqual(SPECIES[SPECIES_GRASS]!.cap);
    expect(maxS).toBeLessThanOrEqual(SPECIES[SPECIES_SHRUB]!.cap);
    expect(maxT).toBeLessThanOrEqual(SPECIES[SPECIES_TREE]!.cap);
    expect(maxG).toBeGreaterThan(200);
    expect(maxT).toBeGreaterThan(5);
    console.log(`scatter 16 chunks, flat ground: max grass ${maxG}, shrub ${maxS}, tree ${maxT}`);
  });

  it('the shrub cap really bites: some chunk has more candidates than 256 and is cut to exactly 256', () => {
    let atCap = 0;
    for (let cz = 0; cz < 4; cz++) for (let cx = 0; cx < 4; cx++) if (count(scatterChunk(input(cx, cz)), SPECIES_SHRUB) === 256) atCap++;
    expect(atCap).toBeGreaterThan(0);
  });

  it('keeps every instance inside its chunk', () => {
    const t = scatterChunk(input(3, 1));
    for (const w of decodeWorld(3, 1, t)) {
      expect(w.x).toBeGreaterThanOrEqual(3 * CHUNK_SIZE_M);
      expect(w.x).toBeLessThan(4 * CHUNK_SIZE_M);
      expect(w.z).toBeGreaterThanOrEqual(CHUNK_SIZE_M);
      expect(w.z).toBeLessThan(2 * CHUNK_SIZE_M);
    }
  });

  it(`places nothing on slopes steeper than ${MAX_SLOPE_DEG} degrees`, () => {
    // x < 32: 40 deg ramp; x >= 32: 20 deg ramp (continuous at x = 32 is not needed for the test)
    const tan = (d: number) => Math.tan((d * Math.PI) / 180);
    const h = (x: number): number => (x < 32 ? tan(40) * x : tan(20) * (x - 32));
    const t = scatterChunk(input(0, 0, { heightAt: (x) => h(x) }));
    const w = decodeWorld(0, 0, t);
    expect(w.length).toBeGreaterThan(100);
    // 1 m margin around the ridge/edge where the central difference straddles both ramps
    expect(w.filter((p) => p.x < 31.5).length).toBe(0);
    expect(w.filter((p) => p.x > 32.5).length).toBeGreaterThan(100);
  });

  it(`keeps ${SPAWN_EXCLUSION_M} m clear around spawn points`, () => {
    const spawns = [
      { x: 20, z: 20 },
      { x: 40.3, z: 33.1 },
    ];
    const free = scatterChunk(input(0, 0));
    const near = (w: World, p: { x: number; z: number }) => Math.hypot(w.x - p.x, w.z - p.z) < SPAWN_EXCLUSION_M;
    expect(decodeWorld(0, 0, free).filter((w) => spawns.some((p) => near(w, p))).length).toBeGreaterThan(20); // sanity: they would be there
    const t = scatterChunk(input(0, 0, { exclusions: spawns }));
    expect(decodeWorld(0, 0, t).filter((w) => spawns.some((p) => near(w, p))).length).toBe(0);
  });

  it(`keeps trees ${TREE_SPACING_M} m and shrubs ${SHRUB_SPACING_M} m apart, also across chunk borders`, () => {
    const all: World[] = [];
    for (let cz = 0; cz < 2; cz++) for (let cx = 0; cx < 2; cx++) all.push(...decodeWorld(cx, cz, scatterChunk(input(cx, cz))));
    const trees = all.filter((w) => w.sp === SPECIES_TREE);
    const shrubs = all.filter((w) => w.sp === SPECIES_SHRUB);
    expect(trees.length).toBeGreaterThan(20);
    expect(shrubs.length).toBeGreaterThan(40);
    const minDist = (a: World[], b: World[], same: boolean): number => {
      let m = Infinity;
      for (let i = 0; i < a.length; i++) for (let j = same ? i + 1 : 0; j < b.length; j++) m = Math.min(m, Math.hypot(a[i]!.x - b[j]!.x, a[i]!.z - b[j]!.z));
      return m;
    };
    const mt = minDist(trees, trees, true);
    const ms = minDist(shrubs, shrubs, true);
    const mst = minDist(shrubs, trees, false);
    console.log(`min spacing over a 2x2 chunk area: tree-tree ${mt.toFixed(3)} m, shrub-shrub ${ms.toFixed(3)} m, shrub-tree ${mst.toFixed(3)} m`);
    expect(mt).toBeGreaterThanOrEqual(TREE_SPACING_M);
    expect(ms).toBeGreaterThanOrEqual(SHRUB_SPACING_M);
    expect(mst).toBeGreaterThanOrEqual(SHRUB_SPACING_M);
    // the chunk seam is crossed: some pair of neighbours straddles x = 64
    expect(trees.some((a) => a.x < 64 && trees.some((b) => b.x >= 64 && Math.hypot(a.x - b.x, a.z - b.z) < 12))).toBe(true);
  });

  it('clusters grass into patches (density varies strongly between 8 m tiles)', () => {
    const w = decodeWorld(0, 0, scatterChunk(input(0, 0))).filter((p) => p.sp === SPECIES_GRASS);
    const tiles = new Array<number>(64).fill(0);
    for (const p of w) tiles[Math.floor(p.z / 8) * 8 + Math.floor(p.x / 8)]! += 1;
    const mean = w.length / 64;
    const sd = Math.sqrt(tiles.reduce((s, v) => s + (v - mean) ** 2, 0) / 64);
    expect(sd / mean).toBeGreaterThan(0.35);
  });
});

describe('instance entry packing', () => {
  it('round-trips 12-bit positions, species, scale and rot in 6 bytes', () => {
    const b = new Uint8Array(6);
    for (const [x, z] of [[0, 0], [4095, 4095], [1234, 3210], [1, 4094]] as const) {
      packInstance(b, 0, x, z, 2, 200, 17);
      const d = unpackInstance(b, 0);
      expect(d.x).toBe(x / POS_STEPS_PER_M);
      expect(d.z).toBe(z / POS_STEPS_PER_M);
      expect([d.species, d.scaleByte, d.rotByte]).toEqual([2, 200, 17]);
    }
    expect(POS_STEPS_PER_M * CHUNK_SIZE_M).toBe(4096);
  });
});
