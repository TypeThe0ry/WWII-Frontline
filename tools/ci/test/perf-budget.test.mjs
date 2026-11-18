import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import { BUDGETS, M1_SCENES, checkRow, checkVariants, flagVariance, summarize, toMarkdown } from '../../perf/budget.mjs';

const good = { frame_ms: { p95: 15 }, draw_calls: 1000, tris: 3_000_000, hitches_gt_50ms: 0, js_heap_mb: 400, wasm_mb: 100, vram_mb_est: 2000, gpu_ms: { csm: 2, opaque: 3.5, transparent: 0.5, taa: 0.7, post: 0.6 } };

test('high tier passes inside every budget', () => {
  assert.ok(checkRow(good, 'high').every((c) => c.verdict === 'PASS'));
});

test('thresholds are inclusive and excess is reported', () => {
  const r = checkRow({ ...good, draw_calls: 1201, frame_ms: { p95: 16.7 } }, 'high');
  const dc = r.find((c) => c.check === 'draw_calls');
  assert.equal(dc.verdict, 'FAIL');
  assert.equal(dc.over, 1);
  assert.equal(r.find((c) => c.check === 'frame_ms.p95').verdict, 'PASS');
});

test('missing values are UNMEASURED, never PASS', () => {
  const r = checkRow({ frame_ms: { p95: 10 } }, 'high');
  assert.equal(r.find((c) => c.check === 'draw_calls').verdict, 'UNMEASURED');
  assert.equal(r.find((c) => c.check === 'gpu_ms.csm').verdict, 'UNMEASURED');
});

test('low tiers use 30 fps and their own draw-call budget; no per-pass table', () => {
  assert.equal(BUDGETS.low.draw_calls, 500);
  assert.equal(BUDGETS.low_webgl2_medium.draw_calls, 900);
  const r = checkRow({ ...good, frame_ms: { p95: 33.4 }, draw_calls: 700 }, 'low_webgl2_medium');
  assert.equal(r.find((c) => c.check === 'frame_ms.p95').verdict, 'FAIL');
  assert.equal(r.find((c) => c.check === 'draw_calls').verdict, 'PASS');
  assert.ok(!r.some((c) => c.check.startsWith('gpu_ms.')));
});

test('hitch limit is 2 and memory sums heap + wasm', () => {
  assert.equal(checkRow({ ...good, hitches_gt_50ms: 3 }, 'high').find((c) => c.check === 'hitches_gt_50ms').verdict, 'FAIL');
  assert.equal(checkRow({ ...good, js_heap_mb: 1500, wasm_mb: 100 }, 'high').find((c) => c.check === 'js_heap_mb+wasm_mb').verdict, 'FAIL');
});

test('variants and variance flag', () => {
  assert.equal(checkVariants(60).verdict, 'PASS');
  assert.equal(checkVariants(61).verdict, 'FAIL');
  assert.equal(checkVariants(undefined).verdict, 'UNMEASURED');
  assert.ok(flagVariance(10.1));
  assert.ok(!flagVariance(10));
});

test('summarize: missing scene means UNMEASURED, a failure wins', () => {
  const all = Object.fromEntries(M1_SCENES.map((s) => [s, good]));
  assert.equal(summarize(all, 'high').overall, 'PASS');
  assert.equal(summarize({ open_field: good }, 'high').overall, 'UNMEASURED');
  assert.equal(summarize({ ...all, old_town: { ...good, tris: 5e6 } }, 'high').overall, 'FAIL');
  assert.match(toMarkdown(summarize({ ...all, old_town: { ...good, tris: 5e6 } }, 'high')), /FAIL \(\+1000000\)/);
});

test('m1-scenes.json lists exactly the three INDEX scenes and valid tiers', () => {
  const j = JSON.parse(readFileSync(new URL('../../perf/m1-scenes.json', import.meta.url), 'utf8'));
  assert.deepEqual(j.scenes.map((s) => s.id), M1_SCENES);
  for (const m of j.machines) assert.ok(BUDGETS[m.tier]);
  assert.equal(j.warmup_s, 10);
  assert.equal(j.capture_s, 60);
});
