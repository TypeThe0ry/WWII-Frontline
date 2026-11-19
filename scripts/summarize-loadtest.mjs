#!/usr/bin/env node
// M0-027: merges a headless-client report, the server's own report, periodic /metrics scrapes and
// RSS samples into one summary JSON (reports/m0/loadtest-*.json) and judges it against the task's
// acceptance numbers. Exit code 1 when a hard criterion fails.
//
// usage: node scripts/summarize-loadtest.mjs --headless h.json --server-report s.json
//          --metrics-dir DIR --rss rss.tsv --out OUT.json --transport wt|ws --bots N
//          [--profile release|debug] [--sample-interval S] [--drive-vehicle] [--aoi-traces DIR]
import { readFileSync, readdirSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import os from 'node:os';
import { auditAoi } from './audit-loadtest-aoi.mjs';

// ---- metrics text -------------------------------------------------------------------------------

/** Parses Prometheus text into [{name, labels:{}, value}]. */
export function parseMetrics(text) {
  const out = [];
  for (const line of text.split('\n')) {
    if (!line || line.startsWith('#')) continue;
    const m = /^([a-zA-Z_:][a-zA-Z0-9_:]*)(?:\{(.*)\})?\s+(\S+)$/.exec(line);
    if (!m) continue;
    const labels = {};
    if (m[2]) for (const kv of m[2].matchAll(/([a-zA-Z_][a-zA-Z0-9_]*)="((?:[^"\\]|\\.)*)"/g)) labels[kv[1]] = kv[2];
    out.push({ name: m[1], labels, value: Number(m[3]) });
  }
  return out;
}

/** Histograms by `name{labels-without-le}` -> {count, sum, buckets:[[le, cumulative]]}. */
export function histograms(series) {
  const h = new Map();
  const get = (name, labels) => {
    const key = `${name}${JSON.stringify(labels)}`;
    if (!h.has(key)) h.set(key, { name, labels, count: 0, sum: 0, buckets: [] });
    return h.get(key);
  };
  for (const s of series) {
    if (s.name.endsWith('_bucket')) {
      const { le, ...rest } = s.labels;
      get(s.name.slice(0, -7), rest).buckets.push([le === '+Inf' ? Infinity : Number(le), s.value]);
    } else if (s.name.endsWith('_count')) {
      get(s.name.slice(0, -6), s.labels).count = s.value;
    } else if (s.name.endsWith('_sum')) {
      get(s.name.slice(0, -4), s.labels).sum = s.value;
    }
  }
  return [...h.values()].filter((x) => x.buckets.length);
}

/** Upper bound of the bucket that holds quantile q (an over-estimate by up to one bucket width). */
export function quantileUpperBound(hist, q) {
  const total = hist.count || (hist.buckets.at(-1)?.[1] ?? 0);
  if (!total) return null;
  const want = q * total;
  for (const [le, cum] of [...hist.buckets].sort((a, b) => a[0] - b[0])) if (cum >= want) return le;
  return null;
}

const r3 = (x) => (x == null || !Number.isFinite(x) ? x : Math.round(x * 1000) / 1000);

// ---- rss ---------------------------------------------------------------------------------------

/** rows: [{t, server_kb, headless_kb}] */
export function rssGrowth(rows, durationS) {
  const s = rows.filter((r) => Number.isFinite(r.server_kb));
  if (s.length < 2) return { note: 'fewer than two RSS samples' };
  const tEnd = s.at(-1).t;
  const window = Math.min(600, Math.floor(durationS / 2));
  const start = s.find((r) => r.t >= tEnd - window) ?? s[0];
  const end = s.at(-1);
  const growth = ((end.server_kb - start.server_kb) / start.server_kb) * 100;
  return {
    window_s: end.t - start.t,
    requested_window_s: window,
    server_rss_start_mb: r3(start.server_kb / 1024),
    server_rss_end_mb: r3(end.server_kb / 1024),
    growth_pct: r3(growth),
    server_rss_max_mb: r3(Math.max(...s.map((r) => r.server_kb)) / 1024),
    samples: s.length,
  };
}

// ---- main --------------------------------------------------------------------------------------

function arg(argv, name, def) {
  const i = argv.indexOf(`--${name}`);
  return i >= 0 ? argv[i + 1] : def;
}

