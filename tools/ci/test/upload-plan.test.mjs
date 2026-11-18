import assert from 'node:assert/strict';
import { test } from 'node:test';
import { buildPlan, planEntry, IMMUTABLE, NO_CACHE, sha256 } from '../../deploy/upload-r2.mjs';

const f = (rel, s) => ({ rel, buf: Buffer.from(s) });

test('.fpak is renamed by content hash and immutable', () => {
  const e = planEntry('chr_us.fpak', Buffer.from('abc'));
  assert.equal(e.cache, IMMUTABLE);
  assert.equal(e.key, `chr_us.${sha256(Buffer.from('abc')).slice(0, 16)}.fpak`);
});

test('same content → same key, different content → different key', () => {
  assert.equal(planEntry('a.fpak', Buffer.from('x')).key, planEntry('a.fpak', Buffer.from('x')).key);
  assert.notEqual(planEntry('a.fpak', Buffer.from('x')).key, planEntry('a.fpak', Buffer.from('y')).key);
});

test('manifest and entry html are no-cache; fingerprinted bundles are immutable', () => {
  assert.equal(planEntry('manifest.json', Buffer.from('{}')).cache, NO_CACHE);
  assert.equal(planEntry('index.html', Buffer.from('<html>')).cache, NO_CACHE);
  assert.equal(planEntry('assets/index-D08udEWn.js', Buffer.from('x')).cache, IMMUTABLE);
});

test('standalone glb/ktx2/ogg are rejected (R11)', () => {
  for (const ext of ['glb', 'ktx2', 'ogg', 'opus']) {
    const p = buildPlan([f(`x.${ext}`, 'a')]);
    assert.equal(p.problems.length, 1, ext);
  }
});

test('budget overflow is reported; within budget is clean', () => {
  assert.equal(buildPlan([f('a.fpak', '12345')], 4).problems.length, 1);
  assert.deepEqual(buildPlan([f('a.fpak', '12345')], 5).problems, []);
});

test('totals add up and duplicates are caught', () => {
  const p = buildPlan([f('a.fpak', 'aa'), f('b.json', 'bbb')]);
  assert.equal(p.totalBytes, 5);
  const d = buildPlan([f('x/index.html', '1'), f('x/index.html', '1')]);
  assert.ok(d.problems.some((m) => m.startsWith('duplicate key')));
});
