#!/usr/bin/env node
// M0-028 acceptance 5: reports/m0/browser-matrix.json and the table pasted into tasks/STATUS.md (M0-028 row) must agree.
// Regenerates the table from the JSON with the same function the spec uses and requires every line to occur in STATUS.md.
// Needs Node >= 22.6 (--experimental-strip-types), no dependencies.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const { renderTable } = await import(`${root}client/e2e/interop/fixtures.ts`);
const matrix = JSON.parse(readFileSync(`${root}reports/m0/browser-matrix.json`, 'utf8'));
const table = renderTable(matrix).split('\n');
const md = readFileSync(`${root}reports/m0/browser-matrix.md`, 'utf8').trim().split('\n');
const status = readFileSync(`${root}tasks/STATUS.md`, 'utf8');
const row = status.split('\n').find((l) => l.startsWith('| M0-028 '));
if (!row) throw new Error('no M0-028 row in tasks/STATUS.md');
let bad = 0;
if (md.join('\n') !== table.join('\n')) (console.error('browser-matrix.md differs from browser-matrix.json'), bad++);
// STATUS.md keeps one row per task, so the table lines are flattened into it: compare cell by cell.
for (const line of table.slice(2)) {
  const cells = line.split('|').slice(1, -1).map((c) => c.trim());
  for (const c of cells) if (!row.includes(c)) (console.error(`STATUS.md M0-028 row lacks cell: ${c}`), bad++);
}
if (bad) process.exit(1);
console.log('browser-matrix.json, browser-matrix.md and the STATUS.md M0-028 row agree');