export function summarize(opts) {
  const headless = JSON.parse(readFileSync(opts.headless, 'utf8'));
  const srv = JSON.parse(readFileSync(opts.serverReport, 'utf8'));
  const files = readdirSync(opts.metricsDir).filter((f) => /^metrics-\d+\.txt$/.test(f)).sort();
  const last = files.length ? parseMetrics(readFileSync(join(opts.metricsDir, files.at(-1)), 'utf8')) : [];
  const hists = histograms(last);
  const pick = (name, labelFilter = () => true) => hists.filter((h) => h.name === name && labelFilter(h.labels));
  const hq = (h, q) => r3(quantileUpperBound(h, q));

  const aoi = pick('aoi_encode_ms')[0];
  const snap = pick('snapshot_bytes')[0];
  const perClient = pick('net_bytes_out_per_client_kbps');
  const rejected = {};
  for (const s of last) if (s.name === 'input_rejected_total' && s.value > 0) rejected[s.labels.reason] = s.value;
  const gauge = (name, labels = {}) =>
    last.find((s) => s.name === name && Object.entries(labels).every(([k, v]) => s.labels[k] === v))?.value;

  const aoiLeakServer = last.find((x) => x.name === 'aoi_enemy_leak_total')?.value;
  const rows = existsRss(opts.rss);
  const agg = headless.aggregate;
  const legacyAoi = {
    records_checked: agg.aoi_records_checked ?? 0,
    beyond_close_records: agg.aoi_beyond_close ?? 0,
    leaks: agg.aoi_leaks ?? 0,
    worst_leak_m: r3(agg.aoi_worst_leak_m ?? 0),
  };
  const sameTick = opts.aoiTraces ? auditAoi(
    readFileSync(join(opts.aoiTraces, 'server.log'), 'utf8'),
    readFileSync(join(opts.aoiTraces, 'headless.log'), 'utf8'),
  ) : null;
  const expectedSnapshots = agg.snapshots + headless.per_client.reduce((n, c) => n + (c.reordered_snapshots ?? 0), 0);
  const isWt = opts.transport === 'wt';
  const th = isWt
    ? { tick_p99_ms: 28, overrun_consecutive_max: 1, down_p50_kbps: 128, up_p50_kbps: 32 }
    : { tick_p99_ms: 40, overrun_consecutive_max: null, down_p50_kbps: 128, up_p50_kbps: 32 };
  const rss = rssGrowth(rows, headless.duration_s);

  const server = {
    ticks: srv.ticks,
    elapsed_s: r3(srv.elapsed_s),
    tick_ms: { p50: r3(srv.tick_ms.p50), p99: r3(srv.tick_ms.p99), max: r3(srv.tick_ms.max), mean: r3(srv.tick_ms.mean) },
    tick_interval_error_ms: { p99: r3(srv.tick_interval_error_ms.p99), max: r3(srv.tick_interval_error_ms.max) },
    overrun_total: srv.tick_overrun_total,
    overrun_consecutive_max: srv.tick_overrun_consecutive_max,
    tick_skipped_slots_total: srv.tick_skipped_slots_total,
    input_queue_full_total: srv.input_queue_full_total,
    aoi_encode_ms: aoi ? { p99: hq(aoi, 0.99), p50: hq(aoi, 0.5), count: aoi.count, note: 'bucket upper bounds of the final scrape' } : null,
    snapshot_bytes: snap ? { p50: hq(snap, 0.5), p99: hq(snap, 0.99), count: snap.count } : null,
    bytes_out_per_client: perClient.map((h) => ({ labels: h.labels, p50_kbps: hq(h, 0.5), p99_kbps: hq(h, 0.99), count: h.count })),
    events: srv.events ?? null,
    input_rejected: rejected,
    bots: gauge('server_bots'),
    rss,
  };

  const checks = {
    tick_p99: { value: server.tick_ms.p99, limit: th.tick_p99_ms, ok: server.tick_ms.p99 <= th.tick_p99_ms },
    overrun_consecutive_max:
      th.overrun_consecutive_max == null
        ? { value: server.overrun_consecutive_max, limit: null, ok: true }
        : { value: server.overrun_consecutive_max, limit: th.overrun_consecutive_max, ok: server.overrun_consecutive_max <= th.overrun_consecutive_max },
    down_p50_each_client: { value: r3(agg.down_kbps_p50_worst_client), limit: th.down_p50_kbps, ok: agg.down_kbps_p50_worst_client <= th.down_p50_kbps },
    up_p50_each_client: { value: r3(agg.up_kbps_p50_worst_client), limit: th.up_p50_kbps, ok: agg.up_kbps_p50_worst_client <= th.up_p50_kbps },
    disconnects: { value: agg.disconnects, limit: 0, ok: agg.disconnects === 0 && agg.clients_failed === 0 },
    decode_errors: { value: agg.decode_errors, limit: 0, ok: agg.decode_errors === 0 },
    // M1-068b / E1-4: no enemy record outside the R3 rule, by the server's own audit counter and by the clients' independent audit
    aoi_leaks_server: { value: aoiLeakServer ?? null, limit: 0, ok: sameTick ? aoiLeakServer === 0 : (aoiLeakServer ?? 0) === 0 },
    aoi_leaks_client_audit: {
      value: sameTick?.leaks ?? agg.aoi_leaks ?? null,
      limit: 0,
      ok: (sameTick?.leaks ?? legacyAoi.leaks) === 0,
      records_checked: sameTick?.records_checked ?? legacyAoi.records_checked,
      beyond_close: sameTick?.beyond_close_records ?? legacyAoi.beyond_close_records,
    },
    client_cpu_pct_of_one_core: { value: r3(headless.env.client_cpu_pct_of_one_core), limit: 50, ok: headless.env.client_cpu_pct_of_one_core < 50 },
    server_rss_growth_pct: rss.growth_pct == null ? { value: null, limit: 2, ok: true, note: rss.note } : { value: rss.growth_pct, limit: 2, ok: rss.growth_pct < 2, window_s: rss.window_s },
  };
  const hard = ['tick_p99', 'overrun_consecutive_max', 'down_p50_each_client', 'up_p50_each_client', 'disconnects', 'decode_errors', 'aoi_leaks_server', 'aoi_leaks_client_audit'];
  if (sameTick) {
    checks.aoi_trace_coverage = {
      ok: expectedSnapshots > 0 && sameTick.matched_snapshots === expectedSnapshots &&
        sameTick.missing_pose_snapshots === 0 && sameTick.legacy_candidates.length === legacyAoi.leaks &&
        (isWt || agg.missing_baseline === 0),
      expected_snapshots: expectedSnapshots,
      matched_snapshots: sameTick.matched_snapshots,
      missing_pose_snapshots: sameTick.missing_pose_snapshots,
      missing_baselines: agg.missing_baseline,
    };
    hard.push('aoi_trace_coverage');
  }
  const vehicle = headless.per_client.find((c) => c.id === 0)?.vehicle;
  if (opts.driveVehicle) {
    const comparisons = vehicle?.corrections?.comparisons ?? null;
    checks.vehicle_driver = {
      ok: vehicle?.entries > 0 && vehicle?.exits > 0 && vehicle?.last_vehicle_slot === 0 &&
        vehicle?.driver_snapshots > 0 && vehicle?.driver_input_ticks > 0 && vehicle?.max_displacement_m >= 1 && comparisons > 0,
      displacement_m: vehicle?.max_displacement_m ?? null,
      comparisons,
      note: 'client 0 must board, drive at least 1 m, have a same-tick reconciliation comparison and exit',
    };
    hard.push('vehicle_driver');
  }
  const verdict = {
    all_ok: Object.values(checks).every((c) => c.ok),
    hard_ok: hard.every((k) => checks[k].ok),
    failed: Object.entries(checks).filter(([, c]) => !c.ok).map(([k]) => k),
  };

  const TYPICAL = 54; // ARCHITECTURE §6.9.2 typical-case derivation, kbps per client
  const summary = {
    task: 'M0-027',
    generated_at: new Date().toISOString(),
    transport: opts.transport,
    count: headless.count,
    bots: Number(opts.bots),
    duration_s: headless.duration_s,
    warmup_s: headless.warmup_s,
    seed: headless.seed,
    build: opts.profile,
    ...(opts.netem ? { netem: opts.netem, netem_note: 'private netns loopback; delay is one-way per packet' } : {}),
    host: {
      cpus: os.cpus().length,
      cpu_model: os.cpus()[0]?.model,
      os: `${os.type()} ${os.release()}`,
      loadavg_at_end: os.loadavg().map(r3),
      pinned: !!opts.pinned,
      note: opts.pinned
        ? 'server on CPUs 0-1 and headless client on CPUs 2-3 of the same VM (taskset); other tenants/jobs on the VM are not controlled'
        : 'server, headless client and the sampler share this host without pinning; P99 figures include that contention',
    },
    server,
    clients: {
      joined: agg.clients_joined,
      failed: agg.clients_failed,
      down_kbps: agg.down_kbps,
      up_kbps: agg.up_kbps,
      down_kbps_p50_worst_client: r3(agg.down_kbps_p50_worst_client),
      up_kbps_p50_worst_client: r3(agg.up_kbps_p50_worst_client),
      rtt_ms: agg.rtt_ms,
      decode_errors: agg.decode_errors,
      missing_baseline: agg.missing_baseline,
      disconnects: agg.disconnects,
      snapshots: agg.snapshots,
      keyframe_record_ratio: r3(agg.keyframe_ratio),
      corrections: agg.corrections,
      deaths_observed: agg.deaths,
      respawns_observed: agg.respawns,
      shots_fired_observed: agg.shots_fired,
      aoi_audit: {
        ...(sameTick ?? legacyAoi),
        mode: sameTick ? 'same_tick' : 'predicted_pose',
        ...(sameTick ? { predicted_pose: legacyAoi } : {}),
        server_enemy_leak_total: aoiLeakServer ?? null,
      },
      client_cpu_pct_of_one_core: r3(headless.env.client_cpu_pct_of_one_core),
    },
    reference: {
      arch_6_9_2_typical_down_kbps: TYPICAL,
      measured_down_p50_over_typical: r3(agg.down_kbps.p50 / TYPICAL),
      exceeds_2x_typical: agg.down_kbps.p50 > 2 * TYPICAL,
    },
    thresholds: th,
    checks,
    verdict,
    per_client: headless.per_client.map((c) => ({
      id: c.id, player_id: c.player_id, team: c.team, join_error: c.join_error,
      down_kbps: c.down_kbps, up_kbps: c.up_kbps, rtt_ms: c.rtt_ms,
      decode_errors: c.decode_errors, missing_baseline: c.missing_baseline, disconnects: c.disconnects,
      snapshots: c.snapshots, keyframe_ratio: r3(c.keyframe_ratio), corrections: c.corrections,
      deaths: c.deaths, respawns: c.respawns,
      ...(c.vehicle != null ? { vehicle: c.vehicle } : {}),
    })),
    rss_samples: rows,
  };
  return summary;
}

