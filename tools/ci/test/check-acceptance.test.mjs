import assert from 'node:assert/strict';
import { test } from 'node:test';
import { checkAcceptance, openTasks, statusRows } from '../check-acceptance.mjs';

const row = (n, v = '未测', mid = 'x') => `| E1-${n} | std | tgt | ${mid} | cmd | ${v} | PI-1 |`;
const full = Array.from({ length: 17 }, (_, i) => row(i + 1)).join('\n');

test('17 complete rows pass', () => assert.deepEqual(checkAcceptance(full), []));
test('missing row, empty cell and bad verdict are reported', () => {
  const md = [...Array.from({ length: 16 }, (_, i) => row(i + 1)), row(17, 'maybe', '')].join('\n');
  const e = checkAcceptance(md.replace('E1-16', 'E1-99'));
  assert.ok(e.some((x) => x.includes('E1-16: row missing')));
  assert.ok(e.some((x) => x.includes("empty cell '实测'")));
  assert.ok(e.some((x) => x.includes("verdict 'maybe'")));
});
test('statusRows / openTasks', () => {
  const md = '| M1-001 | a | done | | | |\n| M1-002 | b | partial（x） | | | |\n| M1-003 | c | todo | | | |\n| M1-066 | d | todo | | | |';
  const rows = statusRows(md);
  assert.deepEqual(rows.map((r) => r.status), ['done', 'partial', 'todo', 'todo']);
  assert.deepEqual(openTasks(rows, ['M1-066']), ['M1-003']);
});
