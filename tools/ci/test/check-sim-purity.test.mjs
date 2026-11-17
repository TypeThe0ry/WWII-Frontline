import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, copyFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const script = join(here, '..', 'check-sim-purity.mjs');
const fx = (n) => join(here, '..', 'fixtures', n);

function withDir(files, fn) {
  const d = mkdtempSync(join(tmpdir(), 'purity-'));
  try {
    mkdirSync(join(d, 'sub'));
    files.forEach(([src, dst]) => copyFileSync(fx(src), join(d, dst)));
    return fn(d);
  } finally {
    rmSync(d, { recursive: true, force: true });
  }
}
const run = (d) => spawnSync(process.execPath, [script, '--dir', d], { encoding: 'utf8' });

test('violating source fails with file:line', () => {
  withDir([['sim-impure.rs', 'sub/lib.rs']], (d) => {
    const r = run(d);
    assert.equal(r.status, 1);
    assert.match(r.stderr, /lib\.rs:1: HashMap/);
    assert.match(r.stderr, /Instant/);
    assert.match(r.stderr, /thread_rng/);
  });
});

test('comments alone do not trip the scan (line 4 of impure fixture)', () => {
  withDir([['sim-impure.rs', 'lib.rs']], (d) => {
    const r = run(d);
    assert.doesNotMatch(r.stderr, /lib\.rs:4:/);
  });
});

test('purity-allow waives the line and lists it', () => {
  withDir([['sim-allowed.rs', 'lib.rs']], (d) => {
    const r = run(d);
    assert.equal(r.status, 0, r.stderr);
    assert.match(r.stdout, /purity-allow .*lib\.rs:1: HashMap -- lookup only/);
  });
});

test('current crates/sim/src is clean', () => {
  const r = spawnSync(process.execPath, [script], { encoding: 'utf8' });
  assert.equal(r.status, 0, r.stderr);
});
