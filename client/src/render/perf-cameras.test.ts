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