function existsRss(path) {
  try {
    return readFileSync(path, 'utf8')
      .split('\n')
      .filter(Boolean)
      .map((l) => {
        const [t, s, h] = l.split('\t');
        return { t: Number(t), server_kb: s === '' || s == null ? NaN : Number(s), headless_kb: h === '' || h == null ? NaN : Number(h) };
      });
  } catch {
    return [];
  }
}

if (process.argv[1] && import.meta.url === new URL(`file://${process.argv[1]}`).href) {
  const a = process.argv;
  const opts = {
    headless: arg(a, 'headless'),
    serverReport: arg(a, 'server-report'),
    metricsDir: arg(a, 'metrics-dir'),
    aoiTraces: arg(a, 'aoi-traces'),
    rss: arg(a, 'rss'),
    out: arg(a, 'out'),
    transport: arg(a, 'transport', 'wt'),
    bots: arg(a, 'bots', '0'),
    profile: arg(a, 'profile', 'release'),
    pinned: arg(a, 'pinned', '0') === '1',
    driveVehicle: a.includes('--drive-vehicle'),
    netem: process.env.LOADTEST_NETEM,
  };
  if (!opts.headless || !opts.serverReport || !opts.out) {
    console.error('usage: summarize-loadtest.mjs --headless h.json --server-report s.json --metrics-dir DIR --rss rss.tsv --out OUT.json [--transport wt|ws] [--bots N] [--profile release|debug]');
    process.exit(2);
  }
  const s = summarize(opts);
  mkdirSync(dirname(opts.out), { recursive: true });
  writeFileSync(opts.out, `${JSON.stringify(s, null, 2)}\n`);
  const c = s.checks;
  console.log(
    `${s.transport}: ${s.count} clients + ${s.bots} bots, ${s.duration_s}s (${s.build} build)\n` +
      `  server tick_ms P50 ${s.server.tick_ms.p50} / P99 ${s.server.tick_ms.p99} / max ${s.server.tick_ms.max}; overruns ${s.server.overrun_total} (max consecutive ${s.server.overrun_consecutive_max}); aoi_encode P99<= ${s.server.aoi_encode_ms?.p99}\n` +
      `  down kbps P50 ${s.clients.down_kbps.p50} / P99 ${s.clients.down_kbps.p99} (worst client P50 ${s.clients.down_kbps_p50_worst_client}); up P50 ${s.clients.up_kbps.p50}\n` +
      `  decode_errors ${s.clients.decode_errors}, disconnects ${s.clients.disconnects}, corrections ${s.clients.corrections.total}, kills ${s.server.events?.kills ?? '?'}, respawns ${s.server.events?.spawns ?? '?'}; server RSS growth ${c.server_rss_growth_pct.value ?? 'n/a'}%\n` +
      `  verdict: ${s.verdict.all_ok ? 'ALL OK' : `FAILED ${s.verdict.failed.join(', ')}`} -> ${opts.out}`,
  );
  process.exit(s.verdict.hard_ok ? 0 : 1);
}
