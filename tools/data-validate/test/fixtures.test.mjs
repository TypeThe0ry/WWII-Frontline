import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync } from 'node:fs';
import { join } from 'node:path';
import { REPO_ROOT, runFixtures, validateRoot } from '../lib.mjs';

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
