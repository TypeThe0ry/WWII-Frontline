import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { REPO_ROOT, checkFile, discover, loadValidators, runFixtures, validateRoot } from '../lib.mjs';

const root = join(REPO_ROOT, 'data');

test('fixtures: all valid pass, every invalid fails with its _expect_error', () => {
  const r = runFixtures(root);
  assert.deepEqual(r.problems, []);
  assert.ok(r.valid >= 6);
  assert.ok(r.invalid >= 10, `need >= 10 invalid fixtures, have ${r.invalid}`);
});

test('real /data validates clean', () => {
  const r = validateRoot(root);
  assert.deepEqual(r.errors, []);
});

test('invalid fixtures cover many rule families', () => {
  const names = readdirSync(join(root, 'schemas/fixtures/invalid'));
  for (const kind of ['weapon', 'role', 'loadout', 'vehicle', 'map_meta', 'spawns']) {
    assert.ok(names.some((n) => n.startsWith(`${kind}.`)), `no invalid fixture for ${kind}`);
  }
});


test('comms config is discovered and validates R3 grid, intervals and limits', () => {
  assert.ok(discover(root).files.some(({ file, kind }) => file === 'comms.json' && kind === 'comms'));
  const validators = loadValidators(root);
  const doc = JSON.parse(readFileSync(join(root, 'comms.json'), 'utf8'));
  assert.deepEqual(checkFile('comms', doc, validators), []);
  doc.enemy_cell_m = 1;
  doc.enemy_refresh_s = 0;
  doc.ping.max_active_markers = 256;
  const errors = checkFile('comms', doc, validators);
  for (const pointer of ['/enemy_cell_m', '/enemy_refresh_s', '/ping/max_active_markers']) {
    assert.ok(errors.some((e) => e.pointer === pointer), `missing error for ${pointer}`);
  }
});
