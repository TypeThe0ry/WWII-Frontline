#!/usr/bin/env node
// M0-030 performance regression check: measure what can be measured objectively on any machine (draw calls, triangles,
// vegetation instances, CPU time per frame stage, JS heap, bundle sizes, Rust micro-benchmarks, server tick / bandwidth /
// RSS growth), compare with reports/baselines/perf-m0.json and FAIL when a metric is beyond its tolerance.
//
//   node scripts/perf-regress.mjs check [--quick] [--skip client,bundle,rust,server]   # measure + compare, exit 1 on regression
//   node scripts/perf-regress.mjs measure [--quick] [--skip ...] [--out FILE]          # measure only (default out: target/perf-current.json)
//   node scripts/perf-regress.mjs compare [--current FILE] [--baseline FILE] [--strict] # compare two files
//   node scripts/perf-regress.mjs update-baseline [--from FILE]                         # adopt a measurement as the new baseline (a human decision)
//
// Real-GPU frame times are NOT part of this check (no GPU here): they are recorded as "未测（需参考机，H-021）" in the
// baseline and measured with tools/perf/capture.mjs on the reference machines. Timing metrics have wide tolerances because
// the machine is a shared VM; counts and sizes are tight.
import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { brotliCompressSync, gzipSync } from 'node:zlib';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { median } from '../tools/perf/lib.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const BASELINE = path.join(ROOT, 'reports/baselines/perf-m0.json');
const CURRENT = path.join(ROOT, 'target/perf-current.json');

/**
 * Tolerance rules, first match wins. A metric fails when it is worse than `baseline * (1 + rel) + abs` (dir 'up': higher is
 * worse; 'both': also fails when lower than `baseline * (1 - rel) - abs`, for counts that must stay identical).
 */
export const DEFAULT_RULES = [
  { match: '^client\\.cam\\d\\.(draw_calls|active_meshes|veg_draw_calls)$', dir: 'both', rel: 0.02, abs: 2, why: 'deterministic given fixed camera + vegetation seed' },
  { match: '^client\\.cam\\d\\.(triangles|veg_instances|veg_tris)$', dir: 'both', rel: 0.02, abs: 100, why: 'deterministic given fixed camera + vegetation seed' },
  { match: '^client\\.cam\\d\\.js_heap_mb$', dir: 'up', rel: 0.25, abs: 5, why: 'GC timing' },
  { match: '^client\\.cam\\d\\.(cpu_p50_ms|stage\\.[a-z_]+\\.p50_ms)$', dir: 'up', rel: 1.0, abs: 1.0, why: 'shared VM, 0.1 ms timer resolution; catches order-of-magnitude regressions' },
  { match: '^bundle\\.', dir: 'up', rel: 0.05, abs: 2048, why: 'build output size' },
  { match: '^rust\\.', dir: 'up', rel: 0.25, abs: 0.002, why: 'shared VM micro-benchmark noise (this machine is not the reference machine)' },
  { match: '^server\\.(tick_p50_ms|tick_p99_ms|aoi_encode_p99_ms)$', dir: 'up', rel: 1.0, abs: 0.5, why: 'shared VM, same-host clients' },
  { match: '^server\\.down_kbps_p50$', dir: 'both', rel: 0.15, abs: 1, why: 'bandwidth model' },
  { match: '^server\\.rss_slope_kb_per_s', dir: 'up', rel: 0, abs: 1.0, why: 'M0-030: a leak shows as KB/s; 0 +- 1 KB/s allowed' },
];

export function ruleFor(key, rules = DEFAULT_RULES) {
  return rules.find((r) => new RegExp(r.match).test(key));
}

/** Compares flat metric maps. Returns { rows, failed, missing }. */
export function compareMetrics(baseline, current, { strict = false } = {}) {
  const rules = baseline.rules ?? DEFAULT_RULES;
  const rows = [];
  let failed = 0;
  let missing = 0;
  for (const [key, b] of Object.entries(baseline.metrics)) {
    const cur = current[key];
    const rule = ruleFor(key, rules);
    if (cur === undefined || cur === null || !Number.isFinite(cur)) {
      missing++;
      rows.push({ key, baseline: b.value, current: null, verdict: strict ? 'FAIL (missing)' : 'skipped' });
      if (strict) failed++;
      continue;
    }
    if (!rule) {
      rows.push({ key, baseline: b.value, current: cur, verdict: 'no rule' });
      continue;
    }
    const hi = b.value * (1 + rule.rel) + rule.abs;
    const lo = b.value * (1 - rule.rel) - rule.abs;
    const bad = cur > hi || (rule.dir === 'both' && cur < lo);
    if (bad) failed++;
    rows.push({ key, baseline: b.value, current: cur, limit: rule.dir === 'both' ? `[${+lo.toFixed(4)}, ${+hi.toFixed(4)}]` : `<= ${+hi.toFixed(4)}`, delta_pct: b.value ? ((cur - b.value) / Math.abs(b.value)) * 100 : null, verdict: bad ? 'FAIL' : 'ok' });
  }
  return { rows, failed, missing };
}

