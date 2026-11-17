#!/usr/bin/env node
// Aggregates the files written by capture.mjs (perf-<machine>-<renderer>-cam<N>.json) into a Markdown table + JSON.
//   node tools/perf/report.mjs [--dir reports/m0/perf] [--out-md reports/m0/perf/summary.md] [--out-json reports/m0/perf/summary.json]
// The median of the three runs is the reported value; "range" is (max - min) / median over the runs (<= 5 % is the
// repeatability criterion of M0-030 on a quiet reference machine). Entries without runs are listed as 未测 with the reason.
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { fmt, mdTable } from './lib.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

export function loadCaptures(dir) {
  return readdirSync(dir)
    .filter((f) => /^perf-.*-cam\d+\.json$/.test(f))
    .sort()
    .map((f) => ({ file: f, ...JSON.parse(readFileSync(path.join(dir, f), 'utf8')) }));
}

export function summarize(captures) {
  const rows = [];
  for (const c of captures) {
    const m = c.meta ?? {};
    if (!c.aggregate) {
      rows.push({ machine: m.machine, renderer: m.renderer_requested, cam: c.cam, viewport: m.viewport, skipped: c.skipped ?? '未测', meta: m });
      continue;
    }
    const a = c.aggregate;
    rows.push({
      machine: m.machine,
      renderer: m.renderer_requested,
      cam: c.cam,
      viewport: m.viewport,
      gpu_mode: m.gpu_mode,
      runs: a.runs,
      fps_mean: a.fps_mean.median,
      fps_range_pct: a.fps_mean.range_pct,
      interval_p50_ms: a.interval_ms_p50.median,
      interval_p50_range_pct: a.interval_ms_p50.range_pct,
      cpu_p50_ms: a.cpu_ms_p50.median,
      cpu_p99_ms: a.cpu_ms_p99.median,
      draw_calls: a.draw_calls.median,
      triangles: a.triangles.median,
      veg_instances: a.vegetation_instances.median,
      heap_mb: a.js_heap_mb.median,
      stages: Object.fromEntries(Object.entries(a.stages).map(([k, v]) => [k, { p50: v.p50.median, p99: v.p99.median }])),
      meta: m,
    });
  }
  return rows;
}

export function toMarkdown(rows) {
  const ok = rows.filter((r) => !r.skipped);
  const skipped = rows.filter((r) => r.skipped);
  const out = [];
  if (ok.length) {
    out.push(
      mdTable(
        ['machine', 'renderer', 'viewport', 'cam', 'runs', 'frame interval P50 (ms)', 'fps (mean)', 'P50 range', 'CPU P50/P99 (ms)', 'draw calls', 'triangles', 'veg instances', 'JS heap (MB)'],
        ok.map((r) => [r.machine, r.renderer, r.viewport, r.cam, r.runs, fmt(r.interval_p50_ms, 1), fmt(r.fps_mean, 2), `${fmt(r.interval_p50_range_pct, 1)} %`, `${fmt(r.cpu_p50_ms, 2)} / ${fmt(r.cpu_p99_ms, 2)}`, r.draw_calls, r.triangles, r.veg_instances, fmt(r.heap_mb, 1)]),
      ),
    );
    const names = [...new Set(ok.flatMap((r) => Object.keys(r.stages)))];
    out.push('', 'CPU time per frame stage, P50 / P99 (ms), median of the runs:', '');
    out.push(mdTable(['machine', 'renderer', 'cam', ...names], ok.map((r) => [r.machine, r.renderer, r.cam, ...names.map((n) => (r.stages[n] ? `${fmt(r.stages[n].p50, 2)} / ${fmt(r.stages[n].p99, 2)}` : 'n/a'))])));
  }
  for (const r of skipped) out.push('', `- ${r.machine} / ${r.renderer} / cam ${r.cam}: 未测 (${r.skipped})`);
  return out.join('\n') + '\n';
}

function main() {
  const argv = process.argv.slice(2);
  const get = (k, d) => (argv.includes(k) ? argv[argv.indexOf(k) + 1] : d);
  const dir = path.resolve(get('--dir', path.join(ROOT, 'reports/m0/perf')));
  const rows = summarize(loadCaptures(dir));
  const md = toMarkdown(rows);
  writeFileSync(path.resolve(get('--out-md', path.join(dir, 'summary.md'))), md);
  writeFileSync(path.resolve(get('--out-json', path.join(dir, 'summary.json'))), JSON.stringify(rows, null, 2) + '\n');
  process.stdout.write(md);
}

import { pathToFileURL } from 'node:url';
if (import.meta.url === pathToFileURL(process.argv[1]).href) main();
