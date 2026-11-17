import { test } from 'node:test';
import assert from 'node:assert/strict';
import { cpSync, mkdtempSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { REPO_ROOT } from '../lib.mjs';

const cli = join(REPO_ROOT, 'tools/data-validate/index.mjs');
const fx = join(REPO_ROOT, 'data/schemas/fixtures');
const DIRS = { weapon: 'weapons', role: 'roles', loadout: 'loadouts', vehicle: 'vehicles' };

function place(root, kind, srcFile, id) {
  const dest = DIRS[kind]
    ? join(root, DIRS[kind], `${id}.json`)
    : join(root, 'maps', 'fixture_map', kind === 'map_meta' ? 'meta.json' : 'spawns.json');
  mkdirSync(join(dest, '..'), { recursive: true });
  const doc = JSON.parse(readFileSync(srcFile, 'utf8'));
  delete doc._expect_error;
  writeFileSync(dest, JSON.stringify(doc, null, 2));
}

function build(invalidName) {
  const root = mkdtempSync(join(tmpdir(), 'dv-'));
  cpSync(join(REPO_ROOT, 'data/schemas'), join(root, 'schemas'), { recursive: true });
  for (const n of readdirSync(join(fx, 'valid'))) {
    const doc = JSON.parse(readFileSync(join(fx, 'valid', n), 'utf8'));
    place(root, n.split('.')[0], join(fx, 'valid', n), doc.id);
  }
  if (invalidName) {
    const doc = JSON.parse(readFileSync(join(fx, 'invalid', invalidName), 'utf8'));
    place(root, invalidName.split('.')[0], join(fx, 'invalid', invalidName), doc.id ?? 'x');
  }
  return root;
}

test('CLI: valid fixture tree exits 0', () => {
  const root = build(null);
  try {
    const r = spawnSync('node', [cli, '--root', root], { encoding: 'utf8' });
    assert.equal(r.status, 0, r.stdout + r.stderr);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

for (const name of readdirSync(join(fx, 'invalid'))) {
  test(`CLI: ${name} exits 1 with expected error`, () => {
    const expect = JSON.parse(readFileSync(join(fx, 'invalid', name), 'utf8'))._expect_error;
    const root = build(name);
    try {
      const r = spawnSync('node', [cli, '--root', root], { encoding: 'utf8' });
      assert.equal(r.status, 1, r.stdout + r.stderr);
      assert.ok(r.stdout.includes(expect), `expected "${expect}" in:\n${r.stdout}`);
      // line format: path:jsonpointer: message
      for (const line of r.stdout.trim().split('\n')) assert.match(line, /^[^:]+:\/[^:]*: .+/);
    } finally { rmSync(root, { recursive: true, force: true }); }
  });
}

test('CLI: broken JSON is reported', () => {
  const root = build(null);
  try {
    writeFileSync(join(root, 'weapons', 'us_fixture_rifle.json'), '{ "id": ');
    const r = spawnSync('node', [cli, '--root', root], { encoding: 'utf8' });
    assert.equal(r.status, 1);
    assert.match(r.stdout, /invalid JSON/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test('CLI: --json output', () => {
  const root = build('weapon.missing_source.json');
  try {
    const r = spawnSync('node', [cli, '--root', root, '--json'], { encoding: 'utf8' });
    assert.equal(r.status, 1);
    const out = JSON.parse(r.stdout);
    assert.equal(out.ok, false);
    assert.ok(out.errors.some((e) => e.message === 'missing: historical._source'));
  } finally { rmSync(root, { recursive: true, force: true }); }
});
