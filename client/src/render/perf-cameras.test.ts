import { describe, expect, it } from 'vitest';
import { PERF_CAMERAS } from './perf-cameras';

describe('perf cameras (M0-030 baseline definition)', () => {
  it('are three fixed poses; changing them invalidates reports/baselines/client-m0.json', () => {
    expect(PERF_CAMERAS.map((c) => [c.name, ...c.pos, ...c.target])).toEqual([
      ['ground-north', 128, 0, 40, 128, 0, 128],
      ['overview', 128, 110, -90, 128, 0, 140],
      ['west-edge', 20, 12, 128, -3000, 0, 128],
    ]);
  });
  it('are plain data (no time dependence)', () => {
    expect(JSON.parse(JSON.stringify(PERF_CAMERAS))).toEqual(PERF_CAMERAS);
  });
});

describe('M1 camera scenes (M1-069)', () => {
  it('pins the three poses (changing one invalidates M1 baselines)', async () => {
    const { M1_PERF_CAMERAS, poseYaw } = await import('./perf-cameras');
    expect(M1_PERF_CAMERAS.map((c) => [c.name, ...c.pos, ...c.target])).toEqual([
      ['open_field', 700, 0, 1330, 700, 0, 1470],
      ['hedgerow', 1500, 0, 500, 1300, 0, 540],
      ['old_town', 1246, 0, 700, 1275, 0, 880],
    ]);
    expect(poseYaw(M1_PERF_CAMERAS[0]!)).toBe(0); // looks north (+z)
    expect(poseYaw(M1_PERF_CAMERAS[1]!)).toBeLessThan(-1.3); // looks west (-x)
  });
});
