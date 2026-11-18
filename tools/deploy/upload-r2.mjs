#!/usr/bin/env node
// Staging asset upload plan (M1-067a, ARCHITECTURE §11.6): content-hash names, immutable cache headers for `.fpak`,
// `no-cache` for the manifest and entry HTML, size budgets re-checked. Dry run only until credentials exist (H-015):
// the S3-compatible PUT is intentionally not implemented here — the plan is what the real uploader will execute.
// Usage: node tools/deploy/upload-r2.mjs --dir <upload_dir> [--budget-bytes N] [--json]
import { createHash } from 'node:crypto';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { extname, join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

export const IMMUTABLE = 'public, max-age=31536000, immutable';
export const NO_CACHE = 'no-cache';
const FORBIDDEN_EXT = new Set(['.glb', '.gltf', '.ktx2', '.ogg', '.opus', '.wav']); // R11: only .fpak is shipped standalone

export const sha256 = (buf) => createHash('sha256').update(buf).digest('hex');

export function walk(dir) {
  const out = [];
  for (const e of readdirSync(dir)) {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) out.push(...walk(p));
    else out.push(p);
  }
  return out.sort();
}

/** One entry per file: key, content type class, cache header, bytes. `.fpak` and fingerprinted assets are renamed by hash. */
export function planEntry(rel, buf) {
  const ext = extname(rel);
  const hash = sha256(buf).slice(0, 16);
  if (ext === '.fpak') {
    const base = rel.slice(0, -ext.length);
    return { source: rel, key: `${base}.${hash}${ext}`, cache: IMMUTABLE, bytes: buf.length, hash };
  }
  if (/\.[0-9a-f]{8,}\.[a-z0-9]+$/i.test(rel) || /-[A-Za-z0-9_-]{8}\.(js|css|woff2)$/.test(rel)) {
    return { source: rel, key: rel, cache: IMMUTABLE, bytes: buf.length, hash }; // bundler-fingerprinted
  }
  return { source: rel, key: rel, cache: NO_CACHE, bytes: buf.length, hash }; // manifest, index.html, fonts.css, …
}

export function buildPlan(files, budgetBytes) {
  const problems = [];
  const entries = [];
  for (const { rel, buf } of files) {
    if (FORBIDDEN_EXT.has(extname(rel).toLowerCase())) problems.push(`standalone ${extname(rel)} not allowed: ${rel}`);
    entries.push(planEntry(rel, buf));
  }
  const total = entries.reduce((a, e) => a + e.bytes, 0);
  if (budgetBytes !== undefined && total > budgetBytes) problems.push(`total ${total} B > budget ${budgetBytes} B`);
  const seen = new Map();
  for (const e of entries) {
    if (seen.has(e.key)) problems.push(`duplicate key ${e.key}`);
    seen.set(e.key, e.source);
  }
  return { entries, totalBytes: total, problems };
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const a = process.argv.slice(2);
  const opt = (n) => (a.includes(n) ? a[a.indexOf(n) + 1] : undefined);
  const dir = opt('--dir');
  if (!dir) {
    console.error('usage: upload-r2.mjs --dir <upload_dir> [--budget-bytes N] [--json]');
    process.exit(2);
  }
  const files = walk(dir).map((p) => ({ rel: relative(dir, p).replaceAll('\\', '/'), buf: readFileSync(p) }));
  const plan = buildPlan(files, opt('--budget-bytes') ? Number(opt('--budget-bytes')) : undefined);
  if (a.includes('--json')) console.log(JSON.stringify(plan, null, 2));
  else {
    for (const e of plan.entries) console.log(`${e.cache === IMMUTABLE ? 'immutable' : 'no-cache '}  ${String(e.bytes).padStart(10)}  ${e.key}`);
    console.log(`DRY RUN: ${plan.entries.length} objects, ${plan.totalBytes} B (nothing uploaded)`);
  }
  for (const p of plan.problems) console.error(p);
  process.exit(plan.problems.length ? 1 : 0);
}
