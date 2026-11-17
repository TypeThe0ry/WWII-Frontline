import { describe, expect, it } from 'vitest';
import { SWAY_OMEGA, defaultWind, instancePhase, windSway, windUniform } from './wind';

describe('windSway', () => {
  it('is bounded by strength x weight x scale', () => {
    for (let t = 0; t < 40; t += 0.37) {
      for (const w of [0, 0.25, 1]) {
        expect(Math.abs(windSway(t, 1.3, w, 0.2, 1.5))).toBeLessThanOrEqual(0.2 * w * 1.5 + 1e-12);
      }
    }
  });
  it('keeps trunks (weight 0) fixed and clamps weights outside [0,1]', () => {
    expect(Math.abs(windSway(3.3, 0.5, 0, 0.5))).toBe(0);
    expect(Math.abs(windSway(3.3, 0.5, 7, 0.5))).toBeLessThanOrEqual(0.5);
    expect(Math.abs(windSway(3.3, 0.5, -2, 0.5))).toBe(0);
  });
  it('is periodic in time with period 2 pi / omega and zero at strength 0', () => {
    const T = (2 * Math.PI) / SWAY_OMEGA;
    expect(windSway(1.1 + T, 0.4, 1, 0.3)).toBeCloseTo(windSway(1.1, 0.4, 1, 0.3), 9);
    expect(Math.abs(windSway(5, 2, 1, 0))).toBe(0);
  });
  it('phase hash is in [0, 2 pi) and differs between positions', () => {
    const ps = new Set<number>();
    for (let i = 0; i < 50; i++) {
      const p = instancePhase(i * 1.7, i * 0.9);
      expect(p).toBeGreaterThanOrEqual(0);
      expect(p).toBeLessThan(Math.PI * 2);
      ps.add(Math.round(p * 1000));
    }
    expect(ps.size).toBeGreaterThan(40);
  });
  it('packs the uniform (dirX, dirZ, strength, time) with a unit direction', () => {
    const w = defaultWind();
    const u = windUniform({ ...w, time: 12.5 });
    expect(Math.hypot(u[0]!, u[1]!)).toBeCloseTo(1, 6);
    expect(u[2]).toBeCloseTo(w.strength, 6);
    expect(u[3]).toBeCloseTo(12.5, 6);
  });
});
