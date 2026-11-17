// Pure helpers shared by capture.mjs, report.mjs and scripts/perf-regress.mjs (tested in lib.test.mjs).

/** Nearest-rank percentile of an unsorted array (same definition as client/src/render/debug/frame-stats.ts). */
export function percentile(values, q) {
  if (!values.length) return 0;
  const a = [...values].sort((x, y) => x - y);
  const idx = Math.min(a.length - 1, Math.max(0, Math.ceil(q * a.length) - 1));
  return a[idx];
}

export function median(values) {
  if (!values.length) return 0;
  const a = [...values].sort((x, y) => x - y);
  const m = a.length >> 1;
  return a.length % 2 ? a[m] : (a[m - 1] + a[m]) / 2;
}

/** (max - min) / median, in percent; 0 for fewer than two values or a zero median. */
export function rangePct(values) {
  if (values.length < 2) return 0;
  const md = median(values);
  if (md === 0) return 0;
  return ((Math.max(...values) - Math.min(...values)) / Math.abs(md)) * 100;
}

/** Walks `obj` along a dotted path ("a.b.c"); undefined when a segment is missing. */
export function getPath(obj, path) {
  let cur = obj;
  for (const seg of path.split('.')) {
    if (cur === null || typeof cur !== 'object' || !(seg in cur)) return undefined;
    cur = cur[seg];
  }
  return cur;
}

/** All dotted paths of numeric leaves of `obj` (arrays are skipped). */
export function numericPaths(obj, prefix = '') {
  const out = [];
  for (const [k, v] of Object.entries(obj ?? {})) {
    const p = prefix ? `${prefix}.${k}` : k;
    if (typeof v === 'number' && Number.isFinite(v)) out.push(p);
    else if (v && typeof v === 'object' && !Array.isArray(v)) out.push(...numericPaths(v, p));
  }
  return out;
}

/**
 * Aggregates the `__perf.dump()` results of repeated runs of one camera: per metric the median over the runs and the
 * range (max - min) / median in percent (the repeatability criterion of M0-030: <= 5 % on a quiet machine).
 * `runs` is an array of PerfDump-like objects (schema 2).
 */
export function aggregateRuns(runs) {
  const pick = (f) => runs.map(f).filter((v) => Number.isFinite(v));
  const stat = (f) => {
    const v = pick(f);
    return { median: median(v), range_pct: rangePct(v), min: v.length ? Math.min(...v) : 0, max: v.length ? Math.max(...v) : 0, runs: v };
  };
  const stageNames = [...new Set(runs.flatMap((r) => Object.keys(r.stages ?? {})))].sort();
  const stages = {};
  for (const n of stageNames) {
    stages[n] = { p50: stat((r) => r.stages?.[n]?.p50), p99: stat((r) => r.stages?.[n]?.p99) };
  }
  return {
    runs: runs.length,
    fps_mean: stat((r) => r.fps),
    cpu_ms_p50: stat((r) => r.cpu_ms?.p50),
    cpu_ms_p99: stat((r) => r.cpu_ms?.p99),
    interval_ms_p50: stat((r) => r.interval_ms?.p50),
    draw_calls: stat((r) => r.draw_calls),
    active_meshes: stat((r) => r.active_meshes),
    triangles: stat((r) => r.triangles),
    vegetation_draw_calls: stat((r) => r.vegetation?.drawCalls),
    vegetation_instances: stat((r) => r.vegetation?.instances),
    vegetation_tris: stat((r) => r.vegetation?.tris),
    js_heap_mb: stat((r) => r.meta?.js_heap_mb),
    frames: stat((r) => r.frames),
    stages,
  };
}

/** Markdown table helper. */
export function mdTable(header, rows) {
  const line = (cells) => `| ${cells.join(' | ')} |`;
  return [line(header), line(header.map(() => '---')), ...rows.map(line)].join('\n');
}

export const fmt = (v, d = 2) => (typeof v === 'number' && Number.isFinite(v) ? v.toFixed(d) : 'n/a');
