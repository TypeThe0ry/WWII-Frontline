import assert from 'node:assert/strict';
import { test } from 'node:test';
import { sizes, verdict } from '../check-wasm-size.mjs';

test('sizes reports raw, gzip and brotli of a buffer', () => {
  const s = sizes(Buffer.alloc(100_000, 7));
  assert.equal(s.bytes, 100_000);
  assert.ok(s.gzip < 1000 && s.brotli < 200);
});

test('verdict flags both limits independently', () => {
  assert.deepEqual(verdict({ bytes: 1_000_000, gzip: 1, brotli: 300_000 }), []);
  assert.equal(verdict({ bytes: 1_200_001, gzip: 1, brotli: 1 }).length, 1);
  assert.equal(verdict({ bytes: 1, gzip: 1, brotli: 400_001 }).length, 1);
  assert.equal(verdict({ bytes: 2_000_000, gzip: 1, brotli: 500_000 }).length, 2);
});
