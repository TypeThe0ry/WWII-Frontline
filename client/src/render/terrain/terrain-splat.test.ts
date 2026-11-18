import { describe, expect, it } from 'vitest';
import { BUILDING, FOREST, ROAD, WATER } from '../world/veg-mask';
import { LAYER, buildSplat, cellWeights, surfaceAt } from './terrain-splat';
import { normalizeSplat } from './splat-math';

const flat = (): number => 2;
const dom = (x: number, z: number, h: number, slope: number, flags: number): number => {
  const w = new Float32Array(8);
  cellWeights(x, z, h, slope, flags, 1, w);
  const n = new Float32Array(8);
  normalizeSplat(w, n);
  return n.indexOf(Math.max(...n));
};

describe('procedural splat (M1-050b)', () => {
  it('flat open ground is grass-dominated; steep slopes are rock; roads gravel; forests forest floor; water mud', () => {
    expect([LAYER.grass, LAYER.dryGrass]).toContain(dom(100, 100, 5, 2, 0));
    expect(dom(100, 100, 5, 55, 0)).toBe(LAYER.rock);
    expect(dom(100, 100, 5, 2, ROAD)).toBe(LAYER.gravel);
    expect(dom(100, 100, 5, 2, FOREST)).toBe(LAYER.forestFloor);
    expect(dom(100, 100, 5, 2, WATER)).toBe(LAYER.mud);
    expect(dom(100, 100, 5, 2, BUILDING)).toBe(LAYER.dirt);
  });
  it('texel weights sum to 255 (+-4) and fill two RGBA textures; deterministic', () => {
    const m = buildSplat({ sizeM: 64, resM: 2, heightAt: (x, z) => 2 + 0.05 * x + Math.sin(z / 5), flags: (x) => (x > 30 && x < 34 ? ROAD : 0) });
    expect(m.n).toBe(32);
    expect(m.a.length).toBe(32 * 32 * 4);
    for (let t = 0; t < 32 * 32; t++) {
      let s = 0;
      for (let k = 0; k < 4; k++) s += m.a[t * 4 + k]! + m.b[t * 4 + k]!;
      expect(Math.abs(s - 255)).toBeLessThanOrEqual(4);
    }
    expect(buildSplat({ sizeM: 64, resM: 2, heightAt: flat }).a).toEqual(buildSplat({ sizeM: 64, resM: 2, heightAt: flat }).a);
  });
  it('surfaceAt agrees with the dominant layer of the texel (1000 random points)', () => {
    const m = buildSplat({ sizeM: 128, resM: 2, heightAt: (x, z) => 3 + 0.3 * x * Math.sin(z / 9) * 0.1, flags: (x, z) => (Math.floor(x / 20 + z / 30) % 3 === 0 ? ROAD : 0) });
    let seed = 7;
    const r = (): number => ((seed = (Math.imul(seed, 1103515245) + 12345) >>> 0) / 4294967296);
    for (let q = 0; q < 1000; q++) {
      const x = r() * 128;
      const z = r() * 128;
      const o = (Math.floor(z / 2) * m.n + Math.floor(x / 2)) * 4;
      const arr = [...m.a.subarray(o, o + 4), ...m.b.subarray(o, o + 4)];
      expect(arr[surfaceAt(m, x, z)]).toBe(Math.max(...arr));
    }
  });
});
