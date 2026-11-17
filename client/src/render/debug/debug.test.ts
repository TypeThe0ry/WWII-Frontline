import { describe, expect, it } from 'vitest';
import { FrameStats, RingStats, percentile } from './frame-stats';
import { PERF_DUMP_KEYS, PERF_SCHEMA_VERSION, WINDOW_STATS_KEYS, buildPerfDump, type RenderSnapshot } from './perf-export';

describe('RingStats', () => {
  it('computes mean / P50 / P99 / max over a sliding window of 600', () => {
    const r = new RingStats(600);
    for (let i = 1; i <= 1000; i++) r.push(i); // window keeps 401..1000
    const s = r.stats();
    expect(s.count).toBe(600);
    expect(s.max).toBe(1000);
    expect(s.p50).toBe(700);
    expect(s.p99).toBe(994);
    expect(s.mean).toBeCloseTo(700.5, 6);
    expect(r.last).toBe(1000);
  });

  it('is zeroed when empty and ignores non-finite values', () => {
    const r = new RingStats();
    r.push(NaN);
    expect(r.stats()).toEqual({ count: 0, mean: 0, p50: 0, p99: 0, max: 0 });
  });

  it('percentile uses nearest rank', () => {
    expect(percentile([1, 2, 3, 4], 0.5)).toBe(2);
    expect(percentile([1, 2, 3, 4], 0.99)).toBe(4);
  });
});

describe('FrameStats', () => {
  it('derives fps from frame intervals', () => {
    const f = new FrameStats();
    for (let i = 0; i < 10; i++) {
      f.begin(i * 16.6667);
      f.end(0.5);
    }
    expect(f.frames).toBe(10);
    expect(f.fps()).toBeCloseTo(60, 1);
    expect(f.cpu.stats().p99).toBe(0.5);
  });
});

describe('perf-export schema', () => {
  const snap: RenderSnapshot = {
    renderer: 'webgl2',
    fallbackReason: 'x',
    frames: 3,
    fps: 60,
    cpu: { count: 3, mean: 1, p50: 1, p99: 2, max: 2 },
    interval: { count: 2, mean: 16, p50: 16, p99: 17, max: 17 },
    gpu: { value: null, source: 'n/a', reason: 'r' },
    drawCalls: 4,
    activeMeshes: 5,
    triangles: 6,
    renderScale: 1,
    width: 1920,
    height: 1080,
  };

  it('has exactly the fixed top-level fields, in a JSON-serialisable form', () => {
    const d = buildPerfDump(snap, { steps_total: 7, stepper_clamped: 1 }, 123);
    expect(Object.keys(d).sort()).toEqual([...PERF_DUMP_KEYS].sort());
    expect(Object.keys(d.cpu_ms).sort()).toEqual([...WINDOW_STATS_KEYS].sort());
    expect(Object.keys(d.interval_ms).sort()).toEqual([...WINDOW_STATS_KEYS].sort());
    expect(Object.keys(d.gpu_ms).sort()).toEqual(['reason', 'source', 'value']);
    expect(Object.keys(d.slots).sort()).toEqual(['corrections', 'rtt', 'transport']);
    expect(d.schema).toBe(PERF_SCHEMA_VERSION);
    expect(d.timestamp_ms).toBe(123);
    expect(d.sim).toEqual({ steps_total: 7, stepper_clamped: 1 });
    expect(JSON.parse(JSON.stringify(d))).toEqual(d);
  });
});
