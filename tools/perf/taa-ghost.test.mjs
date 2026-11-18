import assert from 'node:assert/strict';
import test from 'node:test';
import { meanDL } from './taa-ghost.mjs';

const img = (rows) => ({ w: rows[0].length, h: rows.length, rgba: Buffer.from(rows.flat().flatMap((v) => [v, v, v, 255])) });

test('meanDL averages |dL| over the selected pixels only', () => {
  const a = img([[0, 0], [0, 0]]);
  const b = img([[0, 100], [0, 0]]);
  assert.equal(meanDL(a, a, () => true), 0);
  const all = meanDL(a, b, () => true);
  const one = meanDL(a, b, (x, y) => x === 1 && y === 0);
  assert.ok(one > 20 && Math.abs(one - all * 4) < 1e-6);
  assert.equal(meanDL(a, b, () => false), 0);
});
