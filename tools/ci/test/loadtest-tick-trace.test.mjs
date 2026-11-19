import assert from 'node:assert/strict';
import test from 'node:test';
import { summarizeTickTrace } from '../../../scripts/summarize-tick-trace.mjs';

test('slow tick attribution uses scheduler counters instead of the longest wall phase', () => {
  const sample = (tick, cpu_ms, runq_wait_ms, event = 'tick_slow') => JSON.stringify({
    event, tick, tick_ms: 30, cpu_ms, runq_wait_ms, phase_step_ms: 25, phase_send_ms: 4,
  });
  const trace = summarizeTickTrace([
    '[aoi-pose] ignored', sample(1, 4, 24, 'tick_overrun'), sample(2, 21, 1),
    sample(3, 2, 0), sample(4, 10, 10), sample(5, -1, -1),
    JSON.stringify({ event: 'tick_step_trace', tick: 1, movement_ms: 22, other_ms: 3 }),
  ].join('\n'));
  assert.deepEqual(trace.totals, { tick_slow: 4, tick_overrun: 1 });
  assert.deepEqual(trace.breakdown, { run_queue: 1, on_cpu: 1, unaccounted: 1, mixed: 1, no_schedstat: 1 });
  assert.equal(trace.overrun_breakdown.run_queue, 1);
  const peak = trace.worst_elapsed[0];
  assert.equal(peak.reason, 'run_queue');
  assert.deepEqual(peak.longest_wall_phase, { name: 'step', ms: 25 });
  assert.deepEqual(peak.longest_step_system, { name: 'movement', ms: 22 });
  assert.equal(trace.worst_on_cpu[0].tick, 2);
  assert.equal(trace.worst_unaccounted[0].unaccounted_ms, 28);
});
