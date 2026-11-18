// M1-069: budget comparison for the M1 perf report (ARCHITECTURE §13.5, MILESTONES E1-6/E1-7/E1-8).
// Pure; takes the per-scene summary rows (`reports/m1-perf.json` schema) and returns PASS/FAIL/未测 per check.

export const M1_SCENES = ['open_field', 'hedgerow', 'old_town'];

/** Budgets per machine class. `null` = not applicable / not enforced for that class. */
export const BUDGETS = {
  high: { p95_ms: 16.7, draw_calls: 1200, tris: 4_000_000, hitches: 2, pass_ms: { csm: 2.2, opaque: 4.0, transparent: 0.7, taa: 0.8, post: 0.7 }, mem_mb: 1536, vram_mb: 4096 },
  low: { p95_ms: 33.3, draw_calls: 500, tris: null, hitches: 2, pass_ms: null, mem_mb: 1536, vram_mb: null },
  low_webgl2_medium: { p95_ms: 33.3, draw_calls: 900, tris: null, hitches: 2, pass_ms: null, mem_mb: 1536, vram_mb: null },
};
export const MAX_MATERIAL_VARIANTS = 60;
/** Spread between repeated runs above which a metric is flagged (M1-069 behaviour 4). */
export const VARIANCE_FLAG_PCT = 10;

const verdict = (v, limit) => (v === undefined || v === null || !Number.isFinite(v) ? 'UNMEASURED' : v <= limit ? 'PASS' : 'FAIL');

/**
 * @param {object} row one scene row: {machine, scene, tier, frame_ms:{p95}, draw_calls, tris, gpu_ms:{...}, js_heap_mb, wasm_mb, vram_mb_est, hitches_gt_50ms, range_pct?}
 * @param {'high'|'low'|'low_webgl2_medium'} tier
 * @returns {{check:string, value:number|null, limit:number, verdict:'PASS'|'FAIL'|'UNMEASURED', over?:number}[]}
 */
export function checkRow(row, tier) {
  const b = BUDGETS[tier];
  if (!b) throw new Error(`unknown tier '${tier}'`);
  const out = [];
  const add = (check, value, limit) => {
    if (limit === null || limit === undefined) return;
    const v = verdict(value, limit);
    const e = { check, value: Number.isFinite(value) ? value : null, limit, verdict: v };
    if (v === 'FAIL') e.over = +(value - limit).toFixed(3);
    out.push(e);
  };
  add('frame_ms.p95', row.frame_ms?.p95, b.p95_ms);
  add('draw_calls', row.draw_calls, b.draw_calls);
  add('tris', row.tris, b.tris);
  add('hitches_gt_50ms', row.hitches_gt_50ms, b.hitches);
  if (b.pass_ms) for (const [k, lim] of Object.entries(b.pass_ms)) add(`gpu_ms.${k}`, row.gpu_ms?.[k], lim);
  const mem = Number.isFinite(row.js_heap_mb) && Number.isFinite(row.wasm_mb) ? row.js_heap_mb + row.wasm_mb : null;
  add('js_heap_mb+wasm_mb', mem, b.mem_mb);
  add('vram_mb_est', row.vram_mb_est, b.vram_mb);
  return out;
}

export function checkVariants(count) {
  const v = verdict(count, MAX_MATERIAL_VARIANTS);
  return { check: 'material_variants', value: Number.isFinite(count) ? count : null, limit: MAX_MATERIAL_VARIANTS, verdict: v, ...(v === 'FAIL' ? { over: count - MAX_MATERIAL_VARIANTS } : {}) };
}

/** True when repeated runs of one metric spread more than VARIANCE_FLAG_PCT around the median. */
export const flagVariance = (rangePct) => Number.isFinite(rangePct) && rangePct > VARIANCE_FLAG_PCT;

/** Overall per machine: FAIL if any check fails; UNMEASURED if any scene is missing or any check unmeasured; else PASS. */
export function summarize(rowsByScene, tier) {
  const checks = [];
  let missing = 0;
  for (const s of M1_SCENES) {
    const row = rowsByScene[s];
    if (!row) {
      missing++;
      continue;
    }
    for (const c of checkRow(row, tier)) checks.push({ scene: s, ...c });
  }
  const overall = checks.some((c) => c.verdict === 'FAIL') ? 'FAIL' : missing > 0 || checks.some((c) => c.verdict === 'UNMEASURED') ? 'UNMEASURED' : 'PASS';
  return { tier, overall, missing_scenes: missing, checks };
}

/** Markdown table; over-budget rows list the excess. */
export function toMarkdown(summary) {
  const lines = [`### ${summary.tier}: ${summary.overall}`, '', '| scene | check | value | limit | verdict |', '| --- | --- | --- | --- | --- |'];
  for (const c of summary.checks) lines.push(`| ${c.scene} | ${c.check} | ${c.value ?? '未测'} | ${c.limit} | ${c.verdict}${c.over !== undefined ? ` (+${c.over})` : ''} |`);
  return lines.join('\n');
}
