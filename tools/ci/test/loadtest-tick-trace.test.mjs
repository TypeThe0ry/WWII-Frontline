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

test('bounds every residual-majority tick and joins nested measurements by match and tick', () => {
  const residual = (match_id, tick, phase_step_ms, otherPhases = {}) => ({
    event: 'tick_slow', match_id, tick, tick_ms: 30, cpu_ms: 4, runq_wait_ms: 2,
    phase_step_ms, ...otherPhases,
  });
  const step = (match_id, fields) => ({ event: 'tick_step_trace', match_id, tick: 7, ...fields });
  const trace = summarizeTickTrace([
    step('a', { movement_ms: 22, other_ms: 4 }),
    residual('a', 7, 26),
    residual('b', 7, 11, { phase_snapshot_ms: 10, phase_send_ms: 9 }),
    residual('a', 8, 14, { phase_snapshot_ms: 16 }),
    residual('a', 9, 12), // Exactly half the residual fits inside; no strict majority is proven.
    residual('a', 10, undefined),
    step('b', {
      world_tick: 107, movement_ms: 3, other_ms: 8, schedule_other_ms: 2,
      lagcomp_ms: 4, movement_players_ms: 2.8, movement_slowest_player_ms: 1.4, movement_slowest_entity: 0,
    }),
  ].map((e) => JSON.stringify(e)).join('\n'));

  assert.deepEqual(trace.unaccounted_step_breakdown, {
    inside_step: 1, outside_step: 1, indeterminate: 2, no_step_timing: 1,
  });
  const [inside, outside, ambiguous, boundary, missing] = trace.unaccounted_ticks;
  assert.equal(inside.tick, outside.tick);
  assert.deepEqual(inside.unaccounted_in_step, { majority: 'inside_step', min_ms: 20, max_ms: 24 });
  assert.deepEqual(inside.longest_step_system, { name: 'movement', ms: 22 });
  assert.equal(inside.longest_step_subphase, null);
  assert.deepEqual(outside.unaccounted_in_step, { majority: 'outside_step', min_ms: 5, max_ms: 11 });
  assert.deepEqual(outside.longest_wall_phase, { name: 'step', ms: 11 });
  assert.deepEqual(outside.longest_step_system, { name: 'other', ms: 8 });
  assert.deepEqual(outside.longest_step_subphase, { name: 'lagcomp', ms: 4 });
  assert.equal(outside.world_tick, 107);
  assert.deepEqual(outside.movement_player_wall, { total_ms: 2.8, max_ms: 1.4, entity: 0 });
  assert.deepEqual(ambiguous.unaccounted_in_step, { majority: 'indeterminate', min_ms: 8, max_ms: 14 });
  assert.deepEqual(boundary.unaccounted_in_step, { majority: 'indeterminate', min_ms: 6, max_ms: 12 });
  assert.deepEqual(missing.unaccounted_in_step, { majority: 'no_step_timing', min_ms: null, max_ms: null });
});