export function formatRows(rows) {
  const w = Math.max(...rows.map((r) => r.key.length), 6);
  const f = (v) => (v === null || v === undefined ? 'n/a' : typeof v === 'number' ? String(+v.toFixed(4)) : v);
  return rows.map((r) => `${r.key.padEnd(w)}  base ${f(r.baseline).padStart(10)}  now ${f(r.current).padStart(10)}  ${r.delta_pct == null ? '' : `${r.delta_pct >= 0 ? '+' : ''}${r.delta_pct.toFixed(1)} %`.padStart(9)}  ${r.limit ?? ''}  ${r.verdict}`).join('\n');
}

// ---------------------------------------------------------------------------------------------- measuring

function run(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, { cwd: ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], maxBuffer: 1 << 28, ...opts });
  if (r.status !== 0) throw new Error(`${cmd} ${args.join(' ')} failed (${r.status}): ${(r.stderr || r.stdout).slice(-600)}`);
  return r.stdout;
}

export function measureClient({ quick }) {
  const dir = mkdtempSync(path.join(os.tmpdir(), 'fl-perf-'));
  try {
    const args = ['tools/perf/capture.mjs', '--out-dir', dir, '--machine', 'regress', '--repeats', quick ? '1' : '3', '--duration', quick ? '12' : '30', '--warmup', quick ? '3' : '5'];
    const out = run('node', args);
    process.stdout.write(out.split('\n').filter((l) => l.startsWith('[perf] cam')).join('\n') + '\n');
    const m = {};
    for (const f of readdirSync(dir).filter((x) => x.endsWith('.json'))) {
      const c = JSON.parse(readFileSync(path.join(dir, f), 'utf8'));
      if (!c.aggregate) continue;
      const a = c.aggregate;
      const k = `client.cam${c.cam}`;
      m[`${k}.draw_calls`] = a.draw_calls.median;
      m[`${k}.active_meshes`] = a.active_meshes.median;
      m[`${k}.triangles`] = a.triangles.median;
      m[`${k}.veg_draw_calls`] = a.vegetation_draw_calls.median;
      m[`${k}.veg_instances`] = a.vegetation_instances.median;
      m[`${k}.veg_tris`] = a.vegetation_tris.median;
      m[`${k}.js_heap_mb`] = a.js_heap_mb.median;
      m[`${k}.cpu_p50_ms`] = a.cpu_ms_p50.median;
      for (const [s, v] of Object.entries(a.stages)) m[`${k}.stage.${s}.p50_ms`] = v.p50.median;
    }
    return m;
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const walk = (dir) => readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(path.join(dir, e.name)) : [path.join(dir, e.name)]));

export function measureBundle({ build = true } = {}) {
  if (build) run('pnpm', ['-C', 'client', 'build']);
  const dist = path.join(ROOT, 'client/dist');
  if (!existsSync(dist)) throw new Error('client/dist is missing (pnpm -C client build)');
  const groups = { js: 0, css: 0, wasm: 0, html: 0 };
  const gz = { js: 0, css: 0, wasm: 0, html: 0 };
  const br = { js: 0, css: 0, wasm: 0, html: 0 };
  for (const f of walk(dist)) {
    const ext = path.extname(f).slice(1);
    const g = ext === 'js' || ext === 'mjs' ? 'js' : ext === 'css' ? 'css' : ext === 'wasm' ? 'wasm' : ext === 'html' ? 'html' : null;
    if (!g || f.includes(`${path.sep}maps${path.sep}`)) continue;
    const buf = readFileSync(f);
    groups[g] += buf.length;
    gz[g] += gzipSync(buf, { level: 9 }).length;
    br[g] += brotliCompressSync(buf).length;
  }
  const m = {};
  for (const g of Object.keys(groups)) {
    m[`bundle.${g}_raw_bytes`] = groups[g];
    m[`bundle.${g}_gzip_bytes`] = gz[g];
    m[`bundle.${g}_brotli_bytes`] = br[g];
  }
  const data = walk(path.join(dist, 'maps')).reduce((s, f) => s + statSync(f).size, 0);
  m['bundle.map_assets_raw_bytes'] = data;
  return m;
}

