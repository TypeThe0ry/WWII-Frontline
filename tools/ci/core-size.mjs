#!/usr/bin/env node
// core package gate (M1-054, ARCHITECTURE §11.4): brotli(JS+WASM+CSS+HTML) of client/dist must be <= 8 MiB.
// `dist/data` (map/game data) is reported separately and is not part of the core shell.
// Usage: node tools/ci/core-size.mjs [--dist client/dist] [--max 8388608] [--report path.json]
import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { brotliCompressSync, constants } from 'node:zlib';
import { pathToFileURL } from 'node:url';

export const MAX_CORE = 8_388_608;
const CORE_EXT = new Set(['.js', '.mjs', '.wasm', '.css', '.html']);

export function walk(dir) {
  const out = [];
  for (const e of readdirSync(dir)) {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) out.push(...walk(p));
    else out.push(p);
  }
  return out;
}

export const br = (buf) => brotliCompressSync(buf, { params: { [constants.BROTLI_PARAM_QUALITY]: 11, [constants.BROTLI_PARAM_SIZE_HINT]: buf.length } }).length;

export function measure(dist) {
  let raw = 0, brotli = 0, files = 0, dataBytes = 0;
  for (const f of walk(dist)) {
    const rel = relative(dist, f).replaceAll('\\', '/');
    const ext = rel.slice(rel.lastIndexOf('.'));
    const buf = readFileSync(f);
    if (rel.startsWith('data/')) dataBytes += buf.length;
    else if (CORE_EXT.has(ext)) {
      raw += buf.length;
      brotli += br(buf);
      files++;
    }
  }
  return { files, rawBytes: raw, brotliBytes: brotli, dataBytes };
}

export function verdict(m, max = MAX_CORE) {
  return m.brotliBytes > max ? [`core brotli ${m.brotliBytes} B > ${max} B`] : [];
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const a = process.argv.slice(2);
  const opt = (n, d) => (a.includes(n) ? a[a.indexOf(n) + 1] : d);
  const dist = resolve(opt('--dist', 'client/dist'));
  const m = measure(dist);
  const problems = verdict(m, Number(opt('--max', MAX_CORE)));
  const rep = opt('--report');
  if (rep) {
    mkdirSync(dirname(resolve(rep)), { recursive: true });
    writeFileSync(rep, JSON.stringify({ ...m, max: MAX_CORE, pass: problems.length === 0 }, null, 2));
  }
  console.log(`core: ${m.files} files, raw ${m.rawBytes} B, brotli ${m.brotliBytes} B (max ${MAX_CORE}); data ${m.dataBytes} B`);
  for (const p of problems) console.error(p);
  process.exit(problems.length ? 1 : 0);
}
