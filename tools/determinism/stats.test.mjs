// node --test tools/determinism/stats.test.mjs
import assert from 'node:assert/strict';
import test from 'node:test';
import { driftMetrics, posErrors, rms, slope } from './stats.mjs';

test('rms of known values', () => {
  assert.equal(rms([]), 0);
  assert.equal(rms([3, 4]), Math.sqrt(12.5));
  assert.equal(rms([2, 2, 2]), 2);
});

test('slope recovers a known line and ignores offset', () => {
  const t = Array.from({ length: 100 }, (_, i) => i * 0.5);
  assert.ok(Math.abs(slope(t, t.map((x) => 0.02 * x + 7)) - 0.02) < 1e-12);
  assert.equal(slope(t, t.map(() => 5)), 0);
  assert.equal(slope([1], [1]), 0);
  assert.equal(slope([2, 2, 2], [1, 2, 3]), 0);
});

test('posErrors is the 3D euclidean distance', () => {
  const a = new Float32Array([0, 0, 0, 1, 1, 1]);
  const b = new Float32Array([3, 4, 0, 1, 1, 3]);
  assert.deepEqual(Array.from(posErrors(a, b)), [5, 2]);
  assert.throws(() => posErrors(new Float32Array(3), new Float32Array(6)));
});

test('driftMetrics: linear error 0.01 m/s over 120 s at 30 Hz', () => {
  const hz = 30;
  const n = 120 * hz;
  const err = Float64Array.from({ length: n }, (_, i) => 0.01 * ((i + 1) / hz));
  const m = driftMetrics(err, hz);
  assert.ok(Math.abs(m.cm_per_s - 1.0) < 1e-9, `${m.cm_per_s}`);
  assert.ok(Math.abs(m.max_m - 1.2) < 1e-12);
  assert.equal(m.segment_max_m.length, 2);
  assert.ok(Math.abs(m.segment_max_m[0] - 0.6) < 1e-12);
  // err > 0.05 once t > 5 s => ticks 151..3600
  assert.equal(m.over_0_05m_ticks, n - 150);
  assert.ok(Math.abs(m.over_0_05m_ratio - (n - 150) / n) < 1e-12);
});

test('driftMetrics of identical trajectories is exactly zero', () => {
  const a = new Float32Array(3 * 1000).map((_, i) => Math.sin(i));
  const m = driftMetrics(posErrors(a, a.slice()));
  assert.deepEqual([m.rms_m, m.max_m, m.cm_per_s, m.over_0_05m_ratio], [0, 0, 0, 0]);
});
