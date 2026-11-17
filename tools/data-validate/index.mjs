#!/usr/bin/env node
// CLI: node tools/data-validate/index.mjs [--root data] [--json] [--fixtures]
// One error per line: `path:jsonpointer: message`; exit 1 on any error.
import { resolve } from 'node:path';
import { REPO_ROOT, runFixtures, validateRoot } from './lib.mjs';

const args = process.argv.slice(2);
const flag = (n) => args.includes(n);
const rootIdx = args.indexOf('--root');
const root = resolve(process.cwd(), rootIdx >= 0 ? args[rootIdx + 1] : resolve(REPO_ROOT, 'data'));
const asJson = flag('--json');

let errors = [];
let summary = '';
if (flag('--fixtures')) {
  const r = runFixtures(root);
  errors = r.problems;
  summary = `fixtures: ${r.valid} valid, ${r.invalid} invalid`;
} else {
  const r = validateRoot(root);
  errors = r.errors;
  summary = `checked ${r.checked} file(s), ${r.unchecked.length} without schema (hash only)`;
}

if (asJson) {
  console.log(JSON.stringify({ ok: errors.length === 0, summary, errors }, null, 2));
} else {
  for (const e of errors) console.log(`${e.file}:${e.pointer || '/'}: ${e.message}`);
  console.error(`${summary}; ${errors.length} error(s)`);
}
process.exit(errors.length ? 1 : 0);
