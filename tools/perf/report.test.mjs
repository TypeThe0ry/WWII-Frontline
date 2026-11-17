import assert from 'node:assert/strict';
import { test } from 'node:test';
import { aggregateRuns } from './lib.mjs';
import { summarize, toMarkdown } from './report.mjs';

const dump = (cpu) => ({ fps: 2, frames: 60, cpu_ms: { p50: cpu, p99: cpu * 2 }, interval_ms: { p50: 500 }, draw_calls: 50, active_meshes: 9, triangles: 450000, vegetation: { drawCalls: 20, instances: 8130, tris: 1 }, stages: { scene_render: { p50: cpu, p99: cpu } }, meta: { js_heap_mb: 55 } });

test('summarize + toMarkdown: medians of the runs, skipped entries listed as 未测', () => {
  const runs = [dump(1), dump(1.2), dump(1.1)];
  const rows = summarize([
    { file: 'a', cam: 0, meta: { machine: 'm', renderer_requested: 'webgl2', viewport: '640x360' }, runs, aggregate: aggregateRuns(runs) },
    { file: 'b', cam: 1, meta: { machine: 'm', renderer_requested: 'webgpu', viewport: '640x360' }, skipped: 'no adapter' },
  ]);
  assert.equal(rows.length, 2);
  assert.equal(rows[0].cpu_p50_ms, 1.1);
  assert.equal(rows[0].draw_calls, 50);
  assert.equal(rows[1].skipped, 'no adapter');
  const md = toMarkdown(rows);
  assert.match(md, /\| m \| webgl2 \| 640x360 \| 0 \| 3 \|/);
  assert.match(md, /未测 \(no adapter\)/);
});
