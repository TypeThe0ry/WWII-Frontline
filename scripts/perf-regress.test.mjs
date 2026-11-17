import assert from 'node:assert/strict';
import { test } from 'node:test';
import { compareMetrics, DEFAULT_RULES, ruleFor } from './perf-regress.mjs';

const baseline = {
  metrics: {
    'client.cam0.draw_calls': { value: 50 },
    'client.cam0.triangles': { value: 450341 },
    'client.cam0.stage.scene_render.p50_ms': { value: 1.0 },
    'bundle.js_brotli_bytes': { value: 400000 },
    'rust.kcc_step_median_us': { value: 19.7 },
    'server.tick_p99_ms': { value: 1.2 },
    'server.rss_slope_kb_per_s_c40': { value: 0.1 },
  },
};
const same = Object.fromEntries(Object.entries(baseline.metrics).map(([k, v]) => [k, v.value]));

test('identical measurement passes', () => {
  const r = compareMetrics(baseline, same);
  assert.equal(r.failed, 0);
  assert.equal(r.rows.length, 7);
});

test('noise inside the tolerances passes', () => {
  const r = compareMetrics(baseline, { ...same, 'client.cam0.stage.scene_render.p50_ms': 1.8, 'rust.kcc_step_median_us': 22.5, 'server.tick_p99_ms': 1.9, 'bundle.js_brotli_bytes': 410000 });
  assert.equal(r.failed, 0, JSON.stringify(r.rows.filter((x) => x.verdict !== 'ok')));
});

test('an artificial slowdown / growth is a failure (acceptance 6: the check catches a slower change)', () => {
  const slower = { ...same, 'rust.kcc_step_median_us': 19.7 * 1.5 };
  assert.equal(compareMetrics(baseline, slower).failed, 1);
  const leak = { ...same, 'server.rss_slope_kb_per_s_c40': 5 };
  assert.equal(compareMetrics(baseline, leak).failed, 1);
  const fat = { ...same, 'bundle.js_brotli_bytes': 460000 };
  assert.equal(compareMetrics(baseline, fat).failed, 1);
  const more = { ...same, 'client.cam0.draw_calls': 80 };
  assert.equal(compareMetrics(baseline, more).failed, 1);
  const fewer = { ...same, 'client.cam0.triangles': 300000 }; // counts must also not silently drop (a broken scene)
  assert.equal(compareMetrics(baseline, fewer).failed, 1);
});

test('lower timings are fine; missing metrics are skipped unless strict', () => {
  const faster = { ...same, 'rust.kcc_step_median_us': 10 };
  assert.equal(compareMetrics(baseline, faster).failed, 0);
  const partial = { 'client.cam0.draw_calls': 50 };
  assert.equal(compareMetrics(baseline, partial).failed, 0);
  assert.equal(compareMetrics(baseline, partial).missing, 6);
  assert.equal(compareMetrics(baseline, partial, { strict: true }).failed, 6);
});

test('every baseline key family has a rule', () => {
  for (const k of Object.keys(baseline.metrics)) assert.ok(ruleFor(k, DEFAULT_RULES), k);
});
