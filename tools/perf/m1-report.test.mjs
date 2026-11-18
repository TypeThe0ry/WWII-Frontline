import assert from 'node:assert/strict';
import test from 'node:test';
import { rowFromCapture } from './m1-report.mjs';

const run = (p95, dc, h = 0) => ({ frame_ms: { p50: 10, p95, p99: p95 + 5 }, draw_calls: dc, triangles: 1000, cpu_ms: { p50: 3 }, gpu_ms: { value: 0.3 }, hitches_gt_50ms: h, meta: { scene: 'old_town', js_heap_mb: 70, world: { building_count: 1 } }, vegetation: { instances: 5 } });

test('rowFromCapture takes median p95, worst hitches, and null for no runs', () => {
  const r = rowFromCapture({ runs: [run(16, 100, 0), run(18, 120, 3), run(17, 110, 1)] }, 'm', 'high');
  assert.equal(r.frame_ms.p95, 17);
  assert.equal(r.draw_calls, 110);
  assert.equal(r.hitches_gt_50ms, 3);
  assert.equal(r.scene, 'old_town');
  assert.equal(rowFromCapture({ skipped: 'x' }, 'm', 'high'), null);
});
