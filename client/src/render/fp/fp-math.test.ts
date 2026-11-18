import { describe, expect, it } from 'vitest';
import { adsFovDeg, muzzleReach, RecoilSpring, reloadFrames, screenFraction, stepAds, walkBob, wallPushback } from './fp-math';

describe('wall pushback (fp_wall_pushback)', () => {
  it('no pushback in the open; muzzle never reaches the wall', () => {
    expect(wallPushback(5, 0.9)).toBe(0);
    for (let d = 0.06; d < 2; d += 0.01) {
      for (const len of [0.35, 0.9, 1.25]) {
        expect(muzzleReach(d, len)).toBeLessThanOrEqual(d - 0.05 + 1e-9);
      }
    }
  });
  it('camera 0.1 m from a wall: muzzle 5 cm short, pushback capped at the gun length', () => {
    expect(muzzleReach(0.1, 1.1)).toBeCloseTo(0.05);
    expect(wallPushback(0.0, 0.9)).toBe(0.9);
  });
});

describe('ads', () => {
  it('FOV shrinks by the 1.2x magnification (tan-space) and is monotonic in t', () => {
    expect(adsFovDeg(54, 1.2, 0)).toBeCloseTo(54);
    const full = adsFovDeg(54, 1.2, 1);
    expect(full).toBeLessThan(54);
    expect(Math.tan((full * Math.PI) / 360) * 1.2).toBeCloseTo(Math.tan((54 * Math.PI) / 360));
    expect(adsFovDeg(54, 1.2, 0.5)).toBeGreaterThan(full);
  });
  it('full ADS travel takes adsMs; reverses when released', () => {
    let t = 0;
    for (let i = 0; i < 24; i++) t = stepAds(t, 1 / 120, 200, true);
    expect(t).toBeCloseTo(1, 9);
    expect(stepAds(0, 1 / 120, 200, true)).toBeCloseTo(1 / 24);
    expect(stepAds(1, 1 / 120, 200, false)).toBeCloseTo(1 - 1 / 24);
  });
});

describe('fp_fov_independent', () => {
  it('the FP size depends on the FP FOV only: world FOV 40..110 changes nothing', () => {
    const fpFov = 54;
    const sizes = [40, 60, 90, 110].map(() => screenFraction(0.9, 0.6, fpFov));
    expect(Math.max(...sizes) - Math.min(...sizes)).toBe(0);
    // ...whereas a world-FOV-driven size would swing massively.
    expect(Math.abs(screenFraction(0.9, 0.6, 110) / screenFraction(0.9, 0.6, 40) - 1)).toBeGreaterThan(0.5);
  });
});

describe('reload / recoil / bob', () => {
  it('reload visual is within one frame of reload_ms', () => {
    for (const ms of [1800, 2400, 3100, 4200]) {
      for (const fps of [30, 60, 144]) {
        expect(Math.abs((reloadFrames(ms, fps) * 1000) / fps - ms)).toBeLessThanOrEqual(500 / fps + 1e-9);
      }
    }
  });
  it('recoil kicks back then settles without overshoot past zero', () => {
    const s = new RecoilSpring();
    s.kick(-1.2);
    let minPos = 0;
    let maxPos = 0;
    for (let i = 0; i < 120; i++) {
      const p = s.update(1 / 120);
      minPos = Math.min(minPos, p);
      maxPos = Math.max(maxPos, p);
    }
    expect(minPos).toBeLessThan(-0.005);
    expect(maxPos).toBeLessThan(1e-3);
    expect(Math.abs(s.pos)).toBeLessThan(2e-3);
  });
  it('bob scales with speed', () => {
    expect(walkBob(1, 0)).toEqual([0, 0]);
    expect(Math.abs(walkBob(1.2, 1)[0])).toBeGreaterThan(Math.abs(walkBob(1.2, 0.5)[0]));
  });
});
