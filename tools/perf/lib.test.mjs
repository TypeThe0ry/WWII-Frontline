import assert from 'node:assert/strict';
import { test } from 'node:test';
import { aggregateRuns, getPath, median, numericPaths, percentile, rangePct } from './lib.mjs';

test('percentile is nearest-rank, like the client frame-stats window', () => {
  assert.equal(percentile([1, 2, 3, 4], 0.5), 2);
  assert.equal(percentile([1, 2, 3, 4], 0.99), 4);
  assert.equal(percentile([], 0.5), 0);
  const v = Array.from({ length: 1000 }, (_, i) => 1000 - i); // unsorted input
  assert.equal(percentile(v, 0.99), 990);
  assert.equal(percentile(v, 0.5), 500);
});

test('median and range', () => {
  assert.equal(median([3, 1, 2]), 2);
  assert.equal(median([4, 1, 2, 3]), 2.5);
  assert.ok(Math.abs(rangePct([100, 104, 102]) - (4 / 102) * 100) < 1e-9);
  assert.equal(rangePct([5]), 0);
  assert.equal(rangePct([0, 0]), 0);
});

test('getPath / numericPaths', () => {
  const o = { a: { b: { c: 1 } }, d: 2, e: [1, 2], s: 'x' };
  assert.equal(getPath(o, 'a.b.c'), 1);
  assert.equal(getPath(o, 'a.x.c'), undefined);
  assert.deepEqual(numericPaths(o).sort(), ['a.b.c', 'd']);
});

test('aggregateRuns: median over repeated runs and the range of each metric', () => {
  const run = (cpu, dc, stage) => ({
    fps: 3,
    frames: 90,
    cpu_ms: { p50: cpu, p99: cpu * 2 },
    interval_ms: { p50: 300 },
    draw_calls: dc,
    active_meshes: 10,
    triangles: 1000,
    vegetation: { drawCalls: 5, instances: 100, tris: 900 },
    stages: { scene_render: { p50: stage, p99: stage * 3 } },
    meta: { js_heap_mb: 20 },
  });
  const a = aggregateRuns([run(100, 80, 90), run(104, 80, 94), run(102, 80, 92)]);
  assert.equal(a.runs, 3);
  assert.equal(a.cpu_ms_p50.median, 102);
  assert.equal(a.cpu_ms_p50.range_pct, (4 / 102) * 100);
  assert.equal(a.draw_calls.median, 80);
  assert.equal(a.draw_calls.range_pct, 0);
  assert.equal(a.stages.scene_render.p50.median, 92);
  assert.equal(a.vegetation_instances.median, 100);
  assert.equal(a.js_heap_mb.median, 20);
});
