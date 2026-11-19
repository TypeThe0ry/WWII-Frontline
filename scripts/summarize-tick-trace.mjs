// Classify slow ticks by Linux schedstat, not by the largest wall-clock phase. A descheduled
// thread can wait inside any phase; an unexplained remainder is not proof of I/O blocking.
const PHASES = ['inputs', 'step', 'post_step', 'bots', 'snapshot', 'send'];
const SYSTEMS = ['movement', 'weapons', 'projectiles', 'vehicles', 'rockets', 'shells', 'throwables', 'other'];
const STEP_SUBPHASES = [
  ...SYSTEMS.filter((name) => name !== 'other'),
  'prepare', 'schedule_other', 'lagcomp', 'reap_wrecks', 'sync_squads', 'refresh_views', 'collect_output',
];
const round = (ms) => Math.round(ms * 1000) / 1000;
const tickKey = (e) => `${e.match_id ?? ''}:${e.tick}`;

function largest(sample, fields, key) {
  const values = fields.flatMap((name) => Number.isFinite(sample[key(name)])
    ? [{ name, ms: sample[key(name)] }] : []);
  const peak = values.sort((a, b) => b.ms - a.ms)[0];
  return peak ? { name: peak.name, ms: round(peak.ms) } : null;
}

// The tick-level remainder can occupy as much as the entire measured step or its complement.
// These bounds describe where that remainder could fit. They do not identify a wait mechanism.
function unaccountedInStep(residual, tickMs, stepMs) {
  if (!Number.isFinite(stepMs) || stepMs < 0 || stepMs > tickMs) {
    return { majority: 'no_step_timing', min_ms: null, max_ms: null };
  }
  const min = Math.max(0, residual - (tickMs - stepMs));
  const max = Math.min(stepMs, residual);
  const majority = min > residual / 2 ? 'inside_step'
    : max < residual / 2 ? 'outside_step' : 'indeterminate';
  return { majority, min_ms: round(min), max_ms: round(max) };
}

function stepDetails(step) {
  const at = (name) => `${name}_ms`;
  return {
    longest_step_system: largest(step, SYSTEMS, at),
    // Legacy `other` overlaps the finer spans. Keep it out of the expanded ranking.
    longest_step_subphase: Number.isFinite(step.schedule_other_ms)
      ? largest(step, STEP_SUBPHASES, at) : null,
    ...(Number.isInteger(step.world_tick) ? { world_tick: step.world_tick } : {}),
    ...(Number.isFinite(step.movement_players_ms) && Number.isFinite(step.movement_slowest_player_ms) ? {
      movement_player_wall: {
        total_ms: round(step.movement_players_ms),
        max_ms: round(step.movement_slowest_player_ms),
        entity: step.movement_slowest_entity ?? null,
      },
    } : {}),
  };
}

export function summarizeTickTrace(serverLog) {
  const totals = { tick_slow: 0, tick_overrun: 0 };
  const breakdown = { run_queue: 0, on_cpu: 0, unaccounted: 0, mixed: 0, no_schedstat: 0 };
  const overrunBreakdown = { ...breakdown };
  const unaccountedStepBreakdown = { inside_step: 0, outside_step: 0, indeterminate: 0, no_step_timing: 0 };
  const samples = [];
  const steps = new Map();
  for (const line of serverLog.matchAll(/^\{[^\n]*"event":"tick_(?:slow|overrun|step_trace)"[^\n]*$/gm)) {
    const e = JSON.parse(line[0]);
    if (e.event === 'tick_step_trace') {
      steps.set(tickKey(e), e);
      continue;
    }
    totals[e.event] += 1;
    const schedstat = Number.isFinite(e.cpu_ms) && e.cpu_ms >= 0 &&
      Number.isFinite(e.runq_wait_ms) && e.runq_wait_ms >= 0;
    const residual = schedstat ? Math.max(0, e.tick_ms - e.cpu_ms - e.runq_wait_ms) : null;
    const reason = !schedstat ? 'no_schedstat' : e.runq_wait_ms > e.tick_ms / 2 ? 'run_queue'
      : e.cpu_ms > e.tick_ms / 2 ? 'on_cpu' : residual > e.tick_ms / 2 ? 'unaccounted' : 'mixed';
    breakdown[reason] += 1;
    if (e.event === 'tick_overrun') overrunBreakdown[reason] += 1;
    const stepBounds = reason === 'unaccounted' ? unaccountedInStep(residual, e.tick_ms, e.phase_step_ms) : null;
    if (stepBounds) unaccountedStepBreakdown[stepBounds.majority] += 1;
    samples.push({
      key: tickKey(e),
      ...(e.match_id != null ? { match_id: e.match_id } : {}),
      tick: e.tick,
      event: e.event,
      tick_ms: round(e.tick_ms),
      cpu_ms: schedstat ? round(e.cpu_ms) : null,
      runq_wait_ms: schedstat ? round(e.runq_wait_ms) : null,
      unaccounted_ms: schedstat ? round(residual) : null,
      reason,
      phase_step_ms: Number.isFinite(e.phase_step_ms) ? round(e.phase_step_ms) : null,
      longest_wall_phase: largest(e, PHASES, (phase) => `phase_${phase}_ms`),
      ...(stepBounds ? { unaccounted_in_step: stepBounds } : {}),
    });
  }
  const joinStep = ({ key, ...sample }) => ({ ...sample, ...stepDetails(steps.get(key) ?? {}) });
  const top = (by, n, filter = () => true) => samples.filter((s) => filter(s) && s[by] != null)
    .sort((a, b) => b[by] - a[by]).slice(0, n).map(joinStep);
  return {
    classification: 'strict majority of tick wall time (Linux schedstat); longest phases are wall-clock',
    unaccounted_step_classification: 'strict majority of the residual, bounded by step wall time and the rest of the tick',
    totals,
    breakdown,
    overrun_breakdown: overrunBreakdown,
    unaccounted_step_breakdown: unaccountedStepBreakdown,
    unaccounted_ticks: samples.filter((s) => s.reason === 'unaccounted').map(joinStep),
    ticks_over_20ms: samples.filter((s) => s.tick_ms > 20).length,
    worst_elapsed: top('tick_ms', 5),
    worst_on_cpu: top('cpu_ms', 3),
    worst_unaccounted: top('unaccounted_ms', 3, (s) => s.reason === 'unaccounted'),
  };
}
