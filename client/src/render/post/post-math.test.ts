import { describe, expect, it } from 'vitest';
import {
  acesApprox, ExposureController, getFrameJitter, halton, histogramAverageEv, historyWeight, jitterToClip, lumBin, NUM_BINS,
  reprojectUv, rgbToYCoCg,
} from './post-math';

describe('jitter', () => {
  it('Halton(2) and (3) first 8 elements match the analytic values', () => {
    const h2 = [1, 2, 3, 4, 5, 6, 7, 8].map((i) => halton(i, 2));
    expect(h2).toEqual([0.5, 0.25, 0.75, 0.125, 0.625, 0.375, 0.875, 0.0625]);
    const h3 = [1, 2, 3].map((i) => halton(i, 3));
    expect(h3[0]).toBeCloseTo(1 / 3);
    expect(h3[1]).toBeCloseTo(2 / 3);
    expect(h3[2]).toBeCloseTo(1 / 9);
  });

  it('jitter cycles with period 8, stays inside the pixel, and is distinct per phase', () => {
    const seen = new Set<string>();
    for (let f = 0; f < 8; f++) {
      const [x, y] = getFrameJitter(f);
      expect(Math.abs(x)).toBeLessThanOrEqual(0.5);
      expect(Math.abs(y)).toBeLessThanOrEqual(0.5);
      expect(getFrameJitter(f + 8)).toEqual([x, y]);
      seen.add(`${x},${y}`);
    }
    expect(seen.size).toBe(8);
    expect(getFrameJitter(-1)).toEqual(getFrameJitter(7));
    expect(jitterToClip([0.25, -0.25], 100, 50)).toEqual([0.005, -0.01]);
  });
});

describe('exposure', () => {
  it('histogram quantile band ignores the dark/bright outliers', () => {
    const h = new Array<number>(NUM_BINS).fill(0);
    h[lumBin(0.18)] = 80; // mid grey dominates
    h[lumBin(1000)] = 10; // sun glint, above the 90 % cut
    h[lumBin(0.0001)] = 10; // dark pixels, below the 10 % cut
    const ev = histogramAverageEv(h);
    expect(ev).toBeGreaterThan(-3);
    expect(ev).toBeLessThan(-2);
    expect(histogramAverageEv(new Array<number>(NUM_BINS).fill(0))).toBe(0);
  });

  it('exposure-step: a 2 EV jump settles within 2 s (+-5 % luminance) without overshoot', () => {
    const c = new ExposureController(0, 1.5);
    let settledAt = -1;
    let maxEv = 0;
    for (let f = 1; f <= 120; f++) {
      c.update(1 / 60, 2);
      maxEv = Math.max(maxEv, c.ev);
      if (settledAt < 0 && Math.abs(2 ** (c.ev - 2) - 1) <= 0.05) settledAt = f / 60;
    }
    expect(settledAt).toBeGreaterThan(0);
    expect(settledAt).toBeLessThanOrEqual(2);
    expect(maxEv).toBeLessThanOrEqual(2); // no overshoot
    expect(c.ev).toBe(2);
  });
});

describe('taa maths', () => {
  it('reprojection and history weight', () => {
    expect(reprojectUv({ x: 0.5, y: 0.5 }, { x: 0.1, y: -0.2 })).toEqual({ x: 0.4, y: 0.7 });
    expect(historyWeight(0)).toBeCloseTo(0.9);
    expect(historyWeight(8)).toBe(0);
    expect(historyWeight(0, 1)).toBeCloseTo(0.45);
    expect(historyWeight(2)).toBeLessThan(historyWeight(1));
  });
  it('tonemap and colour space sanity', () => {
    expect(acesApprox(0)).toBe(0);
    expect(acesApprox(100)).toBe(1);
    expect(acesApprox(0.18)).toBeGreaterThan(0.1);
    expect(acesApprox(0.5)).toBeGreaterThan(acesApprox(0.18));
    expect(rgbToYCoCg(1, 1, 1)).toEqual([1, 0, 0]);
  });
});
