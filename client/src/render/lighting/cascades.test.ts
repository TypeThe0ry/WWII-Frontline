import { describe, expect, it } from 'vitest';
import { cascadeIndex, cascadeMatrix, fitCascade, sliceCorners, sliceSphere, transformPoint, type CameraPose, type V3 } from './cascades';

const cam = (yaw: number, pos: V3 = [100, 10, 100]): CameraPose => {
  const f: V3 = [Math.sin(yaw), 0, Math.cos(yaw)];
  return { pos, forward: f, right: [Math.cos(yaw), 0, -Math.sin(yaw)], up: [0, 1, 0], fov: (70 * Math.PI) / 180, aspect: 16 / 9 };
};
const LIGHT: V3 = [0.45, -0.8, 0.35];

describe('cascade fitting', () => {
  it('the sphere contains all 8 corners of the slice', () => {
    const c = cam(0.7);
    const { center, radius } = sliceSphere(c, 12, 40);
    for (const p of sliceCorners(c, 12, 40)) expect(Math.hypot(p[0] - center[0], p[1] - center[1], p[2] - center[2])).toBeLessThanOrEqual(radius + 1e-4);
  });

  it('the radius does not depend on where the camera looks (rotation stable)', () => {
    const r = [0, 1, 2, 3].map((k) => sliceSphere(cam(k * 0.8), 40, 120).radius);
    expect(Math.max(...r) - Math.min(...r)).toBeLessThan(1e-6);
  });

  it('every point of the slice lands inside the window and the depth range [0,1] (WebGPU) / [-1,1] (WebGL)', () => {
    const c = cam(2.1);
    for (const halfZ of [true, false]) {
      const cas = fitCascade(c, 12, 40, LIGHT, 2048);
      const m = cascadeMatrix(cas, 100, halfZ);
      for (const p of sliceCorners(c, 12, 40)) {
        const t = transformPoint(m, p);
        expect(Math.abs(t[0]), 'x').toBeLessThanOrEqual(1.0001);
        expect(Math.abs(t[1]), 'y').toBeLessThanOrEqual(1.0001);
        expect(t[2], 'z').toBeGreaterThanOrEqual(halfZ ? -1e-4 : -1.0001);
        expect(t[2], 'z').toBeLessThanOrEqual(halfZ ? 1.0001 : 1.0001);
        expect(t[3]).toBe(1);
      }
    }
  });

  it('texel snapping: moving the camera by less than a texel does not move the window; by more it moves in whole texels', () => {
    const a = fitCascade(cam(0.3, [100, 10, 100]), 12, 40, LIGHT, 1024);
    const dx = a.texel * 0.3;
    const b = fitCascade(cam(0.3, [100 + dx * a.lightRight[0], 10, 100]), 12, 40, LIGHT, 1024);
    const eps = 1e-3;
    const along = (c: typeof a) => c.snappedCenter[0] * a.lightRight[0] + c.snappedCenter[1] * a.lightRight[1] + c.snappedCenter[2] * a.lightRight[2];
    const moved = Math.abs(along(b) - along(a));
    expect(moved % a.texel < eps || a.texel - (moved % a.texel) < eps).toBe(true);
    const far = fitCascade(cam(0.3, [100 + a.texel * 7.4 * a.lightRight[0], 10, 100 + a.texel * 7.4 * a.lightRight[2]]), 12, 40, LIGHT, 1024);
    const m2 = Math.abs(along(far) - along(a));
    expect(Math.abs(m2 / a.texel - Math.round(m2 / a.texel))).toBeLessThan(1e-2);
  });

  it('cascadeIndex picks the first split that covers the distance', () => {
    expect(cascadeIndex([12, 40, 120, 350], 5)).toBe(0);
    expect(cascadeIndex([12, 40, 120, 350], 12)).toBe(0);
    expect(cascadeIndex([12, 40, 120, 350], 13)).toBe(1);
    expect(cascadeIndex([12, 40, 120, 350], 400)).toBe(-1);
  });
});
