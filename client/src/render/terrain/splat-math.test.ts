import { describe, expect, it } from 'vitest';
import { terrainLayerBytes, terrainLayerSizes, dominantLayer, LAYER_BUDGET_BYTES, layerArrayBytes, normalizeSplat, slopeDeg, tileUv, triplanarWeight } from './splat-math';

describe('splat', () => {
  it('normalises to sum 1 and falls back to layer 0 for all-zero', () => {
    const o = new Float32Array(8);
    normalizeSplat([1, 1, 2, 0, 0, 0, 0, 0], o);
    expect(Array.from(o).reduce((a, b) => a + b, 0)).toBeCloseTo(1);
    expect(o[2]).toBeCloseTo(0.5);
    normalizeSplat([0, 0, 0, 0, 0, 0, 0, 0], o);
    expect(o[0]).toBe(1);
    normalizeSplat([-5, 3, 0, 0, 0, 0, 0, 0], o);
    expect(o[1]).toBe(1);
  });

  it('getSurfaceAt: dominant layer equals argmax of the 8 weights over 1 000 random splats', () => {
    let s = 7;
    const rnd = () => ((s = (s * 1664525 + 1013904223) >>> 0) / 2 ** 32);
    for (let i = 0; i < 1000; i++) {
      const w = Array.from({ length: 8 }, () => Math.floor(rnd() * 256));
      const exp = w.reduce((bi, v, k) => (v > (w[bi] as number) ? k : bi), 0);
      expect(dominantLayer(w.slice(0, 4), w.slice(4))).toBe(exp);
    }
    expect(dominantLayer([10, 10, 0, 0], [0, 0, 0, 0])).toBe(0); // tie -> lowest index
  });
});

describe('triplanar', () => {
  it('off on flat ground, full on steep slopes, smooth in between', () => {
    expect(triplanarWeight(10)).toBe(0);
    expect(triplanarWeight(40)).toBe(0);
    expect(triplanarWeight(45)).toBeCloseTo(0.5);
    expect(triplanarWeight(60)).toBe(1);
    expect(triplanarWeight(42)).toBeLessThan(triplanarWeight(46));
    expect(slopeDeg(1)).toBeCloseTo(0);
    expect(slopeDeg(Math.cos((45 * Math.PI) / 180))).toBeCloseTo(45);
  });
  it('tiling wraps into [0,1)', () => {
    expect(tileUv(0.75, 2)).toBeCloseTo(0.5);
    expect(tileUv(-0.25, 1)).toBeCloseTo(0.75);
  });
});

describe('layer VRAM (PI-102)', () => {
  it('charges at least one GPU block per mip and rounds up partial edge blocks', () => {
    expect(layerArrayBytes(1, 4, 1, 'bc7')).toBe(3 * 16); // 4x4, 2x2, 1x1
    expect(layerArrayBytes(1, 8, 1, 'astc6x6')).toBe((4 + 1 + 1 + 1) * 16);
    expect(layerArrayBytes(1, 4, 1, 'etc2')).toBe(3 * 8);
    expect(layerArrayBytes(1, 4, 1, 'etc2rgba8')).toBe(3 * 16);
  });

  it.each(['bc7', 'astc4x4', 'etc2rgba8'] as const)('drops just the normal and ORM base mips for %s', (format) => {
    expect(terrainLayerSizes(format)).toEqual({ albedo: 1024, normal: 512, orm: 512 });
    expect(layerArrayBytes(8, 1024, 3, format)).toBeGreaterThan(LAYER_BUDGET_BYTES);
    expect(terrainLayerBytes(format)).toBe(16_777_856);
    expect(terrainLayerBytes(format)).toBeLessThanOrEqual(LAYER_BUDGET_BYTES);
  });

  it.each(['astc6x6', 'etc2'] as const)('keeps all three 1024² maps for %s', (format) => {
    expect(terrainLayerSizes(format)).toEqual({ albedo: 1024, normal: 1024, orm: 1024 });
    expect(terrainLayerBytes(format)).toBe(layerArrayBytes(8, 1024, 3, format));
    expect(terrainLayerBytes(format)).toBeLessThanOrEqual(LAYER_BUDGET_BYTES);
  });
});