export function measureRust({ quick }) {
  const dir = mkdtempSync(path.join(os.tmpdir(), 'fl-bench-'));
  const env = { ...process.env, FRONTLINE_BASELINE_DIR: dir, CARGO_INCREMENTAL: '0', CARGO_PROFILE_DEV_DEBUG: '0', CARGO_PROFILE_TEST_DEBUG: '0' };
  try {
    run('cargo', ['bench', '-p', 'frontline-sim', '--bench', 'kcc'], { env });
    const rd = (f) => JSON.parse(readFileSync(path.join(dir, f), 'utf8'));
    const k = rd('sim-m0.json');
    const m = { 'rust.kcc_step_median_us': k.kcc_step_median_us, 'rust.kcc_100_players_tick_median_ms': k.kcc_100_players_tick_median_ms };
    if (!quick) {
      run('cargo', ['bench', '-p', 'frontline-sim', '--bench', 'ballistics'], { env });
      const b = rd('sim-m0-weapon.json');
      m['rust.ballistics_400_projectiles_stage_median_ms'] = b.ballistics_400_projectiles_stage_median_ms;
      m['rust.world_step_20_players_firing_median_ms'] = b.world_step_20_players_firing_median_ms;
      run('cargo', ['bench', '-p', 'frontline-server', '--bench', 'aoi_encode'], { env });
      const s = rd('server-m0.json').aoi_encode;
      m['rust.aoi_encode_100_clients_p50_ms'] = s.clients_100.tick_ms_p50;
      m['rust.aoi_encode_100_clients_us_per_client_packet'] = s.clients_100.us_per_client_packet;
      m['rust.aoi_encode_10_clients_us_per_client_packet'] = s.clients_10.us_per_client_packet;
      run('cargo', ['bench', '-p', 'frontline-bots', '--bench', 'think'], { env });
      const t = rd('bots-m0.json');
      m['rust.bots_think_60_median_ms'] = t.think_60.think_ms?.median ?? t.think_60.median_ms;
    }
    return Object.fromEntries(Object.entries(m).filter(([, v]) => Number.isFinite(v)));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

export function measureServer({ quick }) {
  const out = path.join(os.tmpdir(), `fl-loadtest-${process.pid}.json`);
  // 40 headless clients on WebTransport for 120 s: the per-client leak of M0-030 (125 B/s per client) shows as ~5 KB/s.
  run('scripts/loadtest-m0.sh', ['--transport', 'wt', '--count', quick ? '20' : '40', '--bots', '10', '--duration', quick ? '60' : '120', '--warmup', '5', '--sample-interval', '5', '--out', out, '--no-build'], { env: { ...process.env } });
  const j = JSON.parse(readFileSync(out, 'utf8'));
  rmSync(out, { force: true });
  const samples = (j.rss_samples ?? []).filter((s) => s.server_kb != null && s.t >= 30);
  let slope = null;
  if (samples.length >= 3) {
    // least squares slope of RSS over time, KB/s
    const n = samples.length;
    const mt = samples.reduce((a, s) => a + s.t, 0) / n;
    const mr = samples.reduce((a, s) => a + s.server_kb, 0) / n;
    const num = samples.reduce((a, s) => a + (s.t - mt) * (s.server_kb - mr), 0);
    const den = samples.reduce((a, s) => a + (s.t - mt) ** 2, 0);
    slope = den ? num / den : null;
  }
  return Object.fromEntries(
    Object.entries({
      'server.tick_p50_ms': j.server?.tick_ms?.p50,
      'server.tick_p99_ms': j.server?.tick_ms?.p99,
      'server.aoi_encode_p99_ms': j.server?.aoi_encode_ms?.p99,
      'server.down_kbps_p50': j.clients?.down_kbps?.p50,
      [`server.rss_slope_kb_per_s_c${quick ? 20 : 40}`]: slope,
    }).filter(([, v]) => Number.isFinite(v)),
  );
}

function hostMeta() {
  const git = (...a) => {
    try {
      return execFileSync('git', a, { cwd: ROOT, encoding: 'utf8' }).trim();
    } catch {
      return 'unknown';
    }
  };
  return { machine: os.hostname(), cpu: os.cpus()[0]?.model, cores: os.cpus().length, mem_gb: +(os.totalmem() / 2 ** 30).toFixed(1), os: `${os.type()} ${os.release()}`, node: process.version, commit: git('rev-parse', '--short', 'HEAD'), dirty: git('status', '--porcelain').length > 0, date: new Date().toISOString() };
}

export function measureAll({ quick, skip }) {
  const metrics = {};
  const t0 = Date.now();
  for (const [name, fn] of [
    ['bundle', () => measureBundle()],
    ['client', () => measureClient({ quick })],
    ['rust', () => measureRust({ quick })],
    ['server', () => measureServer({ quick })],
  ]) {
    if (skip.includes(name)) continue;
    console.log(`[perf-regress] measuring ${name} ...`);
    Object.assign(metrics, fn());
  }
  return { schema: 1, meta: { ...hostMeta(), quick, skipped: skip, measure_s: Math.round((Date.now() - t0) / 1000) }, metrics };
}

// ---------------------------------------------------------------------------------------------- CLI

function parse(argv) {
  const o = { cmd: argv[0], quick: false, skip: [], out: null, current: null, baseline: BASELINE, strict: false, from: null };
  for (let i = 1; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--quick') o.quick = true;
    else if (a === '--skip') o.skip = argv[++i].split(',');
    else if (a === '--out') o.out = path.resolve(argv[++i]);
    else if (a === '--current') o.current = path.resolve(argv[++i]);
    else if (a === '--baseline') o.baseline = path.resolve(argv[++i]);
    else if (a === '--from') o.from = path.resolve(argv[++i]);
    else if (a === '--strict') o.strict = true;
    else throw new Error(`unknown option ${a}`);
  }
  return o;
}

function doCompare(o, current) {
  const base = JSON.parse(readFileSync(o.baseline, 'utf8'));
  const { rows, failed, missing } = compareMetrics(base, current.metrics, { strict: o.strict });
  console.log(formatRows(rows));
  console.log(`\n${rows.length} metrics compared against ${path.relative(ROOT, o.baseline)} (baseline from ${base.meta?.commit ?? '?'} ${base.meta?.date?.slice(0, 10) ?? ''}): ${failed} over tolerance, ${missing} not measured.`);
  if (base.gpu_unmeasured) console.log('Real-GPU frame times: 未测（需参考机，H-021）— not part of this check.');
  return failed === 0;
}

function main() {
  const o = parse(process.argv.slice(2));
  if (o.cmd === 'measure' || o.cmd === 'check') {
    const cur = measureAll(o);
    const out = o.out ?? CURRENT;
    mkdirSync(path.dirname(out), { recursive: true });
    writeFileSync(out, JSON.stringify(cur, null, 2) + '\n');
    console.log(`[perf-regress] wrote ${path.relative(ROOT, out)}`);
    if (o.cmd === 'check') process.exit(doCompare(o, cur) ? 0 : 1);
  } else if (o.cmd === 'compare') {
    process.exit(doCompare(o, JSON.parse(readFileSync(o.current ?? CURRENT, 'utf8'))) ? 0 : 1);
  } else if (o.cmd === 'update-baseline') {
    const cur = JSON.parse(readFileSync(o.from ?? CURRENT, 'utf8'));
    const old = existsSync(o.baseline) ? JSON.parse(readFileSync(o.baseline, 'utf8')) : {};
    const doc = {
      schema: 1,
      meta: cur.meta,
      gpu_unmeasured: old.gpu_unmeasured ?? {
        'RTX 3060 / 1080p / WebGPU': '未测（需参考机，H-021）',
        'Iris Xe / 720p / WebGL2 (Low)': '未测（需参考机，H-021）',
      },
      rules: old.rules ?? DEFAULT_RULES,
      metrics: Object.fromEntries(Object.entries(cur.metrics).map(([k, v]) => [k, { value: v }])),
    };
    writeFileSync(o.baseline, JSON.stringify(doc, null, 2) + '\n');
    console.log(`[perf-regress] baseline written: ${path.relative(ROOT, o.baseline)} (${Object.keys(doc.metrics).length} metrics)`);
  } else {
    console.log('usage: perf-regress.mjs check|measure|compare|update-baseline [options] (see the header of this file)');
    process.exit(2);
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (e) {
    console.error(`[perf-regress] ${e instanceof Error ? e.message : e}`);
    process.exit(2);
  }
}
void median;
