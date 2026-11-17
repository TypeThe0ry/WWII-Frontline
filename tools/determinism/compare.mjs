// Combines native + wasm outputs of M0-029 into reports/m0/determinism/summary.json.
// Usage: node tools/determinism/compare.mjs [--strict] [DIR=target/determinism] [REPORT_DIR=reports/m0/determinism]
//  Blocking (exit 1): native x2 or wasm x2 per-tick hashes differ.
//  native vs wasm: informational by default (R5: ::warning:: only). With --strict (or
//  FRONTLINE_DETERMINISM_STRICT=1) any hash or position difference between native and wasm fails.
//  Drift above 2 cm/s is reported (and a ::warning::) but never fails: write PLAN_ISSUES instead.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { driftMetrics, posErrors } from './stats.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '../..');
const args = process.argv.slice(2);
const strict = args.includes('--strict') || process.env.FRONTLINE_DETERMINISM_STRICT === '1';
const [dirArg, repArg] = args.filter((a) => !a.startsWith('--'));
const dir = resolve(dirArg ?? join(root, 'target/determinism'));
const repDir = resolve(repArg ?? join(root, 'reports/m0/determinism'));
const f32 = (p) => {
  const b = readFileSync(p);
  return new Float32Array(b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength));
};
const u64 = (p) => {
  const b = readFileSync(p);
  return new BigUint64Array(b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength));
};
const tryExec = (cmd, a) => {
  try {
    return execFileSync(cmd, a, { encoding: 'utf8' }).trim();
  } catch {
    return 'unknown';
  }
};

const nmeta = JSON.parse(readFileSync(join(dir, 'native_meta.json'), 'utf8'));
const wasmFile = join(root, 'crates/sim-wasm/pkg/frontline_sim_wasm_bg.wasm');
const seeds = [];
let blocking = false;
let nativeVsWasmDiffers = false;
for (const seed of [1, 2, 3]) {
  const nat = JSON.parse(readFileSync(join(dir, `native_seed${seed}.json`), 'utf8'));
  const was = JSON.parse(readFileSync(join(dir, `wasm_seed${seed}.json`), 'utf8'));
  const nh = u64(join(dir, `native_hash_seed${seed}.bin`));
  const wh = u64(join(dir, `wasm_hash_seed${seed}.bin`));
  const np = f32(join(dir, `native_pos_seed${seed}.f32`));
  const wp = f32(join(dir, `wasm_pos_seed${seed}.f32`));
  let hashFirstDiff = null;
  let hashEqual = 0;
  for (let i = 0; i < nh.length; i++) {
    if (nh[i] === wh[i]) hashEqual++;
    else hashFirstDiff ??= i + 1;
  }
  const err = posErrors(np, wp);
  const drift = driftMetrics(err);
  let posFirstDiff = null;
  for (let i = 0; i < err.length; i++) if (err[i] !== 0) { posFirstDiff = i + 1; break; }
  blocking ||= !nat.native_x2.equal || !was.wasm_x2.equal || (nat.world10 && !nat.world10.equal);
  nativeVsWasmDiffers ||= hashFirstDiff !== null || posFirstDiff !== null;
  seeds.push({
    seed,
    ticks: nat.ticks,
    native_x2: { equal: nat.native_x2.equal, equal_ticks: nat.native_x2.equal_ticks, first_diff_tick: nat.native_x2.first_diff_tick },
    wasm_x2: was.wasm_x2,
    world10_native_x2: nat.world10 && { players: nat.world10.players, ticks: nat.world10.ticks, equal: nat.world10.equal, first_diff_tick: nat.world10.first_diff_tick },
    native_final_hash: nat.final_hash,
    wasm_final_hash: was.final_hash,
    native_vs_wasm: { hash_equal_ticks: hashEqual, hash_first_diff_tick: hashFirstDiff, position_first_diff_tick: posFirstDiff },
    drift: {
      rms_m: drift.rms_m,
      max_m: drift.max_m,
      cm_per_s: drift.cm_per_s,
      over_0_05m_ticks: drift.over_0_05m_ticks,
      over_0_05m_ratio: drift.over_0_05m_ratio,
      segment_max_m_per_60s: drift.segment_max_m,
    },
    coverage: { spawn: nat.spawn, ...nat.coverage },
  });
}

const maxDrift = Math.max(...seeds.map((s) => s.drift.cm_per_s));
const maxOver = Math.max(...seeds.map((s) => s.drift.over_0_05m_ratio));
const summary = {
  task: 'M0-029',
  generated_by: 'tools/determinism/compare.mjs',
  versions: {
    data_hash: nmeta.data_hash,
    protocol_version: nmeta.protocol_version,
    rustc: tryExec('rustc', ['--version']),
    rapier3d: nmeta.rapier3d,
    rapier_features: nmeta.rapier_features,
    wasm_sha256: createHash('sha256').update(readFileSync(wasmFile)).digest('hex'),
    node: process.version,
  },
  mode: strict ? 'strict' : 'warn',
  seeds,
  verdict: {
    native_x2_all_equal: seeds.every((s) => s.native_x2.equal),
    wasm_x2_all_equal: seeds.every((s) => s.wasm_x2.equal),
    native_vs_wasm_bit_identical: !nativeVsWasmDiffers,
    max_drift_cm_per_s: maxDrift,
    drift_target_cm_per_s: 2.0,
    drift_target_met: maxDrift < 2.0,
    max_over_0_05m_ratio: maxOver,
    recommend_review_reconciliation_threshold: maxOver > 0.01,
  },
};
mkdirSync(repDir, { recursive: true });
writeFileSync(join(repDir, 'summary.json'), JSON.stringify(summary, null, 2) + '\n');

for (const s of seeds) {
  console.log(`seed ${s.seed}: native x2 ${s.native_x2.equal_ticks}/${s.ticks}, wasm x2 ${s.wasm_x2.equal_ticks}/${s.ticks}, native vs wasm hash ${s.native_vs_wasm.hash_equal_ticks}/${s.ticks}, rms ${s.drift.rms_m} m, max ${s.drift.max_m} m, ${s.drift.cm_per_s} cm/s, >5cm ${s.drift.over_0_05m_ticks}`);
}
if (blocking) {
  for (const s of seeds) {
    if (!s.native_x2.equal) console.error(`::error::native x2 differs, seed ${s.seed}, first tick ${s.native_x2.first_diff_tick}`);
    if (!s.wasm_x2.equal) console.error(`::error::wasm x2 differs, seed ${s.seed}, first tick ${s.wasm_x2.first_diff_tick}`);
    if (s.world10_native_x2 && !s.world10_native_x2.equal) console.error(`::error::10-player World native x2 differs, seed ${s.seed}, first tick ${s.world10_native_x2.first_diff_tick}`);
  }
  process.exit(1);
}
if (nativeVsWasmDiffers) {
  const first = seeds.map((s) => `seed ${s.seed}: hash@${s.native_vs_wasm.hash_first_diff_tick} pos@${s.native_vs_wasm.position_first_diff_tick}`).join('; ');
  const msg = `native vs wasm differ (${first}); max drift ${maxDrift.toFixed(4)} cm/s${strict ? '' : ' (non-blocking per R5)'}`;
  console.log(`${strict ? '::error::' : '::warning::'}${msg}`);
  if (strict) process.exit(1);
} else {
  console.log('native == wasm: per-tick state hash and position bit-identical for all seeds');
}
if (maxDrift >= 2.0) console.log(`::warning::drift ${maxDrift} cm/s >= 2 cm/s target: add a PLAN_ISSUES entry (informational)`);
