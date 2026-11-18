import assert from 'node:assert/strict';
import test from 'node:test';
import { compareImages, deltaE76, srgbToLab } from './delta-e.mjs';

test('Lab of white and black', () => {
  const w = srgbToLab(255, 255, 255);
  assert.ok(Math.abs(w[0] - 100) < 0.01 && Math.abs(w[1]) < 0.01 && Math.abs(w[2]) < 0.01);
  assert.ok(Math.abs(srgbToLab(0, 0, 0)[0]) < 1e-9);
});

test('deltaE76 is a distance; identical images give 0, one off pixel shows up', () => {
  assert.equal(deltaE76([1, 2, 3], [1, 2, 3]), 0);
  assert.equal(deltaE76([0, 0, 0], [3, 4, 0]), 5);
  const img = (v) => ({ w: 2, h: 1, rgba: Buffer.from([v, v, v, 255, 10, 10, 10, 255]) });
  const same = compareImages(img(10), img(10));
  assert.equal(same.max, 0);
  const diff = compareImages(img(10), img(60));
  assert.ok(diff.max > 10 && diff.over5 === 0.5);
});
