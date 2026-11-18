#!/usr/bin/env node
// M1-069b: aggregates the `capture.mjs --m1` outputs into reports/m1-perf.json and judges them with budget.mjs.
//   node tools/perf/m1-report.mjs --machine RTX3080-ws --dir reports/m1-perf-raw [--tier high] [--out reports/m1-perf.json]
// Per scene row (budget.mjs schema): frame_ms.p95 = median over runs of the per-run frame-interval P95; draw_calls / tris = median;
// hitches_gt_50ms = the worst run (conservative); js_heap_mb = median. Per-pass GPU times and wasm_mb are not captured
// yet, so those checks stay UNMEASURED (never PASS by omission).
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { M1_SCENES, checkVariants, flagVariance, summarize, toMarkdown } from './budget.mjs';
import { median, rangePct } from './lib.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

/** One scene row from a capture file (`{meta, runs[]}`); null when the file has no runs (skipped / failed). */
export function rowFromCapture(cap, machine, tier) {
  const runs = cap?.runs ?? [];
  if (runs.length === 0) return null;
  const p95s = runs.map((r) => r.frame_ms.p95);
  const num = (f) => median(runs.map(f).filter((v) => Number.isFinite(v)));
  return {
    machine,
    scene: runs[0].meta.scene,
    tier,
    runs: runs.length,
    frame_ms: { p50: num((r) => r.frame_ms.p50), p95: median(p95s), p99: num((r) => r.frame_ms.p99) },
    frame_p95_runs: p95s,
    range_pct: rangePct(p95s),
    draw_calls: num((r) => r.draw_calls),
    tris: num((r) => r.triangles),
    cpu_ms_p50: num((r) => r.cpu_ms.p50),
    gpu_ms_total: num((r) => r.gpu_ms?.value),
    hitches_gt_50ms: Math.max(...runs.map((r) => r.hitches_gt_50ms)),
    js_heap_mb: num((r) => r.meta.js_heap_mb),
    vegetation_instances: num((r) => r.vegetation?.instances),
    world: runs[0].meta.world,
  };
}

export function buildReport({ dir, machine, tier, veg }) {
  const rows = {};
  const meta = {};
  for (const scene of M1_SCENES) {
    const f = path.join(dir, `perf-${machine}-webgpu-${scene}${veg ? '' : '-noveg'}.json`);
    if (!existsSync(f)) continue;
    const cap = JSON.parse(readFileSync(f, 'utf8'));
    Object.assign(meta, cap.meta ?? {});
    const row = rowFromCapture(cap, machine, tier);
    if (row) rows[scene] = row;
  }
  const summary = summarize(rows, tier);
  return { vegetation: veg, rows, summary, variance_flags: Object.values(rows).filter((r) => flagVariance(r.range_pct)).map((r) => r.scene), capture_meta: meta, markdown: toMarkdown(summary) };
}

function main() {
  const a = process.argv.slice(2);
  const opt = (k, d) => (a.includes(k) ? a[a.indexOf(k) + 1] : d);
  const dir = path.resolve(ROOT, opt('--dir', 'reports/m1-perf-raw'));
  const machine = opt('--machine', 'RTX3080-ws');
  const tier = opt('--tier', 'high');
  const out = path.resolve(ROOT, opt('--out', 'reports/m1-perf.json'));
  const variants = [true, false].map((veg) => buildReport({ dir, machine, tier, veg }));
  const doc = { task: 'M1-069b', machine, tier, variants: variants.map(({ markdown, ...v }) => v), material_variants: checkVariants(undefined) };
  writeFileSync(out, JSON.stringify(doc, null, 2) + '\n');
  for (const v of variants) console.log(`vegetation=${v.vegetation}\n${v.markdown}\n`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main();
