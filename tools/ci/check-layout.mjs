#!/usr/bin/env node
// Verifies the repository layout required by ARCHITECTURE §4.1.
// assets/incoming and assets/processed are gitignored (created manually / by the
// pipeline), so only their parent `assets/` is required.
import { existsSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');

const dirs = [
  'client',
  'client/src/render',
  'client/src/ui',
  'client/src/audio',
  'client/src/input',
  'client/src/net',
  'client/src/game',
  'crates',
  ...[
    'protocol',
    'sim',
    'sim-wasm',
    'server',
    'gateway',
    'bots',
    'headless-client',
    'balance-sim',
  ].map((c) => `crates/${c}`),
  'tools',
  'tools/asset-pipeline',
  'tools/map-pipeline',
  'tools/ci',
  'data',
  'assets',
  'docs',
  'tasks',
];
const files = ['LICENSES.md', 'Cargo.toml', 'rust-toolchain.toml', 'pnpm-workspace.yaml'];

const missing = [];
for (const d of dirs) {
  const p = join(root, d);
  if (!existsSync(p) || !statSync(p).isDirectory()) missing.push(`${d}/`);
}
for (const f of files) {
  if (!existsSync(join(root, f))) missing.push(f);
}

if (missing.length > 0) {
  console.error('Layout check FAILED. Missing:');
  for (const m of missing) console.error(`  - ${m}`);
  process.exit(1);
}
console.log(`Layout check OK (${dirs.length} directories, ${files.length} files).`);
