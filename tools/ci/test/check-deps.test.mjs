import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { checkMetadata } from '../check-deps.mjs';
import { ALLOWED, COLUMNS, CRATES } from '../dep-matrix.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const fx = (n) => join(here, '..', 'fixtures', n);
const run = (n) => spawnSync(process.execPath, [join(here, '..', 'check-deps.mjs'), '--metadata', fx(n)], { encoding: 'utf8' });

test('allowed edges pass (exit 0)', () => {
  const r = run('metadata-ok.json');
  assert.equal(r.status, 0, r.stderr);
});

test('sim -> tokio is rejected and named in the log', () => {
  const r = run('metadata-sim-tokio.json');
  assert.equal(r.status, 1);
  assert.match(r.stderr, /sim -> tokio/);
  assert.match(r.stderr, /crates\/sim\/Cargo\.toml/);
});

test('protocol -> sim and gateway -> sim are rejected', () => {
  const r = run('metadata-forbidden-edges.json');
  assert.equal(r.status, 1);
  assert.match(r.stderr, /protocol -> sim/);
  assert.match(r.stderr, /gateway -> sim/);
});

test('transitive tokio below sim is rejected with the path', () => {
  const r = run('metadata-transitive-tokio.json');
  assert.equal(r.status, 1);
  assert.match(r.stderr, /sim -> tokio \(transitive via sim -> foo -> tokio\)/);
});

test('missing metadata file exits 1', () => {
  const r = run('does-not-exist.json');
  assert.equal(r.status, 1);
});

test('dev-dependency edges are checked too', () => {
  const meta = JSON.parse(readFileSync(fx('metadata-ok.json'), 'utf8'));
  const sim = meta.packages.find((p) => p.name === 'frontline-sim');
  sim.dependencies.push({ name: 'frontline-server', kind: 'dev', path: '/fixture/crates/server' });
  const v = checkMetadata(meta);
  assert.equal(v.length, 1);
  assert.match(v[0], /sim -> server \(dev-dependencies\)/);
});

test('matrix is the §4.5 table (8 crates, 9 columns, spot checks)', () => {
  assert.equal(CRATES.length, 8);
  assert.equal(COLUMNS.length, 9);
  assert.deepEqual(Object.keys(ALLOWED).sort(), [...CRATES].sort());
  assert.deepEqual(ALLOWED.server, ['protocol', 'sim', 'bots']);
  assert.deepEqual(ALLOWED.protocol, []);
  assert.ok(!ALLOWED.gateway.includes('sim'));
  assert.ok(Object.values(ALLOWED).every((t) => !t.includes('client')));
});
