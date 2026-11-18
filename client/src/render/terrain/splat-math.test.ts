import { describe, expect, it } from 'vitest';
import { BPP, dominantLayer, LAYER_BUDGET_BYTES, layerArrayBytes, normalizeSplat, slopeDeg, tileUv, triplanarWeight } from './splat-math';

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

describe('layer VRAM (acceptance 4)', () => {
  const MiB = 1024 * 1024;
  it('one map per layer fits 24 MB in every format; three maps (albedo+normal+ORM) only in ASTC 6x6 / ETC2', () => {
    for (const f of Object.keys(BPP) as (keyof typeof BPP)[]) expect(layerArrayBytes(8, 1024, 1, f)).toBeLessThanOrEqual(LAYER_BUDGET_BYTES);
    expect(layerArrayBytes(8, 1024, 3, 'astc6x6') / MiB).toBeCloseTo(14.2, 0);
    expect(layerArrayBytes(8, 1024, 3, 'etc2') / MiB).toBeCloseTo(16, 0);
    expect(layerArrayBytes(8, 1024, 3, 'bc7') / MiB).toBeCloseTo(32, 0);
    expect(layerArrayBytes(8, 1024, 3, 'bc7')).toBeGreaterThan(LAYER_BUDGET_BYTES);
  });
});
