import { describe, expect, it } from 'vitest';
import { BUILDING, FOREST, ROAD, WATER } from '../world/veg-mask';
import { LAYER, buildSplat, cellWeights, macroSplatMip, splatExtent, surfaceAt, type SplatMaps } from './terrain-splat';
import { normalizeSplat } from './splat-math';

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
    const input = { sizeM: 64, resM: 2, heightAt: (x: number, z: number) => 2 + 0.05 * x + Math.sin(z / 5), flags: (x: number) => x > 30 && x < 34 ? ROAD : FOREST };
    const m = buildSplat(input);
    expect(m.n).toBe(32);
    expect(m.a.length).toBe(32 * 32 * 4);
    for (let t = 0; t < 32 * 32; t++) {
      let s = 0;
      for (let k = 0; k < 4; k++) s += m.a[t * 4 + k]! + m.b[t * 4 + k]!;
      expect(Math.abs(s - 255)).toBeLessThanOrEqual(4);
    }
    expect(buildSplat(input)).toEqual(m);
    const other = buildSplat({ ...input, seed: 2 });
    expect(other.a).not.toEqual(m.a);
    expect(other.b).not.toEqual(m.b);
    expect(m.b.length).toBe(m.a.length);
  });
  it('uses texel centres, clamps edges and resolves ties across the two RGBA textures', () => {
    const m: SplatMaps = {
      n: 2, sizeM: 3, resM: 2,
      a: new Uint8Array([200, 0, 0, 0, 0, 0, 0, 0, 0, 210, 0, 0, 0, 0, 0, 0]),
      b: new Uint8Array([0, 0, 0, 0, 200, 0, 0, 0, 0, 0, 0, 0, 0, 0, 250, 0]),
    };
    expect(splatExtent(m)).toBe(4);
    expect(surfaceAt(m, -5, 1)).toBe(0);
    expect(surfaceAt(m, 2, 1)).toBe(0);
    expect(surfaceAt(m, 2.001, 1)).toBe(4);
    expect(surfaceAt(m, 1, 5)).toBe(1);
    expect(surfaceAt(m, 5, 5)).toBe(6);
  });

  it('chooses a macro mip with an approximately 64 m footprint, clamped to available levels', () => {
    expect(macroSplatMip(buildSplat({ sizeM: 256, resM: 2, heightAt: () => 0 }))).toBe(5);
    expect(macroSplatMip(buildSplat({ sizeM: 5, resM: 2, heightAt: () => 0 }))).toBe(1);
    expect(macroSplatMip(buildSplat({ sizeM: 256, resM: 100, heightAt: () => 0 }))).toBe(0);
  });

  it('surfaceAt agrees with the bilinear argmax at 1000 seeded points on and off the map', () => {
    const m = buildSplat({ sizeM: 127, resM: 2, heightAt: (x, z) => 3 + 0.03 * x * Math.sin(z / 9), flags: (x, z) => (Math.floor(x / 20 + z / 30) % 3 === 0 ? ROAD : 0) });
    let seed = 7;
    const r = () => ((seed = (Math.imul(seed, 1103515245) + 12345) >>> 0) / 4294967296);
    const clamp = (i: number) => Math.max(0, Math.min(m.n - 1, i));
    for (let q = 0; q < 1000; q++) {
      const x = r() * 140 - 6;
      const z = r() * 140 - 6;
      const u = x / splatExtent(m) * m.n - 0.5;
      const v = z / splatExtent(m) * m.n - 0.5;
      const values = new Array<number>(8).fill(0);
      for (const j of [Math.floor(v), Math.ceil(v)]) {
        for (const i of [Math.floor(u), Math.ceil(u)]) {
          const blend = (1 - Math.abs(i - u)) * (1 - Math.abs(j - v));
          const o = (clamp(j) * m.n + clamp(i)) * 4;
          const channels = [...m.a.subarray(o, o + 4), ...m.b.subarray(o, o + 4)];
          for (const [layer, weight] of channels.entries()) values[layer] = values[layer]! + blend * weight;
        }
      }
      expect(surfaceAt(m, x, z)).toBe(values.indexOf(Math.max(...values)));
    }
  });

  it.each([0, -1, Infinity, NaN])('rejects invalid %s m resolution', (resM) => {
    expect(() => buildSplat({ sizeM: 64, resM, heightAt: () => 0 })).toThrow(RangeError);
  });
});
