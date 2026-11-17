#!/usr/bin/env node
// sim-wasm size gate (M0-014, ARCHITECTURE §4.3.3 / E0-9): runs `wasm-opt -O3` over the
// wasm-pack output (in place, so what is measured is what gets shipped) and fails when the
// optimised .wasm exceeds 1.2 MB or its brotli (q11) size exceeds 400 KB. gzip is reported too.
// Usage: node tools/ci/check-wasm-size.mjs [--pkg crates/sim-wasm/pkg] [--max-bytes 1200000]
//        [--max-br 400000] [--no-opt] [--report path.json]
// Environment: WASM_OPT = path of the wasm-opt binary (default: `wasm-opt` from PATH).
// Pinned toolchain (see DEVELOPING.md): wasm-bindgen 0.2.130, binaryen version_132.
import { execFileSync } from 'node:child_process';
import { mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { brotliCompressSync, constants, gzipSync } from 'node:zlib';

export const MAX_BYTES = 1_200_000;
export const MAX_BROTLI = 400_000;

export function sizes(buf) {
  return {
    bytes: buf.length,
    gzip: gzipSync(buf, { level: 9 }).length,
    brotli: brotliCompressSync(buf, { params: { [constants.BROTLI_PARAM_QUALITY]: 11, [constants.BROTLI_PARAM_SIZE_HINT]: buf.length } }).length,
  };
}

export function verdict(s, maxBytes = MAX_BYTES, maxBr = MAX_BROTLI) {
  const problems = [];
  if (s.bytes > maxBytes) problems.push(`wasm ${s.bytes} B > ${maxBytes} B`);
  if (s.brotli > maxBr) problems.push(`brotli ${s.brotli} B > ${maxBr} B`);
  return problems;
}

function parseArgs(argv) {
  const o = { pkg: 'crates/sim-wasm/pkg', max: MAX_BYTES, maxBr: MAX_BROTLI, opt: true, report: null };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--pkg') o.pkg = argv[++i];
    else if (a === '--max-bytes') o.max = Number(argv[++i]);
    else if (a === '--max-br') o.maxBr = Number(argv[++i]);
    else if (a === '--no-opt') o.opt = false;
    else if (a === '--report') o.report = argv[++i];
    else throw new Error(`unknown argument ${a}`);
  }
  return o;
}

function main() {
  const o = parseArgs(process.argv.slice(2));
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  const dir = resolve(root, o.pkg);
  const wasmFiles = readdirSync(dir).filter((f) => f.endsWith('.wasm'));
  if (wasmFiles.length === 0) {
    console.error(`check-wasm-size: no .wasm in ${dir} (run wasm-pack build first)`);
    process.exit(2);
  }
  let failed = false;
  const report = {};
  for (const f of wasmFiles) {
    const p = join(dir, f);
    const raw = readFileSync(p);
    let opt = raw;
    let tool = 'none';
    if (o.opt) {
      const bin = process.env.WASM_OPT || 'wasm-opt';
      tool = execFileSync(bin, ['--version'], { encoding: 'utf8' }).trim();
      execFileSync(bin, ['-O3', p, '-o', p], { stdio: 'inherit' });
      opt = readFileSync(p);
    }
    const s = sizes(opt);
    const problems = verdict(s, o.max, o.maxBr);
    report[f] = { raw_bytes: raw.length, optimised_bytes: s.bytes, gzip_bytes: s.gzip, brotli_bytes: s.brotli, wasm_opt: tool, limits: { bytes: o.max, brotli: o.maxBr }, ok: problems.length === 0 };
    console.log(`${f}: raw ${raw.length} B, wasm-opt -O3 ${s.bytes} B (limit ${o.max}), gzip ${s.gzip} B, brotli-11 ${s.brotli} B (limit ${o.maxBr})  [${tool}]`);
    for (const pr of problems) console.error(`FAIL ${f}: ${pr}`);
    if (problems.length) failed = true;
  }
  if (o.report) {
    mkdirSync(dirname(resolve(root, o.report)), { recursive: true });
    writeFileSync(resolve(root, o.report), `${JSON.stringify(report, null, 2)}\n`);
  }
  process.exit(failed ? 1 : 0);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) main();
