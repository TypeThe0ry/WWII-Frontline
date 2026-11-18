import assert from 'node:assert/strict';
import { test } from 'node:test';
import { auditDoc, problems, toMarkdown } from '../source-audit.mjs';

const doc = (status, src, hist = { caliber: 'x', mass_kg: 1 }) => ({ id: 'a', _meta: { status }, historical: { ...hist, _source: src } });

test('field without a located record is a gap', () => {
  const a = auditDoc('w/a.json', doc('initial', '待复核'));
  assert.deepEqual(a.rows.map((r) => r.state), ['gap', 'gap']);
});

test('record with locator sources its field only', () => {
  const a = auditDoc('w/a.json', doc('initial', [{ field: 'caliber', source_id: 'TM', locator: 'p.3' }, { field: 'mass_kg', source_id: 'TM', locator: ' ' }]));
  assert.deepEqual(a.rows.map((r) => r.state), ['sourced', 'gap']);
});

test('reviewed file with a gap is a problem; initial is not', () => {
  assert.equal(problems([auditDoc('a', doc('reviewed', []))]).length, 1);
  assert.equal(problems([auditDoc('a', doc('initial', []))]).length, 0);
});

test('markdown counts gaps and has no long quotes', () => {
  const md = toMarkdown([auditDoc('w/a.json', doc('initial', []))]);
  assert.match(md, /字段 2 个，已有出处 0，未核实 2/);
});
