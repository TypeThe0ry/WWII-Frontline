// Classify slow ticks by Linux schedstat, not by the largest wall-clock phase. A descheduled
// thread can wait inside any phase; an unexplained remainder is not proof of I/O blocking.
const PHASES = ['inputs', 'step', 'post_step', 'bots', 'snapshot', 'send'];
const SYSTEMS = ['movement', 'weapons', 'projectiles', 'vehicles', 'rockets', 'shells', 'throwables', 'other'];
const round = (ms) => Math.round(ms * 1000) / 1000;

function largest(sample, fields, key) {
  const values = fields.flatMap((name) => Number.isFinite(sample[key(name)])
    ? [{ name, ms: round(sample[key(name)]) }] : []);
  return values.sort((a, b) => b.ms - a.ms)[0] ?? null;
}

export function summarizeTickTrace(serverLog) {
  const totals = { tick_slow: 0, tick_overrun: 0 };
  const breakdown = { run_queue: 0, on_cpu: 0, unaccounted: 0, mixed: 0, no_schedstat: 0 };
  const overrunBreakdown = { ...breakdown };
  const samples = [];
  const steps = new Map();
  for (const line of serverLog.matchAll(/^\{[^\n]*"event":"tick_(?:slow|overrun|step_trace)"[^\n]*$/gm)) {
    const e = JSON.parse(line[0]);
    if (e.event === 'tick_step_trace') {
      steps.set(e.tick, e);
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
    samples.push({
      tick: e.tick,
      event: e.event,
      tick_ms: round(e.tick_ms),
      cpu_ms: schedstat ? round(e.cpu_ms) : null,
      runq_wait_ms: schedstat ? round(e.runq_wait_ms) : null,
      unaccounted_ms: schedstat ? round(residual) : null,
      reason,
      longest_wall_phase: largest(e, PHASES, (phase) => `phase_${phase}_ms`),
    });
  }
  const top = (by, n, filter = () => true) => samples.filter((s) => filter(s) && s[by] != null)
    .sort((a, b) => b[by] - a[by]).slice(0, n).map((s) => ({
      ...s,
      longest_step_system: largest(steps.get(s.tick) ?? {}, SYSTEMS, (system) => `${system}_ms`),
    }));
  return {
    classification: 'strict majority of tick wall time (Linux schedstat); longest phases are wall-clock',
    totals,
    breakdown,
    overrun_breakdown: overrunBreakdown,
    ticks_over_20ms: samples.filter((s) => s.tick_ms > 20).length,
    worst_elapsed: top('tick_ms', 5),
    worst_on_cpu: top('cpu_ms', 3),
    worst_unaccounted: top('unaccounted_ms', 3, (s) => s.reason === 'unaccounted'),
  };
}
